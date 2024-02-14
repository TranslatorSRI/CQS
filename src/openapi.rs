use rocket_okapi::okapi::openapi3::{Contact, Info, License, Object, OpenApi, Server};
use serde_json::json;
use std::env;

pub fn custom_openapi_spec() -> OpenApi {
    let response_url_root = env::var("RESPONSE_URL").unwrap_or("http://localhost:8000".to_string());
    let maturity = env::var("MATURITY").unwrap_or("development".to_string());
    let location = env::var("LOCATION").unwrap_or("RENCI".to_string());
    let trapi_version = env::var("SCHEMA_VERSION").unwrap_or("1.4.0".to_string());
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
                    let raw_extensions = json!({
                        "x-id": "https://github.com/jdr0887",
                        "x-role": "responsible developer"
                    });
                    Object::from_iter(raw_extensions.as_object().unwrap().clone())
                },
            }),
            license: Some(License {
                name: "MIT".to_owned(),
                url: Some("https://github.com/TranslatorSRI/CQS/blob/master/LICENSE".to_owned()),
                ..Default::default()
            }),
            version: env!("CARGO_PKG_VERSION").to_owned(),
            extensions: {
                let raw_extensions = json!({
                    "x-translator": {
                        "component": "ARA",
                        "team": [ "Clinical Data Provider" ],
                        "biolink-version": "3.1.2",
                        "infores": "infores:cqs"
                    },
                    "x-trapi": {
                        "version": trapi_version,
                        "asyncquery": true,
                        "operations": [ "lookup" ],
                        "batch_size_limit": 100,
                        "rate_limit": 10
                    }
                });
                Object::from_iter(raw_extensions.as_object().unwrap().clone())
            },
        },
        servers: vec![Server {
            url: response_url_root,
            description: Some("development".to_owned()),
            extensions: {
                let raw_extensions = json!({
                    "x-maturity": maturity,
                    "x-location": location,
                    "x-trapi": trapi_version
                });
                Object::from_iter(raw_extensions.as_object().unwrap().clone())
            },
            ..Default::default()
        }],
        paths: Default::default(),
        components: None,
        security: vec![],
        tags: vec![],
        external_docs: None,
        extensions: {
            let raw_extensions = json!({
                "tags": [
                    { "name": "translator" },
                    { "name": "trapi" }
                ]
            });
            Object::from_iter(raw_extensions.as_object().unwrap().clone())
        },
    }
}
