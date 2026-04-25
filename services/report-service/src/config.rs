use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    pub database_url: String,
    pub jwt_secret: String,
    #[serde(default = "default_dataset_service_url")]
    pub dataset_service_url: String,
    #[serde(default = "default_geospatial_service_url")]
    pub geospatial_service_url: String,
    pub smtp_host: Option<String>,
    pub smtp_port: Option<u16>,
    pub smtp_username: Option<String>,
    pub smtp_password: Option<String>,
    pub smtp_from_address: Option<String>,
    pub smtp_from_name: Option<String>,
    #[serde(default = "default_object_storage_backend")]
    pub object_storage_backend: String,
    #[serde(default = "default_local_delivery_root")]
    pub local_delivery_root: String,
    pub object_storage_bucket: Option<String>,
    pub object_storage_region: Option<String>,
    pub object_storage_endpoint: Option<String>,
    pub object_storage_access_key: Option<String>,
    pub object_storage_secret_key: Option<String>,
    #[serde(default = "default_delivery_timeout_secs")]
    pub report_delivery_timeout_secs: u64,
    #[serde(default = "default_delivery_max_retries")]
    pub report_delivery_max_retries: u32,
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}

fn default_port() -> u16 {
    50064
}

fn default_dataset_service_url() -> String {
    "http://localhost:50053".to_string()
}

fn default_geospatial_service_url() -> String {
    "http://localhost:50068".to_string()
}

fn default_object_storage_backend() -> String {
    "local".to_string()
}

fn default_local_delivery_root() -> String {
    "/tmp/openfoundry-report-delivery".to_string()
}

fn default_delivery_timeout_secs() -> u64 {
    10
}

fn default_delivery_max_retries() -> u32 {
    2
}

impl AppConfig {
    pub fn from_env() -> Result<Self, config::ConfigError> {
        config::Config::builder()
            .add_source(config::Environment::default().separator("__"))
            .build()?
            .try_deserialize()
    }
}
