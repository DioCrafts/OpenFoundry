use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::{FromRow, types::Json};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub category: String,
    pub execution_mode: String,
    pub execution_config: Value,
    pub status: String,
    pub input_schema: Value,
    pub output_schema: Value,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListToolsResponse {
    pub data: Vec<ToolDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateToolRequest {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_tool_category")]
    pub category: String,
    #[serde(default = "default_execution_mode")]
    pub execution_mode: String,
    #[serde(default = "default_json_object")]
    pub execution_config: Value,
    #[serde(default = "default_tool_status")]
    pub status: String,
    #[serde(default = "default_json_object")]
    pub input_schema: Value,
    #[serde(default = "default_json_object")]
    pub output_schema: Value,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateToolRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub category: Option<String>,
    pub execution_mode: Option<String>,
    pub execution_config: Option<Value>,
    pub status: Option<String>,
    pub input_schema: Option<Value>,
    pub output_schema: Option<Value>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Clone, FromRow)]
pub(crate) struct ToolRow {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub category: String,
    pub execution_mode: String,
    pub execution_config: Json<Value>,
    pub status: String,
    pub input_schema: Json<Value>,
    pub output_schema: Json<Value>,
    pub tags: Json<Vec<String>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<ToolRow> for ToolDefinition {
    fn from(value: ToolRow) -> Self {
        Self {
            id: value.id,
            name: value.name,
            description: value.description,
            category: value.category,
            execution_mode: value.execution_mode,
            execution_config: value.execution_config.0,
            status: value.status,
            input_schema: value.input_schema.0,
            output_schema: value.output_schema.0,
            tags: value.tags.0,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

pub fn supported_execution_modes() -> &'static [&'static str] {
    &[
        "simulated",
        "http_json",
        "openfoundry_api",
        "native_sql",
        "native_dataset",
        "native_ontology",
        "native_pipeline",
        "native_report",
        "native_workflow",
        "native_code_repo",
        "knowledge_search",
    ]
}

fn default_tool_category() -> String {
    "analysis".to_string()
}

fn default_execution_mode() -> String {
    "simulated".to_string()
}

fn default_tool_status() -> String {
    "active".to_string()
}

fn default_json_object() -> Value {
    json!({})
}
