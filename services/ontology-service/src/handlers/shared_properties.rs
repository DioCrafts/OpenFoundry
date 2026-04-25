use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde_json::json;
use uuid::Uuid;

use auth_middleware::layer::AuthUser;

use crate::{
    AppState,
    domain::type_system::{validate_property_type, validate_property_value},
    models::shared_property::{
        CreateSharedPropertyTypeRequest, ListSharedPropertyTypesQuery,
        ListSharedPropertyTypesResponse, ObjectTypeSharedPropertyBinding, SharedPropertyType,
        UpdateSharedPropertyTypeRequest,
    },
};

pub async fn list_shared_property_types(
    _user: AuthUser,
    State(state): State<AppState>,
    Query(params): Query<ListSharedPropertyTypesQuery>,
) -> impl IntoResponse {
    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * per_page;
    let search = params.search.unwrap_or_default();
    let search_pattern = format!("%{search}%");

    let total = sqlx::query_scalar::<_, i64>(
        r#"SELECT COUNT(*)
           FROM shared_property_types
           WHERE name ILIKE $1 OR display_name ILIKE $1 OR property_type ILIKE $1"#,
    )
    .bind(&search_pattern)
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    let data = sqlx::query_as::<_, SharedPropertyType>(
        r#"SELECT id, name, display_name, description, property_type, required, unique_constraint,
                  time_dependent, default_value, validation_rules, owner_id, created_at, updated_at
           FROM shared_property_types
           WHERE name ILIKE $1 OR display_name ILIKE $1 OR property_type ILIKE $1
           ORDER BY created_at DESC
           LIMIT $2 OFFSET $3"#,
    )
    .bind(&search_pattern)
    .bind(per_page)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    Json(ListSharedPropertyTypesResponse {
        data,
        total,
        page,
        per_page,
    })
}

