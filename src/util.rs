use crate::model::{AgentType, CQSCompositeScoreKey, CQSCompositeScoreValue, JobStatus, KnowledgeLevelType, QueryTemplate};
use crate::{job_actions, template, util, CQS_INFORES, REQWEST_CLIENT, WHITELISTED_TEMPLATE_QUERIES};
use chrono::Utc;
use futures::future::join_all;
use itertools::Itertools;
use merge_hashmap::Merge;
use rayon::prelude::*;
use serde_json::Value;
use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap};
use std::time::Duration;
use std::{env, fs};
use trapi_model_rs::{
    Analysis, AsyncQuery, Attribute, AuxiliaryGraph, BiolinkPredicate, Edge, EdgeBinding, KnowledgeType, Message, NodeBinding, QueryGraph, ResourceRoleEnum, Response,
};

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
    cqs_query: &Box<dyn template::CQSTemplate>,
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
                                        let mut analysis = Analysis::new(CQS_INFORES.clone(), edge_binding_map);
                                        analysis.scoring_method = Some("weighted average of log_odds_ratio".into());
                                        analysis.score = Some(cqs_query.compute_score(entry_values.clone()));
                                        debug!("analysis: {:?}", analysis);
                                        r.analyses.push(analysis);
                                    }
                                    _ => {
                                        let entry_key_inverse_searchable = CQSCompositeScoreKey::new(first_object_nb.id.to_string(), first_subject_nb.id.to_string());
                                        let entry = node_binding_to_log_odds_map.iter().find(|(k, _v)| **k == entry_key_inverse_searchable);

                                        if let Some((_entry_key, entry_values)) = entry {
                                            let kg_edge_keys: Vec<_> = entry_values.iter().map(|ev| EdgeBinding::new(ev.knowledge_graph_key.clone())).collect();
                                            let mut analysis = Analysis::new(CQS_INFORES.clone(), BTreeMap::from([(qg_key.clone(), kg_edge_keys)]));
                                            analysis.scoring_method = Some("weighted average of log_odds_ratio".into());
                                            analysis.score = Some(cqs_query.compute_score(entry_values.clone()));
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

pub fn correct_analysis_resource_id(message: &mut Message) {
    if let Some(results) = &mut message.results {
        //likely to have many results...do in parallel
        results.par_iter_mut().for_each(|r| {
            //not likely to have many analyses
            r.analyses.iter_mut().for_each(|a| {
                a.resource_id = CQS_INFORES.clone();
            })
        });
    }
}

pub fn add_support_graphs(response: &mut Response, query_graph: &QueryGraph, cqs_query: &Box<dyn template::CQSTemplate>, query_template: &QueryTemplate) {
    let mut auxiliary_graphs: BTreeMap<String, AuxiliaryGraph> = BTreeMap::new();

    let query_graph_edge_entry = query_graph.edges.iter().next().expect("Could not get QG edge");
    let query_edge = query_graph_edge_entry.1.clone();
    let query_edge_subject_id = query_edge.subject;
    let query_edge_object_id = query_edge.object;

    if let Some(results) = &mut response.message.results {
        for result in results {
            let mut new_node_bindings: BTreeMap<String, Vec<NodeBinding>> = BTreeMap::new();

            if let Some((_disease_node_binding_key, disease_node_binding_value)) = result.node_bindings.iter().find(|(k, _v)| **k == cqs_query.template_disease_node_id()) {
                // new_node_bindings.insert(cqs_query.inferred_disease_node_id(), disease_node_binding_value.to_vec());
                new_node_bindings.insert(query_edge_object_id.clone(), disease_node_binding_value.to_vec());
            }

            if let Some((_drug_node_binding_key, drug_node_binding_value)) = result.node_bindings.iter().find(|(k, _v)| **k == cqs_query.template_drug_node_id()) {
                // new_node_bindings.insert(cqs_query.inferred_drug_node_id(), drug_node_binding_value.to_vec());
                new_node_bindings.insert(query_edge_subject_id.clone(), drug_node_binding_value.to_vec());
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

            match (new_node_bindings.get(&query_edge_subject_id), new_node_bindings.get(&query_edge_object_id)) {
                (Some(drug_node_ids), Some(disease_node_ids)) => match (drug_node_ids.first(), disease_node_ids.first()) {
                    (Some(first_drug_node_id), Some(first_disease_node_id)) => {
                        let auxiliary_graph_ids: Vec<_> = local_auxiliary_graphs.clone().into_keys().collect();
                        let mut new_edge = trapi_model_rs::Edge::new(
                            first_drug_node_id.id.clone(),
                            BiolinkPredicate::from("biolink:treats"),
                            first_disease_node_id.id.clone(),
                            query_template.cqs.edge_sources.clone(),
                        );

                        let support_graphs_attribute = Attribute::new("biolink:support_graphs".to_string(), serde_json::Value::from(auxiliary_graph_ids));

                        let mut agent_type_attribute = Attribute::new("biolink:agent_type".to_string(), serde_json::Value::from(AgentType::ComputationalModel.to_string()));
                        agent_type_attribute.original_attribute_name = Some("biolink:agent_type".to_string());
                        agent_type_attribute.attribute_source = Some(CQS_INFORES.clone());

                        let mut knowledge_level_attribute =
                            Attribute::new("biolink:knowledge_level".to_string(), serde_json::Value::from(KnowledgeLevelType::Prediction.to_string()));
                        knowledge_level_attribute.original_attribute_name = Some("biolink:knowledge_level".to_string());
                        knowledge_level_attribute.attribute_source = Some(CQS_INFORES.clone());

                        let mut new_edge_attributes = vec![support_graphs_attribute, agent_type_attribute, knowledge_level_attribute];

                        if let Some(attribute_type_ids) = &query_template.cqs.attribute_type_ids {
                            if let Some(kg) = &mut response.message.knowledge_graph {
                                if let Some((_edge_key, edge_value)) = kg.edges.iter().find(|(_k, v)| v.object == first_disease_node_id.id && v.subject == first_drug_node_id.id) {
                                    if let Some(edge_attributes) = &edge_value.attributes {
                                        let edge_attributes_to_copy: Vec<Attribute> = edge_attributes
                                            .iter()
                                            .filter_map(|a| match attribute_type_ids.contains(&a.attribute_type_id) {
                                                true => Some(a.clone()),
                                                false => None,
                                            })
                                            .collect();
                                        new_edge_attributes.extend(edge_attributes_to_copy);
                                    }
                                }
                            }
                        }

                        new_edge.attributes = Some(new_edge_attributes);
                        // println!("new_edge: {:?}", new_edge);
                        if let Some(kg) = &mut response.message.knowledge_graph {
                            let new_kg_edge_id = uuid::Uuid::new_v4().to_string();
                            kg.edges.insert(new_kg_edge_id.clone(), new_edge);
                            result.analyses.retain(|analysis| analysis.edge_bindings.iter().all(|(_k, v)| !v.is_empty()));
                            result.analyses.iter_mut().for_each(|analysis| {
                                analysis.edge_bindings.clear();
                                analysis
                                    .edge_bindings
                                    .insert(query_graph_edge_entry.0.clone(), vec![EdgeBinding::new(new_kg_edge_id.clone())]);
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

pub fn compute_composite_score(entry_values: Vec<CQSCompositeScoreValue>) -> f64 {
    let total_sample_sizes: Vec<_> = entry_values.iter().filter_map(|ev| ev.total_sample_size).collect();
    let sum_of_total_sample_sizes: i64 = total_sample_sizes.iter().sum(); // (N1 + N2 + N3)
    let weights: Vec<_> = entry_values
        .iter()
        .map(|ev| ev.total_sample_size.unwrap() as f64 / sum_of_total_sample_sizes as f64)
        .collect();
    let sum_of_weights = weights.iter().sum::<f64>(); // (W1 + W2 + W3)

    let score_numerator = entry_values
        .iter()
        .map(|ev| (ev.total_sample_size.unwrap() as f64 / sum_of_total_sample_sizes as f64) * ev.log_odds_ratio.unwrap())
        .sum::<f64>(); // (W1 * OR1 + W2 * OR2 + W3 * OR3)

    let score = score_numerator / sum_of_weights;
    let score_abs = score.abs();

    if score_abs.is_nan() {
        0.01_f64.atan() * 2.0 / std::f64::consts::PI
    } else {
        score.atan() * 2.0 / std::f64::consts::PI
    }
}

pub async fn process(query_graph: &QueryGraph, cqs_query: &Box<dyn template::CQSTemplate>, ids: &Vec<trapi_model_rs::CURIE>) -> Option<Response> {
    let request_client = REQWEST_CLIENT.get().await;

    let workflow_runner_url = format!(
        "{}/query",
        env::var("WORKFLOW_RUNNER_URL").unwrap_or("https://translator-workflow-runner.renci.org".to_string())
    );

    let backoff_multiplier = 2;
    let retries = 3;

    let mut query_template: QueryTemplate = cqs_query.render_query_template(ids.clone());

    let attribute_constraint = query_template.first_edge_attribute_constraint();

    query_template.remove_edge_attribute_constraints();
    let query = query_template.to_query();
    info!("cqs_query {} being sent to WFR: {}", cqs_query.name(), serde_json::to_string(&query).unwrap());

    let mut trapi_response = None;
    for attempt in 1..=retries {
        debug!("attempt: {} for cqs_query.name(): {}", attempt, cqs_query.name());

        let wfr_response_result = request_client.post(workflow_runner_url.clone()).json(&query).send().await;
        let wfr_response: Option<Response> = match wfr_response_result {
            Ok(response) => {
                info!("WFR response.status(): {} for query {} ", response.status(), cqs_query.name());
                let result_data = response.text().await;
                match result_data {
                    Ok(data) => Some(serde_json::from_str(data.as_str()).expect("could not parse Query")),
                    Err(e) => {
                        warn!("Error reading response from WFR: {}", e);
                        None
                    }
                }
            }
            Err(e) => {
                warn!("Failed to send query to WFR: {}", e);
                None
            }
        };
        if let Some(r) = wfr_response {
            trapi_response = Some(r);
            break;
        } else {
            let retry_backoff_sleep_duration = attempt * backoff_multiplier * 15;
            debug!("retry_backoff_sleep_duration: {}", retry_backoff_sleep_duration);
            tokio::time::sleep(Duration::from_secs(retry_backoff_sleep_duration)).await;
        }
    }

    if let Some(mut tr) = trapi_response {
        let uuid = uuid::Uuid::new_v4().to_string();
        if let Ok(wfr_output_dir) = env::var("WFR_OUTPUT_DIR") {
            let parent_dir = std::path::Path::new(&wfr_output_dir);
            if !parent_dir.exists() {
                fs::create_dir_all(parent_dir).expect(format!("Could not create directory: {:?}", parent_dir).as_str());
            }
            fs::write(
                std::path::Path::join(parent_dir, format!("{}-{}-pre.json", cqs_query.name(), uuid).as_str()),
                serde_json::to_string_pretty(&tr).unwrap(),
            )
                .expect("failed to write output");
        }

        if let (Some(ac), Some(kg)) = (attribute_constraint, &mut tr.message.knowledge_graph) {
            let edge_keys_to_remove = find_edge_keys_to_remove(ac.clone(), &kg.edges);
            debug!("{}, {:?}, removing edges: {:?}", cqs_query.name(), ac, edge_keys_to_remove);
            for ek in edge_keys_to_remove.iter() {
                kg.edges.remove(ek);
            }

            if let Some(results) = &mut tr.message.results {
                let mut results_to_remove = vec![];
                for result in results.iter() {
                    result
                        .analyses
                        .iter()
                        .filter(|a| {
                            a.edge_bindings
                                .iter()
                                .any(|(_eb_key, eb_value)| eb_value.iter().any(|eb| edge_keys_to_remove.contains(&eb.id)))
                        })
                        .for_each(|_a| results_to_remove.push(result.clone()));
                }
                results.retain(|r| !results_to_remove.contains(r));
            }
        }

        add_support_graphs(&mut tr, query_graph, cqs_query, &query_template);

        sort_analysis_by_score(&mut tr.message);
        sort_results_by_analysis_score(&mut tr.message);

        if let Some(results) = &mut tr.message.results {
            if let Some(limit) = query_template.cqs.results_limit {
                results.truncate(limit);
            }
        }

        if let Ok(wfr_output_dir) = env::var("WFR_OUTPUT_DIR") {
            let parent_dir = std::path::Path::new(&wfr_output_dir);
            if !parent_dir.exists() {
                fs::create_dir_all(parent_dir).expect(format!("Could not create directory: {:?}", parent_dir).as_str());
            }
            fs::write(
                std::path::Path::join(parent_dir, format!("{}-{}-post.json", cqs_query.name(), uuid).as_str()),
                serde_json::to_string_pretty(&tr).unwrap(),
            )
                .expect("failed to write output");
        }

        Some(tr)
    } else {
        None
    }

    // let node_binding_to_log_odds_map = util::build_node_binding_to_log_odds_data_map(canned_query_response.message.clone());
    // let trapi_response = util::add_composite_score_attributes(canned_query_response, node_binding_to_log_odds_map, &cqs_query);
    // Some(trapi_response)
}

pub async fn process_asyncquery_jobs() {
    debug!("processing asyncquery jobs");

    if let Ok(mut undone_jobs) = job_actions::find_undone().await {
        for job in undone_jobs.iter_mut() {
            info!("Processing Job: {}", job.id);

            job.date_started = Some(Utc::now().naive_utc());
            job.status = JobStatus::Running;
            job_actions::update(job).await;

            let query: AsyncQuery = serde_json::from_str(&*String::from_utf8_lossy(job.query.as_slice())).expect("Could not deserialize AsyncQuery");

            let mut responses: Vec<trapi_model_rs::Response> = vec![];

            if let Some(query_graph) = &query.message.query_graph {
                if let Some((_edge_key, edge_value)) = &query_graph.edges.iter().find(|(_k, v)| {
                    if let (Some(predicates), Some(knowledge_type)) = (&v.predicates, &v.knowledge_type) {
                        if predicates.contains(&"biolink:treats".to_string()) && knowledge_type == &KnowledgeType::INFERRED {
                            return true;
                        }
                    }
                    return false;
                }) {
                    if let Some((_node_key, node_value)) = &query_graph.nodes.iter().find(|(k, _v)| *k == &edge_value.object) {
                        if let Some(ids) = &node_value.ids {
                            let future_responses: Vec<_> = WHITELISTED_TEMPLATE_QUERIES.iter().map(|cqs_query| util::process(&query_graph, cqs_query, &ids)).collect();
                            let joined_future_responses = join_all(future_responses).await;
                            joined_future_responses
                                .into_iter()
                                .filter_map(std::convert::identity)
                                .for_each(|trapi_response| responses.push(trapi_response));
                        }
                    }
                }
            }

            if responses.is_empty() {
                job.date_finished = Some(Utc::now().naive_utc());
                job.status = JobStatus::Failed;
                job_actions::update(job).await;
            } else {
                let mut message = query.message.clone();

                responses.into_iter().for_each(|r| {
                    message.merge(r.message);
                });

                sort_analysis_by_score(&mut message);
                sort_results_by_analysis_score(&mut message);
                correct_analysis_resource_id(&mut message);

                // if let Some(results) = &mut message.results {
                //     results.truncate(250);
                // }

                // let node_binding_to_log_odds_map = util::build_node_binding_to_log_odds_data_map(&message.knowledge_graph);
                // let message_with_score_attributes = util::add_composite_score_attributes(message, node_binding_to_log_odds_map);
                // let mut ret = trapi_model_rs::Response::new(message_with_score_attributes);
                let mut res = Response::new(message);
                res.status = Some("Success".to_string());
                res.workflow = query.workflow.clone();
                res.biolink_version = Some(env::var("BIOLINK_VERSION").unwrap_or("3.1.2".to_string()));
                res.schema_version = Some(env::var("TRAPI_VERSION").unwrap_or("1.4.0".to_string()));

                job.response = Some(serde_json::to_string(&res).unwrap().into_bytes());
                job.date_finished = Some(Utc::now().naive_utc());
                job.status = JobStatus::Completed;
                job_actions::update(job).await;

                send_callback(query, res).await;
            }
        }
    } else {
        warn!("No Jobs to run");
    }
}

pub async fn send_callback(query: AsyncQuery, ret: Response) {
    let request_client = REQWEST_CLIENT.get().await;
    // 1st attempt
    info!("1st attempt at sending response to: {}", &query.callback);
    match request_client.post(&query.callback).json(&ret).timeout(Duration::from_secs(10)).send().await {
        Ok(first_attempt_callback_response) => {
            let first_attempt_status_code = first_attempt_callback_response.status();
            debug!("first_attempt_status_code: {}", first_attempt_status_code);
            if !first_attempt_status_code.is_success() {
                // update_job(job, JobStatus::Failed);
                warn!("failed to make 1st callback post");
                tokio::time::sleep(Duration::from_secs(10)).await;
                // 2st attempt
                info!("2nd attempt at sending response to: {}", &query.callback);
                match request_client.post(&query.callback).json(&ret).timeout(Duration::from_secs(10)).send().await {
                    Ok(second_attempt_callback_response) => {
                        let second_attempt_status_code = second_attempt_callback_response.status();
                        debug!("second_attempt_status_code: {}", second_attempt_status_code);
                        if second_attempt_status_code.is_success() {
                            // update_job(job, JobStatus::Completed);
                        } else {
                            warn!("failed to make 2nd callback post");
                        }
                    }
                    Err(e) => {
                        warn!("2nd attempt at callback error: {}", e);
                    }
                }
            }
        }
        Err(e) => {
            warn!("1st attempt at callback error: {}", e);
        }
    }
}

pub async fn delete_stale_asyncquery_jobs() {
    debug!("deleting stale asyncquery jobs");
    // let mut connection = AsyncPgConnection::establish(&std::env::var("DATABASE_URL")?).await?;
    if let Ok(jobs) = job_actions::find_all(None).await {
        let now = Utc::now().naive_utc();
        let futures: Vec<_> = jobs
            .iter()
            .filter(|j| {
                let diff = now - j.date_submitted;
                diff.num_seconds() > 3600
            })
            .map(|j| job_actions::delete(&j.id))
            .collect();
        let _ = join_all(futures);
    }
}

pub fn find_edge_keys_to_remove(ac: trapi_model_rs::AttributeConstraint, edge_map: &HashMap<String, Edge>) -> Vec<String> {
    let mut to_remove = vec![];

    match ac.operator.as_str() {
        ">" => {
            edge_map.iter().for_each(|(k, v)| {
                if let Some(edge_attributes) = &v.attributes {
                    if let Some(edge_attribute) = edge_attributes.iter().find(|edge_attribute| edge_attribute.attribute_type_id == ac.id) {
                        match &edge_attribute.value {
                            Value::Null => {}
                            Value::Bool(_) => {}
                            Value::Number(ev) => {
                                if let Some(ac_v) = ac.value.as_i64() {
                                    if ev.as_i64().unwrap() <= ac_v {
                                        to_remove.push(k.clone());
                                    }
                                }
                            }
                            Value::String(ev) => {
                                if let Some(ac_v) = ac.value.as_i64() {
                                    if ev.parse::<i64>().unwrap() <= ac_v {
                                        to_remove.push(k.clone());
                                    }
                                }
                            }
                            Value::Array(ev) => {
                                if !ev.is_empty() {
                                    if let (Some(first), Some(ac_v)) = (ev.first(), ac.value.as_i64()) {
                                        if first.as_i64().unwrap() <= ac_v {
                                            to_remove.push(k.clone());
                                        }
                                    }
                                }
                            }
                            Value::Object(_) => {}
                        }
                    }
                }
            });
        }
        "<" => {
            edge_map.iter().for_each(|(k, v)| {
                if let Some(edge_attributes) = &v.attributes {
                    if let Some(edge_attribute) = edge_attributes.iter().find(|edge_attribute| edge_attribute.attribute_type_id == ac.id) {
                        match &edge_attribute.value {
                            Value::Null => {}
                            Value::Bool(_) => {}
                            Value::Number(ev) => {
                                if let Some(ac_v) = ac.value.as_i64() {
                                    if ev.as_i64().unwrap() >= ac_v {
                                        to_remove.push(k.clone());
                                    }
                                }
                            }
                            Value::String(ev) => {
                                if let Some(ac_v) = ac.value.as_i64() {
                                    if ev.parse::<i64>().unwrap() >= ac_v {
                                        to_remove.push(k.clone());
                                    }
                                }
                            }
                            Value::Array(ev) => {
                                if !ev.is_empty() {
                                    if let (Some(first), Some(ac_v)) = (ev.first(), ac.value.as_i64()) {
                                        if first.as_i64().unwrap() >= ac_v {
                                            to_remove.push(k.clone());
                                        }
                                    }
                                }
                            }
                            Value::Object(_) => {}
                        }
                    }
                }
            });
        }
        "==" => {
            edge_map.iter().for_each(|(k, v)| {
                if let Some(edge_attributes) = &v.attributes {
                    if let Some(edge_attribute) = edge_attributes.iter().find(|edge_attribute| edge_attribute.attribute_type_id == ac.id) {
                        match &edge_attribute.value {
                            Value::Null => {}
                            Value::Bool(ev) => {
                                if let Some(ac_v) = &ac.value.as_bool() {
                                    if ev != ac_v {
                                        to_remove.push(k.clone());
                                    }
                                }
                            }
                            Value::Number(ev) => {
                                if let Some(ac_v) = ac.value.as_i64() {
                                    if ev.as_i64().unwrap() != ac_v {
                                        to_remove.push(k.clone());
                                    }
                                }
                            }
                            Value::String(ev) => {
                                if let Some(ac_v) = ac.value.as_str() {
                                    if ev != ac_v {
                                        to_remove.push(k.clone());
                                    }
                                }
                            }
                            Value::Array(ev) => {
                                // assuming that 'ev' is a vector of strings since the AttributeConstraint.value will likely be a vector of strings
                                let ev_strings = ev.iter().map(|v| v.as_str().unwrap()).collect_vec();
                                let ac_array = ac.value.as_array().unwrap();
                                let ac_strings = ac_array.iter().map(|v| v.as_str().unwrap()).collect_vec();

                                if !ev_strings.iter().any(|x| ac_strings.contains(x)) {
                                    // info!("!ev_strings.iter().any(|x| ac_strings.contains(x)) is true: {}", k.clone());
                                    to_remove.push(k.clone());
                                }
                            }
                            Value::Object(ev) => {
                                // FIXME this feels too restrictive/inaccurate
                                if let Some(ac_v) = ac.value.as_object() {
                                    if ev != ac_v {
                                        to_remove.push(k.clone());
                                    }
                                }
                            }
                        }
                    }
                }
            });
        }
        "===" => {
            edge_map.iter().for_each(|(k, v)| {
                if let Some(edge_attributes) = &v.attributes {
                    if let Some(edge_attribute) = edge_attributes.iter().find(|edge_attribute| edge_attribute.attribute_type_id == ac.id) {
                        match &edge_attribute.value {
                            Value::Null => {}
                            Value::Bool(ev) => {
                                if let Some(ac_v) = &ac.value.as_bool() {
                                    if ev != ac_v {
                                        to_remove.push(k.clone());
                                    }
                                }
                            }
                            Value::Number(ev) => {
                                if let Some(ac_v) = ac.value.as_i64() {
                                    if ev.as_i64().unwrap() != ac_v {
                                        to_remove.push(k.clone());
                                    }
                                }
                            }
                            Value::String(ev) => {
                                if let Some(ac_v) = ac.value.as_str() {
                                    if ev != ac_v {
                                        to_remove.push(k.clone());
                                    }
                                }
                            }
                            Value::Array(ev) => {
                                if let Some(ac_v) = ac.value.as_array() {
                                    if ev != ac_v {
                                        to_remove.push(k.clone());
                                    }
                                }
                            }
                            Value::Object(ev) => {
                                if let Some(ac_v) = ac.value.as_object() {
                                    if ev != ac_v {
                                        to_remove.push(k.clone());
                                    }
                                }
                            }
                        }
                    }
                }
            });
        }
        "matches" => {
            edge_map.iter().for_each(|(k, v)| {
                if let Some(edge_attributes) = &v.attributes {
                    if let Some(edge_attribute) = edge_attributes.iter().find(|edge_attribute| edge_attribute.attribute_type_id == ac.id) {
                        if let Ok(re) = regex::Regex::new(ac.value.as_str().unwrap()) {
                            if !re.is_match(edge_attribute.value.as_str().unwrap()) {
                                to_remove.push(k.clone());
                            }
                        }
                    }
                }
            });
        }
        &_ => {}
    }

    // info!("removing: {:?}", to_remove);
    // for k in to_remove.iter() {
    //     edge_map.remove(k);
    // }
    to_remove
}

#[cfg(test)]
mod test {
    use crate::model::{CQSCompositeScoreKey, CQSCompositeScoreValue};
    use crate::template;
    use crate::template::CQSTemplate;
    use crate::util::{add_support_graphs, build_node_binding_to_log_odds_data_map, find_edge_keys_to_remove};
    use itertools::Itertools;
    use merge_hashmap::Merge;
    use serde_json::{json, Result, Value};
    use std::cmp::Ordering;
    use std::collections::{BTreeMap, HashMap};
    use std::fmt::Debug;
    use std::fs;
    use std::ops::Deref;
    use std::path::Path;
    use trapi_model_rs::{
        Analysis, Attribute, AttributeConstraint, AuxiliaryGraph, BiolinkPredicate, Edge, EdgeBinding, NodeBinding, Query, ResourceRoleEnum, Response, RetrievalSource, CURIE,
    };
    use uuid::uuid;

    #[test]
    fn attribute_constraint_array_equals() {
        let mut edge_map: HashMap<String, Edge> = serde_json::from_value(json!({
            "76fc96c78b9d": {
              "subject": "PUBCHEM.COMPOUND:158781",
              "predicate": "biolink:affects",
              "object": "CHEMBL.TARGET:CHEMBL227",
              "sources": [],
              "attributes": [
                {
                  "attribute_type_id": "biolink:max_research_phase",
                  "original_attribute_name": "max_research_phase",
                  "value": [ "2.0" ],
                  "value_type_id": "EDAM:data_1772"
                },
              ]
            }
        }))
            .unwrap();

        let ac = AttributeConstraint::new("biolink:evidence_count".to_string(), "asdf".to_string(), "==".to_string(), vec!["1.0", "2.0"].into());
        let edge_keys_to_remove = find_edge_keys_to_remove(ac, &mut edge_map);
        for ek in edge_keys_to_remove.iter() {
            edge_map.remove(ek);
        }
        assert_eq!(1, edge_map.len());

        let ac = AttributeConstraint::new(
            "biolink:max_research_phase".to_string(),
            "asdf".to_string(),
            "==".to_string(),
            vec!["clinical_trial_phase_1", "clinical_trial_phase_2", "clinical_trial_phase_3"].into(),
        );
        let edge_keys_to_remove = find_edge_keys_to_remove(ac, &mut edge_map);
        for ek in edge_keys_to_remove.iter() {
            edge_map.remove(ek);
        }
        assert_eq!(0, edge_map.len());
    }

    #[test]
    fn attribute_constraint_string_equals() {
        let mut edge_map: HashMap<String, Edge> = serde_json::from_value(json!({
            "76fc96c78b9d": {
              "subject": "PUBCHEM.COMPOUND:158781",
              "predicate": "biolink:affects",
              "object": "CHEMBL.TARGET:CHEMBL227",
              "sources": [],
              "attributes": [
                {
                  "attribute_type_id": "biolink:evidence_count",
                  "original_attribute_name": "evidence_count",
                  "value": "qwer",
                  "value_type_id": "EDAM:data_1772"
                },
              ]
            }
        }))
            .unwrap();

        let ac = AttributeConstraint::new("biolink:evidence_count".to_string(), "asdf".to_string(), "==".to_string(), "qwer".into());
        let edge_keys_to_remove = find_edge_keys_to_remove(ac, &mut edge_map);
        for ek in edge_keys_to_remove.iter() {
            edge_map.remove(ek);
        }
        assert_eq!(1, edge_map.len());

        let ac = AttributeConstraint::new("biolink:evidence_count".to_string(), "asdf".to_string(), "==".to_string(), "zxcv".into());
        let edge_keys_to_remove = find_edge_keys_to_remove(ac, &mut edge_map);
        for ek in edge_keys_to_remove.iter() {
            edge_map.remove(ek);
        }
        assert_eq!(0, edge_map.len());
    }

    #[test]
    fn attribute_constraint_numeric_equals() {
        let mut edge_map: HashMap<String, Edge> = serde_json::from_value(json!({
            "76fc96c78b9d": {
              "subject": "PUBCHEM.COMPOUND:158781",
              "predicate": "biolink:affects",
              "object": "CHEMBL.TARGET:CHEMBL227",
              "sources": [],
              "attributes": [
                {
                  "attribute_type_id": "biolink:evidence_count",
                  "original_attribute_name": "evidence_count",
                  "value": 100,
                  "value_type_id": "EDAM:data_1772"
                },
              ]
            }
        }))
            .unwrap();

        let ac = AttributeConstraint::new("biolink:evidence_count".to_string(), "asdf".to_string(), "==".to_string(), 100.into());
        let edge_keys_to_remove = find_edge_keys_to_remove(ac, &mut edge_map);
        for ek in edge_keys_to_remove.iter() {
            edge_map.remove(ek);
        }
        assert_eq!(1, edge_map.len());

        let ac = AttributeConstraint::new("biolink:evidence_count".to_string(), "asdf".to_string(), "==".to_string(), 200.into());
        let edge_keys_to_remove = find_edge_keys_to_remove(ac, &mut edge_map);
        for ek in edge_keys_to_remove.iter() {
            edge_map.remove(ek);
        }
        assert_eq!(0, edge_map.len());
    }

    #[test]
    fn attribute_constraint_gt() {
        let mut edge_map: HashMap<String, Edge> = serde_json::from_value(json!({
            "76fc96c78b9d": {
              "subject": "PUBCHEM.COMPOUND:158781",
              "predicate": "biolink:affects",
              "object": "CHEMBL.TARGET:CHEMBL227",
              "sources": [],
              "attributes": [
                {
                  "attribute_type_id": "biolink:evidence_count",
                  "original_attribute_name": "evidence_count",
                  "value": 100,
                  "value_type_id": "EDAM:data_1772"
                },
              ]
            }
        }))
            .unwrap();

        let ac = AttributeConstraint::new("biolink:evidence_count".to_string(), "asdf".to_string(), ">".to_string(), 20.into());
        let edge_keys_to_remove = find_edge_keys_to_remove(ac, &mut edge_map);
        for ek in edge_keys_to_remove.iter() {
            edge_map.remove(ek);
        }
        assert_eq!(1, edge_map.len());

        let ac = AttributeConstraint::new("biolink:evidence_count".to_string(), "asdf".to_string(), ">".to_string(), 200.into());
        let edge_keys_to_remove = find_edge_keys_to_remove(ac, &mut edge_map);
        for ek in edge_keys_to_remove.iter() {
            edge_map.remove(ek);
        }
        assert_eq!(0, edge_map.len());
    }

    #[test]
    fn attribute_constraint_lt() {
        let mut edge_map: HashMap<String, Edge> = serde_json::from_value(json!({
            "76fc96c78b9d": {
              "subject": "PUBCHEM.COMPOUND:158781",
              "predicate": "biolink:affects",
              "object": "CHEMBL.TARGET:CHEMBL227",
              "sources": [],
              "attributes": [
                {
                  "attribute_type_id": "biolink:evidence_count",
                  "original_attribute_name": "evidence_count",
                  "value": 100,
                  "value_type_id": "EDAM:data_1772"
                },
              ]
            }
        }))
            .unwrap();

        let ac = AttributeConstraint::new("biolink:evidence_count".to_string(), "asdf".to_string(), "<".to_string(), 200.into());
        let edge_keys_to_remove = find_edge_keys_to_remove(ac, &mut edge_map);
        for ek in edge_keys_to_remove.iter() {
            edge_map.remove(ek);
        }
        assert_eq!(1, edge_map.len());

        let ac = AttributeConstraint::new("biolink:evidence_count".to_string(), "asdf".to_string(), "<".to_string(), 20.into());
        let edge_keys_to_remove = find_edge_keys_to_remove(ac, &mut edge_map);
        for ek in edge_keys_to_remove.iter() {
            edge_map.remove(ek);
        }
        assert_eq!(0, edge_map.len());
    }

    #[test]
    fn attribute_constraint_matches() {
        let mut edge_map: HashMap<String, Edge> = serde_json::from_value(json!({
            "76fc96c78b9d": {
              "subject": "PUBCHEM.COMPOUND:158781",
              "predicate": "biolink:affects",
              "object": "CHEMBL.TARGET:CHEMBL227",
              "sources": [],
              "attributes": [
                {
                  "attribute_type_id": "biolink:asdf",
                  "original_attribute_name": "asdf",
                  "value": "123asdf456",
                  "value_type_id": "EDAM:data_1772"
                },
              ]
            }
        }))
            .unwrap();

        let ac = AttributeConstraint::new("biolink:asdf".to_string(), "asdf".to_string(), "matches".to_string(), "^.+asdf.+$".into());
        let edge_keys_to_remove = find_edge_keys_to_remove(ac, &mut edge_map);
        for ek in edge_keys_to_remove.iter() {
            edge_map.remove(ek);
        }
        assert_eq!(1, edge_map.len());

        let ac = AttributeConstraint::new("biolink:asdf".to_string(), "asdf".to_string(), "matches".to_string(), "^.+zxcv.+$".into());
        let edge_keys_to_remove = find_edge_keys_to_remove(ac, &mut edge_map);
        for ek in edge_keys_to_remove.iter() {
            edge_map.remove(ek);
        }
        assert_eq!(0, edge_map.len());
    }

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
    fn test_add_aux_graphs() {
        let data = fs::read_to_string(Path::new("/tmp/cqs/a3522bf3-6c73-4ed4-98f4-aada6746ed1d.json")).unwrap();
        // let data = fs::read_to_string(Path::new("/tmp/cqs/fa62acca-ce27-4b7d-8d84-22ab4906bdcc.json")).unwrap();

        let mut response: Response = serde_json::from_str(data.as_str()).unwrap();

        let cqs_query = template::ClinicalKPs::new();
        let mut new_results: Vec<trapi_model_rs::Result> = vec![];
        let mut auxiliary_graphs: BTreeMap<String, AuxiliaryGraph> = BTreeMap::new();

        if let (Some(results), Some(query_graph)) = (&mut response.message.results, &response.message.query_graph) {
            let query_graph_edge_entry = query_graph.edges.iter().next().expect("Could not get edge");
            let query_edge = query_graph_edge_entry.1.clone();
            let query_edge_subject_id = query_edge.subject;
            let query_edge_object_id = query_edge.object;

            for result in results {
                let mut new_node_bindings: BTreeMap<String, Vec<NodeBinding>> = BTreeMap::new();

                // ($foo:ident, $bar:literal, $inferred_drug_node_id:literal, $inferred_predicate_id:literal, $inferred_disease_node_id:literal, $template_drug_node_id:literal, $template_disease_node_id:literal, $func:expr) => {
                // crate::impl_wrapper!(CQSQueryA, "a", "n0", "e0", "n1", "n3", "n0", compute_composite_score);

                if let Some((disease_node_binding_key, disease_node_binding_value)) = result.node_bindings.iter().find(|(k, v)| **k == cqs_query.template_disease_node_id()) {
                    // println!("disease_node_binding_value: {:?}", disease_node_binding_value);
                    new_node_bindings.insert(query_edge_subject_id.clone(), disease_node_binding_value.to_vec());
                }

                if let Some((drug_node_binding_key, drug_node_binding_value)) = result.node_bindings.iter().find(|(k, v)| **k == cqs_query.template_drug_node_id()) {
                    // println!("drug_node_binding_value: {:?}", drug_node_binding_value);
                    new_node_bindings.insert(query_edge_object_id.clone(), drug_node_binding_value.to_vec());
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

                match (new_node_bindings.get(&query_edge_object_id), new_node_bindings.get(&query_edge_subject_id)) {
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
                                            .insert(query_graph_edge_entry.0.clone(), vec![EdgeBinding::new(new_kg_edge_id.clone())]);
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
                // for result in new_results.iter_mut() {
                //     let analyses: Vec<_> = results
                //         .iter()
                //         .filter(|orig| result.node_bindings == orig.node_bindings)
                //         .flat_map(|r| r.analyses.clone())
                //         .collect();
                //
                //     let asdf = analyses.into_iter().map(|a| ((a.resource_id.clone(), OrderedFloat(a.score.unwrap())), a)).into_group_map();
                //
                //     for ((resource_id, score), v) in asdf.into_iter() {
                //         match v.len() {
                //             1 => {
                //                 result.analyses.extend(v);
                //             }
                //             _ => {
                //                 let edge_binding_map = v
                //                     .iter()
                //                     .flat_map(|a| {
                //                         a.edge_bindings
                //                             .iter()
                //                             .flat_map(|(eb_key, eb_value)| eb_value.iter().map(|eb| (eb_key.clone(), eb.clone())).collect::<Vec<_>>())
                //                             .collect::<Vec<_>>()
                //                     })
                //                     .into_group_map();
                //
                //                 if let Some(analysis) = v.iter().next() {
                //                     let mut a = analysis.clone();
                //                     a.edge_bindings = edge_binding_map.into_iter().map(|(k, v)| (k, v.into_iter().collect())).collect();
                //                     result.analyses.push(a);
                //                 }
                //             }
                //         }
                //     }
                //
                //     // println!("{:?}", asdf);
                // }
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

        let cqs_query = template::ClinicalKPs::new();
        let score = cqs_query.compute_score(values);
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
                let cqs_query = template::ClinicalKPs::new();

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
                                                let score = cqs_query.compute_score(entry_values.clone());
                                                println!("score: {:?}", score);
                                                // subject: "MONDO:0009061", object: "PUBCHEM.COMPOUND:16220172"
                                                if first_subject_nb.id == "MONDO:0009061" && first_object_nb.id == "PUBCHEM.COMPOUND:16220172" {
                                                    println!("GOT HERE");
                                                }

                                                let kg_edge_keys: Vec<_> = entry_values.iter().map(|ev| EdgeBinding::new(ev.knowledge_graph_key.clone())).collect();
                                                let mut analysis = Analysis::new("infores:cqs".into(), BTreeMap::from([(qg_key.clone(), kg_edge_keys)]));
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
                                                    let score = cqs_query.compute_score(entry_values.clone());
                                                    println!("score: {:?}", score);

                                                    let kg_edge_keys: Vec<_> = entry_values.iter().map(|ev| EdgeBinding::new(ev.knowledge_graph_key.clone())).collect();
                                                    let mut analysis = Analysis::new("infores:cqs".into(), BTreeMap::from([(qg_key.clone(), kg_edge_keys)]));
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
