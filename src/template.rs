use crate::model::CQSCompositeScoreValue;
use crate::model::QueryTemplate;
use crate::util;
use std::fs;

pub trait CQSTemplate: Send + Sync {
    fn name(&self) -> String;
    fn render_query_template(&self, ids: Vec<trapi_model_rs::CURIE>) -> QueryTemplate;
    fn inferred_drug_node_id(&self) -> String;
    fn inferred_predicate_id(&self) -> String;
    fn inferred_disease_node_id(&self) -> String;
    fn template_drug_node_id(&self) -> String;
    fn template_disease_node_id(&self) -> String;
    fn compute_score(&self, entry_values: Vec<CQSCompositeScoreValue>) -> f64;
}

macro_rules! impl_wrapper {
    ($name:ident, $file:literal, $inferred_drug_node_id:literal, $inferred_predicate_id:literal, $inferred_disease_node_id:literal, $template_drug_node_id:literal, $template_disease_node_id:literal, $func:expr) => {
        pub struct $name {}

        impl $name {
            pub fn new() -> $name {
                $name {}
            }
        }

        impl CQSTemplate for $name {
            fn name(&self) -> String {
                stringify!($name).to_string()
            }

            fn inferred_drug_node_id(&self) -> String {
                $inferred_drug_node_id.to_string()
            }

            fn inferred_predicate_id(&self) -> String {
                $inferred_predicate_id.to_string()
            }

            fn inferred_disease_node_id(&self) -> String {
                $inferred_disease_node_id.to_string()
            }

            fn template_drug_node_id(&self) -> String {
                $template_drug_node_id.to_string()
            }

            fn template_disease_node_id(&self) -> String {
                $template_disease_node_id.to_string()
            }

            fn render_query_template(&self, ids: Vec<trapi_model_rs::CURIE>) -> QueryTemplate {
                let file = format!("./templates/mvp1-templates/{}", $file.to_string());
                let mut query: QueryTemplate = serde_json::from_str(&fs::read_to_string(file).unwrap()).unwrap();
                if let Some(qg) = &mut query.message.query_graph {
                    if let Some(q_node) = qg.nodes.get_mut($template_disease_node_id) {
                        q_node.ids = Some(ids);
                    }
                }
                query
            }

            fn compute_score(&self, entry_values: Vec<CQSCompositeScoreValue>) -> f64 {
                $func(entry_values)
            }
        }
    };
}

impl_wrapper!(
    ClinicalKPs,
    "mvp1-template1-clinical-kps/mvp1-template1-clinical-kps.json",
    "n0", //inferred_drug_node_id
    "e0", //inferred_predicate_id
    "n1", //inferred_disease_node_id
    "n3", //template_drug_node_id
    "n0", //template_disease_node_id
    util::compute_composite_score
);
// impl_wrapper!(
//     ConnectionHypothesis,
//     "mvp1-template2-connections-hypothesis/mvp1-template2-connection-hypothesis.json",
//     "n0", //inferred_drug_node_id
//     "e0", //inferred_predicate_id
//     "n1", //inferred_disease_node_id
//     "n0", //template_drug_node_id
//     "n1", //template_disease_node_id
//     compute_composite_score
// );
impl_wrapper!(
    OpenPredict,
    "mvp1-template3-openpredict/mvp1-template3-openpredict.json",
    "n0", //inferred_drug_node_id
    "e0", //inferred_predicate_id
    "n1", //inferred_disease_node_id
    "n0", //template_drug_node_id
    "n1", //template_disease_node_id
    util::compute_composite_score
);
impl_wrapper!(
    ServiceProviderAeolus,
    "mvp1-template4-service-provider-aeolus/mvp1-template4-service-provider-aeolus.json",
    "n0", //inferred_drug_node_id
    "e0", //inferred_predicate_id
    "n1", //inferred_disease_node_id
    "n0", //template_drug_node_id
    "n1", //template_disease_node_id
    util::compute_composite_score
);
impl_wrapper!(
    SpokeChembl,
    "mvp1-template5-spoke-chembl/mvp1-template5-spoke-chembl.json",
    "n0",  //inferred_drug_node_id
    "e0",  //inferred_predicate_id
    "n1",  //inferred_disease_node_id
    "n00", //template_drug_node_id
    "n01", //template_disease_node_id
    util::compute_composite_score
);
impl_wrapper!(
    MoleProChembl,
    "mvp1-template6-molepro-chembl/mvp1-template6-molepro-chembl.json",
    "n0",  //inferred_drug_node_id
    "e0",  //inferred_predicate_id
    "n1",  //inferred_disease_node_id
    "n00", //template_drug_node_id
    "n01", //template_disease_node_id
    util::compute_composite_score
);
impl_wrapper!(
    RTXKG2SemMed,
    "mvp1-template7-rtxkg2-semmed/mvp1-template7-rtxkg2-semmed.json",
    "n0", //inferred_drug_node_id
    "e0", //inferred_predicate_id
    "n1", //inferred_disease_node_id
    "n0", //template_drug_node_id
    "n1", //template_disease_node_id
    util::compute_composite_score
);
impl_wrapper!(
    ServiceProviderSemMed,
    "mvp1-template8-service-provider-semmed/mvp1-template8-service-provider-semmed.json",
    "n0", //inferred_drug_node_id
    "e0", //inferred_predicate_id
    "n1", //inferred_disease_node_id
    "n0", //template_drug_node_id
    "n1", //template_disease_node_id
    util::compute_composite_score
);
impl_wrapper!(
    ServiceProviderChembl,
    "mvp1-template9-service-provider-chembl/mvp1-template9-service-provider-chembl.json",
    "n0",  //inferred_drug_node_id
    "e0",  //inferred_predicate_id
    "n1",  //inferred_disease_node_id
    "n00", //template_drug_node_id
    "n01", //template_disease_node_id
    util::compute_composite_score
);
impl_wrapper!(
    ServiceProviderTMKPTargeted,
    "mvp1-template10-service-provider-tmkp-targeted/mvp1-template10-service-provider-tmkp-targeted.json",
    "n0", //inferred_drug_node_id
    "e0", //inferred_predicate_id
    "n1", //inferred_disease_node_id
    "n0", //template_drug_node_id
    "n1", //template_disease_node_id
    util::compute_composite_score
);
