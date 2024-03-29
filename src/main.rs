#![feature(get_many_mut)]
#[macro_use]
extern crate rocket;
#[macro_use]
extern crate diesel;
extern crate diesel_migrations;
#[macro_use]
extern crate log;
#[macro_use]
extern crate lazy_static;

use crate::model::{JobStatus, NewJob};
use chrono::Utc;
use dotenvy::dotenv;
use futures::future::join_all;
use merge_hashmap::Merge;
use rocket::response::status;
use rocket::serde::json::Json;
use rocket::{Build, Rocket, State};
use rocket_okapi::okapi::openapi3::*;
use rocket_okapi::{mount_endpoints_and_merged_docs, openapi, openapi_get_routes_spec, swagger_ui::*};
use std::env;
use std::time::Duration;
use tokio::time::timeout;
use trapi_model_rs::{AsyncQuery, AsyncQueryResponse, AsyncQueryStatusResponse, KnowledgeType, Query};

mod db;
mod job_actions;
mod model;
mod openapi;
mod schema;
mod scoring;
mod util;

lazy_static! {
    pub static ref WHITELISTED_CANNED_QUERIES: Vec<Box<dyn scoring::CQSQuery>> = vec![
        Box::new(scoring::CQSQueryA::new()),
        // Box::new(scoring::CQSQueryB::new()),
        // Box::new(scoring::CQSQueryC::new()),
        // Box::new(scoring::CQSQueryD::new()),
        Box::new(scoring::CQSQueryE::new())
    ];
}

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
            let mut ret = AsyncQueryResponse::new(job_id.to_string());
            ret.status = Some(JobStatus::Queued.to_string());
            return Ok(Json(ret));
        }
    }
    return Err(status::BadRequest("Invalid Query".to_string()));
}

// #[openapi]
// #[get("/view_asyncquery/<job_id>")]
// async fn view_asyncquery(job_id: i32) -> Result<Json<AsyncQuery>, status::BadRequest<String>> {
//     let job = job_actions::find_by_id(job_id).expect("Could not find Job").unwrap();
//     let ret: AsyncQuery = serde_json::from_str(&String::from_utf8_lossy(&job.query.as_slice())).unwrap();
//     return Ok(Json(ret));
// }

#[openapi]
#[get("/asyncquery_status/<job_id>")]
async fn asyncquery_status(job_id: i32) -> Result<Json<AsyncQueryStatusResponse>, status::BadRequest<String>> {
    debug!("job id: {}", job_id);
    if let Some(job) = job_actions::find_by_id(job_id).expect("Could not find Job") {
        let mut status_response = AsyncQueryStatusResponse {
            status: job.status.to_string(),
            description: job.status.to_string(),
            logs: vec![],
            response_url: Some(format!("{}/download/{}", env::var("RESPONSE_URL").unwrap_or("http://localhost:8000".to_string()), job.id)),
        };

        if let Some(job_response) = job.response {
            let response: trapi_model_rs::Response = serde_json::from_str(&*String::from_utf8_lossy(job_response.as_slice())).unwrap();
            if let Some(logs) = response.logs {
                status_response.logs = logs.clone();
            }
        }
        return Ok(Json(status_response));
    }
    return Err(status::BadRequest("Job not found".to_string()));
}

#[openapi]
#[post("/query", data = "<data>")]
async fn query(data: Json<Query>, reqwest_client: &State<reqwest::Client>) -> Json<trapi_model_rs::Response> {
    let query: Query = data.into_inner();
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
                    let future_responses: Vec<_> = WHITELISTED_CANNED_QUERIES.iter().map(|cqs_query| process(cqs_query, &ids, &reqwest_client)).collect();
                    let joined_future_responses = join_all(future_responses).await;
                    joined_future_responses
                        .into_iter()
                        .filter_map(std::convert::identity)
                        .for_each(|trapi_response| responses.push(trapi_response));
                }
            }
        }
    }

    let mut message = query.message.clone();

    responses.into_iter().for_each(|r| {
        message.merge(r.message);
    });

    util::group_results(&mut message);
    util::sort_analysis_by_score(&mut message);
    util::sort_results_by_analysis_score(&mut message);

    // let node_binding_to_log_odds_map = util::build_node_binding_to_log_odds_data_map(&message.knowledge_graph);
    //
    // let mut ret = trapi_model_rs::Response::new(util::add_composite_score_attributes(message, node_binding_to_log_odds_map));
    let mut ret = trapi_model_rs::Response::new(message);
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
    return Err(status::BadRequest("Job not found".to_string()));
}

