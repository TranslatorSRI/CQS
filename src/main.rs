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
use crate::util::send_callback;
use async_once::AsyncOnce;
use diesel_async::pooled_connection::bb8::Pool;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::AsyncPgConnection;
use dotenvy::dotenv;
use futures::future::join_all;
use reqwest::header;
use reqwest::redirect::Policy;
use rocket::fairing::AdHoc;
use rocket::response::status;
use rocket::serde::json::Json;
use rocket::{Build, Rocket};
use rocket_okapi::okapi::openapi3::*;
use rocket_okapi::{mount_endpoints_and_merged_docs, openapi, openapi_get_routes_spec, swagger_ui::*};
use serde_json::json;
use std::collections::HashMap;
use std::env;
use std::time::Duration;
use tokio::time::timeout;
use trapi_model_rs::{AsyncQuery, AsyncQueryResponse, AsyncQueryStatusResponse, KnowledgeGraph, KnowledgeType, Query};
// use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use peak_alloc::PeakAlloc;

#[global_allocator]
static PEAK_ALLOC: PeakAlloc = PeakAlloc;

mod job_actions;
mod model;
mod openapi;
mod schema;
mod template;
mod util;

lazy_static! {
    pub static ref WHITELISTED_TEMPLATE_QUERIES: Vec<Box<dyn template::CQSTemplate>> = vec![
        Box::new(template::ClinicalKPs::new()),
        Box::new(template::OpenPredict::new()),
        Box::new(template::RTXKG2SemMed::new()),
        Box::new(template::ServiceProviderSemMed::new()),
        Box::new(template::ServiceProviderTMKPTargeted::new()),
        Box::new(template::MultiomicsCTKP::new()),
        Box::new(template::MultiomicsDrugApprovalsFAERS::new()),
        Box::new(template::CAMKP::new()),
    ];
    pub static ref DB_POOL: AsyncOnce<bb8::Pool<AsyncDieselConnectionManager<AsyncPgConnection>>> = AsyncOnce::new(async {
        let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        let config = AsyncDieselConnectionManager::<AsyncPgConnection>::new(database_url);
        let result = Pool::builder().connection_timeout(Duration::from_secs(120)).build(config).await;
        match result {
            Ok(pool) => pool,
            Err(e) => panic!("Could not create DB Connection Pool: {}", e),
        }
    });
    pub static ref REQWEST_CLIENT: AsyncOnce<reqwest::Client> = AsyncOnce::new(async {
        let mut headers = header::HeaderMap::new();
        headers.insert(header::ACCEPT, header::HeaderValue::from_static("application/json"));
        headers.insert(header::CONTENT_TYPE, header::HeaderValue::from_static("application/json"));
        let result = reqwest::Client::builder()
            .redirect(Policy::limited(5))
            .timeout(Duration::from_secs(900))
            .default_headers(headers)
            .build();

        match result {
            Ok(request_client) => request_client,
            Err(e) => panic!("Could not create Reqwest Client: {}", e),
        }
    });
    pub static ref CQS_INFORES: String = "infores:cqs".to_string();
    pub static ref TRAPI_MESSAGE_RESULT_LIMIT: i32 = 500;
}

#[openapi]
#[post("/asyncquery", data = "<data>")]
async fn asyncquery(data: Json<AsyncQuery>) -> Result<Json<AsyncQueryResponse>, status::Custom<Json<AsyncQuery>>> {
    let query: AsyncQuery = data.clone().into_inner();

    if let Some(query_graph) = &query.message.query_graph {
        if let Some((_edge_key, _edge_value)) = &query_graph.edges.iter().find(|(_k, v)| {
            if let (Some(predicates), Some(knowledge_type)) = (&v.predicates, &v.knowledge_type) {
                if predicates.contains(&"biolink:treats".to_string()) && knowledge_type == &KnowledgeType::INFERRED {
                    return true;
                }
            }
            return false;
        }) {
            let job = NewJob::new(JobStatus::Queued, serde_json::to_vec(&query).expect("Could not serialize query"));
            let job_id = job_actions::insert(&job).await.expect("Could not insert Job into DB");
            let mut ret = AsyncQueryResponse::new(job_id.to_string());
            ret.status = Some(JobStatus::Queued.to_string());
            return Ok(Json(ret));
        } else {
            let mut message = query.message.clone();
            message.results = Some(vec![]);
            message.knowledge_graph = Some(KnowledgeGraph::new(HashMap::new(), HashMap::new()));

            let mut res = trapi_model_rs::Response::new(message);
            res.status = Some("Success".to_string());
            res.workflow = query.workflow.clone();
            res.biolink_version = Some(env::var("BIOLINK_VERSION").unwrap_or("3.1.2".to_string()));
            res.schema_version = Some(env::var("TRAPI_VERSION").unwrap_or("1.4.0".to_string()));

            send_callback(query, res).await;
            warn!("returning: {:?}", status::Custom(rocket::http::Status::Ok, data.clone()));
            return Err(status::Custom(rocket::http::Status::Ok, data.clone()));
        }
    }
    warn!("returning: {:?}", status::Custom(rocket::http::Status::Ok, data.clone()));
    Err(status::Custom(rocket::http::Status::Ok, data.clone()))
}

