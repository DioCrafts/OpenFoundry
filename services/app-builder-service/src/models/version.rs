use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, types::Json};
use uuid::Uuid;

use crate::models::{app::AppSettings, page::AppPage, theme::AppTheme};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppSnapshot {
    pub name: String,
    pub slug: String,
    pub description: String,
    pub status: String,
    pub pages: Vec<AppPage>,
    pub theme: AppTheme,
    pub settings: AppSettings,
    pub template_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppVersion {
    pub id: Uuid,
    pub app_id: Uuid,
    pub version_number: i32,
    pub status: String,
    pub app_snapshot: AppSnapshot,
    pub notes: String,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub published_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListAppVersionsResponse {
    pub data: Vec<AppVersion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishAppRequest {
    #[serde(default)]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, FromRow)]
pub(crate) struct AppVersionRow {
    pub id: Uuid,
    pub app_id: Uuid,
    pub version_number: i32,
    pub status: String,
    pub app_snapshot: Json<AppSnapshot>,
    pub notes: String,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub published_at: Option<DateTime<Utc>>,
}

impl From<AppVersionRow> for AppVersion {
    fn from(value: AppVersionRow) -> Self {
        Self {
            id: value.id,
            app_id: value.app_id,
            version_number: value.version_number,
            status: value.status,
            app_snapshot: value.app_snapshot.0,
            notes: value.notes,
            created_by: value.created_by,
            created_at: value.created_at,
            published_at: value.published_at,
        }
    }
}
