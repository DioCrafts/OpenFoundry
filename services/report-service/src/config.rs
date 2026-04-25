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

impl AppConfig {
    pub fn from_env() -> Result<Self, config::ConfigError> {
        config::Config::builder()
            .add_source(config::Environment::default().separator("__"))
            .build()?
            .try_deserialize()
    }
}
