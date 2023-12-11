use crate::model::{CQSCompositeScoreKey, CQSCompositeScoreValue};
use crate::scoring;
use reqwest::header;
use reqwest::redirect::Policy;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;
use std::{env, error, fs};
use trapi_model_rs::{Analysis, Attribute, AuxiliaryGraph, BiolinkPredicate, EdgeBinding, Message, NodeBinding, Query, ResourceRoleEnum, Response, RetrievalSource};

pub fn build_node_binding_to_log_odds_data_map(message: Message) -> HashMap<CQSCompositeScoreKey, Vec<CQSCompositeScoreValue>> {
    let mut map = HashMap::new();

    if let Some(kg) = message.knowledge_graph {
        kg.edges.iter().for_each(|(kg_key, kg_edge)| {
            let map_key = CQSCompositeScoreKey::new(kg_edge.subject.to_string(), kg_edge.object.to_string());
            if let Some(source) = kg_edge.sources.iter().find(|a| a.resource_role.eq(&ResourceRoleEnum::PrimaryKnowledgeSource)) {
                if let Some(attributes) = &kg_edge.attributes {
                    attributes
                        .iter()
                        .filter(|attribute| attribute.attribute_type_id == "biolink:has_supporting_study_result")
                        .for_each(|attribute| {
                            if let Some(second_level_attributes) = &attribute.attributes {
                                let atts: Vec<_> = second_level_attributes.iter().filter_map(|a| serde_json::from_value::<Attribute>(a.clone()).ok()).collect();

                                if let (Some(log_odds_ratio_attribute), Some(total_sample_size_attribute)) = (
                                    atts.iter().find(|a| a.attribute_type_id == "biolink:log_odds_ratio"),
                                    atts.iter().find(|a| a.attribute_type_id == "biolink:total_sample_size"),
                                ) {
                                    let mut value = CQSCompositeScoreValue::new(source.resource_id.to_string(), kg_key.to_string());
                                    match (log_odds_ratio_attribute.value.as_f64(), total_sample_size_attribute.value.as_i64()) {
                                        (Some(log_odds_ratio_value), Some(total_sample_size_value)) => {
                                            value.log_odds_ratio = Some(log_odds_ratio_value);
                                            value.total_sample_size = Some(total_sample_size_value);
                                        }
                                        (_, _) => {
                                            value.log_odds_ratio = Some(0.01);
                                            value.total_sample_size = Some(0);
                                        }
                                    }
                                    map.entry(map_key.clone()).or_insert(Vec::new()).push(value);
                                }
                            }
                        });

                    //ICEES does not use nested attributes keyed off of 'biolink:has_supporting_study_result' attribute_type_id
                    if let (Some(log_odds_ratio_attribute), Some(total_sample_size_attribute)) = (
                        attributes.iter().find(|a| a.original_attribute_name == Some("log_odds_ratio".to_string())),
                        attributes.iter().find(|a| a.original_attribute_name == Some("total_sample_size".to_string())),
                    ) {
                        let mut value = CQSCompositeScoreValue::new(source.resource_id.to_string(), kg_key.to_string());
                        // icees treats total_sample_size as a float, should be an int
                        match (log_odds_ratio_attribute.value.as_f64(), total_sample_size_attribute.value.as_f64()) {
                            (Some(log_odds_ratio_value), Some(total_sample_size_value)) => {
                                value.log_odds_ratio = Some(log_odds_ratio_value);
                                value.total_sample_size = Some(total_sample_size_value as i64);
                            }
                            (_, _) => {
                                value.log_odds_ratio = Some(0.01);
                                value.total_sample_size = Some(0);
                            }
                        }
                        map.entry(map_key.clone()).or_insert(Vec::new()).push(value);
                    }

                    // entry may exist, but not have either a 'log_odds_ratio' or a 'total_sample_size'
                    match map.get(&map_key) {
                        None => {
                            let mut value = CQSCompositeScoreValue::new(source.resource_id.to_string(), kg_key.to_string());
                            value.log_odds_ratio = Some(0.01);
                            value.total_sample_size = Some(0);
                            map.entry(map_key.clone()).or_insert(Vec::new()).push(value);
                        }
                        _ => {}
                    }
                }
            }
        });
    }
    map
}

