use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, sqlx::Type, PartialEq, Eq)]
#[sqlx(type_name = "text", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum OntologyProjectRole {
    Viewer,
    Editor,
    Owner,
}

impl OntologyProjectRole {
    pub fn rank(self) -> u8 {
        match self {
            Self::Viewer => 1,
            Self::Editor => 2,
            Self::Owner => 3,
        }
    }
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct OntologyProject {
    pub id: Uuid,
    pub slug: String,
    pub display_name: String,
    pub description: String,
    pub workspace_slug: Option<String>,
    pub owner_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct OntologyProjectMembership {
    pub project_id: Uuid,
    pub user_id: Uuid,
    pub role: OntologyProjectRole,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct OntologyProjectResourceBinding {
    pub project_id: Uuid,
    pub resource_kind: String,
    pub resource_id: Uuid,
    pub bound_by: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateOntologyProjectRequest {
    pub slug: String,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub workspace_slug: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateOntologyProjectRequest {
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub workspace_slug: Option<Option<String>>,
}

#[derive(Debug, Deserialize)]
pub struct ListOntologyProjectsQuery {
    pub search: Option<String>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct ListOntologyProjectsResponse {
    pub data: Vec<OntologyProject>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
}

#[derive(Debug, Deserialize)]
pub struct UpsertOntologyProjectMembershipRequest {
    pub user_id: Uuid,
    pub role: OntologyProjectRole,
}

#[derive(Debug, Serialize)]
pub struct ListOntologyProjectMembershipsResponse {
    pub data: Vec<OntologyProjectMembership>,
}

#[derive(Debug, Deserialize)]
pub struct BindOntologyProjectResourceRequest {
    pub resource_kind: String,
    pub resource_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct ListOntologyProjectResourcesResponse {
    pub data: Vec<OntologyProjectResourceBinding>,
}
