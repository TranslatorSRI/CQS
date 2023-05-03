extern crate serde;
// extern crate serde_derive;

use rocket_okapi::okapi::schemars;
use rocket_okapi::okapi::schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
pub enum LogLevel {
    ERROR,
    #[default]
    WARNING,
    INFO,
    DEBUG,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
pub struct LogEntry {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<LogLevel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
pub struct NodeBinding {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attributes: Option<Vec<Attribute>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
pub struct EdgeBinding {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attributes: Option<Vec<Attribute>>,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
pub struct Result {
    pub node_bindings: HashMap<String, Vec<NodeBinding>>,
    pub edge_bindings: HashMap<String, Vec<EdgeBinding>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
pub struct Attribute {
    pub attribute_type_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_attribute_name: Option<String>,
    pub value: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_type_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attribute_source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    // #[serde(skip_serializing_if = "Option::is_none")]
    // pub attributes: Option<Vec<Attribute>>,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
pub struct AttributeConstraint {
    pub id: String,
    pub name: String,
    pub not: bool,
    pub operator: String,
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit_name: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
pub struct Qualifier {
    pub qualifier_type_id: String,
    pub qualifier_value: String,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
pub struct QualifierConstraint {
    qualifier_set: Vec<Qualifier>,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
pub struct QNode {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub categories: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_set: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub constraints: Option<Vec<AttributeConstraint>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum KnowledgeType {
    LOOKUP,
    INFERRED,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
pub struct QEdge {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub knowledge_type: Option<KnowledgeType>,
    // #/components/schemas/BiolinkPredicate
    #[serde(skip_serializing_if = "Option::is_none")]
    pub predicates: Option<Vec<String>>,
    pub subject: String,
    pub object: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attribute_constraints: Option<Vec<AttributeConstraint>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub qualifier_constraints: Option<Vec<QualifierConstraint>>,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
pub struct QueryGraph {
    pub nodes: HashMap<String, QNode>,
    pub edges: HashMap<String, QEdge>,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
pub struct Node {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub categories: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attributes: Option<Vec<Attribute>>,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
pub struct Edge {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub predicate: Option<String>,
    pub subject: String,
    pub object: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attributes: Option<Vec<Attribute>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub qualifiers: Option<Vec<Qualifier>>,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
pub struct KnowledgeGraph {
    pub nodes: HashMap<String, Node>,
    pub edges: HashMap<String, Edge>,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
pub struct Message {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub results: Option<Vec<Result>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query_graph: Option<QueryGraph>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub knowledge_graph: Option<KnowledgeGraph>,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
pub struct Workflow {
    pub id: String,
    pub parameters: Option<HashMap<String, Value>>,
    pub runner_parameters: Option<HashMap<String, Value>>,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
pub struct Response {
    pub message: Message,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logs: Option<Vec<LogEntry>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workflow: Option<Vec<Workflow>>,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
pub struct Query {
    pub message: Message,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub log_level: Option<LogLevel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logs: Option<Vec<LogEntry>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workflow: Option<Vec<Workflow>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub submitter: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
pub struct AsyncQuery {
    pub callback: String,
    pub message: Message,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub log_level: Option<LogLevel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logs: Option<Vec<LogEntry>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workflow: Option<Vec<Workflow>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub submitter: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
pub struct MetaAttribute {
    pub attribute_type_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attribute_source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_attribute_names: Option<Vec<String>>,
    pub constraint_use: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub constraint_name: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
pub struct MetaNode {
    pub id_prefixes: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attributes: Option<Vec<MetaAttribute>>,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
pub struct MetaEdge {
    pub subject: String,
    pub predicate: String,
    pub object: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub knowledge_types: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attributes: Option<Vec<MetaAttribute>>,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
pub struct MetaKnowledgeGraph {
    pub nodes: HashMap<String, MetaNode>,
    pub edges: HashMap<String, MetaEdge>,
}

#[cfg(test)]
mod test {
    use crate::model::{Edge, EdgeBinding, Message, NodeBinding, Query, QueryGraph};
    use crate::query;
    use futures::SinkExt;
    use itertools::Itertools;
    use serde::de::Error;
    use serde_json::{Result, Value};
    use std::collections::{HashMap, HashSet};
    use std::fs;
    use std::ops::Deref;

