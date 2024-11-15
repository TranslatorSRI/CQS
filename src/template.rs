use crate::model::CQSCompositeScoreValue;
use crate::model::QueryTemplate;
use crate::util;
use std::fs;

pub trait CQSTemplate: Send + Sync {
    fn name(&self) -> String;
    fn render_query_template(&self, ids: Vec<trapi_model_rs::CURIE>) -> QueryTemplate;
    fn template_drug_node_id(&self) -> String;
    fn template_disease_node_id(&self) -> String;
    fn compute_score(&self, entry_values: Vec<CQSCompositeScoreValue>) -> f64;
}

macro_rules! impl_wrapper {
    ($name:ident, $file:literal, $template_drug_node_id:literal, $template_disease_node_id:literal, $func:expr) => {
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

            fn template_drug_node_id(&self) -> String {
                $template_drug_node_id.to_string()
            }

            fn template_disease_node_id(&self) -> String {
                $template_disease_node_id.to_string()
            }

            fn render_query_template(&self, ids: Vec<trapi_model_rs::CURIE>) -> QueryTemplate {
                let file = format!("./templates/{}", $file.to_string());
                let file_contents = fs::read_to_string(file).unwrap();
                let mut query: QueryTemplate = serde_json::from_str(&file_contents).unwrap();
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
    "mvp1-templates/mvp1-template1-clinical-kps/mvp1-template1-clinical-kps.json",
    "n3", //template_drug_node_id
    "n0", //template_disease_node_id
    util::compute_composite_score
);
impl_wrapper!(
    OpenPredict,
    "mvp1-templates/mvp1-template3-openpredict/mvp1-template3-openpredict.json",
    "n0", //template_drug_node_id
    "n1", //template_disease_node_id
    util::compute_composite_score
);
impl_wrapper!(
    RTXKG2SemMed,
    "mvp1-templates/mvp1-template7-rtxkg2-semmed/mvp1-template7-rtxkg2-semmed.json",
    "n0", //template_drug_node_id
    "n1", //template_disease_node_id
    util::compute_composite_score
);
impl_wrapper!(
    ServiceProviderSemMed,
    "mvp1-templates/mvp1-template8-service-provider-semmed/mvp1-template8-service-provider-semmed.json",
    "n0", //template_drug_node_id
    "n1", //template_disease_node_id
    util::compute_composite_score
);
impl_wrapper!(
    ServiceProviderTMKPTargeted,
    "mvp1-templates/mvp1-template10-service-provider-tmkp-targeted/mvp1-template10-service-provider-tmkp-targeted.json",
    "n0", //template_drug_node_id
    "n1", //template_disease_node_id
    util::compute_composite_score
);
impl_wrapper!(
    MultiomicsCTKP,
    "mvp1-templates/mvp-template11-multiomics-ctkp/mvp1-template11-multiomics-ctkp.json",
    "n00", //template_drug_node_id
    "n01", //template_disease_node_id
    util::compute_composite_score
);
impl_wrapper!(
    MultiomicsDrugApprovalsFAERS,
    "mvp1-templates/mvp-template-12-multiomics-drugapprovals-faers/mvp-template-12-multiomics-drugapprovals-faers.json",
    "n0", //template_drug_node_id
    "n1", //template_disease_node_id
    util::compute_composite_score
);
impl_wrapper!(
    CAMKP,
    "mvp2-templates/mvp2-template1-clinical-kps-cam-kp/mvp2-template1-clinical-kps-cam-kp.json",
    "n1", //template_drug_node_id
    "n0", //template_disease_node_id
    util::compute_composite_score
);

#[cfg(test)]
mod test {
    use crate::model::QueryTemplate;
    use std::fmt::Debug;
    use std::fs;

    #[test]
    fn deserialize_query_template() {
        let file = format!(
            "./templates/{}",
            "mvp2-templates/mvp2-template1-clinical-kps-cam-kp/mvp2-template1-clinical-kps-cam-kp.json"
        );
        let ids: Vec<trapi_model_rs::CURIE> = vec![trapi_model_rs::CURIE::from("MONDO:0004979")];
        let file_contents = fs::read_to_string(file).unwrap();
        let mut query: QueryTemplate = serde_json::from_str(&file_contents).unwrap();
        if let Some(qg) = &mut query.message.query_graph {
            if let Some(q_node) = qg.nodes.get_mut("n0") {
                q_node.ids = Some(ids);
            }
        }
        assert!(true);
    }
}
