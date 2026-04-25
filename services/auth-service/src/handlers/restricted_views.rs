use auth_middleware::layer::AuthUser;
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde_json::json;
use uuid::Uuid;

use crate::{
    AppState,
    domain::access,
    models::restricted_view::{RestrictedView, RestrictedViewRow, UpsertRestrictedViewRequest},
};

use super::common::{json_error, require_permission};

pub async fn list_restricted_views(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> impl IntoResponse {
    if let Err(response) = require_permission(&claims, "restricted_views", "read") {
        return response;
    }

    match load_restricted_views(&state.db).await {
        Ok(views) => Json(views).into_response(),
        Err(error) => {
            tracing::error!("failed to list restricted views: {error}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn create_restricted_view(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(body): Json<UpsertRestrictedViewRequest>,
) -> impl IntoResponse {
    if let Err(response) = require_permission(&claims, "restricted_views", "write") {
        return response;
    }
    if let Err(message) = validate_restricted_view_request(&body) {
        return json_error(StatusCode::BAD_REQUEST, message);
    }

    match sqlx::query_as::<_, RestrictedViewRow>(
        r#"INSERT INTO restricted_views (
                id, name, description, resource, action, conditions, row_filter,
                hidden_columns, allowed_org_ids, allowed_markings,
                consumer_mode_enabled, allow_guest_access, enabled, created_by
           )
           VALUES (
                $1, $2, $3, $4, $5, $6, $7,
                $8::jsonb, $9::jsonb, $10::jsonb,
                $11, $12, $13, $14
           )
           RETURNING id, name, description, resource, action, conditions, row_filter, hidden_columns,
                     allowed_org_ids, allowed_markings, consumer_mode_enabled, allow_guest_access,
                     enabled, created_by, created_at, updated_at"#,
    )
    .bind(Uuid::now_v7())
    .bind(body.name.trim())
    .bind(body.description.clone())
    .bind(body.resource.trim())
    .bind(body.action.trim())
    .bind(body.conditions.clone())
    .bind(body.row_filter.clone())
    .bind(serde_json::to_value(&body.hidden_columns).unwrap_or_else(|_| json!([])))
    .bind(serde_json::to_value(&body.allowed_org_ids).unwrap_or_else(|_| json!([])))
    .bind(serde_json::to_value(&body.allowed_markings).unwrap_or_else(|_| json!([])))
    .bind(body.consumer_mode_enabled)
    .bind(body.allow_guest_access)
    .bind(body.enabled)
    .bind(claims.sub)
    .fetch_one(&state.db)
    .await
    {
        Ok(row) => match RestrictedView::try_from(row) {
            Ok(view) => (StatusCode::CREATED, Json(view)).into_response(),
            Err(error) => {
                tracing::error!("failed to decode restricted view after create: {error}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        },
        Err(error) => {
            tracing::error!("failed to create restricted view: {error}");
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to create restricted view",
            )
        }
    }
}

pub async fn update_restricted_view(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(view_id): Path<Uuid>,
    Json(body): Json<UpsertRestrictedViewRequest>,
) -> impl IntoResponse {
    if let Err(response) = require_permission(&claims, "restricted_views", "write") {
        return response;
    }
    if let Err(message) = validate_restricted_view_request(&body) {
        return json_error(StatusCode::BAD_REQUEST, message);
    }

    match sqlx::query_as::<_, RestrictedViewRow>(
        r#"UPDATE restricted_views
           SET name = $2,
               description = $3,
               resource = $4,
               action = $5,
               conditions = $6,
               row_filter = $7,
               hidden_columns = $8::jsonb,
               allowed_org_ids = $9::jsonb,
               allowed_markings = $10::jsonb,
               consumer_mode_enabled = $11,
               allow_guest_access = $12,
               enabled = $13,
               updated_at = NOW()
           WHERE id = $1
           RETURNING id, name, description, resource, action, conditions, row_filter, hidden_columns,
                     allowed_org_ids, allowed_markings, consumer_mode_enabled, allow_guest_access,
                     enabled, created_by, created_at, updated_at"#,
    )
    .bind(view_id)
    .bind(body.name.trim())
    .bind(body.description.clone())
    .bind(body.resource.trim())
    .bind(body.action.trim())
    .bind(body.conditions.clone())
    .bind(body.row_filter.clone())
    .bind(serde_json::to_value(&body.hidden_columns).unwrap_or_else(|_| json!([])))
    .bind(serde_json::to_value(&body.allowed_org_ids).unwrap_or_else(|_| json!([])))
    .bind(serde_json::to_value(&body.allowed_markings).unwrap_or_else(|_| json!([])))
    .bind(body.consumer_mode_enabled)
    .bind(body.allow_guest_access)
    .bind(body.enabled)
    .fetch_optional(&state.db)
    .await
    {
        Ok(Some(row)) => match RestrictedView::try_from(row) {
            Ok(view) => Json(view).into_response(),
            Err(error) => {
                tracing::error!("failed to decode restricted view after update: {error}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        },
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(error) => {
            tracing::error!("failed to update restricted view: {error}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn delete_restricted_view(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(view_id): Path<Uuid>,
) -> impl IntoResponse {
    if let Err(response) = require_permission(&claims, "restricted_views", "write") {
        return response;
    }

    match sqlx::query("DELETE FROM restricted_views WHERE id = $1")
        .bind(view_id)
        .execute(&state.db)
        .await
    {
        Ok(result) if result.rows_affected() > 0 => StatusCode::NO_CONTENT.into_response(),
        Ok(_) => StatusCode::NOT_FOUND.into_response(),
        Err(error) => {
            tracing::error!("failed to delete restricted view: {error}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

async fn load_restricted_views(pool: &sqlx::PgPool) -> Result<Vec<RestrictedView>, sqlx::Error> {
    let rows = sqlx::query_as::<_, RestrictedViewRow>(
        r#"SELECT id, name, description, resource, action, conditions, row_filter, hidden_columns,
		          allowed_org_ids, allowed_markings, consumer_mode_enabled, allow_guest_access,
		          enabled, created_by, created_at, updated_at
		   FROM restricted_views
		   ORDER BY created_at DESC"#,
    )
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(RestrictedView::try_from)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|cause| {
            sqlx::Error::Decode(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                cause,
            )))
        })
}

fn validate_restricted_view_request(body: &UpsertRestrictedViewRequest) -> Result<(), String> {
    if body.name.trim().is_empty() {
        return Err("name is required".to_string());
    }
    if body.resource.trim().is_empty() {
        return Err("resource is required".to_string());
    }
    if body.action.trim().is_empty() {
        return Err("action is required".to_string());
    }

    for column in &body.hidden_columns {
        if column.trim().is_empty() {
            return Err("hidden_columns cannot contain empty values".to_string());
        }
    }

    access::normalize_markings(&body.allowed_markings)?;

    Ok(())
}
