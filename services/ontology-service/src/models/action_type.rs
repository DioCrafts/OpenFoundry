use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq, Eq)]
#[sqlx(type_name = "text", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ActionOperationKind {
    UpdateObject,
    CreateLink,
    DeleteObject,
    InvokeFunction,
    InvokeWebhook,
}

impl std::fmt::Display for ActionOperationKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            Self::UpdateObject => "update_object",
            Self::CreateLink => "create_link",
            Self::DeleteObject => "delete_object",
            Self::InvokeFunction => "invoke_function",
            Self::InvokeWebhook => "invoke_webhook",
        };

        write!(f, "{value}")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionInputField {
    pub name: String,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub property_type: String,
    #[serde(default)]
    pub required: bool,
    pub default_value: Option<Value>,
}

#[derive(Debug, Clone, FromRow)]
pub struct ActionTypeRow {
    pub id: Uuid,
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub object_type_id: Uuid,
    pub operation_kind: String,
    pub input_schema: Value,
    pub config: Value,
    pub confirmation_required: bool,
    pub permission_key: Option<String>,
    pub owner_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionType {
    pub id: Uuid,
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub object_type_id: Uuid,
    pub operation_kind: String,
    pub input_schema: Vec<ActionInputField>,
    pub config: Value,
    pub confirmation_required: bool,
    pub permission_key: Option<String>,
    pub owner_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TryFrom<ActionTypeRow> for ActionType {
    type Error = serde_json::Error;

    fn try_from(row: ActionTypeRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            name: row.name,
            display_name: row.display_name,
            description: row.description,
            object_type_id: row.object_type_id,
            operation_kind: row.operation_kind,
            input_schema: serde_json::from_value(row.input_schema).unwrap_or_default(),
            config: row.config,
            confirmation_required: row.confirmation_required,
            permission_key: row.permission_key,
            owner_id: row.owner_id,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateActionTypeRequest {
    pub name: String,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub object_type_id: Uuid,
    pub operation_kind: String,
    pub input_schema: Option<Vec<ActionInputField>>,
    pub config: Option<Value>,
    pub confirmation_required: Option<bool>,
    pub permission_key: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateActionTypeRequest {
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub operation_kind: Option<String>,
    pub input_schema: Option<Vec<ActionInputField>>,
    pub config: Option<Value>,
    pub confirmation_required: Option<bool>,
    pub permission_key: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListActionTypesQuery {
    pub object_type_id: Option<Uuid>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub search: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ListActionTypesResponse {
    pub data: Vec<ActionType>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
}

#[derive(Debug, Deserialize)]
pub struct ValidateActionRequest {
    pub target_object_id: Option<Uuid>,
    #[serde(default)]
    pub parameters: Value,
}

#[derive(Debug, Serialize)]
pub struct ValidateActionResponse {
    pub valid: bool,
    pub errors: Vec<String>,
    pub preview: Value,
}

#[derive(Debug, Deserialize)]
pub struct ExecuteActionRequest {
    pub target_object_id: Option<Uuid>,
    #[serde(default)]
    pub parameters: Value,
}

#[derive(Debug, Serialize)]
pub struct ExecuteActionResponse {
    pub action: ActionType,
    pub target_object_id: Option<Uuid>,
    pub deleted: bool,
    pub preview: Value,
    pub object: Option<Value>,
    pub link: Option<Value>,
    pub result: Option<Value>,
}
