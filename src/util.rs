use crate::model::Query;
use itertools::Itertools;
use std::error::Error;

pub fn merge_query_results(mut query: Query) -> Result<Query, Box<dyn Error>> {
    if let Some(ref mut results) = query.message.results {
        let tc = (0..results.len()).tuple_combinations::<(usize, usize)>();
        for (a, b) in tc {
            if let Ok([result_a, result_b]) = results.get_many_mut([a, b]) {
                if result_a.score.is_none() && result_b.score.is_none() && result_a.node_bindings == result_b.node_bindings {
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
        results.dedup();
    }
    Ok(query)
}

#[cfg(test)]
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
