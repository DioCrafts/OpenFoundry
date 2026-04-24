use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct NotificationPreference {
    pub user_id: Uuid,
    pub in_app_enabled: bool,
    pub email_enabled: bool,
    pub email_address: Option<String>,
    pub slack_webhook_url: Option<String>,
    pub teams_webhook_url: Option<String>,
    pub digest_frequency: String,
    pub quiet_hours: Value,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateNotificationPreferenceRequest {
    pub in_app_enabled: Option<bool>,
    pub email_enabled: Option<bool>,
    pub email_address: Option<String>,
    pub slack_webhook_url: Option<String>,
    pub teams_webhook_url: Option<String>,
    pub digest_frequency: Option<String>,
    pub quiet_hours: Option<Value>,
}
