use rocket_okapi::okapi::schemars;
use rocket_okapi::okapi::schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_with::skip_serializing_none;
use std::collections::HashMap;

pub type BiolinkEntity = String;
pub type BiolinkPredicate = String;
pub type CURIE = String;

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
pub enum LogLevel {
    ERROR,
    #[default]
    WARNING,
    INFO,
    DEBUG,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum KnowledgeType {
    LOOKUP,
    INFERRED,
}

#[skip_serializing_none]
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
pub struct LogEntry {
    pub timestamp: Option<String>,

    pub level: Option<LogLevel>,

    pub code: Option<String>,

    pub message: Option<String>,
}

#[skip_serializing_none]
#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
pub struct NodeBinding {
    pub id: CURIE,

    pub query_id: Option<String>,

    pub attributes: Option<Vec<Attribute>>,
}

#[skip_serializing_none]
#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
pub struct EdgeBinding {
    pub id: String,

    pub attributes: Option<Vec<Attribute>>,
}

#[skip_serializing_none]
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
pub struct Result {
    pub node_bindings: HashMap<String, Vec<NodeBinding>>,

    pub edge_bindings: HashMap<String, Vec<EdgeBinding>>,

    pub score: Option<f64>,
}

#[skip_serializing_none]
#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
pub struct Attribute {
    pub attribute_type_id: CURIE,

    pub original_attribute_name: Option<String>,

    pub value: Value,

    pub value_type_id: Option<CURIE>,

    pub attribute_source: Option<String>,

    pub value_url: Option<String>,

    pub description: Option<String>,

    // #[serde(skip_serializing_if = "Value::is_null")]
    pub attributes: Option<Vec<Value>>,
    // pub attributes: Option<Vec<Attribute>>,
}

#[skip_serializing_none]
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
pub struct AttributeConstraint {
    pub id: CURIE,

    pub name: String,

    pub not: bool,

    pub operator: String,

    pub value: String,

    pub unit_id: Option<String>,

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

#[skip_serializing_none]
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
pub struct QNode {
    pub ids: Option<Vec<CURIE>>,

    #[schemars(regex(pattern = r"^biolink:[A-Z][a-zA-Z]*$"))]
    pub categories: Option<Vec<BiolinkEntity>>,

    pub is_set: Option<bool>,

    pub constraints: Option<Vec<AttributeConstraint>>,
}

#[skip_serializing_none]
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
pub struct QEdge {
    pub knowledge_type: Option<KnowledgeType>,

    #[schemars(regex(pattern = r"^biolink:[a-z][a-z_]*$"))]
    pub predicates: Option<Vec<BiolinkPredicate>>,

    pub subject: String,

    pub object: String,

    pub attribute_constraints: Option<Vec<AttributeConstraint>>,

    pub qualifier_constraints: Option<Vec<QualifierConstraint>>,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
pub struct QueryGraph {
    pub nodes: HashMap<String, QNode>,

    pub edges: HashMap<String, QEdge>,
}

#[skip_serializing_none]
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
pub struct Node {
    pub name: Option<String>,

    #[schemars(regex(pattern = r"^biolink:[A-Z][a-zA-Z]*$"))]
    pub categories: Option<Vec<BiolinkEntity>>,

    pub attributes: Option<Vec<Attribute>>,
}

#[skip_serializing_none]
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
pub struct Edge {
    #[schemars(regex(pattern = r"^biolink:[a-z][a-z_]*$"))]
    pub predicate: Option<BiolinkPredicate>,

    pub subject: CURIE,

    pub object: CURIE,

    pub attributes: Option<Vec<Attribute>>,

    pub qualifiers: Option<Vec<Qualifier>>,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
pub struct KnowledgeGraph {
    pub nodes: HashMap<String, Node>,

    pub edges: HashMap<String, Edge>,
}

#[skip_serializing_none]
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
pub struct Message {
    pub results: Option<Vec<Result>>,

    pub query_graph: Option<QueryGraph>,

    pub knowledge_graph: Option<KnowledgeGraph>,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
pub struct Workflow {
    pub id: String,

    pub parameters: Option<HashMap<String, Value>>,

    pub runner_parameters: Option<HashMap<String, Value>>,
}

#[skip_serializing_none]
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
pub struct Response {
    pub message: Message,

    pub status: Option<String>,

    pub description: Option<String>,

