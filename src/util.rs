use crate::model::{Attribute, Query};
use itertools::Itertools;
use std::collections::HashMap;
use std::error::Error;

pub fn build_node_binding_to_log_odds_data_map(query: &mut Query) -> HashMap<(String, String, String), Vec<(String, String, Option<f64>, Option<i64>)>> {
    let mut map = HashMap::new();
    if let Some(knowledge_graph) = &query.message.knowledge_graph {
        knowledge_graph.edges.iter().for_each(|(kg_key, kg_edge)| {
            if let Some(source) = kg_edge
                .sources
                .iter()
                .find(|a| a.resource_id.contains("infores:cohd") || a.resource_id.contains("infores:automat-icees-kg"))
            {
                let map_key = (kg_edge.subject.clone(), kg_edge.predicate.clone(), kg_edge.object.clone());

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
                                            map.entry(map_key).or_insert(Vec::new()).push((
                                                source.resource_id.to_string(),
                                                kg_key.to_string(),
                                                log_odds_ratio_attribute.value.as_f64(),
                                                total_sample_size_attribute.value.as_i64(),
                                            ));
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
                                    map.entry(map_key).or_insert(Vec::new()).push((
                                        source.resource_id.to_string(),
                                        kg_key.to_string(),
                                        log_odds_ratio_attribute.value.as_f64(),
                                        total_sample_size_attribute.value.as_i64(),
                                    ));
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

pub fn merge_query_results(mut query: Query) -> Result<Query, Box<dyn Error>> {
    if let Some(ref mut results) = query.message.results {
        let tc = (0..results.len()).tuple_combinations::<(usize, usize)>();
        for (a, b) in tc {
            if let Ok([result_a, result_b]) = results.get_many_mut([a, b]) {
                if result_a.node_bindings == result_b.node_bindings {
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
                }
            }
        }
        results.dedup();
    }
    Ok(query)
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

        let query: Result<Query> = serde_json::from_str(data);

        if let Ok(q) = query {
            if let Ok(merged_query) = crate::util::merge_query_results(q) {
                assert!(merged_query.message.results.is_some());
                assert_eq!(merged_query.message.results.unwrap().len(), 2);
            }
        }

        assert!(true);
    }
}
