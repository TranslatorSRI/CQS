#![feature(get_many_mut)]
#[macro_use]
extern crate rocket;
#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;
#[macro_use]
extern crate log;
#[macro_use]
extern crate lazy_static;

use crate::model::{JobStatus, NewJob};
use chrono::Utc;
use dotenvy::dotenv;
use futures::future::join_all;
use itertools::Itertools;
use merge_hashmap::Merge;
use reqwest::StatusCode;
use rocket::response::status;
use rocket::serde::json::Json;
use rocket::{tokio, Build, Rocket, State};
use rocket_okapi::okapi::openapi3::*;
use rocket_okapi::{mount_endpoints_and_merged_docs, openapi, openapi_get_routes_spec, swagger_ui::*};
use std::path::Path;
use std::time::Duration;
use std::{env, error, fs};
use trapi_model_rs::{AsyncQuery, AsyncQueryResponse, AsyncQueryStatusResponse, KnowledgeType, Query, CURIE};

mod db;
mod job_actions;
mod model;
mod openapi;
mod schema;
mod util;

#[openapi]
#[post("/asyncquery", data = "<data>")]
async fn asyncquery(data: Json<AsyncQuery>) -> Result<Json<AsyncQueryResponse>, status::BadRequest<String>> {
    let query: AsyncQuery = data.into_inner();

    if let Some(query_graph) = &query.message.query_graph {
        if let Some((_edge_key, _edge_value)) = &query_graph.edges.iter().find(|(_k, v)| {
            if let (Some(predicates), Some(knowledge_type)) = (&v.predicates, &v.knowledge_type) {
                if predicates.contains(&"biolink:treats".to_string()) && knowledge_type == &KnowledgeType::INFERRED {
                    return true;
                }
            }
            return false;
        }) {
            let job = NewJob::new(JobStatus::Queued, serde_json::to_string(&query).unwrap().into_bytes());
            let job_id = job_actions::insert(&job).expect("did not insert");
            let ret = AsyncQueryResponse::new(job_id.to_string());
            return Ok(Json(ret));
        }
    }
    return Err(status::BadRequest(Some("Not a valid query".to_string())));
}

#[openapi]
#[get("/asyncquery_status/<job_id>")]
async fn asyncquery_status(job_id: i32) -> Result<Json<AsyncQueryStatusResponse>, status::BadRequest<String>> {
    if let Ok(job_result) = job_actions::find_by_id(job_id) {
        if let Some(job) = job_result {
            if let Some(job_response) = job.response {
                let response: trapi_model_rs::Response = serde_json::from_str(&*String::from_utf8_lossy(job_response.as_slice())).unwrap();
                if let Some(logs) = response.logs {
                    let response_url = format!("{}/download/{}", env::var("RESPONSE_URL_ROOT").unwrap_or("http://localhost".to_string()), job.id);
                    let status_response = AsyncQueryStatusResponse {
                        status: job.status.to_string(),
                        description: job.status.to_string(),
                        logs,
                        response_url: Some(response_url),
                    };
                    return Ok(Json(status_response));
                }
            }
        }
    }
    return Err(status::BadRequest(Some("Not a valid query".to_string())));
}

