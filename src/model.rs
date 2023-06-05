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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ResourceRoleEnum {
    PrimaryKnowledgeSource,
    AggregatorKnowledgeSource,
    SupportingDataSource,
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

    pub query_id: Option<CURIE>,

    pub attributes: Option<Vec<Attribute>>,
}

#[skip_serializing_none]
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
pub struct Analysis {
    pub resource_id: String,

    pub score: Option<f64>,

    pub scoring_method: Option<String>,

    pub support_graphs: Option<Vec<String>>,

    pub edge_bindings: HashMap<String, Vec<EdgeBinding>>,

    pub attributes: Option<Vec<Attribute>>,
}

impl Analysis {
    pub fn new(resource_id: String, edge_bindings: HashMap<String, Vec<EdgeBinding>>) -> Analysis {
        Analysis {
            resource_id,
            score: None,
            scoring_method: None,
            support_graphs: None,
            edge_bindings,
            attributes: None,
        }
    }
}

#[skip_serializing_none]
#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
pub struct EdgeBinding {
    pub id: String,

    pub attributes: Option<Vec<Attribute>>,
}

impl EdgeBinding {
    pub fn new(id: String) -> EdgeBinding {
        EdgeBinding { id, attributes: None }
    }
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
pub struct Result {
    pub node_bindings: HashMap<String, Vec<NodeBinding>>,

    pub analyses: Vec<Analysis>,
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
    #[schemars(regex(pattern = r"^biolink:[a-z][a-z_]*$"))]
    pub qualifier_type_id: CURIE,

    pub qualifier_value: String,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
pub struct QualifierConstraint {
    pub qualifier_set: Vec<Qualifier>,
}

#[skip_serializing_none]
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
pub struct QNode {
    #[schemars(regex(pattern = r"^biolink:[a-z][a-z_]*$"))]
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

    pub subject: String,

    #[schemars(regex(pattern = r"^biolink:[a-z][a-z_]*$"))]
    pub predicates: Option<Vec<BiolinkPredicate>>,

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
    pub subject: CURIE,

    #[schemars(regex(pattern = r"^biolink:[a-z][a-z_]*$"))]
    pub predicate: BiolinkPredicate,

    pub object: CURIE,

    pub sources: Vec<RetrievalSource>,

    pub attributes: Option<Vec<Attribute>>,

    pub qualifiers: Option<Vec<Qualifier>>,
}

#[skip_serializing_none]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct RetrievalSource {
    pub resource_id: CURIE,

    pub resource_role: ResourceRoleEnum,

    pub upstream_resource_ids: Option<Vec<CURIE>>,

    pub source_record_urls: Option<Vec<String>>,
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

    pub auxiliary_graphs: Option<HashMap<String, AuxiliaryGraph>>,
}

#[skip_serializing_none]
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
pub struct AuxiliaryGraph {
    pub edges: Vec<String>,

    pub attributes: Option<Vec<Attribute>>,
}

#[skip_serializing_none]
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

    pub schema_version: Option<String>,

    pub biolink_version: Option<String>,
}

#[skip_serializing_none]
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
#[schemars(example = "example_query")]
pub struct Query {
    pub message: Message,

    pub log_level: Option<LogLevel>,

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

    pub workflow: Option<Vec<Workflow>>,

    pub submitter: Option<String>,
}

#[skip_serializing_none]
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
pub struct AsyncQueryResponse {
    pub job_id: String,

    pub status: Option<String>,

    pub description: Option<String>,
}

#[skip_serializing_none]
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
pub struct AsyncQueryStatusResponse {
    pub status: String,

    pub description: String,

    pub logs: Vec<LogEntry>,

    pub response_url: Option<String>,
}

#[skip_serializing_none]
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
pub struct MetaAttribute {
    pub attribute_type_id: CURIE,

    pub attribute_source: Option<String>,

    pub original_attribute_names: Option<Vec<String>>,

    pub constraint_use: Option<bool>,

    pub constraint_name: Option<String>,
}

#[skip_serializing_none]
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
pub struct MetaQualifier {
    pub qualifier_type_id: CURIE,

    pub applicable_values: Option<Vec<String>>,
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
    pub subject: BiolinkEntity,

