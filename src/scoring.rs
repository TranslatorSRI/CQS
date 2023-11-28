use crate::model::CQSCompositeScoreValue;
use std::fs;
use trapi_model_rs::Query;

pub fn compute_composite_score(entry_values: &Vec<CQSCompositeScoreValue>) -> f64 {
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

pub trait CQSQuery: Send + Sync {
    fn name(&self) -> String;
    fn query(&self, curie_token: &str) -> Query;
    fn compute_score(&self, entry_values: &Vec<CQSCompositeScoreValue>) -> f64;
}

macro_rules! impl_wrapper {
    ($foo:ident, $bar:literal, $func:expr) => {
        pub struct $foo {}

        impl $foo {
            pub fn new() -> $foo {
                $foo {}
            }
        }

        impl CQSQuery for $foo {
            fn name(&self) -> String {
                $bar.to_string()
            }

            fn query(&self, curie_token: &str) -> Query {
                let file = format!("./src/data/path_{}.template.json", $bar.to_string());
                let mut template = fs::read_to_string(&file).expect(format!("Could not find file: {}", &file).as_str());
                template = template.replace("CURIE_TOKEN", curie_token);
                debug!("template: {}", template);
                let query: Query = serde_json::from_str(template.as_str()).unwrap();
                query
            }

            fn compute_score(&self, entry_values: &Vec<CQSCompositeScoreValue>) -> f64 {
                $func(entry_values)
            }
        }
    };
}

impl_wrapper!(CQSQueryA, "a", compute_composite_score);
impl_wrapper!(CQSQueryB, "b", compute_composite_score);
impl_wrapper!(CQSQueryC, "c", compute_composite_score);
impl_wrapper!(CQSQueryD, "d", compute_composite_score);
impl_wrapper!(CQSQueryE, "e", compute_composite_score);
