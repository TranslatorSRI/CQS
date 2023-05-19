#![feature(get_many_mut)]
#[macro_use]
extern crate rocket;

use crate::model::{Analysis, EdgeBinding, KnowledgeType, Query};
use futures::future::join_all;
use hyper::body::HttpBody;
use hyper_tls::HttpsConnector;
use itertools::Itertools;
use rocket::fairing::AdHoc;
use rocket::serde::{json::Json, Deserialize};
use rocket::{Build, Rocket, State};
use rocket_okapi::okapi::openapi3::*;
use rocket_okapi::{mount_endpoints_and_merged_docs, openapi, openapi_get_routes_spec, swagger_ui::*};
use std::collections::HashMap;
use std::error::Error;
use std::fs;

mod model;
mod openapi;
mod util;

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

    responses.into_iter().for_each(|r| {
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
    });

    let map = util::build_node_binding_to_log_odds_data_map(&mut query);

    if let Some(query_graph) = &query.message.query_graph {
        //this should be a one-hop query so assume only one entry
        if let Some((qg_key, qg_edge)) = query_graph.edges.iter().next() {
            let subject = qg_edge.subject.as_str(); // something like 'n0'
            let object = qg_edge.object.as_str(); // something like 'n1'

            match &mut query.message.results {
                None => {}
                Some(results) => {
                    results.iter_mut().for_each(|r| {
                        if let (Some(subject_nb), Some(object_nb)) = (r.node_bindings.get(subject), r.node_bindings.get(object)) {
                            if let (Some(first_subject_nb), Some(first_object_nb)) = (subject_nb.iter().next(), object_nb.iter().next()) {
                                if let Some((_entry_key, entry_values)) = map.iter().find(|(k, _v)| first_subject_nb.id == k.0 && first_object_nb.id == k.2) {
                                    let sum_of_n: i64 = entry_values.iter().map(|a| a.3.unwrap()).sum(); // (N1 + N2 + N3)
                                    let sum_of_weights = entry_values.iter().map(|ev| (ev.3.unwrap() / sum_of_n) as f64).sum::<f64>(); // (W1 + W2 + W3)
                                    let score_numerator = entry_values.iter().map(|ev| (ev.3.unwrap() / sum_of_n) as f64 * ev.2.unwrap()).sum::<f64>(); // (W1 * OR1 + W2 * OR2 + W3 * OR3)
                                    entry_values.iter().for_each(|ev| {
                                        let mut edge_binding_map = HashMap::new();
                                        edge_binding_map.insert(qg_key.clone(), vec![EdgeBinding::new(ev.1.parse().unwrap())]);
                                        let mut analysis = Analysis::new("infores:cqs".into(), edge_binding_map);
                                        analysis.score = Some(score_numerator / sum_of_weights);
                                        analysis.scoring_method = Some("weighted average of log_odds_ratio".into());
                                        r.analyses.push(analysis);
                                    });
                                }
                            }
                        }
                    });
                }
            }
        }
    }

    Json(query)
    // Json(util::merge_query_results(query).expect("failed to merge results"))
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
    // fs::write("/tmp/asdf.json", response_data.as_str()).expect("could not write data");
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
    let custom_route_spec = (vec![], openapi::custom_openapi_spec());
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
