use crate::model::CQSCompositeScoreValue;
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
    fn render_query_template(&self, ids: &Vec<trapi_model_rs::CURIE>) -> Query;
    fn inferred_drug_node_id(&self) -> String;
    fn inferred_predicate_id(&self) -> String;
    fn inferred_disease_node_id(&self) -> String;
    fn template_drug_node_id(&self) -> String;
    fn template_disease_node_id(&self) -> String;
    fn compute_score(&self, entry_values: &Vec<CQSCompositeScoreValue>) -> f64;
}

macro_rules! impl_wrapper {
    ($foo:ident, $bar:literal, $inferred_drug_node_id:literal, $inferred_predicate_id:literal, $inferred_disease_node_id:literal, $template_drug_node_id:literal, $template_disease_node_id:literal, $func:expr) => {
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

            fn render_query_template(&self, ids: &Vec<trapi_model_rs::CURIE>) -> Query {
                let file = format!("./src/data/path_{}.template.json", $bar.to_string());
                let parser = liquid::ParserBuilder::with_stdlib().build().unwrap().parse_file(file).unwrap();

                let globals = liquid::object!({
                    "curies": ids
                });

                let template = parser.render(&globals).unwrap();
                let query: Query = serde_json::from_str(template.as_str()).unwrap();
                query
            }

            fn compute_score(&self, entry_values: &Vec<CQSCompositeScoreValue>) -> f64 {
                $func(entry_values)
            }
        }
    };
}

impl_wrapper!(CQSQueryA, "a", "n0", "e0", "n1", "n3", "n0", compute_composite_score);
impl_wrapper!(CQSQueryB, "b", "n0", "e0", "n1", "n3", "n0", compute_composite_score);
// impl_wrapper!(CQSQueryC, "c", "n0", "e0", "n1", "n0", "n1", compute_composite_score);
// impl_wrapper!(CQSQueryD, "d", "n0", "e0", "n1", "n0", "n1", compute_composite_score);
impl_wrapper!(CQSQueryE, "e", "n0", "e0", "n1", "n0", "n1", compute_composite_score);
