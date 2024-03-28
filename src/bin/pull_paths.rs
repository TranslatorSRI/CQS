use std::error::Error;
use std::fs;
use std::path;

struct TRAPIPath {
    pub file: path::PathBuf,
    pub node: String,
    pub output: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let paths = vec![
        TRAPIPath {
            file: path::PathBuf::from("paths/Path_A/Path_A_e0-e1-e2-allowlist.json"),
            node: "n0".to_string(),
            output: "mvp1-template1-clinical-kps.json".to_string(),
        },
        TRAPIPath {
            file: path::PathBuf::from("paths/Path_B/Path_B_TRAPI.json"),
            node: "n0".to_string(),
            output: "path_b.template.json".to_string(),
        },
        TRAPIPath {
            file: path::PathBuf::from("paths/Path_E/path_e_query.json"),
            node: "n1".to_string(),
            output: "mvp1-template3-openpredict.json".to_string(),
        },
    ];

    for path in paths.iter() {
        let mut output = "./src/data/".to_string();
        output.push_str(path.output.as_str());

        let mut body = fs::read_to_string(path.file.as_path()).expect("Could not read path");

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
        // fs::write(path::Path::new(&output), serde_json::to_string_pretty(&altered_query).unwrap()).expect("failed to write output");

        fs::write(path::Path::new(&output), body).expect("failed to write output");
    }
    Ok(())
}