pub fn add_composite_score_attributes(
    mut response: Response,
    node_binding_to_log_odds_map: HashMap<CQSCompositeScoreKey, Vec<CQSCompositeScoreValue>>,
    cqs_query: &Box<dyn scoring::CQSQuery>,
) -> Response {
    if let Some(query_graph) = &response.message.query_graph {
        //this should be a one-hop query so assume only one entry
        if let Some((qg_key, qg_edge)) = query_graph.edges.iter().next() {
            let subject = qg_edge.subject.as_str(); // something like 'n0'
            let object = qg_edge.object.as_str(); // something like 'n1'

            match &mut response.message.results {
                None => {}
                Some(results) => {
                    debug!("clearing out analyses");
                    results.iter_mut().for_each(|r| r.analyses.clear());

                    debug!("sorting before deduping");
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

                    debug!("deduping");
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

                    debug!("adding Analyses");
                    results.iter_mut().for_each(|r| {
                        if let (Some(subject_nb), Some(object_nb)) = (r.node_bindings.get(subject), r.node_bindings.get(object)) {
                            if let (Some(first_subject_nb), Some(first_object_nb)) = (subject_nb.iter().next(), object_nb.iter().next()) {
                                let entry_key_searchable = CQSCompositeScoreKey::new(first_subject_nb.id.to_string(), first_object_nb.id.to_string());
                                let entry = node_binding_to_log_odds_map.iter().find(|(k, _v)| **k == entry_key_searchable);
                                match entry {
                                    Some((_entry_key, entry_values)) => {
                                        let kg_edge_keys: Vec<_> = entry_values.iter().map(|ev| EdgeBinding::new(ev.knowledge_graph_key.clone())).collect();
                                        let mut edge_binding_map = HashMap::new();
                                        edge_binding_map.insert(qg_key.clone(), kg_edge_keys);
                                        let mut analysis = Analysis::new("infores:cqs".into(), edge_binding_map);
                                        analysis.scoring_method = Some("weighted average of log_odds_ratio".into());
                                        analysis.score = Some(cqs_query.compute_score(entry_values));
                                        debug!("analysis: {:?}", analysis);
                                        r.analyses.push(analysis);
                                    }
                                    _ => {
                                        let entry_key_inverse_searchable = CQSCompositeScoreKey::new(first_object_nb.id.to_string(), first_subject_nb.id.to_string());
                                        let entry = node_binding_to_log_odds_map.iter().find(|(k, _v)| **k == entry_key_inverse_searchable);

                                        if let Some((_entry_key, entry_values)) = entry {
                                            let kg_edge_keys: Vec<_> = entry_values.iter().map(|ev| EdgeBinding::new(ev.knowledge_graph_key.clone())).collect();
                                            let mut analysis = Analysis::new("infores:cqs".into(), HashMap::from([(qg_key.clone(), kg_edge_keys)]));
                                            analysis.scoring_method = Some("weighted average of log_odds_ratio".into());
                                            analysis.score = Some(cqs_query.compute_score(entry_values));
                                            debug!("analysis: {:?}", analysis);
                                            r.analyses.push(analysis);
                                        }
                                    }
                                }
                            }
                        }
                    });

                    debug!("sorting by cqs score");
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

    response
}

pub fn sort_results_by_score(message: &mut Message) {
    match &mut message.results {
        None => {}
        Some(results) => {
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

pub fn add_support_graphs(response: &mut Response, cqs_query: &Box<dyn scoring::CQSQuery>) {
    let mut new_results: Vec<trapi_model_rs::Result> = vec![];
    let mut auxiliary_graphs: HashMap<String, AuxiliaryGraph> = HashMap::new();

    if let Some(results) = &mut response.message.results {
        for result in results {
            let mut new_node_bindings: HashMap<String, Vec<NodeBinding>> = HashMap::new();

            if let Some((disease_node_binding_key, disease_node_binding_value)) = result.node_bindings.iter().find(|(k, v)| **k == cqs_query.template_disease_node_id()) {
                // println!("disease_node_binding_value: {:?}", disease_node_binding_value);
                new_node_bindings.insert(cqs_query.inferred_disease_node_id(), disease_node_binding_value.to_vec());
            }

            if let Some((drug_node_binding_key, drug_node_binding_value)) = result.node_bindings.iter().find(|(k, v)| **k == cqs_query.template_drug_node_id()) {
                // println!("drug_node_binding_value: {:?}", drug_node_binding_value);
                new_node_bindings.insert(cqs_query.inferred_drug_node_id(), drug_node_binding_value.to_vec());
            }

            // let mut new_analyses = result.analyses.clone();

            let mut local_auxiliary_graphs: HashMap<String, AuxiliaryGraph> = HashMap::new();
            result.analyses.iter().for_each(|analysis| {
                // analysis.edge_bindings.remove()
                let eb_ids: Vec<String> = analysis
                    .edge_bindings
                    .iter()
                    .map(|(k, v)| v.iter().map(|eb| eb.id.clone()).collect::<Vec<String>>())
                    .flatten()
                    .collect();
                let ag = AuxiliaryGraph::new(eb_ids);

                let auxiliary_graph_id = uuid::Uuid::new_v4().to_string();
                local_auxiliary_graphs.insert(auxiliary_graph_id, ag);
            });

            match (
                new_node_bindings.get(&cqs_query.inferred_drug_node_id()),
                new_node_bindings.get(&cqs_query.inferred_disease_node_id()),
            ) {
                (Some(drug_node_ids), Some(disease_node_ids)) => {
                    // println!("drug_node_ids: {:?}", drug_node_ids);
                    // println!("disease_node_ids: {:?}", disease_node_ids);

                    match (drug_node_ids.first(), disease_node_ids.first()) {
                        (Some(first_drug_node_id), Some(first_disease_node_id)) => {
                            let auxiliary_graph_ids: Vec<_> = local_auxiliary_graphs.clone().into_keys().collect();
                            let mut new_edge = trapi_model_rs::Edge::new(
                                first_drug_node_id.id.clone(),
                                BiolinkPredicate::from("biolink:treats"),
                                first_disease_node_id.id.clone(),
                                vec![RetrievalSource::new("infores:cqs".to_string(), ResourceRoleEnum::PrimaryKnowledgeSource)],
                            );
                            new_edge.attributes = Some(vec![Attribute::new("biolink:support_graphs".to_string(), serde_json::Value::from(auxiliary_graph_ids))]);
                            println!("new_edge: {:?}", new_edge);
                            if let Some(kg) = &mut response.message.knowledge_graph {
                                let new_kg_edge_id = uuid::Uuid::new_v4().to_string();
                                kg.edges.insert(new_kg_edge_id.clone(), new_edge);
                                result.analyses.iter_mut().for_each(|analysis| {
                                    analysis.edge_bindings.clear();
                                    analysis
                                        .edge_bindings
                                        .insert(cqs_query.inferred_predicate_id(), vec![EdgeBinding::new(new_kg_edge_id.clone())]);
                                });
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
            result.node_bindings = new_node_bindings;

            auxiliary_graphs.extend(local_auxiliary_graphs.into_iter());
        }
        response.message.auxiliary_graphs = Some(auxiliary_graphs);
    }
}

// analysis.edge_bindings.iter().flat_map(|(k, v)| v.iter().map(|eb| eb.id).collect())
pub async fn post_query_to_workflow_runner(client: &reqwest::Client, query: &Query) -> Result<trapi_model_rs::Response, Box<dyn error::Error + Send + Sync>> {
    let workflow_runner_url = format!(
        "{}/query",
        env::var("WORKFLOW_RUNNER_URL").unwrap_or("https://translator-workflow-runner.renci.org".to_string())
    );

    let response_result = client.post(workflow_runner_url).json(query).send().await;
    match response_result {
        Ok(response) => {
            debug!("response.status(): {}", response.status());
            let data = response.text().await?;
            let trapi_response: trapi_model_rs::Response = serde_json::from_str(data.as_str()).expect("could not parse Query");
            fs::write(
                Path::new(format!("/tmp/cqs/{}.json", uuid::Uuid::new_v4().to_string()).as_str()),
                serde_json::to_string_pretty(&trapi_response).unwrap(),
            )
            .expect("failed to write output");
            Ok(trapi_response)
        }
        Err(e) => Err(Box::new(e)),
    }
}

pub fn build_http_client() -> reqwest::Client {
    let mut headers = header::HeaderMap::new();
    headers.insert(header::ACCEPT, header::HeaderValue::from_static("application/json"));
    headers.insert(header::CONTENT_TYPE, header::HeaderValue::from_static("application/json"));
    reqwest::Client::builder()
        .redirect(Policy::limited(3))
        .timeout(Duration::from_secs(900))
        .default_headers(headers)
        .build()
        .expect("Could not build reqwest client")
}

#[cfg(test)]
mod test {
    use crate::model::{CQSCompositeScoreKey, CQSCompositeScoreValue};
    use crate::scoring::{CQSQuery, CQSQueryA, CQSQueryB, CQSQueryD};
    use crate::util;
    use crate::util::{add_support_graphs, build_node_binding_to_log_odds_data_map};
    use itertools::Itertools;
    use petgraph::graph::NodeIndex;
    use petgraph::Direction;
    use serde_json::Result;
    use std::cmp::Ordering;
    use std::collections::HashMap;
    use std::fs;
    use std::path::Path;
    use trapi_model_rs::{Analysis, Attribute, AuxiliaryGraph, BiolinkPredicate, EdgeBinding, NodeBinding, Query, ResourceRoleEnum, Response, RetrievalSource};
    use uuid::uuid;

    #[test]
    #[ignore]
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

        // let query: Result<Query> = serde_json::from_str(data);
        //
        // if let Ok(q) = query {
        //     if let Ok(merged_query) = crate::util::merge_query_results(q) {
        //         assert!(merged_query.message.results.is_some());
        //         assert_eq!(merged_query.message.results.unwrap().len(), 2);
        //     }
        // }

        assert!(true);
    }

    #[test]
    fn test_scratch() {
        // let data = fs::read_to_string(Path::new("/tmp/cqs/0df3b8e7-bc53-4d24-b89e-c16b883efbf6.json")).unwrap();
        // let data = fs::read_to_string(Path::new("/tmp/cqs/31f9eb8e-ad7f-4b9f-b828-fe658d6d4a2e.json")).unwrap();
        let data = fs::read_to_string(Path::new("/tmp/cqs/f6010434-7a51-4acb-ae26-e4a36692c451.json")).unwrap();
        let mut response: Response = serde_json::from_str(data.as_str()).unwrap();

        let cqs_query = CQSQueryB::new();
        let mut new_results: Vec<trapi_model_rs::Result> = vec![];
        let mut auxiliary_graphs: HashMap<String, AuxiliaryGraph> = HashMap::new();

        if let Some(results) = &mut response.message.results {
            for result in results {
                let mut new_node_bindings: HashMap<String, Vec<NodeBinding>> = HashMap::new();

                if let Some((disease_node_binding_key, disease_node_binding_value)) = result.node_bindings.iter().find(|(k, v)| **k == cqs_query.template_disease_node_id()) {
                    // println!("disease_node_binding_value: {:?}", disease_node_binding_value);
                    new_node_bindings.insert(cqs_query.inferred_disease_node_id(), disease_node_binding_value.to_vec());
                }

                if let Some((drug_node_binding_key, drug_node_binding_value)) = result.node_bindings.iter().find(|(k, v)| **k == cqs_query.template_drug_node_id()) {
                    // println!("drug_node_binding_value: {:?}", drug_node_binding_value);
                    new_node_bindings.insert(cqs_query.inferred_drug_node_id(), drug_node_binding_value.to_vec());
                }

                // let mut new_analyses = result.analyses.clone();

                let mut local_auxiliary_graphs: HashMap<String, AuxiliaryGraph> = HashMap::new();
                result.analyses.iter().for_each(|analysis| {
                    // analysis.edge_bindings.remove()
                    let eb_ids: Vec<String> = analysis
                        .edge_bindings
                        .iter()
                        .map(|(k, v)| v.iter().map(|eb| eb.id.clone()).collect::<Vec<String>>())
                        .flatten()
                        .collect();
                    let ag = AuxiliaryGraph::new(eb_ids);

                    let auxiliary_graph_id = uuid::Uuid::new_v4().to_string();
                    local_auxiliary_graphs.insert(auxiliary_graph_id, ag);
                });

                match (
                    new_node_bindings.get(&cqs_query.inferred_drug_node_id()),
                    new_node_bindings.get(&cqs_query.inferred_disease_node_id()),
                ) {
                    (Some(drug_node_ids), Some(disease_node_ids)) => {
                        // println!("drug_node_ids: {:?}", drug_node_ids);
                        // println!("disease_node_ids: {:?}", disease_node_ids);

                        match (drug_node_ids.first(), disease_node_ids.first()) {
                            (Some(first_drug_node_id), Some(first_disease_node_id)) => {
                                let auxiliary_graph_ids: Vec<_> = local_auxiliary_graphs.clone().into_keys().collect();
                                let mut new_edge = trapi_model_rs::Edge::new(
                                    first_drug_node_id.id.clone(),
                                    BiolinkPredicate::from("biolink:treats"),
                                    first_disease_node_id.id.clone(),
                                    vec![RetrievalSource::new("infores:cqs".to_string(), ResourceRoleEnum::PrimaryKnowledgeSource)],
                                );
                                new_edge.attributes = Some(vec![Attribute::new("biolink:support_graphs".to_string(), serde_json::Value::from(auxiliary_graph_ids))]);
                                println!("new_edge: {:?}", new_edge);
                                if let Some(kg) = &mut response.message.knowledge_graph {
                                    let new_kg_edge_id = uuid::Uuid::new_v4().to_string();
                                    kg.edges.insert(new_kg_edge_id.clone(), new_edge);
                                    result.analyses.iter_mut().for_each(|analysis| {
                                        analysis.edge_bindings.clear();
                                        analysis
                                            .edge_bindings
                                            .insert(cqs_query.inferred_predicate_id(), vec![EdgeBinding::new(new_kg_edge_id.clone())]);
                                    });
                                }
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
                result.node_bindings = new_node_bindings;

                auxiliary_graphs.extend(local_auxiliary_graphs.into_iter());
            }
            response.message.auxiliary_graphs = Some(auxiliary_graphs);
        }
        fs::write("/tmp/response.pretty.json", serde_json::to_string_pretty(&response).unwrap()).unwrap();
    }

    #[test]
    #[ignore]
    fn test_compute_composite_score() {
        let values = vec![
            CQSCompositeScoreValue {
                resource_id: "infores:automat-icees-kg".to_string(),
                knowledge_graph_key: "84707a7f1b70".to_string(),
                log_odds_ratio: Some(-0.09315839547658776),
                total_sample_size: Some(4753),
            },
            CQSCompositeScoreValue {
                resource_id: "infores:automat-icees-kg".to_string(),
                knowledge_graph_key: "7cf0de2cf152".to_string(),
                log_odds_ratio: Some(0.2341933875007947),
                total_sample_size: Some(1392),
            },
            CQSCompositeScoreValue {
                resource_id: "infores:automat-icees-kg".to_string(),
                knowledge_graph_key: "e34a01832e65".to_string(),
                log_odds_ratio: Some(-0.4179196879347687),
                total_sample_size: Some(5450),
            },
        ];

        let cqs_query = CQSQueryD::new();
        let score = cqs_query.compute_score(&values);
        let normalized_score = score.atan() * 2.0 / std::f64::consts::PI;
        println!("score: {:?}, normalized_score: {:?}", score, normalized_score);
        assert!(true);
    }

    #[test]
    // #[ignore]
    fn test_build_node_binding_to_log_odds_data_map() {
        let data = fs::read_to_string("/tmp/cqs/220373f2-7c16-40eb-8bd8-070adbfbc9ea.json").unwrap();
        let potential_query: Result<Query> = serde_json::from_str(data.as_str());
        if let Some(mut query) = potential_query.ok() {
            let mut map = build_node_binding_to_log_odds_data_map(query.message.clone());
            map.iter().for_each(|(k, v)| println!("k: {:?}, values: {:?}", k, v));
        }
        assert!(true);
    }

    #[test]
    // #[ignore]
    fn test_find_missing_edges() {
        let wfr_response_data = fs::read_to_string("/tmp/cqs/220373f2-7c16-40eb-8bd8-070adbfbc9ea.json").unwrap();
        let wfr_response: trapi_model_rs::Response = serde_json::from_str(wfr_response_data.as_str()).unwrap();

        let cqs_response_data = fs::read_to_string("/tmp/sample_output.pretty.json").unwrap();
        let cqs_response: trapi_model_rs::Response = serde_json::from_str(cqs_response_data.as_str()).unwrap();

        let mut map: HashMap<String, Vec<i32>> = HashMap::new();

        if let Some(wfr_kg) = wfr_response.message.knowledge_graph {
            if let Some(cqs_message_results) = cqs_response.message.results {
                cqs_message_results.iter().for_each(|a| {
                    a.analyses.iter().for_each(|b| {
                        b.edge_bindings.iter().for_each(|(c_key, c_value)| {
                            c_value.iter().for_each(|d| {
                                if wfr_kg.edges.keys().contains(&d.id) {
                                    map.entry(d.id.clone()).or_insert(Vec::new()).push(1);
                                }
                            })
                        });
                    });
                });
            }
        }

        println!("looking for missing entries");
        map.iter().for_each(|(k, v)| {
            if v.is_empty() {
                println!("{} is missing", k);
            }
        });

        assert!(true);
    }

    #[test]
    #[ignore]
    fn composite_score() {
        // let data = fs::read_to_string("/tmp/message.pretty.json").unwrap();
        let data = fs::read_to_string("/tmp/asdf.pretty.json").unwrap();
        // let data = fs::read_to_string("/tmp/response_1683229618787.json").unwrap();
        let potential_query: Result<Query> = serde_json::from_str(data.as_str());
        if let Some(mut query) = potential_query.ok() {
            let mut map = build_node_binding_to_log_odds_data_map(query.message.clone());
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
                let cqs_query = CQSQueryD::new();

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
                                                let score = cqs_query.compute_score(entry_values);
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
                                                    let score = cqs_query.compute_score(entry_values);
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
}
