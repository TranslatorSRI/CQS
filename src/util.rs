use crate::model::{CQSCompositeScoreKey, CQSCompositeScoreValue};
use itertools::Itertools;
use merge_hashmap::Merge;
use reqwest::redirect::Policy;
use reqwest::{header, RequestBuilder};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;
use std::{env, error, fs};
use trapi_model_rs::{Analysis, AsyncQuery, Attribute, EdgeBinding, KnowledgeGraph, Message, Query, CURIE};

pub fn build_node_binding_to_log_odds_data_map(knowledge_graph: &Option<KnowledgeGraph>) -> HashMap<CQSCompositeScoreKey, Vec<CQSCompositeScoreValue>> {
    let mut map = HashMap::new();
    if let Some(kg) = knowledge_graph {
        kg.edges.iter().for_each(|(kg_key, kg_edge)| {
            if let Some(source) = kg_edge.sources.iter().find(|a| {
                a.resource_id.contains("infores:cohd") || a.resource_id.contains("infores:automat-icees-kg") || a.resource_id.contains("infores:biothings-multiomics-ehr-risk")
            }) {
                let map_key = CQSCompositeScoreKey::new(kg_edge.subject.to_string(), kg_edge.object.to_string());
                let mut value = CQSCompositeScoreValue::new(source.resource_id.to_string(), kg_key.to_string());

                match source.resource_id.as_str() {
                    // "infores:cohd" => {
                    //     if let Some(attributes) = &kg_edge.attributes {
                    //         if let Some(log_odds_attribute) = attributes
                    //             .iter()
                    //             .find(|a| a.attribute_type_id == "biolink:has_supporting_study_result" && a.value_type_id == Some("biolink:LogOddsAnalysisResult".to_string()))
                    //         {
                    //             if let Some(second_level_attributes) = &log_odds_attribute.attributes {
                    //                 let atts: Vec<_> = second_level_attributes.iter().filter_map(|a| serde_json::from_value::<Attribute>(a.clone()).ok()).collect();
                    //
                    //                 let potential_log_odds_ratio_attribute = atts
                    //                     .iter()
                    //                     .find(|a| a.attribute_type_id == "biolink:log_odds_ratio" && a.original_attribute_name == Some("log_odds".to_string()));
                    //
                    //                 let potential_total_sample_size_attribute = atts
                    //                     .iter()
                    //                     .find(|a| a.attribute_type_id == "biolink:total_sample_size" && a.original_attribute_name == Some("concept_pair_count".to_string()));
                    //
                    //                 match (potential_log_odds_ratio_attribute, potential_total_sample_size_attribute) {
                    //                     (Some(log_odds_ratio_attribute), Some(total_sample_size_attribute)) => {
                    //                         match (log_odds_ratio_attribute.value.as_f64(), total_sample_size_attribute.value.as_i64()) {
                    //                             (Some(log_odds_ratio_value), Some(total_sample_size_value)) => {
                    //                                 value.log_odds_ratio = Some(log_odds_ratio_value);
                    //                                 value.total_sample_size = Some(total_sample_size_value as i64);
                    //                                 map.entry(map_key).or_insert(Vec::new()).push(value);
                    //                             }
                    //                             (_, _) => {
                    //                                 value.log_odds_ratio = Some(0.01);
                    //                                 value.total_sample_size = Some(0);
                    //                                 map.entry(map_key).or_insert(Vec::new()).push(value);
                    //                             }
                    //                         }
                    //                     }
                    //                     (_, _) => {
                    //                         value.log_odds_ratio = Some(0.01);
                    //                         value.total_sample_size = Some(0);
                    //                         map.entry(map_key).or_insert(Vec::new()).push(value);
                    //                     }
                    //                 }
                    //             }
                    //         }
                    //     }
                    // }
                    "infores:automat-icees-kg" => {
                        if let Some(attributes) = &kg_edge.attributes {
                            let potential_log_odds_ratio_attribute = attributes
                                .iter()
                                .find(|a| a.attribute_type_id == "biolink:Attribute" && a.original_attribute_name == Some("log_odds_ratio".to_string()));

                            let potential_total_sample_size_attribute = attributes
                                .iter()
                                .find(|a| a.attribute_type_id == "biolink:Attribute" && a.original_attribute_name == Some("total_sample_size".to_string()));

                            match (potential_log_odds_ratio_attribute, potential_total_sample_size_attribute) {
                                (Some(log_odds_ratio_attribute), Some(total_sample_size_attribute)) => {
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
                                }
                                (_, _) => {
                                    value.log_odds_ratio = Some(0.01);
                                    value.total_sample_size = Some(0);
                                }
                            }
                            map.entry(map_key).or_insert(Vec::new()).push(value);
                        }
                    }
                    "infores:cohd" | "infores:biothings-multiomics-ehr-risk" => {
                        if let Some(attributes) = &kg_edge.attributes {
                            if let Some(log_odds_attribute) = attributes.iter().find(|a| a.attribute_type_id == "biolink:has_supporting_study_result") {
                                if let Some(second_level_attributes) = &log_odds_attribute.attributes {
                                    let atts: Vec<_> = second_level_attributes.iter().filter_map(|a| serde_json::from_value::<Attribute>(a.clone()).ok()).collect();

                                    let potential_log_odds_ratio_attribute = atts.iter().find(|a| a.attribute_type_id == "biolink:log_odds_ratio");
                                    let potential_total_sample_size_attribute = atts.iter().find(|a| a.attribute_type_id == "biolink:total_sample_size");

                                    match (potential_log_odds_ratio_attribute, potential_total_sample_size_attribute) {
                                        (Some(log_odds_ratio_attribute), Some(total_sample_size_attribute)) => {
                                            match (log_odds_ratio_attribute.value.as_f64(), total_sample_size_attribute.value.as_i64()) {
                                                (Some(log_odds_ratio_value), Some(total_sample_size_value)) => {
                                                    value.log_odds_ratio = Some(log_odds_ratio_value);
                                                    value.total_sample_size = Some(total_sample_size_value as i64);
                                                }
                                                (_, _) => {
                                                    value.log_odds_ratio = Some(0.01);
                                                    value.total_sample_size = Some(0);
                                                }
                                            }
                                        }
                                        (_, _) => {
                                            value.log_odds_ratio = Some(0.01);
                                            value.total_sample_size = Some(0);
                                        }
                                    }
                                    map.entry(map_key).or_insert(Vec::new()).push(value);
                                }
                            }
                        }
                    }
                    &_ => {}
                };
            }
        });
    }
    map
}

