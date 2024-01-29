use crate::model::{CQSCompositeScoreKey, CQSCompositeScoreValue};
use crate::scoring;
use hyper::body::HttpBody;
use itertools::Itertools;
use ordered_float::OrderedFloat;
use reqwest::header;
use reqwest::redirect::Policy;
use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap};
use std::time::Duration;
use std::{env, error};
use trapi_model_rs::{Analysis, Attribute, AuxiliaryGraph, BiolinkPredicate, EdgeBinding, Message, NodeBinding, ResourceRoleEnum, Response, RetrievalSource};

#[allow(dead_code)]
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

#[allow(dead_code)]
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
                                        let mut edge_binding_map = BTreeMap::new();
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
                                            let mut analysis = Analysis::new("infores:cqs".into(), BTreeMap::from([(qg_key.clone(), kg_edge_keys)]));
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

pub fn sort_analysis_by_score(message: &mut Message) {
    if let Some(results) = &mut message.results {
        // 1st sort Analyses
        results.iter_mut().for_each(|a| {
            a.analyses.sort_by(|aa, ba| {
                if let (Some(a_score), Some(b_score)) = (aa.score, ba.score) {
                    return if b_score < a_score {
                        Ordering::Less
                    } else if b_score > a_score {
                        Ordering::Greater
                    } else {
                        b_score.partial_cmp(&a_score).unwrap_or(Ordering::Equal)
                    };
                }
                return Ordering::Less;
            });
        });
    }
}