    #[test]
    fn untyped_example() {
        // Some JSON input data as a &str. Maybe this comes from the user.
        let data = r#"{
    "query_graph": {
        "nodes": {
            "n0": { "categories": ["biolink:Disease"], "ids": ["MONDO:0005737"] },
            "n1": { "categories": ["biolink:Gene"] }
        },
        "edges": {
            "e01": { "subject": "n0", "object": "n1" }
        }
    },
    "knowledge_graph": {
        "nodes": {
            "MONDO:0005737": { "categories": ["biolink:Disease"], "name": "Ebola hemorrhagic fever" },
            "HGNC:17770": { "categories": ["biolink:Gene"], "name": "RALGAPA1" },
            "HGNC:13236": { "categories": ["biolink:Gene"], "name": "URI1" }
        },
        "edges": {
            "x17770": { "predicate": "biolink:related_to", "subject": "MONDO:0005737", "object": "HGNC:17770" },
            "x13236": { "predicate": "biolink:related_to", "subject": "MONDO:0005737", "object": "HGNC:13236" }
        }
    },
    "results": [
        {
            "node_bindings": {
                "n0": [ { "id": "MONDO:0005737" } ],
                "n1": [ { "id": "HGNC:17770" } ]
            },
            "edge_bindings": { 
                "e01": [ { "id": "x17770" } ]
            }
        },
        {
            "node_bindings": {
                "n0": [ { "id": "MONDO:0005737" } ],
                "n1": [ { "id": "HGNC:13236" } ]
            },
            "edge_bindings": {
                "e01": [ { "id": "x13236" } ]
            }
        }
    ]
}"#;

        let message: Message = serde_json::from_str(data).expect("could not parse Message");
        let query_graph = message.query_graph;
        assert!(query_graph.is_some());
        let ids = query_graph.and_then(|a| a.nodes.get("n0").and_then(|b| b.ids.clone()));
        assert!(ids.is_some());
        println!("{:?}", ids);

        assert!(true);
    }

    #[test]
    fn treats_inferred() {
        // Some JSON input data as a &str. Maybe this comes from the user.
        let data = r#"{ 
            "message": { 
                "query_graph": { 
                    "nodes": {"n1": {"ids": ["MONDO:0009061", "MONDO:0004979"]}, "n0": {"categories": ["biolink:ChemicalEntity"]}}, 
                    "edges": {"e0": {"subject": "n0", "object": "n1", "predicates": ["biolink:treats"], "knowledge_type": "inferred"}} 
                } 
            } 
        }"#;

        let potential_query: Result<Query> = serde_json::from_str(data);
        assert!(potential_query.is_ok());
    }

    // #[test]
    // fn scratch() {
    //     let data = fs::read_to_string("/tmp/message.pretty.json").unwrap();
    //     let potential_query: Result<Query> = serde_json::from_str(data.as_str());
    //     assert!(potential_query.is_ok());
    // }

    #[test]
    fn simple_merge() {
        let data = r#"{
            "message": {
                "query_graph": {
                    "nodes": {"drug": {"categories": ["biolink:ChemicalEntity"]}, "disease": {"ids": ["MONDO:1234"]}},
                    "edges": {"e": {"subject": "drug", "object": "disease", "predicate": "biolink:treats"}}
                },
                "knowledge_graph": {
                    "nodes": {},
                    "edges": {}
                },
                "results": [
                    {
                        "node_bindings": {"disease": [{"id": "MONDO:1234"}], "drug": [{"id": "drug1"}]}, 
                        "edge_bindings": {"_1": [{"id": "e1"}]}
                    },
                    {
                        "node_bindings": {"disease": [{"id": "MONDO:1234"}], "drug": [{"id": "drug1"}]}, 
                        "edge_bindings": {"_1": [{"id": "e2"}]}
                    },
                    {
                        "node_bindings": {"disease": [{"id": "MONDO:1234"}], "drug": [{"id": "drug1"}]},
                        "edge_bindings": {"_1": [{"id": "e3"}]}
                    },
                    {
                        "node_bindings": {"disease": [{"id": "MONDO:4321"}], "drug": [{"id": "drug1"}]},
                        "edge_bindings": {"_1": [{"id": "e1"}]}
                    }
                ]
            }
        }"#;

        let query: Result<Query> = serde_json::from_str(data);
        if let Ok(q) = query {
            if let Some(mut results) = q.message.results {
                let tc = (0..results.len()).tuple_combinations::<(usize, usize)>();
                for (a, b) in tc {
                    println!("{:?}", (a, b));
                    if let Ok([result_a, result_b]) = results.get_many_mut([a, b]) {
                        if result_a.score.is_none() && result_b.score.is_none() {
                            if result_a.node_bindings == result_b.node_bindings {
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
                }
                results.dedup();
                assert_eq!(results.len(), 2);
            }
        }

        assert!(true);
    }
}