#[openapi]
#[post("/query", data = "<data>")]
async fn query(data: Json<Query>, reqwest_client: &State<reqwest::Client>) -> Json<trapi_model_rs::Response> {
    let mut query: Query = data.into_inner();
    let mut responses: Vec<trapi_model_rs::Response> = vec![];

    if let Some(query_graph) = &query.message.query_graph {
        if let Some((_edge_key, edge_value)) = &query_graph.edges.iter().find(|(_k, v)| {
            if let (Some(predicates), Some(knowledge_type)) = (&v.predicates, &v.knowledge_type) {
                if predicates.contains(&"biolink:treats".to_string()) && knowledge_type == &KnowledgeType::INFERRED {
                    return true;
                }
            }
            return false;
        }) {
            if let Some((_node_key, node_value)) = &query_graph.nodes.iter().find(|(k, _v)| *k == &edge_value.object) {
                if let Some(ids) = &node_value.ids {
                    let canned_queries = util::get_canned_queries(ids);
                    let future_responses: Vec<_> = canned_queries
                        .iter()
                        .map(|canned_query| util::post_query_to_workflow_runner(&reqwest_client, &canned_query))
                        .collect();
                    let joined_future_responses: Vec<Result<trapi_model_rs::Response, Box<dyn error::Error + Send + Sync>>> = join_all(future_responses).await;
                    joined_future_responses.into_iter().for_each(|result| match result {
                        Ok(trapi_response) => responses.push(trapi_response),
                        Err(e) => warn!("{}", e),
                    });
                }
            }
        }
    }

    let mut message = query.message.clone();

    responses.into_iter().for_each(|r| {
        message.merge(r.message);
    });

    let node_binding_to_log_odds_map = util::build_node_binding_to_log_odds_data_map(&message.knowledge_graph);

    let mut ret = trapi_model_rs::Response::new(util::add_composite_score_attributes(message, node_binding_to_log_odds_map));
    ret.status = Some("Success".to_string());
    ret.workflow = query.workflow.clone();
    ret.biolink_version = Some(env::var("BIOLINK_VERSION").unwrap_or("3.1.2".to_string()));
    ret.schema_version = Some(env::var("SCHEMA_VERSION").unwrap_or("1.4.0".to_string()));

    Json(ret)
}

#[openapi]
#[get("/download/<job_id>")]
async fn download(job_id: i32) -> Result<Json<trapi_model_rs::Response>, status::BadRequest<String>> {
    if let Ok(job_result) = job_actions::find_by_id(job_id) {
        if let Some(job) = job_result {
            if let Some(job_response) = job.response {
                let response: trapi_model_rs::Response = serde_json::from_str(&*String::from_utf8_lossy(job_response.as_slice())).unwrap();
                return Ok(Json(response));
            }
        }
    }
    return Err(status::BadRequest(Some("Job not found".to_string())));
}

#[rocket::main]
async fn main() {
    dotenv().ok();

    env_logger::init();

    db::init_db().expect("failed to initialize the db");

    let start = tokio::time::Instant::now() + Duration::from_secs(10);
    tokio::task::spawn(async move {
        let mut interval_timer = tokio::time::interval_at(start, chrono::Duration::seconds(30).to_std().unwrap());
        loop {
            // Wait for the next interval tick
            interval_timer.tick().await;
            process_asyncqueries().await;
        }
    });

    let launch_result = create_server().launch().await;
    match launch_result {
        Ok(_) => info!("Rocket shut down gracefully."),
        Err(err) => warn!("Rocket had an error: {}", err),
    };
}