    pub predicate: BiolinkPredicate,

    pub object: BiolinkEntity,

    pub knowledge_types: Option<Vec<String>>,

    pub attributes: Option<Vec<MetaAttribute>>,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
pub struct MetaKnowledgeGraph {
    pub nodes: HashMap<String, MetaNode>,

    pub edges: HashMap<String, MetaEdge>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct CQSCompositeScoreKey {
    pub subject: String,
    // pub predicate: String,
    pub object: String,
}

impl CQSCompositeScoreKey {
    pub fn new(subject: String, object: String) -> CQSCompositeScoreKey {
        CQSCompositeScoreKey { subject, object }
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct CQSCompositeScoreValue {
    pub resource_id: String,
    pub knowledge_graph_key: String,
    pub log_odds_ratio: Option<f64>,
    pub total_sample_size: Option<i64>,
}

impl CQSCompositeScoreValue {
    pub fn new(resource_id: String, knowledge_graph_key: String) -> CQSCompositeScoreValue {
        CQSCompositeScoreValue {
            resource_id,
            knowledge_graph_key,
            log_odds_ratio: None,
            total_sample_size: None,
        }
    }
}

#[cfg(test)]
mod test {
    use crate::model::{Analysis, Attribute, CQSCompositeScoreKey, CQSCompositeScoreValue, EdgeBinding, Message, NodeBinding, Query, ResourceRoleEnum, CURIE};
    use crate::util;
    use itertools::{all, Itertools};
    use serde::Deserializer;
    use serde_json::{Result, Value};
    use std::cmp::Ordering;
    use std::collections::HashMap;
    use std::fs;
    use std::num::FpCategory::Nan;

    #[test]
    #[ignore]
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
                    "x17770": { 
                        "predicate": "biolink:related_to", 
                        "subject": "MONDO:0005737", 
                        "object": "HGNC:17770", 
                        "sources": [{
                            "resource_id": "infores:kp0",
                            "resource_role": "primary_knowledge_source"
                        }]
                    },
                    "x13236": { 
                        "predicate": "biolink:related_to", 
                        "subject": "MONDO:0005737", 
                        "object": "HGNC:13236",
                        "sources": [{
                            "resource_id": "infores:kp1",
                            "resource_role": "primary_knowledge_source"
                        }]
                    }
                }
            },
            "results": [
                {
                    "node_bindings": {
                        "n0": [ { "id": "MONDO:0005737" } ],
                        "n1": [ { "id": "HGNC:17770" } ]
                    },
                    "analyses": [{
                        "resource_id": "infores:kp0",
                        "edge_bindings": {
                          "e01": [{ "id": "x13236" }]
                        }
                    }]
                }
            ]
        }"#;

        let message_result: Result<Message> = serde_json::from_str(data);
        match message_result {
            Err(err) => {
                print!("{}", err);
                assert!(false);
            }
            Ok(message) => {
                let query_graph = message.query_graph;
                assert!(query_graph.is_some());
                let ids = query_graph.and_then(|a| a.nodes.get("n0").and_then(|b| b.ids.clone()));
                assert!(ids.is_some());
                println!("{:?}", ids);
            }
        }
    }

    #[test]
    #[ignore]
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
    #[ignore]
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
    #[ignore]
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
    #[ignore]
    fn test_build_node_binding_to_log_odds_data_map() {
        let data = fs::read_to_string("/tmp/asdf.pretty.json").unwrap();
        let potential_query: Result<Query> = serde_json::from_str(data.as_str());
        if let Some(mut query) = potential_query.ok() {
            let mut map = util::build_node_binding_to_log_odds_data_map(&mut query);
            map.iter().for_each(|(k, v)| println!("k: {:?}, values: {:?}", k, v));
        }
        assert!(true);
    }

