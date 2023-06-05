#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct CQSCompositeScoreKey {
    pub subject: String,
    // pub predicate: String,
    pub object: String,
}

impl CQSCompositeScoreKey {
    pub fn new(subject: String, object: String) -> CQSCompositeScoreKey {
        CQSCompositeScoreKey { subject, object }
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct CQSCompositeScoreValue {
    pub resource_id: String,
    pub knowledge_graph_key: String,
    pub log_odds_ratio: Option<f64>,
    pub total_sample_size: Option<i64>,
}

impl CQSCompositeScoreValue {
    pub fn new(resource_id: String, knowledge_graph_key: String) -> CQSCompositeScoreValue {
        CQSCompositeScoreValue {
            resource_id,
            knowledge_graph_key,
            log_odds_ratio: None,
            total_sample_size: None,
        }
    }
}
