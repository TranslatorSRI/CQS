#[macro_use]
extern crate log;

use clap::Parser;
use itertools::Itertools;
use reqwest::header;
use reqwest::redirect::Policy;
use serde_json::json;
use std::error::Error;
use std::fs;
use std::path;
use std::time::Duration;
use trapi_model_rs::AsyncQuery;

#[derive(Parser, PartialEq, Debug)]
#[clap(author, version, about, long_about = None)]
struct Options {
    #[clap(short, long, required = true)]
    test_assets_dir: path::PathBuf,

    #[clap(short, long, required = true)]
    cqs_url: String,

    #[clap(short, long)]
    limit: Option<i32>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let options = Options::parse();
    debug!("{:?}", options);

    let test_assets_dir = options.test_assets_dir;
    let cqs_url = options.cqs_url;

    let test_assets_dir_files = fs::read_dir(test_assets_dir.as_path()).expect("reading contents of test assets dir");
    let test_assets_files = test_assets_dir_files
        .filter_map(|res| res.ok())
        .filter(|a| a.path().file_name().unwrap().to_string_lossy().starts_with("Asset_"))
        .collect::<Vec<_>>();

    let mut headers = header::HeaderMap::new();
    headers.insert(header::ACCEPT, header::HeaderValue::from_static("application/json"));
    headers.insert(header::CONTENT_TYPE, header::HeaderValue::from_static("application/json"));
    let client = reqwest::Client::builder()
        .redirect(Policy::limited(3))
        .timeout(Duration::from_secs(900))
        .default_headers(headers)
        .build()
        .expect("Could not build reqwest client");

    let files = match options.limit {
        Some(l) => test_assets_files.into_iter().take(l as usize).collect_vec(),
        None => test_assets_files,
    };

    for test_asset_file in files {
        debug!("{:?}", test_asset_file);
        let contents = fs::read_to_string(test_asset_file.path()).unwrap();
        let v: serde_json::Value = serde_json::from_str(contents.as_str()).unwrap();
        let subject_node = match (v["input_id"].as_str(), v["input_category"].as_str()) {
            (Some(id), Some(category)) => {
                json!({"ids": [id],"categories": [category]})
            }
            (Some(id), None) => {
                json!({"ids": [id],"categories": []})
            }
            (None, Some(category)) => {
                json!({"ids": [],"categories": [category]})
            }
            _ => {
                json!({"ids": [],"categories": []})
            }
        };

        let object_node = match (v["output_id"].as_str(), v["output_category"].as_str()) {
            (Some(_id), Some(category)) => {
                json!({"ids": [],"categories": [category]})
            }
            (Some(id), None) => {
                json!({"ids": [id],"categories": []})
            }
            (None, Some(category)) => {
                json!({"ids": [],"categories": [category]})
            }
            _ => {
                json!({"ids": [],"categories": []})
            }
        };

        let _output_id = v["output_id"].as_str().expect("could not get output id").trim();
        let predicate = v["predicate_id"].as_str().expect("could not get predicate id").trim();
        let qualifiers = v["qualifiers"].as_array().expect("could not get qualifiers");
        let real_qualfiers = qualifiers
            .iter()
            .map(|vv| {
                let parameter_key = vv["parameter"].as_str().expect("could not get parameter");
                let parameter_value = vv["value"].as_str().expect("could not get parameter");
                let asdf = json!({
                    "qualifier_type_id": parameter_key.replace("biolink_", "biolink:"),
                    "qualifier_value": parameter_value
                });
                asdf
            })
            .collect_vec();

        let trapi_query_json = json!({
            "message": {
                "query_graph": {
                    "nodes": {
                        "ON": object_node,
                        "SN": subject_node,
                    },
                    "edges": {
                        "t_edge": {
                            "object": "ON",
                            "subject": "SN",
                            "predicates": [
                                predicate
                            ],
                            "knowledge_type": "inferred",
                            "qualifier_constraints": [
                                {
                                    "qualifier_set": real_qualfiers
                                }
                            ]
                        }
                    }
                }
            },
            "callback": "http://callback-test-app:8008/receive_callback_query"
        });
        let trapi_query = trapi_query_json.to_string();
        info!("{:?} - {}", test_asset_file.file_name(), trapi_query.to_string());

        let _query: AsyncQuery = serde_json::from_str(&trapi_query).expect("could not serialize json");

        let response_result = client.post(cqs_url.clone()).json(&trapi_query_json).send().await;
        match response_result {
            Ok(response) => {
                info!("response.status(): {}", response.status());
                let data = response.text().await?;
                info!("response.text(): {}", data);
            }
            _ => {}
        }
    }

    Ok(())
}