pub fn add_composite_score_attributes(mut message: Message, node_binding_to_log_odds_map: HashMap<CQSCompositeScoreKey, Vec<CQSCompositeScoreValue>>) -> Message {
    if let Some(query_graph) = &message.query_graph {
        //this should be a one-hop query so assume only one entry
        if let Some((qg_key, qg_edge)) = query_graph.edges.iter().next() {
            let subject = qg_edge.subject.as_str(); // something like 'n0'
            let object = qg_edge.object.as_str(); // something like 'n1'

            match &mut message.results {
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
                                        let score = compute_composite_score(entry_values);
                                        let kg_edge_keys: Vec<_> = entry_values.iter().map(|ev| EdgeBinding::new(ev.knowledge_graph_key.clone())).collect();
                                        let mut edge_binding_map = HashMap::new();
                                        edge_binding_map.insert(qg_key.clone(), kg_edge_keys);
                                        let mut analysis = Analysis::new("infores:cqs".into(), edge_binding_map);
                                        analysis.scoring_method = Some("weighted average of log_odds_ratio".into());
                                        if score.is_nan() {
                                            analysis.score = Some(0.01_f64.atan() * 2.0 / std::f64::consts::PI);
                                        } else {
                                            analysis.score = Some(score.atan() * 2.0 / std::f64::consts::PI);
                                        }
                                        debug!("analysis: {:?}", analysis);
                                        r.analyses.push(analysis);
                                    }
                                    _ => {
                                        let entry_key_inverse_searchable = CQSCompositeScoreKey::new(first_object_nb.id.to_string(), first_subject_nb.id.to_string());
                                        let entry = node_binding_to_log_odds_map.iter().find(|(k, _v)| **k == entry_key_inverse_searchable);

                                        if let Some((_entry_key, entry_values)) = entry {
                                            let score = compute_composite_score(entry_values);
                                            let kg_edge_keys: Vec<_> = entry_values.iter().map(|ev| EdgeBinding::new(ev.knowledge_graph_key.clone())).collect();
                                            let mut analysis = Analysis::new("infores:cqs".into(), HashMap::from([(qg_key.clone(), kg_edge_keys)]));
                                            analysis.scoring_method = Some("weighted average of log_odds_ratio".into());
                                            if score.is_nan() {
                                                analysis.score = Some(0.01_f64.atan() * 2.0 / std::f64::consts::PI);
                                            } else {
                                                analysis.score = Some(score.atan() * 2.0 / std::f64::consts::PI);
                                            }
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

    message
}

pub fn get_canned_queries(ids: &Vec<CURIE>) -> Vec<Query> {
    let path_whitelist = env::var("PATH_WHITELIST").unwrap_or("d".to_string());
    let whitelisted_paths: Vec<_> = path_whitelist.split(",").collect();
    let curie_token = format!("\"{}\"", ids.clone().into_iter().join("\",\""));
    let queries = whitelisted_paths
        .iter()
        .map(|path| {
            let file = format!("./src/data/path_{}.template.json", path);
            let mut template = fs::read_to_string(&file).expect(format!("Could not find file: {}", &file).as_str());
            template = template.replace("CURIE_TOKEN", curie_token.as_str());
            debug!("template: {}", template);
            let query: Query = serde_json::from_str(template.as_str()).unwrap();
            query
        })
        .collect();
    queries
}

pub fn compute_composite_score(entry_values: &Vec<CQSCompositeScoreValue>) -> f64 {
    let total_sample_sizes: Vec<_> = entry_values.iter().filter_map(|ev| ev.total_sample_size).collect();
    let sum_of_total_sample_sizes: i64 = total_sample_sizes.iter().sum(); // (N1 + N2 + N3)

    let weights: Vec<_> = entry_values
        .iter()
        // .filter(|ev| ev.total_sample_size.is_some())
        .map(|ev| ev.total_sample_size.unwrap() as f64 / sum_of_total_sample_sizes as f64)
        .collect();
    let sum_of_weights = weights.iter().sum::<f64>(); // (W1 + W2 + W3)

    let score_numerator = entry_values
        .iter()
        // .filter(|ev| ev.total_sample_size.is_some())
        .map(|ev| (ev.total_sample_size.unwrap() as f64 / sum_of_total_sample_sizes as f64) * ev.log_odds_ratio.unwrap())
        .sum::<f64>(); // (W1 * OR1 + W2 * OR2 + W3 * OR3)

    let score = score_numerator / sum_of_weights;
    score.abs()
}

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
            fs::write(Path::new("/tmp/asdf.json"), &data).unwrap();
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
    use crate::util;
    use crate::util::{build_node_binding_to_log_odds_data_map, compute_composite_score};
    use serde_json::Result;
    use std::cmp::Ordering;
    use std::collections::HashMap;
    use std::fs;
    use trapi_model_rs::{Analysis, EdgeBinding, Query};

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

        let score = compute_composite_score(&values);
        let normalized_score = score.atan() * 2.0 / std::f64::consts::PI;
        println!("score: {:?}, normalized_score: {:?}", score, normalized_score);
        assert!(true);
    }

    #[test]
    // #[ignore]
    fn test_build_node_binding_to_log_odds_data_map() {
        let data = fs::read_to_string("mondo_0004979_output.pretty.json").unwrap();
        let potential_query: Result<Query> = serde_json::from_str(data.as_str());
        if let Some(mut query) = potential_query.ok() {
            let mut map = build_node_binding_to_log_odds_data_map(&query.message.knowledge_graph);
            map.iter().for_each(|(k, v)| println!("k: {:?}, values: {:?}", k, v));
        }
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
            let mut map = build_node_binding_to_log_odds_data_map(&query.message.knowledge_graph);
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
}