pub fn sort_results_by_analysis_score(message: &mut Message) {
    if let Some(results) = &mut message.results {
        // 2nd sort Results by 1st Analysis
        results.sort_by(|a, b| {
            if let (Some(aa), Some(ab)) = (a.analyses.iter().next(), b.analyses.iter().next()) {
                if let (Some(a_score), Some(b_score)) = (aa.score, ab.score) {
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

pub fn add_support_graphs(response: &mut Response, cqs_query: &Box<dyn scoring::CQSQuery>) {
    let mut auxiliary_graphs: BTreeMap<String, AuxiliaryGraph> = BTreeMap::new();

    if let Some(results) = &mut response.message.results {
        for result in results {
            let mut new_node_bindings: BTreeMap<String, Vec<NodeBinding>> = BTreeMap::new();

            if let Some((_disease_node_binding_key, disease_node_binding_value)) = result.node_bindings.iter().find(|(k, _v)| **k == cqs_query.template_disease_node_id()) {
                new_node_bindings.insert(cqs_query.inferred_disease_node_id(), disease_node_binding_value.to_vec());
            }

            if let Some((_drug_node_binding_key, drug_node_binding_value)) = result.node_bindings.iter().find(|(k, _v)| **k == cqs_query.template_drug_node_id()) {
                new_node_bindings.insert(cqs_query.inferred_drug_node_id(), drug_node_binding_value.to_vec());
            }

            let mut local_auxiliary_graphs: BTreeMap<String, AuxiliaryGraph> = BTreeMap::new();
            result.analyses.iter().for_each(|analysis| {
                let eb_ids: Vec<String> = analysis
                    .edge_bindings
                    .iter()
                    .map(|(_k, v)| v.iter().map(|eb| eb.id.clone()).collect::<Vec<String>>())
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
                (Some(drug_node_ids), Some(disease_node_ids)) => match (drug_node_ids.first(), disease_node_ids.first()) {
                    (Some(first_drug_node_id), Some(first_disease_node_id)) => {
                        let auxiliary_graph_ids: Vec<_> = local_auxiliary_graphs.clone().into_keys().collect();
                        let mut new_edge = trapi_model_rs::Edge::new(
                            first_drug_node_id.id.clone(),
                            BiolinkPredicate::from("biolink:treats"),
                            first_disease_node_id.id.clone(),
                            vec![RetrievalSource::new("infores:cqs".to_string(), ResourceRoleEnum::PrimaryKnowledgeSource)],
                        );
                        new_edge.attributes = Some(vec![Attribute::new("biolink:support_graphs".to_string(), serde_json::Value::from(auxiliary_graph_ids))]);
                        // println!("new_edge: {:?}", new_edge);
                        if let Some(kg) = &mut response.message.knowledge_graph {
                            let new_kg_edge_id = uuid::Uuid::new_v4().to_string();
                            kg.edges.insert(new_kg_edge_id.clone(), new_edge);
                            result.analyses.retain(|analysis| analysis.edge_bindings.iter().all(|(_k, v)| !v.is_empty()));
                            result.analyses.iter_mut().for_each(|analysis| {
                                analysis.edge_bindings.clear();
                                analysis
                                    .edge_bindings
                                    .insert(cqs_query.inferred_predicate_id(), vec![EdgeBinding::new(new_kg_edge_id.clone())]);
                            });
                        }
                    }
                    _ => {}
                },
                _ => {}
            }
            result.node_bindings = new_node_bindings;

            auxiliary_graphs.extend(local_auxiliary_graphs.into_iter());
        }

        response.message.auxiliary_graphs = Some(auxiliary_graphs);
    }
}

pub fn group_results(message: &mut Message) {
    let mut new_results: Vec<trapi_model_rs::Result> = vec![];

    if let Some(results) = &mut message.results {
        // 1st pass is to create unique vec of results
        results
            .iter()
            .for_each(|result| match new_results.iter_mut().find(|nr| nr.node_bindings == result.node_bindings) {
                None => {
                    let mut new_result = result.clone();
                    new_result.analyses.clear();
                    new_results.push(new_result);
                }
                Some(_found_result) => {}
            });

        // 2nd pass is to add analyses
        for result in new_results.iter_mut() {
            let analyses: Vec<_> = results
                .iter()
                .filter(|orig| result.node_bindings == orig.node_bindings)
                .flat_map(|r| r.analyses.clone())
                .collect();

            let asdf = analyses.into_iter().map(|a| ((a.resource_id.clone(), OrderedFloat(a.score.unwrap())), a)).into_group_map();

            for ((_resource_id, _score), v) in asdf.into_iter() {
                match v.len() {
                    1 => {
                        result.analyses.extend(v);
                    }
                    _ => {
                        let edge_binding_map = v
                            .iter()
                            .flat_map(|a| {
                                a.edge_bindings
                                    .iter()
                                    .flat_map(|(eb_key, eb_value)| eb_value.iter().map(|eb| (eb_key.clone(), eb.clone())).collect::<Vec<_>>())
                                    .collect::<Vec<_>>()
                            })
                            .into_group_map();

                        if let Some(analysis) = v.iter().next() {
                            let mut a = analysis.clone();
                            a.edge_bindings = edge_binding_map.into_iter().collect();
                            result.analyses.push(a);
                        }
                    }
                }
            }
        }
    }
    message.results = Some(new_results);
}

pub async fn post_query_to_workflow_runner(client: &reqwest::Client, query: &crate::Query) -> Result<trapi_model_rs::Response, Box<dyn error::Error + Send + Sync>> {
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
    use hyper::body::HttpBody;
    use itertools::Itertools;
    use merge_hashmap::Merge;
    use ordered_float::OrderedFloat;
    use serde_json::{json, Result, Value};
    use std::cmp::Ordering;
    use std::collections::HashMap;
    use std::fs;
    use std::ops::Deref;
    use std::path::Path;
    use trapi_model_rs::{Analysis, Attribute, AuxiliaryGraph, BiolinkPredicate, EdgeBinding, NodeBinding, Query, ResourceRoleEnum, Response, RetrievalSource, CURIE};
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
    fn test_scratch_liquid() {
        let ids: Vec<String> = vec![
            "MONDO:0004979".to_string(),
            "MONDO:0016575".to_string(),
            "MONDO:0009061".to_string(),
            "MONDO:0018956".to_string(),
            "MONDO:0011705".to_string(),
            "MONDO:0008345".to_string(),
            "MONDO:0020066".to_string(),
        ];

        let curie_token = serde_json::to_string(&ids).unwrap();
        // let curie_token = serde_json::to_value(&ids).unwrap();
        // let asdf = liquid::model::value!(ids);

        let template = liquid::ParserBuilder::with_stdlib().build().unwrap().parse_file("./src/data/path_a.template.json").unwrap();
        // let template = liquid::ParserBuilder::with_stdlib().build().unwrap().parse_file("/tmp/path_a.template.json").unwrap();

        let mut globals = liquid::object!({
            "curies": ids
        });

        let output = template.render(&globals).unwrap();
        println!("{}", output);

        let query_json_value: serde_json::Value = serde_json::from_str(&output).expect("Could not cast rendered json");
        println!("{}", serde_json::to_string_pretty(&query_json_value).unwrap());

        let query_model: trapi_model_rs::Query = serde_json::from_str(&output).expect("Could not cast rendered json");
        println!("{}", serde_json::to_string_pretty(&query_model).unwrap());

        // assert_eq!(output, "Liquid! 2".to_string());
    }

    #[test]
    fn test_add_aux_graphs() {
        let data = fs::read_to_string(Path::new("/tmp/cqs/a3522bf3-6c73-4ed4-98f4-aada6746ed1d.json")).unwrap();
        // let data = fs::read_to_string(Path::new("/tmp/cqs/fa62acca-ce27-4b7d-8d84-22ab4906bdcc.json")).unwrap();

        let mut response: Response = serde_json::from_str(data.as_str()).unwrap();

        let cqs_query = CQSQueryA::new();
        let mut new_results: Vec<trapi_model_rs::Result> = vec![];
        let mut auxiliary_graphs: HashMap<String, AuxiliaryGraph> = HashMap::new();

        if let Some(results) = &mut response.message.results {
            for result in results {
                let mut new_node_bindings: HashMap<String, Vec<NodeBinding>> = HashMap::new();

                // ($foo:ident, $bar:literal, $inferred_drug_node_id:literal, $inferred_predicate_id:literal, $inferred_disease_node_id:literal, $template_drug_node_id:literal, $template_disease_node_id:literal, $func:expr) => {
                // crate::impl_wrapper!(CQSQueryA, "a", "n0", "e0", "n1", "n3", "n0", compute_composite_score);

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
                                    // TODO: this should be removed...added to filter out bug in either WFR or Aragorn
                                    result.analyses.retain(|analysis| analysis.edge_bindings.iter().all(|(k, v)| !v.is_empty()));
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

            // if let Some(results) = &mut response.message.results {
            //     let tmp_results = results.clone();
            //     for result in results {
            //         let tmp: Vec<_> = tmp_results.iter().filter(|tmp_result| tmp_result.node_bindings == result.node_bindings).collect();
            //
            //         tmp.iter().map(|r| r.analyses).map(|a| a.iter().map(|b| b.edge_bindings)).collect();
            //     }
            // }
            // .iter()
            // .map(|a| a.edge_bindings.iter().map(|(k, v)| v.iter().map(|eb| eb.id.clone())).collect::<Vec<String>>())
            // .collect()

            let mut new_results: Vec<trapi_model_rs::Result> = vec![];

            if let Some(results) = &mut response.message.results {
                // first pass is to create unique vec of results
                results
                    .iter()
                    .for_each(|result| match new_results.iter_mut().find(|nr| nr.node_bindings == result.node_bindings) {
                        None => {
                            let mut new_result = result.clone();
                            new_result.analyses.clear();
                            new_results.push(new_result);
                        }
                        Some(found_result) => {}
                    });

                // 2nd pass is to add analyses
                for result in new_results.iter_mut() {
                    let analyses: Vec<_> = results
                        .iter()
                        .filter(|orig| result.node_bindings == orig.node_bindings)
                        .flat_map(|r| r.analyses.clone())
                        .collect();

                    let asdf = analyses.into_iter().map(|a| ((a.resource_id.clone(), OrderedFloat(a.score.unwrap())), a)).into_group_map();

                    for ((resource_id, score), v) in asdf.into_iter() {
                        match v.len() {
                            1 => {
                                result.analyses.extend(v);
                            }
                            _ => {
                                let edge_binding_map = v
                                    .iter()
                                    .flat_map(|a| {
                                        a.edge_bindings
                                            .iter()
                                            .flat_map(|(eb_key, eb_value)| eb_value.iter().map(|eb| (eb_key.clone(), eb.clone())).collect::<Vec<_>>())
                                            .collect::<Vec<_>>()
                                    })
                                    .into_group_map();

                                if let Some(analysis) = v.iter().next() {
                                    let mut a = analysis.clone();
                                    a.edge_bindings = edge_binding_map;
                                    result.analyses.push(a);
                                }
                            }
                        }
                    }

                    // println!("{:?}", asdf);
                }
            }
            response.message.results = Some(new_results);

            response.message.auxiliary_graphs = Some(auxiliary_graphs);
        }
        fs::write("/tmp/grouped_analyses_response.pretty.json", serde_json::to_string_pretty(&response).unwrap()).unwrap();
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
        let potential_query: serde_json::Result<Query> = serde_json::from_str(data.as_str());
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
        let potential_query: serde_json::Result<Query> = serde_json::from_str(data.as_str());
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