#[rocket::main]
async fn main() {
    dotenv().ok();

    env_logger::init();

    db::init_db().expect("failed to initialize the db");

    let start = tokio::time::Instant::now() + Duration::from_secs(5);
    tokio::task::spawn(async move {
        let mut interval_timer = tokio::time::interval_at(start, Duration::from_secs(600));
        loop {
            interval_timer.tick().await;
            let res = timeout(Duration::from_secs(30), delete_stale_asyncquery_jobs()).await;
            if res.is_err() {
                warn!("deleting asyncquery jobs timed out");
            }
        }
    });

    let start = tokio::time::Instant::now() + Duration::from_secs(15);
    tokio::task::spawn(async move {
        let mut interval_timer = tokio::time::interval_at(start, Duration::from_secs(30));
        loop {
            // Wait for the next interval tick
            interval_timer.tick().await;
            let res = timeout(Duration::from_secs(300), process_asyncqueries()).await;
            if res.is_err() {
                warn!("processing asyncqueries timed out");
            }
        }
    });

    let launch_result = create_server().launch().await;
    match launch_result {
        Ok(_) => info!("Rocket shut down gracefully."),
        Err(err) => warn!("Rocket had an error: {}", err),
    };
}

async fn delete_stale_asyncquery_jobs() {
    debug!("deleting stale asyncquery jobs");
    if let Ok(jobs) = job_actions::find_all(None) {
        let now = Utc::now().naive_utc();
        jobs.iter()
            .filter(|j| {
                let diff = now - j.date_submitted;
                diff.num_seconds() > 7200
            })
            .for_each(|j| {
                job_actions::delete(&j.id).expect(format!("Could not delete job id: {}", j.id).as_str());
            });
    }
}