pub async fn create_shared_property_type(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Json(body): Json<CreateSharedPropertyTypeRequest>,
) -> impl IntoResponse {
    if body.name.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "shared property type name is required" })),
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
    let name = body.name.trim().to_string();
    let display_name = body.display_name.unwrap_or_else(|| name.clone());
    let description = body.description.unwrap_or_default();

    match sqlx::query_as::<_, SharedPropertyType>(
        r#"INSERT INTO shared_property_types (
               id, name, display_name, description, property_type, required,
               unique_constraint, time_dependent, default_value, validation_rules, owner_id
           )
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
           RETURNING id, name, display_name, description, property_type, required, unique_constraint,
                     time_dependent, default_value, validation_rules, owner_id, created_at, updated_at"#,
    )
    .bind(id)
    .bind(&name)
    .bind(display_name)
    .bind(description)
    .bind(&body.property_type)
    .bind(body.required.unwrap_or(false))
    .bind(body.unique_constraint.unwrap_or(false))
    .bind(body.time_dependent.unwrap_or(false))
    .bind(body.default_value)
    .bind(body.validation_rules)
    .bind(claims.sub)
    .fetch_one(&state.db)
    .await
    {
        Ok(shared_property_type) => {
            (StatusCode::CREATED, Json(json!(shared_property_type))).into_response()
        }
        Err(error) => {
            tracing::error!("create shared property type failed: {error}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn get_shared_property_type(
    _user: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match sqlx::query_as::<_, SharedPropertyType>(
        r#"SELECT id, name, display_name, description, property_type, required, unique_constraint,
                  time_dependent, default_value, validation_rules, owner_id, created_at, updated_at
           FROM shared_property_types
           WHERE id = $1"#,
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await
    {
        Ok(Some(shared_property_type)) => Json(json!(shared_property_type)).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(error) => {
            tracing::error!("get shared property type failed: {error}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn update_shared_property_type(
    _user: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateSharedPropertyTypeRequest>,
) -> impl IntoResponse {
    let existing = match sqlx::query_as::<_, SharedPropertyType>(
        r#"SELECT id, name, display_name, description, property_type, required, unique_constraint,
                  time_dependent, default_value, validation_rules, owner_id, created_at, updated_at
           FROM shared_property_types
           WHERE id = $1"#,
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await
    {
        Ok(Some(shared_property_type)) => shared_property_type,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(error) => {
            tracing::error!("shared property type lookup failed: {error}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let next_default = body.default_value.or(existing.default_value.clone());
    if let Some(default_value) = &next_default {
        if let Err(error) = validate_property_value(&existing.property_type, default_value) {
            return (StatusCode::BAD_REQUEST, Json(json!({ "error": error }))).into_response();
        }
    }

    match sqlx::query_as::<_, SharedPropertyType>(
        r#"UPDATE shared_property_types
           SET display_name = COALESCE($2, display_name),
               description = COALESCE($3, description),
               required = COALESCE($4, required),
               unique_constraint = COALESCE($5, unique_constraint),
               time_dependent = COALESCE($6, time_dependent),
               default_value = $7,
               validation_rules = $8,
               updated_at = NOW()
           WHERE id = $1
           RETURNING id, name, display_name, description, property_type, required, unique_constraint,
                     time_dependent, default_value, validation_rules, owner_id, created_at, updated_at"#,
    )
    .bind(id)
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
        Ok(Some(shared_property_type)) => Json(json!(shared_property_type)).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(error) => {
            tracing::error!("update shared property type failed: {error}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn delete_shared_property_type(
    _user: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match sqlx::query("DELETE FROM shared_property_types WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await
    {
        Ok(result) if result.rows_affected() > 0 => StatusCode::NO_CONTENT.into_response(),
        Ok(_) => StatusCode::NOT_FOUND.into_response(),
        Err(error) => {
            tracing::error!("delete shared property type failed: {error}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn attach_shared_property_type_to_type(
    _user: AuthUser,
    State(state): State<AppState>,
    Path((type_id, shared_property_type_id)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
    match sqlx::query_as::<_, ObjectTypeSharedPropertyBinding>(
        r#"INSERT INTO object_type_shared_property_types (object_type_id, shared_property_type_id)
           VALUES ($1, $2)
           ON CONFLICT (object_type_id, shared_property_type_id) DO NOTHING
           RETURNING object_type_id, shared_property_type_id, created_at"#,
    )
    .bind(type_id)
    .bind(shared_property_type_id)
    .fetch_optional(&state.db)
    .await
    {
        Ok(Some(binding)) => (StatusCode::CREATED, Json(json!(binding))).into_response(),
        Ok(None) => Json(json!({
            "object_type_id": type_id,
            "shared_property_type_id": shared_property_type_id,
            "status": "attached",
        }))
        .into_response(),
        Err(error) => {
            tracing::error!("attach shared property type failed: {error}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn list_type_shared_property_types(
    _user: AuthUser,
    State(state): State<AppState>,
    Path(type_id): Path<Uuid>,
) -> impl IntoResponse {
    match sqlx::query_as::<_, SharedPropertyType>(
        r#"SELECT spt.id, spt.name, spt.display_name, spt.description, spt.property_type,
                  spt.required, spt.unique_constraint, spt.time_dependent, spt.default_value,
                  spt.validation_rules, spt.owner_id, spt.created_at, spt.updated_at
           FROM shared_property_types spt
           INNER JOIN object_type_shared_property_types otsp
                ON otsp.shared_property_type_id = spt.id
           WHERE otsp.object_type_id = $1
           ORDER BY otsp.created_at ASC, spt.created_at ASC"#,
    )
    .bind(type_id)
    .fetch_all(&state.db)
    .await
    {
        Ok(data) => Json(json!({ "data": data })).into_response(),
        Err(error) => {
            tracing::error!("list type shared property types failed: {error}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn detach_shared_property_type_from_type(
    _user: AuthUser,
    State(state): State<AppState>,
    Path((type_id, shared_property_type_id)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
    match sqlx::query(
        r#"DELETE FROM object_type_shared_property_types
           WHERE object_type_id = $1 AND shared_property_type_id = $2"#,
    )
    .bind(type_id)
    .bind(shared_property_type_id)
    .execute(&state.db)
    .await
    {
        Ok(result) if result.rows_affected() > 0 => StatusCode::NO_CONTENT.into_response(),
        Ok(_) => StatusCode::NOT_FOUND.into_response(),
        Err(error) => {
            tracing::error!("detach shared property type failed: {error}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