    #[test]
    // #[ignore]
    fn composite_score() {
        // let data = fs::read_to_string("/tmp/message.pretty.json").unwrap();
        let data = fs::read_to_string("/tmp/asdf.pretty.json").unwrap();
        // let data = fs::read_to_string("/tmp/response_1683229618787.json").unwrap();
        let potential_query: Result<Query> = serde_json::from_str(data.as_str());
        if let Some(mut query) = potential_query.ok() {
            let mut map = util::build_node_binding_to_log_odds_data_map(&mut query);
            // map.iter().for_each(|(k, v)| println!("k: {:?}, values: {:?}", k, v));

            // icees-kg: log_odds_ratio = OR1
            // total_sample_size =  N1
            // weight = W1 = N1/(N1 + N2 + N3)
            //
            // cohd: log_odds_ratio = OR2
            // total_sample_size =  N2
            // weight = W2 = N2/(N1 + N2 + N3)
            //
            // multiomics-ehr-risk-provider: log_odds_ratio = OR3
            // total_sample_size =  N3
            // weight = W3  = N3/(N1 + N2 + N3)
            //
            // Score = (W1 * OR1 + W2 * OR2 + W3 * OR3) / (W1 + W2 + W3)

            if let Some(query_graph) = &query.message.query_graph {
                //this should be a one-hop query so assume only one entry
                if let Some((qg_key, qg_edge)) = query_graph.edges.iter().next() {
                    let subject = qg_edge.subject.as_str(); // something like 'n0'
                    let object = qg_edge.object.as_str(); // something like 'n1'
                    println!("subject: {:?}, object: {:?}", subject, object);

                    match &mut query.message.results {
                        None => {}
                        Some(results) => {
                            results.iter_mut().for_each(|r| r.analyses.clear());

                            results.sort_by(|a, b| {
                                if let (Some(a_nb_subject), Some(a_nb_object), Some(b_nb_subject), Some(b_nb_object)) = (
                                    a.node_bindings.get(subject),
                                    a.node_bindings.get(object),
                                    b.node_bindings.get(subject),
                                    b.node_bindings.get(object),
                                ) {
                                    return if let (Some(a_nb_subject_first), Some(a_nb_object_first), Some(b_nb_subject_first), Some(b_nb_object_first)) =
                                        (a_nb_subject.iter().next(), a_nb_object.iter().next(), b_nb_subject.iter().next(), b_nb_object.iter().next())
                                    {
                                        (a_nb_subject_first.id.to_string(), a_nb_object_first.id.to_string())
                                            .partial_cmp(&(b_nb_subject_first.id.to_string(), b_nb_object_first.id.to_string()))
                                            .unwrap_or(Ordering::Less)
                                    } else {
                                        Ordering::Less
                                    };
                                }
                                Ordering::Less
                            });

                            results.dedup_by(|a, b| {
                                if let (Some(a_nb_subject), Some(a_nb_object), Some(b_nb_subject), Some(b_nb_object)) = (
                                    a.node_bindings.get(subject),
                                    a.node_bindings.get(object),
                                    b.node_bindings.get(subject),
                                    b.node_bindings.get(object),
                                ) {
                                    return if let (Some(a_nb_subject_first), Some(a_nb_object_first), Some(b_nb_subject_first), Some(b_nb_object_first)) =
                                        (a_nb_subject.iter().next(), a_nb_object.iter().next(), b_nb_subject.iter().next(), b_nb_object.iter().next())
                                    {
                                        a_nb_subject_first.id == b_nb_subject_first.id && a_nb_object_first.id == b_nb_object_first.id
                                    } else {
                                        false
                                    };
                                }
                                return false;
                            });
                            results.iter_mut().for_each(|r| {
                                if let (Some(subject_nb), Some(object_nb)) = (r.node_bindings.get(subject), r.node_bindings.get(object)) {
                                    if let (Some(first_subject_nb), Some(first_object_nb)) = (subject_nb.iter().next(), object_nb.iter().next()) {
                                        let entry_key_searchable = CQSCompositeScoreKey::new(first_subject_nb.id.to_string(), first_object_nb.id.to_string());
                                        let entry = map.iter().find(|(k, v)| **k == entry_key_searchable);
                                        match entry {
                                            Some((entry_key, entry_values)) => {
                                                println!("entry_key: {:?}, entry_values: {:?}", entry_key, entry_values);
                                                let score = util::compute_composite_score(entry_values);
                                                println!("score: {:?}", score);
                                                // subject: "MONDO:0009061", object: "PUBCHEM.COMPOUND:16220172"
                                                if first_subject_nb.id == "MONDO:0009061" && first_object_nb.id == "PUBCHEM.COMPOUND:16220172" {
                                                    println!("GOT HERE");
                                                }

                                                let kg_edge_keys: Vec<_> = entry_values.iter().map(|ev| EdgeBinding::new(ev.knowledge_graph_key.clone())).collect();
                                                let mut analysis = Analysis::new("infores:cqs".into(), HashMap::from([(qg_key.clone(), kg_edge_keys)]));
                                                analysis.scoring_method = Some("weighted average of log_odds_ratio".into());
                                                if score.is_nan() {
                                                    analysis.score = Some(0.01_f64.atan() * 2.0 / std::f64::consts::PI);
                                                } else {
                                                    analysis.score = Some(score.atan() * 2.0 / std::f64::consts::PI);
                                                }
                                                println!("analysis: {:?}", analysis);
                                                r.analyses.push(analysis);
                                            }
                                            _ => {
                                                println!("KEY NOT FOUND: {:?}", entry_key_searchable);
                                                let entry_key_inverse_searchable = CQSCompositeScoreKey::new(first_object_nb.id.to_string(), first_subject_nb.id.to_string());
                                                let entry = map.iter().find(|(k, v)| **k == entry_key_inverse_searchable);

                                                if let Some((entry_key, entry_values)) = entry {
                                                    println!("entry_key: {:?}, entry_values: {:?}", entry_key, entry_values);
                                                    let score = util::compute_composite_score(entry_values);
                                                    println!("score: {:?}", score);

                                                    let kg_edge_keys: Vec<_> = entry_values.iter().map(|ev| EdgeBinding::new(ev.knowledge_graph_key.clone())).collect();
                                                    let mut analysis = Analysis::new("infores:cqs".into(), HashMap::from([(qg_key.clone(), kg_edge_keys)]));
                                                    analysis.scoring_method = Some("weighted average of log_odds_ratio".into());
                                                    if score.is_nan() {
                                                        analysis.score = Some(0.01_f64.atan() * 2.0 / std::f64::consts::PI);
                                                    } else {
                                                        analysis.score = Some(score.atan() * 2.0 / std::f64::consts::PI);
                                                    }
                                                    println!("analysis: {:?}", analysis);
                                                    r.analyses.push(analysis);
                                                }
                                            }
                                        }
                                    }
                                }
                            });

                            results.sort_by(|a, b| {
                                if let (Some(a_analysis), Some(b_analysis)) = (a.analyses.iter().next(), b.analyses.iter().next()) {
                                    if let (Some(a_score), Some(b_score)) = (a_analysis.score, b_analysis.score) {
                                        return if b_score < a_score {
                                            Ordering::Less
                                        } else if b_score > a_score {
                                            Ordering::Greater
                                        } else {
                                            b_score.partial_cmp(&a_score).unwrap_or(Ordering::Equal)
                                        };
                                    }
                                }
                                return Ordering::Less;
                            });
                        }
                    }
                }
            }
            let sample_output_result = serde_json::to_string_pretty(&query);
            match sample_output_result {
                Err(error) => {
                    println!("{}", error);
                    assert!(false);
                }
                Ok(sample_output) => {
                    fs::write("/tmp/sample_output.pretty.json", sample_output).unwrap();
                    assert!(true);
                }
            }
        }
    }

    #[test]
    #[ignore]
    fn scratch() {
        let data = fs::read_to_string("/tmp/asdf.pretty.json").unwrap();
        // let data = fs::read_to_string("/tmp/scratch-icees.json").unwrap();
        // let data = fs::read_to_string("/tmp/response_1683229618787.json").unwrap();
        let potential_query: Result<Query> = serde_json::from_str(data.as_str());
        match potential_query {
            Err(error) => {
                println!("{}", error);
                assert!(false);
            }
            Ok(query) => {
                // let pretty_query = serde_json::to_string_pretty(&query).unwrap();
                // fs::write("/tmp/scratch-icees.pretty.json", pretty_query).unwrap();
                assert!(true);
            }
        }
    }
}