async fn process_asyncqueries() {
    debug!("processing asyncquery jobs");
    let reqwest_client = util::build_http_client();

    if let Ok(mut undone_jobs) = job_actions::find_undone() {
        let update_job = |job: &mut model::Job, job_status: JobStatus| {
            job.date_finished = Some(Utc::now().naive_utc());
            job.status = job_status;
            job_actions::update(job).expect(format!("Could not update Job: {}", job.id).as_str());
        };

        for job in undone_jobs.iter_mut() {
            info!("Processing Job: {}", job.id);

            job.date_started = Some(Utc::now().naive_utc());
            job.status = JobStatus::Running;
            job_actions::update(job).expect(format!("Could not update Job: {}", job.id).as_str());

            let query: AsyncQuery = serde_json::from_str(&*String::from_utf8_lossy(job.query.as_slice())).expect("Could not deserialize AsyncQuery");

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
                            let future_responses: Vec<_> = WHITELISTED_CANNED_QUERIES.iter().map(|cqs_query| process(cqs_query, &ids, &reqwest_client)).collect();
                            let joined_future_responses = join_all(future_responses).await;
                            joined_future_responses
                                .into_iter()
                                .filter_map(std::convert::identity)
                                .for_each(|trapi_response| responses.push(trapi_response));
                        }
                    }
                }
            }

            if responses.is_empty() {
                update_job(job, JobStatus::Failed);
            } else {
                let mut message = query.message.clone();

                responses.into_iter().for_each(|r| {
                    message.merge(r.message);
                });

                util::group_results(&mut message);
                util::sort_analysis_by_score(&mut message);
                util::sort_results_by_analysis_score(&mut message);

                // let node_binding_to_log_odds_map = util::build_node_binding_to_log_odds_data_map(&message.knowledge_graph);
                // let message_with_score_attributes = util::add_composite_score_attributes(message, node_binding_to_log_odds_map);
                // let mut ret = trapi_model_rs::Response::new(message_with_score_attributes);
                let mut ret = trapi_model_rs::Response::new(message);
                ret.status = Some("Success".to_string());
                ret.workflow = query.workflow.clone();
                ret.biolink_version = Some(env::var("BIOLINK_VERSION").unwrap_or("3.1.2".to_string()));
                ret.schema_version = Some(env::var("SCHEMA_VERSION").unwrap_or("1.4.0".to_string()));

                job.response = Some(serde_json::to_string(&ret).unwrap().into_bytes());
                update_job(job, JobStatus::Completed);

                // 1st attempt
                info!("1st attempt at sending response to: {}", &query.callback);
                match reqwest_client.post(&query.callback).json(&ret).timeout(Duration::from_secs(10)).send().await {
                    Ok(first_attempt_callback_response) => {
                        let first_attempt_status_code = first_attempt_callback_response.status();
                        debug!("first_attempt_status_code: {}", first_attempt_status_code);
                        if !first_attempt_status_code.is_success() {
                            // update_job(job, JobStatus::Failed);
                            warn!("failed to make 1st callback post");
                            tokio::time::sleep(Duration::from_secs(10)).await;
                            // 2st attempt
                            info!("2nd attempt at sending response to: {}", &query.callback);
                            match reqwest_client.post(&query.callback).json(&ret).timeout(Duration::from_secs(10)).send().await {
                                Ok(second_attempt_callback_response) => {
                                    let second_attempt_status_code = second_attempt_callback_response.status();
                                    debug!("second_attempt_status_code: {}", second_attempt_status_code);
                                    if second_attempt_status_code.is_success() {
                                        // update_job(job, JobStatus::Completed);
                                    } else {
                                        warn!("failed to make 2nd callback post");
                                    }
                                }
                                Err(e) => {
                                    warn!("2nd attempt at callback error: {}", e);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        warn!("1st attempt at callback error: {}", e);
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

async fn process(cqs_query: &Box<dyn scoring::CQSQuery>, ids: &Vec<trapi_model_rs::CURIE>, reqwest_client: &reqwest::Client) -> Option<trapi_model_rs::Response> {
    let canned_query: Query = cqs_query.render_query_template(ids);
    info!("rendered query template {}: {}", cqs_query.name(), serde_json::to_string_pretty(&canned_query).unwrap());
    let mut canned_query_response = util::post_query_to_workflow_runner(&reqwest_client, &canned_query).await.unwrap();

    match env::var("WRITE_WFR_OUTPUT").unwrap_or("false".to_string()).as_str() {
        "true" => {
            std::fs::write(
                std::path::Path::new(format!("/tmp/cqs/path_{}-{}.json", cqs_query.name(), uuid::Uuid::new_v4().to_string()).as_str()),
                serde_json::to_string_pretty(&canned_query_response).unwrap(),
            )
            .expect("failed to write output");
        }
        _ => {}
    };

    util::add_support_graphs(&mut canned_query_response, cqs_query);
    Some(canned_query_response)
    // let node_binding_to_log_odds_map = util::build_node_binding_to_log_odds_data_map(canned_query_response.message.clone());
    // let trapi_response = util::add_composite_score_attributes(canned_query_response, node_binding_to_log_odds_map, &cqs_query);
    // Some(trapi_response)
}

pub fn get_routes_and_docs(settings: &rocket_okapi::settings::OpenApiSettings) -> (Vec<rocket::Route>, OpenApi) {
    openapi_get_routes_spec![settings: query, asyncquery, asyncquery_status, download/*, view_asyncquery*/]
}
