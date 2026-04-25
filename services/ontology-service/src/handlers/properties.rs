use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde_json::json;
use uuid::Uuid;

use auth_middleware::layer::AuthUser;

use crate::{
    AppState,
    domain::type_system::{validate_property_type, validate_property_value},
    models::property::{CreatePropertyRequest, Property, UpdatePropertyRequest},
};

pub async fn list_properties(
    _user: AuthUser,
    State(state): State<AppState>,
    Path(type_id): Path<Uuid>,
) -> impl IntoResponse {
    match sqlx::query_as::<_, Property>(
        r#"SELECT id, object_type_id, name, display_name, description, property_type, required,
                  unique_constraint, time_dependent, default_value, validation_rules, created_at, updated_at
           FROM properties
           WHERE object_type_id = $1
           ORDER BY created_at ASC"#,
    )
    .bind(type_id)
    .fetch_all(&state.db)
    .await
    {
        Ok(data) => Json(json!({ "data": data })).into_response(),
        Err(error) => {
            tracing::error!("list properties failed: {error}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn create_property(
    _user: AuthUser,
    State(state): State<AppState>,
    Path(type_id): Path<Uuid>,
    Json(body): Json<CreatePropertyRequest>,
) -> impl IntoResponse {
    if body.name.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "property name is required" })),
        )
            .into_response();
    }
    if let Err(error) = validate_property_type(&body.property_type) {
        return (StatusCode::BAD_REQUEST, Json(json!({ "error": error }))).into_response();
    }
    if let Some(default_value) = &body.default_value {
        if let Err(error) = validate_property_value(&body.property_type, default_value) {
            return (StatusCode::BAD_REQUEST, Json(json!({ "error": error }))).into_response();
        }
    }

    let id = Uuid::now_v7();
    let display_name = body.display_name.unwrap_or_else(|| body.name.clone());
    let result = sqlx::query_as::<_, Property>(
        r#"INSERT INTO properties (
               id, object_type_id, name, display_name, description, property_type,
               required, unique_constraint, time_dependent, default_value, validation_rules
           )
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
           RETURNING id, object_type_id, name, display_name, description, property_type, required,
                     unique_constraint, time_dependent, default_value, validation_rules, created_at, updated_at"#,
    )
    .bind(id)
    .bind(type_id)
    .bind(&body.name)
    .bind(display_name)
    .bind(body.description.unwrap_or_default())
    .bind(&body.property_type)
    .bind(body.required.unwrap_or(false))
    .bind(body.unique_constraint.unwrap_or(false))
    .bind(body.time_dependent.unwrap_or(false))
    .bind(body.default_value)
    .bind(body.validation_rules)
    .fetch_one(&state.db)
    .await;

    match result {
        Ok(property) => (StatusCode::CREATED, Json(json!(property))).into_response(),
        Err(error) => {
            tracing::error!("create property failed: {error}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn update_property(
    _user: AuthUser,
    State(state): State<AppState>,
    Path((_type_id, property_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<UpdatePropertyRequest>,
) -> impl IntoResponse {
    let existing = match sqlx::query_as::<_, Property>(
        r#"SELECT id, object_type_id, name, display_name, description, property_type, required,
                  unique_constraint, time_dependent, default_value, validation_rules, created_at, updated_at
           FROM properties WHERE id = $1"#,
    )
    .bind(property_id)
    .fetch_optional(&state.db)
    .await
    {
        Ok(Some(property)) => property,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(error) => {
            tracing::error!("update property lookup failed: {error}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let next_default = body.default_value.or(existing.default_value.clone());
    if let Some(default_value) = &next_default {
        if let Err(error) = validate_property_value(&existing.property_type, default_value) {
            return (StatusCode::BAD_REQUEST, Json(json!({ "error": error }))).into_response();
        }
    }

    match sqlx::query_as::<_, Property>(
        r#"UPDATE properties
           SET display_name = COALESCE($2, display_name),
               description = COALESCE($3, description),
               required = COALESCE($4, required),
               unique_constraint = COALESCE($5, unique_constraint),
               time_dependent = COALESCE($6, time_dependent),
               default_value = $7,
               validation_rules = $8,
               updated_at = NOW()
           WHERE id = $1
           RETURNING id, object_type_id, name, display_name, description, property_type, required,
                     unique_constraint, time_dependent, default_value, validation_rules, created_at, updated_at"#,
    )
    .bind(property_id)
    .bind(body.display_name)
    .bind(body.description)
    .bind(body.required)
    .bind(body.unique_constraint)
    .bind(body.time_dependent)
    .bind(next_default)
    .bind(body.validation_rules.or(existing.validation_rules))
    .fetch_optional(&state.db)
    .await
    {
        Ok(Some(property)) => Json(json!(property)).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(error) => {
            tracing::error!("update property failed: {error}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn delete_property(
    _user: AuthUser,
    State(state): State<AppState>,
    Path((_type_id, property_id)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
    match sqlx::query("DELETE FROM properties WHERE id = $1")
        .bind(property_id)
        .execute(&state.db)
        .await
    {
        Ok(result) if result.rows_affected() > 0 => StatusCode::NO_CONTENT.into_response(),
        Ok(_) => StatusCode::NOT_FOUND.into_response(),
        Err(error) => {
            tracing::error!("delete property failed: {error}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
