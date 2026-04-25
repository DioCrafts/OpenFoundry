use auth_middleware::layer::AuthUser;
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use chrono::Utc;
use serde_json::{Value, json};
use sqlx::types::Json as SqlJson;
use uuid::Uuid;

use crate::{
    AppState,
    domain::object_sets,
    models::object_set::{
        CreateObjectSetRequest, EvaluateObjectSetRequest, ListObjectSetsResponse,
        ObjectSetDefinition, ObjectSetRow, UpdateObjectSetRequest,
    },
};

fn bad_request(message: impl Into<String>) -> Response {
    (
        StatusCode::BAD_REQUEST,
        Json(json!({ "error": message.into() })),
    )
        .into_response()
}

fn not_found(message: impl Into<String>) -> Response {
    (
        StatusCode::NOT_FOUND,
        Json(json!({ "error": message.into() })),
    )
        .into_response()
}

fn internal_error(message: impl Into<String>) -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({ "error": message.into() })),
    )
        .into_response()
}

async fn load_object_set(
    state: &AppState,
    id: Uuid,
) -> Result<Option<ObjectSetDefinition>, String> {
    let row = sqlx::query_as::<_, ObjectSetRow>(
        r#"SELECT id, name, description, base_object_type_id, filters, traversals, join_config,
                  projections, what_if_label, policy, materialized_snapshot, materialized_at,
                  materialized_row_count, owner_id, created_at, updated_at
           FROM ontology_object_sets
           WHERE id = $1"#,
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await
    .map_err(|error| format!("failed to load object set: {error}"))?;

    Ok(row.map(Into::into))
}

async fn object_type_exists(state: &AppState, object_type_id: Uuid) -> Result<bool, String> {
    sqlx::query_scalar::<_, bool>("SELECT EXISTS (SELECT 1 FROM object_types WHERE id = $1)")
        .bind(object_type_id)
        .fetch_one(&state.db)
        .await
        .map_err(|error| format!("failed to validate object type: {error}"))
}

fn build_definition_from_create(
    owner_id: Uuid,
    request: CreateObjectSetRequest,
) -> ObjectSetDefinition {
    ObjectSetDefinition {
        id: Uuid::now_v7(),
        name: request.name,
        description: request.description,
        base_object_type_id: request.base_object_type_id,
        filters: request.filters,
        traversals: request.traversals,
        join: request.join,
        projections: request.projections,
        what_if_label: request.what_if_label,
        policy: request.policy,
        materialized_snapshot: None,
        materialized_at: None,
        materialized_row_count: 0,
        owner_id,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

pub async fn list_object_sets(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let rows = match sqlx::query_as::<_, ObjectSetRow>(
        r#"SELECT id, name, description, base_object_type_id, filters, traversals, join_config,
                  projections, what_if_label, policy, materialized_snapshot, materialized_at,
                  materialized_row_count, owner_id, created_at, updated_at
           FROM ontology_object_sets
           WHERE owner_id = $1
              OR policy->>'required_restricted_view_id' IS NOT NULL
           ORDER BY updated_at DESC"#,
    )
    .bind(claims.sub)
    .fetch_all(&state.db)
    .await
    {
        Ok(rows) => rows,
        Err(error) => {
            tracing::error!("list object sets failed: {error}");
            return internal_error("failed to load object sets");
        }
    };

    Json(ListObjectSetsResponse {
        data: rows.into_iter().map(Into::into).collect(),
    })
    .into_response()
}

pub async fn create_object_set(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Json(request): Json<CreateObjectSetRequest>,
) -> impl IntoResponse {
    let definition = build_definition_from_create(claims.sub, request);
    if let Err(error) = object_sets::validate_object_set_definition(&definition) {
        return bad_request(error);
    }
    match object_type_exists(&state, definition.base_object_type_id).await {
        Ok(true) => {}
        Ok(false) => return bad_request("base_object_type_id does not exist"),
        Err(error) => return internal_error(error),
    }
    if let Some(join) = definition.join.as_ref() {
        match object_type_exists(&state, join.secondary_object_type_id).await {
            Ok(true) => {}
            Ok(false) => return bad_request("join.secondary_object_type_id does not exist"),
            Err(error) => return internal_error(error),
        }
    }

    let definition_id = definition.id;
    let join_config = definition.join.clone().map(SqlJson);
    let result = sqlx::query(
        r#"INSERT INTO ontology_object_sets (
               id, name, description, base_object_type_id, filters, traversals, join_config,
               projections, what_if_label, policy, materialized_snapshot, materialized_at,
               materialized_row_count, owner_id
           ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, NULL, NULL, 0, $11)"#,
    )
    .bind(definition.id)
    .bind(&definition.name)
    .bind(&definition.description)
    .bind(definition.base_object_type_id)
    .bind(SqlJson(definition.filters.clone()))
    .bind(SqlJson(definition.traversals.clone()))
    .bind(join_config)
    .bind(SqlJson(definition.projections.clone()))
    .bind(&definition.what_if_label)
    .bind(SqlJson(definition.policy.clone()))
    .bind(definition.owner_id)
    .execute(&state.db)
    .await;

    if let Err(error) = result {
        tracing::error!("create object set failed: {error}");
        return internal_error("failed to create object set");
    }

    match load_object_set(&state, definition_id).await {
        Ok(Some(object_set)) => (StatusCode::CREATED, Json(object_set)).into_response(),
        Ok(None) => internal_error("created object set could not be reloaded"),
        Err(error) => internal_error(error),
    }
}

pub async fn get_object_set(
    AuthUser(_claims): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match load_object_set(&state, id).await {
        Ok(Some(object_set)) => Json(object_set).into_response(),
        Ok(None) => not_found("object set not found"),
        Err(error) => internal_error(error),
    }
}

pub async fn update_object_set(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(request): Json<UpdateObjectSetRequest>,
) -> impl IntoResponse {
    let Some(existing) = (match load_object_set(&state, id).await {
        Ok(object_set) => object_set,
        Err(error) => return internal_error(error),
    }) else {
        return not_found("object set not found");
    };

    if existing.owner_id != claims.sub && !claims.has_role("admin") {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({ "error": "forbidden: only the owner can update this object set" })),
        )
            .into_response();
    }

    let next = ObjectSetDefinition {
        id: existing.id,
        name: request.name.unwrap_or(existing.name),
        description: request.description.unwrap_or(existing.description),
        base_object_type_id: request
            .base_object_type_id
            .unwrap_or(existing.base_object_type_id),
        filters: request.filters.unwrap_or(existing.filters),
        traversals: request.traversals.unwrap_or(existing.traversals),
        join: request.join.or(existing.join),
        projections: request.projections.unwrap_or(existing.projections),
        what_if_label: request.what_if_label.or(existing.what_if_label),
        policy: request.policy.unwrap_or(existing.policy),
        materialized_snapshot: existing.materialized_snapshot,
        materialized_at: existing.materialized_at,
        materialized_row_count: existing.materialized_row_count,
        owner_id: existing.owner_id,
        created_at: existing.created_at,
        updated_at: Utc::now(),
    };

    if let Err(error) = object_sets::validate_object_set_definition(&next) {
        return bad_request(error);
    }

    if let Err(error) = sqlx::query(
        r#"UPDATE ontology_object_sets
           SET name = $2,
               description = $3,
               base_object_type_id = $4,
               filters = $5,
               traversals = $6,
               join_config = $7,
               projections = $8,
               what_if_label = $9,
               policy = $10,
               updated_at = now()
           WHERE id = $1"#,
    )
    .bind(id)
    .bind(&next.name)
    .bind(&next.description)
    .bind(next.base_object_type_id)
    .bind(SqlJson(next.filters.clone()))
    .bind(SqlJson(next.traversals.clone()))
    .bind(next.join.clone().map(SqlJson))
    .bind(SqlJson(next.projections.clone()))
    .bind(&next.what_if_label)
    .bind(SqlJson(next.policy.clone()))
    .execute(&state.db)
    .await
    {
        tracing::error!("update object set failed: {error}");
        return internal_error("failed to update object set");
    }

    match load_object_set(&state, id).await {
        Ok(Some(object_set)) => Json(object_set).into_response(),
        Ok(None) => internal_error("updated object set could not be reloaded"),
        Err(error) => internal_error(error),
    }
}

