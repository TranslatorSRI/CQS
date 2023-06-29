#[macro_use]
extern crate log;

use clap::Parser;
use hyper;
use k8s_openapi::api::core::v1::{ConfigMap, Pod, Service, ServicePort, ServiceSpec};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use k8s_openapi::apimachinery::pkg::util::intstr::IntOrString;
use kube::api::PostParams;
use kube::client::ConfigExt;
use kube::{Api, Client, Config, ResourceExt};
use serde_json::json;
use std::collections::BTreeMap;
use std::{error, fs};

#[derive(Parser, PartialEq, Debug)]
#[clap(author, version, about, long_about = None)]
struct Options {
    #[clap(short, long)]
    namespace: String,

    #[clap(short, long)]
    release: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn error::Error>> {
    env_logger::init();

    let options = Options::parse();
    debug!("{:?}", options);

    let mut config = Config::infer().await?;
    config.default_namespace = options.namespace.clone();

    let https = config.openssl_https_connector()?;

    let tower_service = tower::ServiceBuilder::new()
        .layer(config.base_uri_layer())
        .option_layer(config.auth_layer()?)
        .service(hyper::Client::builder().build(https));

    let client = Client::new(tower_service, config.default_namespace);

    let services: Api<Service> = Api::namespaced(client, options.namespace.as_str());

    let kube_app_service = build_app_service(options.release.as_str());
    fs::write("charts/app-service.yaml", serde_yaml::to_string(&kube_app_service).unwrap()).unwrap();
    fs::write("charts/app-service.json", serde_json::to_string_pretty(&kube_app_service).unwrap()).unwrap();

    let kube_db_service = build_db_service(options.release.as_str());
    fs::write("charts/db-service.yaml", serde_yaml::to_string(&kube_db_service).unwrap()).unwrap();
    fs::write("/tmp/db-service.json", serde_json::to_string_pretty(&kube_db_service).unwrap()).unwrap();

    let kube_config_map = build_config_map(options.release.as_str());
    fs::write("charts/env-config-map.yaml", serde_yaml::to_string(&kube_config_map).unwrap()).unwrap();
    fs::write("charts/env-config-map.json", serde_json::to_string_pretty(&kube_config_map).unwrap()).unwrap();

    // let pp = PostParams::default();
    // match pods.create(&pp, &p).await {
    //     Ok(o) => {
    //         let name = o.name_any();
    //         assert_eq!(p.name_any(), name);
    //         info!("Created {}", name);
    //     }
    //     Err(kube::Error::Api(ae)) => assert_eq!(ae.code, 409), // if you skipped delete, for instance
    //     Err(e) => return Err(e.into()),                        // any other case is probably bad
    // }
    Ok(())
}

fn build_app_service(release: &str) -> Service {
    let mut service = Service::default();

    let mut meta = ObjectMeta::default();
    meta.name = Some(format!("{}-app-service", release));
    let label_map = BTreeMap::from([
        ("app.kubernetes.io/name".to_string(), format!("{}-app-deployment", release)),
        ("app.kubernetes.io/instance".to_string(), format!("{}-app-deployment", release)),
        ("type".to_string(), "webserver".to_string()),
    ]);
    meta.labels = Some(label_map.clone());
    service.metadata = meta;

    let mut spec = ServiceSpec::default();
    spec.selector = Some(label_map.clone());
    let mut service_port = ServicePort::default();
    service_port.name = Some("http".to_string());
    service_port.port = 8000;
    service_port.target_port = Some(IntOrString::String("http".to_string()));
    service_port.protocol = Some("TCP".to_string());

    spec.ports = Some(vec![service_port]);

    service.spec = Some(spec);
    service
}

fn build_db_service(release: &str) -> Service {
    let mut service = Service::default();

    let mut meta = ObjectMeta::default();
    meta.name = Some(format!("{}-postgres", release));
    let label_map = BTreeMap::from([
        ("app.kubernetes.io/name".to_string(), format!("{}-db-deployment", release)),
        ("app.kubernetes.io/instance".to_string(), format!("{}-db-deployment", release)),
        ("type".to_string(), "dbms".to_string()),
    ]);
    meta.labels = Some(label_map.clone());
    service.metadata = meta;

    let mut spec = ServiceSpec::default();
    spec.selector = Some(label_map.clone());
    spec.type_ = Some("ClusterIP".to_string());
    let mut service_port = ServicePort::default();
    service_port.name = Some("db-port".to_string());
    service_port.port = 5432;
    service_port.target_port = Some(IntOrString::String("db-port".to_string()));
    service_port.protocol = Some("TCP".to_string());

    spec.ports = Some(vec![service_port]);

    service.spec = Some(spec);
    service
}

fn build_config_map(release: &str) -> ConfigMap {
    let mut config_map = ConfigMap::default();

    let mut meta = ObjectMeta::default();
    meta.name = Some(format!("{}-configmap", release));
    config_map.metadata = meta;

    let data_map = BTreeMap::from([
        ("PATH_WHITELIST".to_string(), "{{ .Values.app.path_whitelist }}".to_string()),
        ("WORKFLOW_RUNNER_URL".to_string(), "{{ .Values.app.workflow_runner_url }}".to_string()),
        ("RUST_LOG".to_string(), "{{ .Values.app.log_level }}".to_string()),
        ("RESPONSE_URL".to_string(), "{{ .Values.app.response_url }}".to_string()),
        ("SCHEMA_VERSION".to_string(), "{{ .Values.x_trapi.version }}".to_string()),
        ("MATURITY".to_string(), "{{ .Values.x_trapi.maturity }}".to_string()),
        ("LOCATION".to_string(), "{{ .Values.x_trapi.location }}".to_string()),
        ("POSTGRES_DB".to_string(), "{{ .Values.postgres.dbName }}".to_string()),
        ("POSTGRES_USER".to_string(), "{{ .Values.postgres.user }}".to_string()),
        ("POSTGRES_PASSWORD".to_string(), "{{ .Values.postgres.password }}".to_string()),
        ("POSTGRES_SERVER".to_string(), "{{ .Values.postgres.server }}".to_string()),
    ]);

    config_map.data = Some(data_map);

    config_map
}
