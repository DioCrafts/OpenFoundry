use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{FromRow, types::Json};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptVersion {
    pub version_number: i32,
    pub content: String,
    #[serde(default)]
    pub input_variables: Vec<String>,
    pub notes: String,
    pub created_at: DateTime<Utc>,
    pub created_by: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptTemplate {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub category: String,
    pub status: String,
    pub tags: Vec<String>,
    pub latest_version_number: i32,
    pub current_version: PromptVersion,
    pub versions: Vec<PromptVersion>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListPromptTemplatesResponse {
    pub data: Vec<PromptTemplate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePromptTemplateRequest {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_prompt_category")]
    pub category: String,
    pub content: String,
    #[serde(default)]
    pub input_variables: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdatePromptTemplateRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub category: Option<String>,
    pub status: Option<String>,
    pub content: Option<String>,
    pub input_variables: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderPromptRequest {
    #[serde(default)]
    pub variables: Value,
    #[serde(default)]
    pub strict: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderPromptResponse {
    pub prompt_id: Uuid,
    pub version_number: i32,
    pub rendered_content: String,
    pub missing_variables: Vec<String>,
}

#[derive(Debug, Clone, FromRow)]
pub(crate) struct PromptTemplateRow {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub category: String,
    pub status: String,
    pub tags: Json<Vec<String>>,
    pub latest_version_number: i32,
    pub versions: Json<Vec<PromptVersion>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<PromptTemplateRow> for PromptTemplate {
    fn from(value: PromptTemplateRow) -> Self {
        let current_version = value.versions.0.last().cloned().unwrap_or(PromptVersion {
            version_number: value.latest_version_number,
            content: String::new(),
            input_variables: Vec::new(),
            notes: String::new(),
            created_at: value.created_at,
            created_by: None,
        });

        Self {
            id: value.id,
            name: value.name,
            description: value.description,
            category: value.category,
            status: value.status,
            tags: value.tags.0,
            latest_version_number: value.latest_version_number,
            current_version,
            versions: value.versions.0,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

fn default_prompt_category() -> String {
    "copilot".to_string()
}
