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
    models::interface::{
        CreateInterfacePropertyRequest, CreateInterfaceRequest, InterfaceProperty,
        ListInterfacesQuery, ListInterfacesResponse, ObjectTypeInterfaceBinding,
        OntologyInterface, UpdateInterfacePropertyRequest, UpdateInterfaceRequest,
    },
};

pub async fn create_interface(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Json(body): Json<CreateInterfaceRequest>,
) -> impl IntoResponse {
    if body.name.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "interface name is required" })),
        )
            .into_response();
    }

    let id = Uuid::now_v7();
    let display_name = body.display_name.unwrap_or_else(|| body.name.clone());
    match sqlx::query_as::<_, OntologyInterface>(
        r#"INSERT INTO ontology_interfaces (id, name, display_name, description, owner_id)
           VALUES ($1, $2, $3, $4, $5)
           RETURNING *"#,
    )
    .bind(id)
    .bind(&body.name)
    .bind(display_name)
    .bind(body.description.unwrap_or_default())
    .bind(claims.sub)
    .fetch_one(&state.db)
    .await
    {
        Ok(interface) => (StatusCode::CREATED, Json(json!(interface))).into_response(),
        Err(error) => {
            tracing::error!("create interface failed: {error}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn list_interfaces(
    _user: AuthUser,
    State(state): State<AppState>,
    Query(params): Query<ListInterfacesQuery>,
) -> impl IntoResponse {
    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * per_page;
    let search_pattern = format!("%{}%", params.search.unwrap_or_default());

    let total = sqlx::query_scalar::<_, i64>(
        r#"SELECT COUNT(*) FROM ontology_interfaces
           WHERE name ILIKE $1 OR display_name ILIKE $1"#,
    )
    .bind(&search_pattern)
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    match sqlx::query_as::<_, OntologyInterface>(
        r#"SELECT * FROM ontology_interfaces
           WHERE name ILIKE $1 OR display_name ILIKE $1
           ORDER BY created_at DESC
           LIMIT $2 OFFSET $3"#,
    )
    .bind(&search_pattern)
    .bind(per_page)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    {
        Ok(data) => Json(ListInterfacesResponse {
            data,
            total,
            page,
            per_page,
        })
        .into_response(),
        Err(error) => {
            tracing::error!("list interfaces failed: {error}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn get_interface(
    _user: AuthUser,
    State(state): State<AppState>,
    Path(interface_id): Path<Uuid>,
) -> impl IntoResponse {
    match sqlx::query_as::<_, OntologyInterface>("SELECT * FROM ontology_interfaces WHERE id = $1")
        .bind(interface_id)
        .fetch_optional(&state.db)
        .await
    {
        Ok(Some(interface)) => Json(json!(interface)).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(error) => {
            tracing::error!("get interface failed: {error}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn update_interface(
    _user: AuthUser,
    State(state): State<AppState>,
    Path(interface_id): Path<Uuid>,
    Json(body): Json<UpdateInterfaceRequest>,
) -> impl IntoResponse {
    match sqlx::query_as::<_, OntologyInterface>(
        r#"UPDATE ontology_interfaces
           SET display_name = COALESCE($2, display_name),
               description = COALESCE($3, description),
               updated_at = NOW()
           WHERE id = $1
           RETURNING *"#,
    )
    .bind(interface_id)
    .bind(body.display_name)
    .bind(body.description)
    .fetch_optional(&state.db)
    .await
    {
        Ok(Some(interface)) => Json(json!(interface)).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(error) => {
            tracing::error!("update interface failed: {error}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn delete_interface(
    _user: AuthUser,
    State(state): State<AppState>,
    Path(interface_id): Path<Uuid>,
) -> impl IntoResponse {
    match sqlx::query("DELETE FROM ontology_interfaces WHERE id = $1")
        .bind(interface_id)
        .execute(&state.db)
        .await
    {
        Ok(result) if result.rows_affected() > 0 => StatusCode::NO_CONTENT.into_response(),
        Ok(_) => StatusCode::NOT_FOUND.into_response(),
        Err(error) => {
            tracing::error!("delete interface failed: {error}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn list_interface_properties(
    _user: AuthUser,
    State(state): State<AppState>,
    Path(interface_id): Path<Uuid>,
) -> impl IntoResponse {
    match sqlx::query_as::<_, InterfaceProperty>(
        r#"SELECT * FROM interface_properties
           WHERE interface_id = $1
           ORDER BY created_at ASC"#,
    )
    .bind(interface_id)
    .fetch_all(&state.db)
    .await
    {
        Ok(data) => Json(json!({ "data": data })).into_response(),
        Err(error) => {
            tracing::error!("list interface properties failed: {error}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn create_interface_property(
    _user: AuthUser,
    State(state): State<AppState>,
    Path(interface_id): Path<Uuid>,
    Json(body): Json<CreateInterfacePropertyRequest>,
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
    match sqlx::query_as::<_, InterfaceProperty>(
        r#"INSERT INTO interface_properties (
               id, interface_id, name, display_name, description, property_type,
               required, unique_constraint, time_dependent, default_value, validation_rules
           )
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
           RETURNING *"#,
    )
    .bind(id)
    .bind(interface_id)
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
    .await
    {
        Ok(property) => (StatusCode::CREATED, Json(json!(property))).into_response(),
        Err(error) => {
            tracing::error!("create interface property failed: {error}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn update_interface_property(
    _user: AuthUser,
    State(state): State<AppState>,
    Path((_interface_id, property_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<UpdateInterfacePropertyRequest>,
) -> impl IntoResponse {
    let existing =
        match sqlx::query_as::<_, InterfaceProperty>("SELECT * FROM interface_properties WHERE id = $1")
            .bind(property_id)
            .fetch_optional(&state.db)
            .await
        {
            Ok(Some(property)) => property,
            Ok(None) => return StatusCode::NOT_FOUND.into_response(),
            Err(error) => {
                tracing::error!("update interface property lookup failed: {error}");
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
        };

    let next_default = body.default_value.or(existing.default_value.clone());
    if let Some(default_value) = &next_default {
        if let Err(error) = validate_property_value(&existing.property_type, default_value) {
            return (StatusCode::BAD_REQUEST, Json(json!({ "error": error }))).into_response();
        }
    }

    match sqlx::query_as::<_, InterfaceProperty>(
        r#"UPDATE interface_properties
           SET display_name = COALESCE($2, display_name),
               description = COALESCE($3, description),
               required = COALESCE($4, required),
               unique_constraint = COALESCE($5, unique_constraint),
               time_dependent = COALESCE($6, time_dependent),
               default_value = $7,
               validation_rules = $8,
               updated_at = NOW()
           WHERE id = $1
           RETURNING *"#,
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
            tracing::error!("update interface property failed: {error}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn delete_interface_property(
    _user: AuthUser,
    State(state): State<AppState>,
    Path((_interface_id, property_id)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
    match sqlx::query("DELETE FROM interface_properties WHERE id = $1")
        .bind(property_id)
        .execute(&state.db)
        .await
    {
        Ok(result) if result.rows_affected() > 0 => StatusCode::NO_CONTENT.into_response(),
        Ok(_) => StatusCode::NOT_FOUND.into_response(),
        Err(error) => {
            tracing::error!("delete interface property failed: {error}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn attach_interface_to_type(
    _user: AuthUser,
    State(state): State<AppState>,
    Path((type_id, interface_id)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
    match sqlx::query_as::<_, ObjectTypeInterfaceBinding>(
        r#"INSERT INTO object_type_interfaces (object_type_id, interface_id)
           VALUES ($1, $2)
           ON CONFLICT (object_type_id, interface_id) DO NOTHING
           RETURNING object_type_id, interface_id, created_at"#,
    )
    .bind(type_id)
    .bind(interface_id)
    .fetch_optional(&state.db)
    .await
    {
        Ok(Some(binding)) => (StatusCode::CREATED, Json(json!(binding))).into_response(),
        Ok(None) => Json(json!({
            "object_type_id": type_id,
            "interface_id": interface_id,
            "status": "attached",
        }))
        .into_response(),
        Err(error) => {
            tracing::error!("attach interface failed: {error}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn list_type_interfaces(
    _user: AuthUser,
    State(state): State<AppState>,
    Path(type_id): Path<Uuid>,
) -> impl IntoResponse {
    match sqlx::query_as::<_, OntologyInterface>(
        r#"SELECT i.*
           FROM ontology_interfaces i
           INNER JOIN object_type_interfaces oti ON oti.interface_id = i.id
           WHERE oti.object_type_id = $1
           ORDER BY i.created_at ASC"#,
    )
    .bind(type_id)
    .fetch_all(&state.db)
    .await
    {
        Ok(data) => Json(json!({ "data": data })).into_response(),
        Err(error) => {
            tracing::error!("list type interfaces failed: {error}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn detach_interface_from_type(
    _user: AuthUser,
    State(state): State<AppState>,
    Path((type_id, interface_id)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
    match sqlx::query(
        "DELETE FROM object_type_interfaces WHERE object_type_id = $1 AND interface_id = $2",
    )
    .bind(type_id)
    .bind(interface_id)
    .execute(&state.db)
    .await
    {
        Ok(result) if result.rows_affected() > 0 => StatusCode::NO_CONTENT.into_response(),
        Ok(_) => StatusCode::NOT_FOUND.into_response(),
        Err(error) => {
            tracing::error!("detach interface failed: {error}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
