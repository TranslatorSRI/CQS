use lazy_static::lazy_static;
use std::error::Error;

lazy_static! {
    pub static ref BASE_URL: String =
        "https://raw.githubusercontent.com/NCATSTranslator/Clinical-Data-Committee-Tracking-Voting/main/GetCreative()_DrugDiscoveryRepurposing_RarePulmonaryDisease/".to_string();
}

struct Path {
    pub url: String,
    pub node: String,
    pub output: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let paths = vec![
        Path {
            url: "Path_A/Path_A_e0-e1-e2-allowlist.json".to_string(),
            node: "n0".to_string(),
            output: "path_a.template.json".to_string(),
        },
        Path {
            url: "Path_B/Path_B_TRAPI.json".to_string(),
            node: "n0".to_string(),
            output: "path_b.template.json".to_string(),
        },
        Path {
            url: "Path_E/path_e_query.json".to_string(),
            node: "n1".to_string(),
            output: "path_e.template.json".to_string(),
        },
    ];

    for path in paths.iter() {
        let mut url = BASE_URL.clone();
        url.push_str(path.url.as_str());

        let mut output = "./src/data/".to_string();
        output.push_str(path.output.as_str());

        let mut body = reqwest::get(url).await?.text().await?;

        let mut query: trapi_model_rs::Query = serde_json::from_str(&*body).unwrap();
        if let Some(ref mut query_graph) = query.message.query_graph {
            if let Some(node) = query_graph.nodes.get_mut(&path.node) {
                let asdf = "{{ curies | join: '\",\"' }}";
                // println!("asdf: {}", asdf);
                // node.ids = Some(vec![asdf]);
                // println!("{:?}", node);
                if let Some(ids) = &node.ids {
                    let first = ids.iter().next().unwrap();
                    // println!("ids: {:?}", first);
                    body = body.replace(first, asdf);
                }
            }
        };
        // let altered_query: trapi_model_rs::Query = serde_json::from_str(&*body).unwrap();
        // std::fs::write(std::path::Path::new(&output), serde_json::to_string_pretty(&altered_query).unwrap()).expect("failed to write output");

        std::fs::write(std::path::Path::new(&output), body).expect("failed to write output");
    }
    Ok(())
}
