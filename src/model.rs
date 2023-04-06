extern crate serde;
extern crate serde_derive;

use rocket::serde::{Deserialize, Serialize};
// use serde::ser::{Serialize, SerializeMap, Serializer};
// use serde::Deserialize;
use std::collections::HashSet;
use std::collections::{BTreeMap, HashMap};

#[derive(PartialEq, Eq, Debug, Default, Display, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub enum LogLevel {
    ERROR,
    #[default]
    WARNING,
    INFO,
    DEBUG,
}

#[derive(PartialEq, Eq, Debug, Default, Display, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct LogEntry {
    timestamp: Option<String>,
    level: Option<LogLevel>,
    code: Option<String>,
    message: Option<String>,
}

#[derive(PartialEq, Eq, Debug, Default, Display, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct NodeBinding {
    id: String,
    query_id: Option<String>,
    attributes: Option<Vec<Attribute>>,
}

#[derive(PartialEq, Eq, Debug, Default, Display, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct EdgeBinding {
    id: String,
    attributes: Option<Vec<Attribute>>,
}

#[derive(PartialEq, Eq, Debug, Default, Display, Display, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct Result {
    node_bindings: Vec<NodeBinding>,
    edge_bindings: Vec<EdgeBinding>,
    // score: Option<f64>,
}

#[derive(PartialEq, Eq, Debug, Default, Display, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct Attribute {
    attribute_type_id: String,
    original_attribute_name: Option<String>,
    value: String,
    value_type_id: Option<String>,
    attribute_source: Option<String>,
    value_url: Option<String>,
    description: Option<String>,
    attributes: Option<Vec<Attribute>>,
}

#[derive(PartialEq, Eq, Debug, Default, Display, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct AttributeConstraint {
    id: String,
    name: String,
    not: bool,
    operator: String,
    value: String,
    unit_id: Option<String>,
    unit_name: Option<String>,
}

#[derive(PartialEq, Eq, Debug, Default, Display, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct Qualifier {
    qualifier_type_id: String,
    qualifier_value: String,
}

#[derive(PartialEq, Eq, Debug, Default, Display, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct QualifierConstraint {
    qualifier_set: Vec<Qualifier>,
}

#[derive(PartialEq, Eq, Debug, Default, Display, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct QNode {
    ids: Option<Vec<String>>,
    categories: Option<Vec<String>>,
    #[serde(default = "false")]
    is_set: Option<bool>,
    constraints: Option<Vec<AttributeConstraint>>,
}

#[derive(PartialEq, Eq, Debug, Default, Display, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct QEdge {
    knowledge_type: Option<String>,
    // #/components/schemas/BiolinkPredicate
    predicates: Option<Vec<String>>,
    subject: String,
    object: String,
    attribute_constraints: Option<Vec<AttributeConstraint>>,
    qualifier_constraints: Option<Vec<QualifierConstraint>>,
}

#[derive(PartialEq, Eq, Debug, Default, Display, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct QueryGraph {
    nodes: HashMap<String, QNode>,
    edges: HashMap<String, QEdge>,
}

#[derive(PartialEq, Eq, Debug, Default, Display, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct Node {
    name: String,
    categories: Option<Vec<String>>,
    attributes: Option<Vec<Attribute>>,
}

#[derive(PartialEq, Eq, Debug, Default, Display, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct Edge {
    predicate: Option<String>,
    subject: String,
    object: String,
    attributes: Option<Vec<Attribute>>,
    qualifiers: Option<Vec<Qualifier>>,
}

#[derive(PartialEq, Eq, Debug, Default, Display, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct KnowledgeGraph {
    nodes: Vec<Node>,
    edges: Vec<Edge>,
}

#[derive(PartialEq, Eq, Debug, Default, Display, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct Message {
    results: Option<Vec<Result>>,
    query_graph: Option<QueryGraph>,
    knowledge_graph: Option<KnowledgeGraph>,
}

