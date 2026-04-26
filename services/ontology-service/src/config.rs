use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    pub database_url: String,
    pub jwt_secret: String,
    #[serde(default = "default_audit_service_url")]
    pub audit_service_url: String,
    #[serde(default = "default_dataset_service_url")]
    pub dataset_service_url: String,
    #[serde(default = "default_ontology_service_url")]
    pub ontology_service_url: String,
    #[serde(default = "default_pipeline_service_url")]
    pub pipeline_service_url: String,
    #[serde(default = "default_ai_service_url")]
    pub ai_service_url: String,
    #[serde(default = "default_search_embedding_provider")]
    pub search_embedding_provider: String,
    #[serde(default = "default_notification_service_url")]
    pub notification_service_url: String,
    #[serde(default = "default_node_runtime_command")]
    pub node_runtime_command: String,
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}
fn default_port() -> u16 {
    50057
}

fn default_audit_service_url() -> String {
    "http://localhost:50070".to_string()
}

fn default_dataset_service_url() -> String {
    "http://localhost:50053".to_string()
}

fn default_ontology_service_url() -> String {
    "http://localhost:50057".to_string()
}

fn default_pipeline_service_url() -> String {
    "http://localhost:50056".to_string()
}

fn default_ai_service_url() -> String {
    "http://localhost:50060".to_string()
}

fn default_search_embedding_provider() -> String {
    "deterministic-hash".to_string()
}

fn default_notification_service_url() -> String {
    "http://localhost:50062".to_string()
}

fn default_node_runtime_command() -> String {
    "node".to_string()
}

impl AppConfig {
    pub fn from_env() -> Result<Self, config::ConfigError> {
        config::Config::builder()
            .add_source(config::Environment::default().separator("__"))
            .build()?
            .try_deserialize()
    }
}