pub async fn delete_object_set(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let Some(existing) = (match load_object_set(&state, id).await {
        Ok(object_set) => object_set,
        Err(error) => return internal_error(error),
    }) else {
        return not_found("object set not found");
    };

    if existing.owner_id != claims.sub && !claims.has_role("admin") {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({ "error": "forbidden: only the owner can delete this object set" })),
        )
            .into_response();
    }

    match sqlx::query("DELETE FROM ontology_object_sets WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await
    {
        Ok(result) if result.rows_affected() > 0 => StatusCode::NO_CONTENT.into_response(),
        Ok(_) => not_found("object set not found"),
        Err(error) => {
            tracing::error!("delete object set failed: {error}");
            internal_error("failed to delete object set")
        }
    }
}

pub async fn evaluate_object_set(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(request): Json<EvaluateObjectSetRequest>,
) -> impl IntoResponse {
    let Some(definition) = (match load_object_set(&state, id).await {
        Ok(object_set) => object_set,
        Err(error) => return internal_error(error),
    }) else {
        return not_found("object set not found");
    };

    let limit = request.limit.unwrap_or(250).clamp(1, 2_000);
    match object_sets::evaluate_object_set(&state, &claims, &definition, limit, false).await {
        Ok(evaluation) => Json(evaluation).into_response(),
        Err(error) if error.contains("forbidden") => {
            (StatusCode::FORBIDDEN, Json(json!({ "error": error }))).into_response()
        }
        Err(error) => bad_request(error),
    }
}

pub async fn materialize_object_set(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(request): Json<EvaluateObjectSetRequest>,
) -> impl IntoResponse {
    let Some(definition) = (match load_object_set(&state, id).await {
        Ok(object_set) => object_set,
        Err(error) => return internal_error(error),
    }) else {
        return not_found("object set not found");
    };

    let limit = request.limit.unwrap_or(2_000).clamp(1, 5_000);
    let evaluation =
        match object_sets::evaluate_object_set(&state, &claims, &definition, limit, true).await {
            Ok(evaluation) => evaluation,
            Err(error) if error.contains("forbidden") => {
                return (StatusCode::FORBIDDEN, Json(json!({ "error": error }))).into_response();
            }
            Err(error) => return bad_request(error),
        };

    let snapshot = Value::Array(evaluation.rows.clone());
    if let Err(error) = sqlx::query(
        r#"UPDATE ontology_object_sets
           SET materialized_snapshot = $2,
               materialized_at = now(),
               materialized_row_count = $3,
               updated_at = now()
           WHERE id = $1"#,
    )
    .bind(id)
    .bind(snapshot)
    .bind(evaluation.total_rows as i32)
    .execute(&state.db)
    .await
    {
        tracing::error!("materialize object set failed: {error}");
        return internal_error("failed to materialize object set");
    }

    Json(evaluation).into_response()
}
