#![feature(get_many_mut)]
#[macro_use]
extern crate rocket;

use crate::model::{KnowledgeType, Query};
use futures::future::join_all;
use hyper::body::HttpBody;
use hyper_tls::HttpsConnector;
use itertools::Itertools;
use rocket::fairing::AdHoc;
use rocket::serde::{json::Json, Deserialize};
use rocket::{Build, Rocket, State};
use rocket_okapi::okapi::openapi3::*;
use rocket_okapi::{mount_endpoints_and_merged_docs, openapi, openapi_get_routes_spec, swagger_ui::*};
use serde_json::Value;
use std::collections::HashMap;
use std::error::Error;
use std::fs;

pub mod model;

#[derive(Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
struct CQSConfig {
    path_whitelist: Vec<String>,
    workflow_runner_url: String,
}

#[openapi]
#[post("/query", data = "<data>")]
async fn query(data: Json<Query>, config: &State<CQSConfig>) -> Json<Query> {
    // info!("{:?}", config);
    // let mut query: Query = serde_json::from_str(data).expect("could not parse Query");
    let mut query: Query = data.into_inner();
    let workflow_runner_url = &config.workflow_runner_url;
    let whitelisted_paths = &config.path_whitelist;
    let mut responses: Vec<Query> = vec![];
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
                    let curie_token = format!("\"{}\"", ids.clone().into_iter().join("\",\""));
                    let future_responses: Vec<_> = whitelisted_paths
                        .iter()
                        .map(|path| post_to_workflow_runner(path.as_str(), curie_token.as_str(), workflow_runner_url.as_str()))
                        .collect();
                    let joined_future_responses: Vec<std::result::Result<Query, Box<dyn Error + Send + Sync>>> = join_all(future_responses).await;
                    joined_future_responses.into_iter().for_each(|r| {
                        if let Ok(query) = r {
                            responses.push(query);
                        }
                    });
                }
            }
        }
    }

    for r in responses.into_iter() {
        if let Some(kg) = r.message.knowledge_graph {
            match &mut query.message.knowledge_graph {
                Some(qmkg) => {
                    qmkg.nodes.extend(kg.nodes);
                    qmkg.edges.extend(kg.edges);
                }
                None => {
                    query.message.knowledge_graph = Some(kg);
                }
            }
        }
        if let Some(results) = r.message.results {
            match &mut query.message.results {
                Some(qmr) => {
                    qmr.extend(results);
                }
                None => {
                    query.message.results = Some(results);
                }
            }
        }
    }
    if query.message.knowledge_graph.is_none() && query.message.results.is_none() {
        Json(query)
    } else {
        Json(merge_query_results(query).expect("failed to merge results"))
    }
}

fn merge_query_results(mut query: Query) -> Result<Query, Box<dyn Error>> {
    if let Some(ref mut results) = query.message.results {
        let tc = (0..results.len()).tuple_combinations::<(usize, usize)>();
        for (a, b) in tc {
            if let Ok([result_a, result_b]) = results.get_many_mut([a, b]) {
                if result_a.score.is_none() && result_b.score.is_none() && result_a.node_bindings == result_b.node_bindings {
                    for (asdf_edge_key, asdf_edge_value) in result_a.edge_bindings.iter_mut() {
                        if let Some(qwer_edge_value) = result_b.edge_bindings.get_mut(asdf_edge_key) {
                            let mut set = asdf_edge_value.clone();
                            qwer_edge_value.iter().for_each(|a| {
                                if !set.contains(a) {
                                    set.push(a.clone());
                                }
                            });
                            *asdf_edge_value = set.clone();
                            *qwer_edge_value = set.clone();
                        }
                    }
                }
            }
        }
        results.dedup();
    }
    Ok(query)
}

async fn post_to_workflow_runner(path: &str, curie_token: &str, workflow_runner_url: &str) -> std::result::Result<Query, Box<dyn Error + Send + Sync>> {
    let file = format!("./src/data/path_{}.template.json", path);
    let mut template = fs::read_to_string(&file).expect(format!("Could not find file: {}", &file).as_str());
    template = template.replace("CURIE_TOKEN", curie_token);
    info!("template: {}", template);
    let request = hyper::Request::builder()
        .uri(workflow_runner_url)
        .header(hyper::header::CONTENT_TYPE, "application/json")
        .header(hyper::header::ACCEPT, "application/json")
        .method(hyper::Method::POST)
        .body(hyper::Body::from(template))?;
    let https = HttpsConnector::new();
    let client = hyper::Client::builder().build::<_, hyper::Body>(https);
    let mut response = client.request(request).await?;
    info!("response.status(): {}", response.status());

    let mut response_data = std::string::String::new();
    while let Some(chunk) = response.body_mut().data().await {
        response_data.push_str(std::str::from_utf8(&*chunk?)?);
    }
    let query = serde_json::from_str(response_data.as_str()).expect("could not parse Query");
    Ok(query)
}

