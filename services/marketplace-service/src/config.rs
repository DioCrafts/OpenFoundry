use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    pub database_url: String,
    pub jwt_secret: String,
    #[serde(default = "default_app_builder_service_url")]
    pub app_builder_service_url: String,
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}

fn default_port() -> u16 {
    50066
}

fn default_app_builder_service_url() -> String {
    "http://localhost:50063".to_string()
}

impl AppConfig {
    pub fn from_env() -> Result<Self, config::ConfigError> {
        config::Config::builder()
            .add_source(config::Environment::default().separator("__"))
            .build()?
            .try_deserialize()
    }
}
