use rocket_okapi::okapi::openapi3::{Contact, Info, License, Object, OpenApi, Server};
use serde_json::Value;
use std::collections::HashMap;

pub fn custom_openapi_spec() -> OpenApi {
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
