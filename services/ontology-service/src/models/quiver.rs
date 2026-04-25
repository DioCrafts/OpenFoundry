use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct QuiverVisualFunction {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub primary_type_id: Uuid,
    pub secondary_type_id: Option<Uuid>,
    pub join_field: String,
    pub secondary_join_field: String,
    pub date_field: String,
    pub metric_field: String,
    pub group_field: String,
    pub selected_group: Option<String>,
    pub chart_kind: String,
    pub shared: bool,
    pub vega_spec: Value,
    pub owner_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuiverVisualFunctionDraft {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub primary_type_id: Uuid,
    pub secondary_type_id: Option<Uuid>,
    pub join_field: String,
    #[serde(default)]
    pub secondary_join_field: String,
    pub date_field: String,
    pub metric_field: String,
    pub group_field: String,
    pub selected_group: Option<String>,
    #[serde(default = "default_chart_kind")]
    pub chart_kind: String,
    #[serde(default)]
    pub shared: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateQuiverVisualFunctionRequest {
    pub name: String,
    pub description: Option<String>,
    pub primary_type_id: Uuid,
    pub secondary_type_id: Option<Uuid>,
    pub join_field: String,
    pub secondary_join_field: Option<String>,
    pub date_field: String,
    pub metric_field: String,
    pub group_field: String,
    pub selected_group: Option<String>,
    pub chart_kind: Option<String>,
    pub shared: Option<bool>,
}

impl CreateQuiverVisualFunctionRequest {
    pub fn into_draft(self) -> QuiverVisualFunctionDraft {
        QuiverVisualFunctionDraft {
            name: self.name,
            description: self.description.unwrap_or_default(),
            primary_type_id: self.primary_type_id,
            secondary_type_id: self.secondary_type_id,
            join_field: self.join_field,
            secondary_join_field: self.secondary_join_field.unwrap_or_default(),
            date_field: self.date_field,
            metric_field: self.metric_field,
            group_field: self.group_field,
            selected_group: self.selected_group,
            chart_kind: self.chart_kind.unwrap_or_else(default_chart_kind),
            shared: self.shared.unwrap_or(false),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateQuiverVisualFunctionRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub primary_type_id: Option<Uuid>,
    pub secondary_type_id: Option<Uuid>,
    pub join_field: Option<String>,
    pub secondary_join_field: Option<String>,
    pub date_field: Option<String>,
    pub metric_field: Option<String>,
    pub group_field: Option<String>,
    pub selected_group: Option<Option<String>>,
    pub chart_kind: Option<String>,
    pub shared: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ListQuiverVisualFunctionsQuery {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub search: Option<String>,
    pub include_shared: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ListQuiverVisualFunctionsResponse {
    pub data: Vec<QuiverVisualFunction>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
}

pub fn default_chart_kind() -> String {
    "line".to_string()
}
