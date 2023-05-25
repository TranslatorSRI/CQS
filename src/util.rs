use crate::model::{Analysis, Attribute, CQSCompositeScoreKey, CQSCompositeScoreValue, EdgeBinding, Query};
use serde_json::Value;
use std::collections::HashMap;
use std::error;

pub fn build_node_binding_to_log_odds_data_map(query: &mut Query) -> HashMap<CQSCompositeScoreKey, Vec<CQSCompositeScoreValue>> {
    let mut map = HashMap::new();
    if let Some(knowledge_graph) = &query.message.knowledge_graph {
        knowledge_graph.edges.iter().for_each(|(kg_key, kg_edge)| {
            if let Some(source) = kg_edge
                .sources
                .iter()
                .find(|a| a.resource_id.contains("infores:cohd") || a.resource_id.contains("infores:automat-icees-kg"))
            {
                let map_key = CQSCompositeScoreKey {
                    subject: kg_edge.subject.clone(),
                    predicate: kg_edge.predicate.clone(),
                    object: kg_edge.object.clone(),
                };

                match source.resource_id.as_str() {
                    "infores:cohd" => {
                        if let Some(attributes) = &kg_edge.attributes {
                            if let Some(log_odds_attribute) = attributes
                                .iter()
                                .find(|a| a.attribute_type_id == "biolink:has_supporting_study_result" && a.value_type_id == Some("biolink:LogOddsAnalysisResult".to_string()))
                            {
                                if let Some(second_level_attributes) = &log_odds_attribute.attributes {
                                    let atts: Vec<_> = second_level_attributes.iter().filter_map(|a| serde_json::from_value::<Attribute>(a.clone()).ok()).collect();

                                    let potential_log_odds_ratio_attribute = atts
                                        .iter()
                                        .find(|a| a.attribute_type_id == "biolink:log_odds_ratio" && a.original_attribute_name == Some("log_odds".to_string()));

                                    let potential_total_sample_size_attribute = atts
                                        .iter()
                                        .find(|a| a.attribute_type_id == "biolink:total_sample_size" && a.original_attribute_name == Some("concept_pair_count".to_string()));

                                    match (potential_log_odds_ratio_attribute, potential_total_sample_size_attribute) {
                                        (Some(log_odds_ratio_attribute), Some(total_sample_size_attribute)) => {
                                            match (log_odds_ratio_attribute.value.as_f64(), total_sample_size_attribute.value.as_i64()) {
                                                (Some(log_odds_ratio_value), Some(total_sample_size_value)) => {
                                                    map.entry(map_key).or_insert(Vec::new()).push(CQSCompositeScoreValue {
                                                        resource_id: source.resource_id.to_string(),
                                                        knowledge_graph_key: kg_key.to_string(),
                                                        log_odds_ratio: Some(log_odds_ratio_value),
                                                        total_sample_size: Some(total_sample_size_value),
                                                    });
                                                }
                                                (None, Some(_)) => {
                                                    warn!("no log_odds_ratio, yes total_sample_size: {:?}", map_key);
                                                }
                                                (Some(_), None) => {
                                                    warn!("yes log_odds_ratio, no total_sample_size: {:?}", map_key);
                                                }
                                                (None, None) => {
                                                    warn!("no log_odds_ratio, no total_sample_size: {:?}", map_key);
                                                }
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        }
                    }
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
                                            map.entry(map_key).or_insert(Vec::new()).push(CQSCompositeScoreValue {
                                                resource_id: source.resource_id.to_string(),
                                                knowledge_graph_key: kg_key.to_string(),
                                                log_odds_ratio: Some(log_odds_ratio_value),
                                                total_sample_size: Some(total_sample_size_value as i64),
                                            });
                                        }
                                        (None, Some(_)) => {
                                            warn!("no log_odds_ratio, yes total_sample_size: {:?}", map_key);
                                        }
                                        (Some(_), None) => {
                                            warn!("yes log_odds_ratio, no total_sample_size: {:?}", map_key);
                                        }
                                        (None, None) => {
                                            warn!("no log_odds_ratio, no total_sample_size: {:?}", map_key);
                                        }
                                    }
                                }
                                _ => {}
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

pub fn build_name_resovler_map(query: &mut Query) -> HashMap<String, Vec<String>> {
    let map = HashMap::new();

    map
}

pub fn calculate_composite_score(mut query: Query, node_binding_to_log_odds_map: HashMap<CQSCompositeScoreKey, Vec<CQSCompositeScoreValue>>) -> Query {
    if let Some(query_graph) = &query.message.query_graph {
        //this should be a one-hop query so assume only one entry
        if let Some((qg_key, qg_edge)) = query_graph.edges.iter().next() {
            let subject = qg_edge.subject.as_str(); // something like 'n0'
            let object = qg_edge.object.as_str(); // something like 'n1'

            match &mut query.message.results {
                None => {}
                Some(results) => {
                    results.iter_mut().for_each(|r| {
                        if let (Some(subject_nb), Some(object_nb)) = (r.node_bindings.get(subject), r.node_bindings.get(object)) {
                            if let (Some(first_subject_nb), Some(first_object_nb)) = (subject_nb.iter().next(), object_nb.iter().next()) {
                                if let Some((_entry_key, entry_values)) = node_binding_to_log_odds_map
                                    .iter()
                                    .find(|(k, _v)| first_subject_nb.id == k.subject && first_object_nb.id == k.object)
                                {
                                    let sum_of_n: i64 = entry_values.iter().map(|a| a.total_sample_size.unwrap()).sum(); // (N1 + N2 + N3)
                                    let sum_of_weights = entry_values.iter().map(|ev| (ev.total_sample_size.unwrap() / sum_of_n) as f64).sum::<f64>(); // (W1 + W2 + W3)
                                    let score_numerator = entry_values
                                        .iter()
                                        .map(|ev| (ev.total_sample_size.unwrap() / sum_of_n) as f64 * ev.log_odds_ratio.unwrap())
                                        .sum::<f64>(); // (W1 * OR1 + W2 * OR2 + W3 * OR3)
                                    entry_values.iter().for_each(|ev| {
                                        let mut edge_binding_map = HashMap::new();
                                        edge_binding_map.insert(qg_key.clone(), vec![EdgeBinding::new(ev.knowledge_graph_key.parse().unwrap())]);
                                        let mut analysis = Analysis::new("infores:cqs".into(), edge_binding_map);
                                        analysis.score = Some(score_numerator / sum_of_weights);
                                        analysis.scoring_method = Some("weighted average of log_odds_ratio".into());
                                        r.analyses.push(analysis);
                                    });
                                }
                            }
                        }
                    });
                }
            }
        }
    }

    query
}

pub fn merge_query_responses(query: &mut Query, responses: Vec<Query>) {
    responses.into_iter().for_each(|r| {
        if let Some(kg) = r.message.knowledge_graph {
            match &mut query.message.knowledge_graph {
                Some(qmkg) => {
                    qmkg.nodes.extend(kg.nodes);
                    qmkg.edges.extend(kg.edges);
                }
                None => {
                    query.message.knowledge_graph = Some(kg);
                }
            }
        }
        if let Some(results) = r.message.results {
            match &mut query.message.results {
                Some(qmr) => {
                    qmr.extend(results);
                }
                None => {
                    query.message.results = Some(results);
                }
            }
        }
    });

    // if let Some(ref mut results) = query.message.results {
    //     let tc = (0..results.len()).tuple_combinations::<(usize, usize)>();
    //     for (a, b) in tc {
    //         if let Ok([result_a, result_b]) = results.get_many_mut([a, b]) {
    //             if result_a.node_bindings == result_b.node_bindings {
    // for (asdf_edge_key, asdf_edge_value) in result_a.analyses.edge_bindings.iter_mut() {
    //     if let Some(qwer_edge_value) = result_b.edge_bindings.get_mut(asdf_edge_key) {
    //         let mut set = asdf_edge_value.clone();
    //         qwer_edge_value.iter().for_each(|a| {
    //             if !set.contains(a) {
    //                 set.push(a.clone());
    //             }
    //         });
    //         *asdf_edge_value = set.clone();
    //         *qwer_edge_value = set.clone();
    //     }
    // }
    //             }
    //         }
    //     }
    //     results.dedup();
    // }
}

#[cfg(test)]
#[ignore]
mod test {
    use crate::model::Query;
    use serde_json::Result;

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
}
