use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    pub database_url: String,
    pub jwt_secret: String,
    #[serde(default = "default_data_dir")]
    pub data_dir: String,
    #[serde(default = "default_dataset_service_url")]
    pub dataset_service_url: String,
    #[serde(default = "default_workflow_service_url")]
    pub workflow_service_url: String,
    #[serde(default = "default_storage_backend")]
    pub storage_backend: String,
    #[serde(default = "default_storage_bucket")]
    pub storage_bucket: String,
    #[serde(default)]
    pub s3_endpoint: Option<String>,
    #[serde(default)]
    pub s3_region: Option<String>,
    #[serde(default)]
    pub s3_access_key: Option<String>,
    #[serde(default)]
    pub s3_secret_key: Option<String>,
    #[serde(default)]
    pub local_storage_root: Option<String>,
    #[serde(default = "default_distributed_pipeline_workers")]
    pub distributed_pipeline_workers: usize,
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}
fn default_port() -> u16 {
    50056
}
fn default_data_dir() -> String {
    "/tmp/pipeline-data".to_string()
}
fn default_dataset_service_url() -> String {
    "http://localhost:50053".to_string()
}
fn default_workflow_service_url() -> String {
    "http://localhost:50061".to_string()
}
fn default_storage_backend() -> String {
    "s3".to_string()
}
fn default_storage_bucket() -> String {
    "datasets".to_string()
}
fn default_distributed_pipeline_workers() -> usize {
    1
}

impl AppConfig {
    pub fn from_env() -> Result<Self, config::ConfigError> {
        config::Config::builder()
            .add_source(config::Environment::default().separator("__"))
            .build()?
            .try_deserialize()
    }
}
