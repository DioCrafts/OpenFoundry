use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct GatewayConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    pub jwt_secret: String,
    #[serde(default)]
    pub cors_origins: Vec<String>,
    #[serde(default = "default_auth_url")]
    pub auth_service_url: String,
    #[serde(default = "default_dataset_url")]
    pub dataset_service_url: String,
    #[serde(default = "default_query_url")]
    pub query_service_url: String,
    #[serde(default = "default_pipeline_url")]
    pub pipeline_service_url: String,
    #[serde(default = "default_ontology_url")]
    pub ontology_service_url: String,
    #[serde(default = "default_workflow_url")]
    pub workflow_service_url: String,
    #[serde(default = "default_notification_url")]
    pub notification_service_url: String,
    #[serde(default = "default_app_builder_url")]
    pub app_builder_service_url: String,
    #[serde(default = "default_ml_service_url")]
    pub ml_service_url: String,
    #[serde(default = "default_ai_service_url")]
    pub ai_service_url: String,
    #[serde(default = "default_fusion_service_url")]
    pub fusion_service_url: String,
    #[serde(default = "default_streaming_service_url")]
    pub streaming_service_url: String,
    #[serde(default = "default_report_service_url")]
    pub report_service_url: String,
    #[serde(default = "default_geospatial_service_url")]
    pub geospatial_service_url: String,
    #[serde(default = "default_code_repo_service_url")]
    pub code_repo_service_url: String,
    #[serde(default = "default_marketplace_service_url")]
    pub marketplace_service_url: String,
    #[serde(default = "default_audit_service_url")]
    pub audit_service_url: String,
    #[serde(default = "default_nexus_service_url")]
    pub nexus_service_url: String,
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}
fn default_port() -> u16 {
    8080
}
fn default_auth_url() -> String {
    "http://localhost:50051".to_string()
}
fn default_dataset_url() -> String {
    "http://localhost:50053".to_string()
}
fn default_query_url() -> String {
    "http://localhost:50055".to_string()
}
fn default_pipeline_url() -> String {
    "http://localhost:50056".to_string()
}
fn default_ontology_url() -> String {
    "http://localhost:50057".to_string()
}
fn default_workflow_url() -> String {
    "http://localhost:50061".to_string()
}
fn default_notification_url() -> String {
    "http://localhost:50069".to_string()
}
fn default_app_builder_url() -> String {
    "http://localhost:50063".to_string()
}
fn default_ml_service_url() -> String {
    "http://localhost:50059".to_string()
}
fn default_ai_service_url() -> String {
    "http://localhost:50060".to_string()
}
fn default_fusion_service_url() -> String {
    "http://localhost:50058".to_string()
}

fn default_streaming_service_url() -> String {
	"http://localhost:50054".to_string()
}

fn default_report_service_url() -> String {
	"http://localhost:50064".to_string()
}

fn default_geospatial_service_url() -> String {
	"http://localhost:50068".to_string()
}

fn default_code_repo_service_url() -> String {
    "http://localhost:50065".to_string()
}

fn default_marketplace_service_url() -> String {
    "http://localhost:50066".to_string()
}

fn default_audit_service_url() -> String {
    "http://localhost:50070".to_string()
}

fn default_nexus_service_url() -> String {
	"http://localhost:50067".to_string()
}

impl GatewayConfig {
    pub fn from_env() -> Result<Self, config::ConfigError> {
        config::Config::builder()
            .add_source(config::Environment::default().separator("__"))
            .build()?
            .try_deserialize()
    }
}