#[derive(PartialEq, Eq, Debug, Default, Display, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct Response {
    message: Message,
    status: Option<String>,
    description: Option<String>,
    logs: Option<String>,
    // http://standards.ncats.io/workflow/1.3.2/schema
    workflow: Option<Vec<String>>,
}

#[derive(PartialEq, Eq, Debug, Default, Display, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct Query {
    message: Message,
    log_level: Option<LogLevel>,
    // http://standards.ncats.io/workflow/1.3.2/schema
    workflow: Option<Vec<String>>,
    submitter: Option<String>,
}

#[derive(PartialEq, Eq, Debug, Default, Display, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct AsyncQuery {
    callback: String,
    message: Message,
    log_level: Option<LogLevel>,
    // http://standards.ncats.io/workflow/1.3.2/schema
    workflow: Option<Vec<String>>,
    submitter: Option<String>,
}

#[derive(PartialEq, Eq, Debug, Default, Display, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct MetaAttribute {
    attribute_type_id: String,
    attribute_source: Option<String>,
    original_attribute_names: Option<Vec<String>>,
    constraint_use: bool,
    constraint_name: Option<String>,
}

#[derive(PartialEq, Eq, Debug, Default, Display, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct MetaNode {
    id_prefixes: Vec<String>,
    attributes: Option<Vec<MetaAttribute>>,
}

#[derive(PartialEq, Eq, Debug, Default, Display, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct MetaEdge {
    subject: String,
    predicate: String,
    object: String,
    knowledge_types: Option<Vec<String>>,
    attributes: Option<Vec<MetaAttribute>>,
}

#[derive(PartialEq, Eq, Debug, Default, Display, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct MetaKnowledgeGraph {
    nodes: HashMap<String, MetaNode>,
    edges: HashMap<String, MetaEdge>,
}

#[cfg(test)]
mod test {
    use crate::model::{Message, QueryGraph};
    use serde_json::{Result, Value};

    #[test]
    fn untyped_example() {
        // Some JSON input data as a &str. Maybe this comes from the user.
        let data = r#"{
    "query_graph": {
        "nodes": {
            "n0": {
                "categories": ["biolink:Disease"],
                "ids": ["MONDO:0005737"]
            },
            "n1": {
                "categories": ["biolink:Gene"]
            }
        },
        "edges": {
            "e01": {
                "subject": "n0",
                "object": "n1"
            }
        }
    },
    "knowledge_graph": {
        "nodes": {
            "MONDO:0005737": {
                "categories": ["biolink:Disease"],
                "name": "Ebola hemorrhagic fever"
            },
            "HGNC:17770": {
                "categories": ["biolink:Gene"],
                "name": "RALGAPA1"
            },
            "HGNC:13236": {
                "categories": ["biolink:Gene"],
                "name": "URI1"
            }
        },
        "edges": {
            "x17770": {
                "predicate": "biolink:related_to",
                "subject": "MONDO:0005737",
                "object": "HGNC:17770"
            },
            "x13236": {
                "predicate": "biolink:related_to",
                "subject": "MONDO:0005737",
                "object": "HGNC:13236"
            }
        }
    },
    "results": [
        {
            "node_bindings": {
                "n0": [
                    {
                        "id": "MONDO:0005737"
                    }
                ],
                "n1": [
                    {
                        "id": "HGNC:17770"
                    }
                ]
            },
            "edge_bindings": {
                "e01": [
                    {
                        "id": "x17770"
                    }
                ]
            }
        },
        {
            "node_bindings": {
                "n0": [
                    {
                        "id": "MONDO:0005737"
                    }
                ],
                "n1": [
                    {
                        "id": "HGNC:13236"
                    }
                ]
            },
            "edge_bindings": {
                "e01": [
                    {
                        "id": "x13236"
                    }
                ]
            }
        }
    ]
}"#;

        let query_graph: Message = serde_json::from_str(data).expect("could not parse Message");
        assert!(true);
    }
}
