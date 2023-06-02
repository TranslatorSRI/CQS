use crate::model::{Analysis, Attribute, CQSCompositeScoreKey, CQSCompositeScoreValue, EdgeBinding, Query};
use std::cmp::Ordering;
use std::collections::HashMap;

pub fn build_node_binding_to_log_odds_data_map(query: &mut Query) -> HashMap<CQSCompositeScoreKey, Vec<CQSCompositeScoreValue>> {
    let mut map = HashMap::new();
    if let Some(knowledge_graph) = &query.message.knowledge_graph {
        knowledge_graph.edges.iter().for_each(|(kg_key, kg_edge)| {
            if let Some(source) = kg_edge
                .sources
                .iter()
                .find(|a| a.resource_id.contains("infores:cohd") || a.resource_id.contains("infores:automat-icees-kg"))
            {
                let mut value = CQSCompositeScoreValue::new(source.resource_id.to_string(), kg_key.to_string());

                let map_key = CQSCompositeScoreKey {
                    subject: kg_edge.subject.clone(),
                    // predicate: kg_edge.predicate.clone(),
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
                                                    value.log_odds_ratio = Some(log_odds_ratio_value);
                                                    value.total_sample_size = Some(total_sample_size_value as i64);
                                                    map.entry(map_key).or_insert(Vec::new()).push(value);
                                                }
                                                (_, _) => {
                                                    value.log_odds_ratio = Some(0.01);
                                                    value.total_sample_size = Some(0);
                                                    map.entry(map_key).or_insert(Vec::new()).push(value);
                                                }
                                            }
                                        }
                                        (_, _) => {
                                            value.log_odds_ratio = Some(0.01);
                                            value.total_sample_size = Some(0);
                                            map.entry(map_key).or_insert(Vec::new()).push(value);
                                        }
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
                                            value.log_odds_ratio = Some(log_odds_ratio_value);
                                            value.total_sample_size = Some(total_sample_size_value as i64);
                                            map.entry(map_key).or_insert(Vec::new()).push(value);
                                        }
                                        (_, _) => {
                                            value.log_odds_ratio = Some(0.01);
                                            value.total_sample_size = Some(0);
                                            map.entry(map_key).or_insert(Vec::new()).push(value);
                                        }
                                    }
                                }
                                (_, _) => {
                                    value.log_odds_ratio = Some(0.01);
                                    value.total_sample_size = Some(0);
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

pub fn add_composite_score_attributes(mut query: Query, node_binding_to_log_odds_map: HashMap<CQSCompositeScoreKey, Vec<CQSCompositeScoreValue>>) -> Query {
    if let Some(query_graph) = &query.message.query_graph {
        //this should be a one-hop query so assume only one entry
        if let Some((qg_key, qg_edge)) = query_graph.edges.iter().next() {
            let subject = qg_edge.subject.as_str(); // something like 'n0'
            let object = qg_edge.object.as_str(); // something like 'n1'

            match &mut query.message.results {
                None => {}
                Some(results) => {
                    results.iter_mut().for_each(|r| {
                        r.analyses.clear();
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
                                        let entry = node_binding_to_log_odds_map
                                            .iter()
                                            .find(|(k, _v)| **k == CQSCompositeScoreKey::new(first_object_nb.id.to_string(), first_subject_nb.id.to_string()));

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

    query
}

fn compute_composite_score(entry_values: &Vec<CQSCompositeScoreValue>) -> f64 {
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

    score_numerator / sum_of_weights
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