    pub logs: Option<Vec<LogEntry>>,

    pub workflow: Option<Vec<Workflow>>,
}

#[skip_serializing_none]
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
#[schemars(example = "example_query")]
pub struct Query {
    pub message: Message,

    pub log_level: Option<LogLevel>,

    pub logs: Option<Vec<LogEntry>>,

    pub workflow: Option<Vec<Workflow>>,

    pub submitter: Option<String>,
}

fn example_query() -> Query {
    let data = r#"{
      "message": {
        "query_graph": {
          "nodes": {"n1": {"ids": ["MONDO:0009061", "MONDO:0004979"]}, "n0": {"categories": ["biolink:ChemicalEntity"]}},
          "edges": {"e0": {"subject": "n0", "object": "n1", "predicates": ["biolink:treats"], "knowledge_type": "inferred"}}
        }
      }
    }"#;
    let query: Query = serde_json::from_str(data).expect("could not parse example Query data");
    query
}

#[skip_serializing_none]
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
pub struct AsyncQuery {
    pub callback: String,

    pub message: Message,

    pub log_level: Option<LogLevel>,

    pub logs: Option<Vec<LogEntry>>,

    pub workflow: Option<Vec<Workflow>>,

    pub submitter: Option<String>,
}

#[skip_serializing_none]
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
pub struct MetaAttribute {
    pub attribute_type_id: CURIE,

    pub attribute_source: Option<String>,

    pub original_attribute_names: Option<Vec<String>>,

    pub constraint_use: bool,

    pub constraint_name: Option<String>,
}

#[skip_serializing_none]
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
pub struct MetaNode {
    pub id_prefixes: Vec<String>,

    pub attributes: Option<Vec<MetaAttribute>>,
}

#[skip_serializing_none]
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
pub struct MetaEdge {
    #[schemars(regex(pattern = r"^biolink:[A-Z][a-zA-Z]*$"))]
    pub subject: BiolinkEntity,

    #[schemars(regex(pattern = r"^biolink:[a-z][a-z_]*$"))]
    pub predicate: BiolinkPredicate,

    #[schemars(regex(pattern = r"^biolink:[A-Z][a-zA-Z]*$"))]
    pub object: BiolinkEntity,

    pub knowledge_types: Option<Vec<String>>,

    pub attributes: Option<Vec<MetaAttribute>>,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
pub struct MetaKnowledgeGraph {
    pub nodes: HashMap<String, MetaNode>,

    pub edges: HashMap<String, MetaEdge>,
}

#[cfg(test)]
mod test {
    use crate::model::{Message, Query};
    use serde_json::Result;
    use std::fs;

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

    #[test]
    #[should_panic]
    fn invalid_biolink_entity() {
        let data = r#"{ 
            "message": { 
                "query_graph": { 
                    "nodes": {"n1": {"ids": ["donkey", "frizzle chicken"]}, "n0": {"categories": ["biolink:ChemicalEntity"]}}, 
                    "edges": {"e0": {"subject": "n0", "object": "n1", "predicates": ["biolink:treats"], "knowledge_type": "inferred"}} 
                } 
            } 
        }"#;

        let potential_query: Result<Query> = serde_json::from_str(data);
        assert!(potential_query.is_err());
    }

    #[test]
    #[should_panic]
    fn invalid_biolink_predicate() {
        let data = r#"{ 
            "message": { 
                "query_graph": { 
                    "nodes": {"n1": {"ids": ["MONDO:0009061", "MONDO:0004979"]}, "n0": {"categories": ["poopy pants"]}}, 
                    "edges": {"e0": {"subject": "n0", "object": "n1", "predicates": ["biolink:treats"], "knowledge_type": "inferred"}} 
                } 
            } 
        }"#;

        let potential_query: Result<Query> = serde_json::from_str(data);
        assert!(potential_query.is_err());
    }

    #[test]
    // #[ignore]
    fn scratch() {
        // let data = fs::read_to_string("/tmp/message.pretty.json").unwrap();
        let data = fs::read_to_string("/tmp/sample_query.pretty.json").unwrap();
        // let data = fs::read_to_string("/tmp/response_1683229618787.json").unwrap();
        let potential_query: Result<Query> = serde_json::from_str(data.as_str());
        if let Err(error) = potential_query {
            println!("{}", error);
            assert!(false);
        }
        assert!(true);
    }
}