async fn process_asyncqueries() {
    info!("processing asyncquery jobs");
    let reqwest_client = util::build_http_client();

    if let Ok(mut undone_jobs) = job_actions::find_undone() {
        let a_job_is_running = undone_jobs.iter().any(|a| a.status == JobStatus::Running);
        match a_job_is_running {
            true => {
                info!("A job is currently running.");
            }
            false => {
                if let Some(job) = undone_jobs.iter_mut().next() {
                    // info!("Sending Job: {} to WFR: {}", job.id, workflow_runner_url);

                    job.date_started = Some(Utc::now().naive_utc());
                    job.status = JobStatus::Running;
                    job_actions::update(job).expect(format!("Could not update Job: {}", job.id).as_str());

                    let query: AsyncQuery = serde_json::from_str(&*String::from_utf8_lossy(job.query.as_slice())).unwrap();

                    let mut responses: Vec<trapi_model_rs::Response> = vec![];

                    if let Some(query_graph) = &query.message.query_graph {
                        if let Some((_edge_key, edge_value)) = &query_graph.edges.iter().find(|(_k, v)| {
                            if let (Some(predicates), Some(knowledge_type)) = (&v.predicates, &v.knowledge_type) {
                                if predicates.contains(&"biolink:treats".to_string()) && knowledge_type == &KnowledgeType::INFERRED {
                                    return true;
                                }
                            }
                            return false;
                        }) {
                            if let Some((_node_key, node_value)) = &query_graph.nodes.iter().find(|(k, _v)| *k == &edge_value.object) {
                                if let Some(ids) = &node_value.ids {
                                    let canned_queries = util::get_canned_queries(ids);
                                    let future_responses: Vec<_> = canned_queries
                                        .iter()
                                        .map(|canned_query| util::post_query_to_workflow_runner(&reqwest_client, &canned_query))
                                        .collect();
                                    let joined_future_responses: Vec<Result<trapi_model_rs::Response, Box<dyn error::Error + Send + Sync>>> = join_all(future_responses).await;
                                    joined_future_responses.into_iter().for_each(|result| match result {
                                        Ok(trapi_response) => responses.push(trapi_response),
                                        Err(e) => warn!("{}", e),
                                    });
                                }
                            }
                        }
                    }

                    if responses.is_empty() {
                        job.date_finished = Some(Utc::now().naive_utc());
                        job.status = JobStatus::Failed;
                        job_actions::update(job).expect(format!("Could not update Job: {}", job.id).as_str());
                    } else {
                        let mut message = query.message.clone();

                        responses.into_iter().for_each(|r| {
                            message.merge(r.message);
                        });

                        let node_binding_to_log_odds_map = util::build_node_binding_to_log_odds_data_map(&message.knowledge_graph);

                        let message_with_score_attributes = util::add_composite_score_attributes(message, node_binding_to_log_odds_map);

                        let mut ret = trapi_model_rs::Response::new(message_with_score_attributes);
                        ret.status = Some("Success".to_string());
                        ret.workflow = query.workflow.clone();
                        ret.biolink_version = Some(env::var("BIOLINK_VERSION").unwrap_or("3.1.2".to_string()));
                        ret.schema_version = Some(env::var("SCHEMA_VERSION").unwrap_or("1.4.0".to_string()));

                        // fs::write(Path::new("/tmp/zxcv.json"), &serde_json::to_string_pretty(&ret).unwrap()).unwrap();

                        job.response = Some(serde_json::to_string(&ret).unwrap().into_bytes());
                        job.date_finished = Some(Utc::now().naive_utc());
                        job.status = JobStatus::Completed;
                        job_actions::update(job).expect(format!("Could not update Job: {}", job.id).as_str());

                        // 1st attempt
                        let callback_response_result = reqwest_client.post(&query.callback).json(&ret).send().await;
                        match callback_response_result {
                            Ok(callback_response) => {
                                if !callback_response.status().is_success() {
                                    tokio::time::sleep(Duration::from_secs(5)).await;
                                    // 2st attempt
                                    let callback_response = reqwest_client.post(&query.callback).json(&ret).send().await.unwrap();
                                    if !callback_response.status().is_success() {
                                        job.date_finished = Some(Utc::now().naive_utc());
                                        job.status = JobStatus::Failed;
                                        job_actions::update(job).expect(format!("Could not update Job: {}", job.id).as_str());
                                    }
                                }
                            }
                            Err(e) => {
                                warn!("{}", e);
                            }
                        }
                    }
                }
            }
        }
    } else {
        warn!("No Jobs to run");
    }
}

pub fn create_server() -> Rocket<Build> {
    let client = util::build_http_client();
    let mut building_rocket = rocket::build()
        .mount(
            "/docs/",
            make_swagger_ui(&SwaggerUIConfig {
                url: "../openapi.json".to_owned(),
                ..Default::default()
            }),
        )
        .manage(client);

    let openapi_settings = rocket_okapi::settings::OpenApiSettings::default();
    let custom_route_spec = (vec![], openapi::custom_openapi_spec());
    mount_endpoints_and_merged_docs! {
        building_rocket, "/".to_owned(), openapi_settings,
        "/external" => custom_route_spec,
        "" => get_routes_and_docs(&openapi_settings),
    };
    building_rocket
}

pub fn get_routes_and_docs(settings: &rocket_okapi::settings::OpenApiSettings) -> (Vec<rocket::Route>, OpenApi) {
    openapi_get_routes_spec![settings: query, asyncquery, asyncquery_status, download]
}
