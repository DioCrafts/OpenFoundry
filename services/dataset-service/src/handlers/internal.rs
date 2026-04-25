use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

use crate::{AppState, models::dataset::Dataset};

#[derive(Debug, Serialize)]
pub struct InternalDatasetMetadata {
    pub id: Uuid,
    pub name: String,
    pub format: String,
    pub marking: String,
    pub tags: Vec<String>,
    pub current_version: i32,
    pub active_branch: String,
    pub owner_id: Uuid,
    pub updated_at: DateTime<Utc>,
}

pub async fn get_dataset_metadata(
    State(state): State<AppState>,
    Path(dataset_id): Path<Uuid>,
) -> impl IntoResponse {
    match sqlx::query_as::<_, Dataset>("SELECT * FROM datasets WHERE id = $1")
        .bind(dataset_id)
        .fetch_optional(&state.db)
        .await
    {
        Ok(Some(dataset)) => Json(InternalDatasetMetadata {
            id: dataset.id,
            name: dataset.name,
            format: dataset.format,
            marking: marking_from_tags(&dataset.tags),
            tags: dataset.tags,
            current_version: dataset.current_version,
            active_branch: dataset.active_branch,
            owner_id: dataset.owner_id,
            updated_at: dataset.updated_at,
        })
        .into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(error) => {
            tracing::error!("internal dataset metadata lookup failed: {error}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

fn marking_from_tags(tags: &[String]) -> String {
    for prefix in ["marking:", "classification:"] {
        if let Some(marking) = tags
            .iter()
            .find_map(|tag| tag.strip_prefix(prefix).and_then(normalize_marking))
        {
            return marking.to_string();
        }
    }

    if tags.iter().any(|tag| tag.eq_ignore_ascii_case("pii")) {
        "pii".to_string()
    } else if tags
        .iter()
        .any(|tag| tag.eq_ignore_ascii_case("confidential"))
    {
        "confidential".to_string()
    } else {
        "public".to_string()
    }
}

fn normalize_marking(value: &str) -> Option<&'static str> {
    if value.eq_ignore_ascii_case("public") {
        Some("public")
    } else if value.eq_ignore_ascii_case("confidential") {
        Some("confidential")
    } else if value.eq_ignore_ascii_case("pii") {
        Some("pii")
    } else {
        None
    }
}
