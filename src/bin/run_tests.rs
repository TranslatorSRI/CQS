#[macro_use]
extern crate log;

use clap::Parser;
use reqwest::header;
use reqwest::redirect::Policy;
use serde_json::json;
use std::error::Error;
use std::fs;
use std::path;
use std::time::Duration;

#[derive(Parser, PartialEq, Debug)]
#[clap(author, version, about, long_about = None)]
struct Options {
    #[clap(short, long, required = true)]
    test_case_dir: path::PathBuf,

    #[clap(short, long, required = true)]
    cqs_url: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let options = Options::parse();
    debug!("{:?}", options);

    let test_case_dir = options.test_case_dir;
    let cqs_url = options.cqs_url;

    let test_case_dir_files = fs::read_dir(test_case_dir.as_path()).expect("reading contents of test case dir");
    let test_case_files = test_case_dir_files
        .filter_map(|res| res.ok())
        .filter(|a| a.path().file_name().unwrap().to_string_lossy().starts_with("TestCase_"))
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

    for test_case_file in test_case_files {
        debug!("{:?}", test_case_file);
        let contents = fs::read_to_string(test_case_file.path()).unwrap();
        let v: serde_json::Value = serde_json::from_str(contents.as_str()).unwrap();
        let test_case_input_id = v["test_case_input_id"].as_str().expect("asdfasdf").trim();

        let trapi_query_json = json!({
            "message": {
                "query_graph": {
                    "nodes": {
                        "n0": {"categories": ["biolink:ChemicalEntity"]},
                        "n1": {"ids": [test_case_input_id]}
                    },
                    "edges": {
                        "e0": {"subject": "n0", "object": "n1", "predicates": ["biolink:treats"], "knowledge_type": "inferred"}
                    }
                }
            },
            "callback": "http://callback-test-app:8008/receive_callback_query"
        });
        let trapi_query = trapi_query_json.to_string();
        info!("{}", trapi_query.to_string());

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