#[openapi]
#[get("/asyncquery_status/<job_id>")]
async fn asyncquery_status(job_id: i32) -> Result<Json<AsyncQueryStatusResponse>, status::BadRequest<String>> {
    debug!("job id: {}", job_id);
    if let Ok(job_result) = job_actions::find_by_id(job_id).await {
        if let Some(job) = job_result {
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
    }
    Err(status::BadRequest("Job not found".to_string()))
}

#[openapi]
#[post("/query", data = "<data>")]
async fn query(data: Json<Query>) -> Json<trapi_model_rs::Response> {
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
                    let future_responses: Vec<_> = WHITELISTED_TEMPLATE_QUERIES.iter().map(|cqs_query| util::process(&query_graph, cqs_query, &ids)).collect();
                    let joined_future_responses = join_all(future_responses).await;
                    joined_future_responses
                        .into_iter()
                        .filter_map(std::convert::identity)
                        .for_each(|trapi_response| responses.push(trapi_response));
                }
            }
        }
    }

    let res = util::merge_sort_truncate(query.message.clone(), query.workflow.clone(), responses).await;

    // let node_binding_to_log_odds_map = util::build_node_binding_to_log_odds_data_map(&message.knowledge_graph);
    // let mut ret = trapi_model_rs::Response::new(util::add_composite_score_attributes(message, node_binding_to_log_odds_map));

    Json(res)
}

#[openapi]
#[get("/download/<job_id>")]
async fn download(job_id: i32) -> Result<Json<trapi_model_rs::Response>, status::BadRequest<String>> {
    if let Ok(job_result) = job_actions::find_by_id(job_id).await {
        if let Some(job) = job_result {
            if let Some(job_response) = job.response {
                let response: trapi_model_rs::Response = serde_json::from_str(&*String::from_utf8_lossy(job_response.as_slice())).unwrap();
                return Ok(Json(response));
            }
        }
    }
    Err(status::BadRequest("Job not found".to_string()))
}

#[openapi]
#[get("/version")]
async fn version() -> serde_json::Value {
    let app_version = env!("CARGO_PKG_VERSION");
    let maturity = env::var("MATURITY").unwrap_or("development".to_string());
    let trapi_version = env::var("TRAPI_VERSION").unwrap_or("1.4.0".to_string());
    json!({"app_version": app_version, "trapi_version": trapi_version, "maturity": maturity})
}

#[rocket::main]
async fn main() {
    dotenv().ok();
    env_logger::init();

    let launch_result = create_server().launch().await;
    match launch_result {
        Ok(_) => info!("Rocket shut down gracefully."),
        Err(err) => warn!("Rocket had an error: {}", err),
    };
}

pub fn create_server() -> Rocket<Build> {
    let mut building_rocket = rocket::build()
        .mount(
            "/docs/",
            make_swagger_ui(&SwaggerUIConfig {
                url: "../openapi.json".to_owned(),
                ..Default::default()
            }),
        )
        .attach(AdHoc::on_liftoff("delete stale asyncquery jobs", |_| {
            Box::pin(async {
                tokio::task::spawn(async {
                    let start = tokio::time::Instant::now() + Duration::from_secs(5);
                    let mut interval_timer = tokio::time::interval_at(start, Duration::from_secs(600));
                    loop {
                        interval_timer.tick().await;
                        match timeout(Duration::from_secs(30), util::delete_stale_asyncquery_jobs()).await {
                            Ok(_) => {}
                            Err(_) => {
                                warn!("deleting asyncquery jobs timed out")
                            }
                        }
                    }
                });
            })
        }))
        .attach(AdHoc::on_liftoff("process asyncquery jobs", |_| {
            Box::pin(async {
                tokio::task::spawn(async {
                    let start = tokio::time::Instant::now() + Duration::from_secs(15);
                    let mut interval_timer = tokio::time::interval_at(start, Duration::from_secs(30));
                    loop {
                        interval_timer.tick().await;
                        if let Ok(undone_jobs) = job_actions::find_undone().await {
                            debug!("processing async jobs - current memory: {}MB", PEAK_ALLOC.peak_usage_as_mb());
                            match timeout(Duration::from_secs(450), util::process_asyncquery_jobs(undone_jobs)).await {
                                Ok(_) => {}
                                Err(_) => {
                                    warn!("processing asyncqueries timed out")
                                }
                            }
                        }
                    }
                });
            })
        }));

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
    openapi_get_routes_spec![settings: query, asyncquery, asyncquery_status, download, version/*, view_asyncquery*/]
}