#[rocket::main]
async fn main() {
    let launch_result = create_server().launch().await;
    match launch_result {
        Ok(_) => println!("Rocket shut down gracefully."),
        Err(err) => println!("Rocket had an error: {}", err),
    };
}

pub fn create_server() -> Rocket<Build> {
    let mut building_rocket = rocket::build()
        // .mount("/", openapi_get_routes![query])
        .mount(
            "/docs/",
            make_swagger_ui(&SwaggerUIConfig {
                url: "../openapi.json".to_owned(),
                ..Default::default()
            }),
        )
        .attach(AdHoc::config::<CQSConfig>());

    let openapi_settings = rocket_okapi::settings::OpenApiSettings::default();
    let custom_route_spec = (vec![], custom_openapi_spec());
    mount_endpoints_and_merged_docs! {
        building_rocket, "/".to_owned(), openapi_settings,
        "/external" => custom_route_spec,
        "" => get_routes_and_docs(&openapi_settings),
    };
    building_rocket
}

pub fn get_routes_and_docs(settings: &rocket_okapi::settings::OpenApiSettings) -> (Vec<rocket::Route>, OpenApi) {
    openapi_get_routes_spec![settings: query]
}

fn custom_openapi_spec() -> OpenApi {
    OpenApi {
        openapi: OpenApi::default_version(),
        info: Info {
            title: "Curated Query Service".to_owned(),
            description: Some("When a TRAPI message meets the condition of using a one-hop query with a 'biolink:treats' predicate AND a 'knowledge_type' of 'inferred', then run the templated queries using the specified curie identifiers.".to_owned()),
            terms_of_service: Some("https://github.com/TranslatorSRI/CQS/blob/master/LICENSE".to_owned()),
            contact: Some(Contact {
                name: Some("CQS".to_owned()),
                url: Some("https://github.com/TranslatorSRI/CQS".to_owned()),
                email: Some("jdr0887@renci.org".to_owned()),
                extensions: {
                    let raw_extensions = r#"{
                        "x-id": "https://github.com/jdr0887",
                        "x-role": "responsible developer"
                    }"#;
                    let raw_extensions_map: HashMap<String, Value> = serde_json::from_str(raw_extensions).unwrap();
                    Object::from_iter(raw_extensions_map)
                },
            }),
            license: Some(License {
                name: "MIT".to_owned(),
                url: Some("https://github.com/TranslatorSRI/CQS/blob/master/LICENSE".to_owned()),
                ..Default::default()
            }),
            version: env!("CARGO_PKG_VERSION").to_owned(),
            extensions: {
                let raw_extensions = r#"{
                    "x-translator": {
                        "component": "KP",
                        "team": [ "Clinical Data Provider" ],
                        "biolink-version": "3.1.2",
                        "infores": "infores:cqs"
                    },
                    "x-trapi": {
                        "version": "1.3.0",
                        "asyncquery": false,
                        "operations": [ "lookup" ],
                        "batch_size_limit": 100,
                        "rate_limit": 10
                    }
                }"#;
                let raw_extensions_map: HashMap<String, Value> = serde_json::from_str(raw_extensions).unwrap();
                Object::from_iter(raw_extensions_map)
            },
        },
        servers: vec![Server {
            url: "https://cqs-dev.apps.renci.org/v0.1".to_owned(),
            description: Some("development".to_owned()),
            extensions: {
                let raw_extensions = r#"{
                    "x-maturity": "development",
                    "x-location": "RENCI",
                    "x-trapi": "1.3.0"
                }"#;
                let raw_extensions_map: HashMap<String, Value> = serde_json::from_str(raw_extensions).unwrap();
                Object::from_iter(raw_extensions_map)
            },
            ..Default::default()
        }],
        paths: Default::default(),
        components: None,
        security: vec![],
        tags: vec![],
        // tags: vec![
        //     Tag {
        //         name: "translator".to_owned(),
        //         ..Default::default()
        //     },
        //     Tag {
        //         name: "trapi".to_owned(),
        //         ..Default::default()
        //     },
        // ],
        external_docs: None,
        extensions: {
            let raw_extensions = r#"{
                "tags": [
                    { "name": "translator" },
                    { "name": "trapi" }
                ]
            }"#;
            let raw_extensions_map: HashMap<String, Value> = serde_json::from_str(raw_extensions).unwrap();
            Object::from_iter(raw_extensions_map)
        },
    }
}
