use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    pub database_url: String,
    pub jwt_secret: String,
    #[serde(default = "default_access_ttl")]
    pub jwt_access_ttl_secs: i64,
    #[serde(default = "default_refresh_ttl")]
    pub jwt_refresh_ttl_secs: i64,
    #[serde(default = "default_public_web_origin")]
    pub public_web_origin: String,
    #[serde(default)]
    pub nats_url: Option<String>,
    #[serde(default)]
    pub redis_url: Option<String>,
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}
fn default_port() -> u16 {
    50051
}
fn default_access_ttl() -> i64 {
    3600
}
fn default_refresh_ttl() -> i64 {
    604_800
}
fn default_public_web_origin() -> String {
    "http://localhost:5173".to_string()
}

impl AppConfig {
    pub fn from_env() -> Result<Self, config::ConfigError> {
        config::Config::builder()
            .add_source(config::Environment::default().separator("__"))
            .build()?
            .try_deserialize()
    }
}