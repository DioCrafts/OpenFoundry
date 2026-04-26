use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::{Value, json};
use std::collections::HashSet;
use uuid::Uuid;

use auth_middleware::layer::AuthUser;

use crate::{
    AppState,
    domain::{
        project_access::{
            OntologyResourceKind, ensure_resource_manage_access, ensure_resource_view_access,
            load_resource_owner_id, load_resource_project_id,
        },
        type_system::{validate_property_type, validate_property_value},
    },
    models::{
        action_type::{ActionType, ActionTypeRow, UpdateObjectActionConfig},
        property::{
            CreatePropertyRequest, Property, PropertyInlineEditConfig, UpdatePropertyRequest,
        },
    },
};

fn forbidden(message: impl Into<String>) -> Response {
    (
        StatusCode::FORBIDDEN,
        Json(json!({ "error": message.into() })),
    )
        .into_response()
}

async fn ensure_object_type_view_access(
    state: &AppState,
    claims: &auth_middleware::Claims,
    object_type_id: Uuid,
) -> Result<(), String> {
    let project_id =
        load_resource_project_id(&state.db, OntologyResourceKind::ObjectType, object_type_id)
            .await
            .map_err(|error| format!("failed to load object type binding: {error}"))?;
    ensure_resource_view_access(&state.db, claims, project_id).await
}

async fn ensure_object_type_manage_access(
    state: &AppState,
    claims: &auth_middleware::Claims,
    object_type_id: Uuid,
) -> Result<(), String> {
    let owner_id =
        load_resource_owner_id(&state.db, OntologyResourceKind::ObjectType, object_type_id)
            .await?
            .ok_or_else(|| "object type not found".to_string())?;
    let project_id =
        load_resource_project_id(&state.db, OntologyResourceKind::ObjectType, object_type_id)
            .await
            .map_err(|error| format!("failed to load object type binding: {error}"))?;
    ensure_resource_manage_access(&state.db, claims, owner_id, project_id).await
}

fn extract_operation_config(config: &Value) -> Value {
    config
        .as_object()
        .and_then(|object| {
            if object.contains_key("operation") || object.contains_key("notification_side_effects")
            {
                Some(object.get("operation").cloned().unwrap_or(Value::Null))
            } else {
                None
            }
        })
        .unwrap_or_else(|| config.clone())
}

fn resolve_inline_edit_input_name(
    action: &ActionType,
    property_name: &str,
    inline_edit_config: &PropertyInlineEditConfig,
) -> Result<String, String> {
    let operation_config = extract_operation_config(&action.config);
    let update_config: UpdateObjectActionConfig = serde_json::from_value(operation_config)
        .map_err(|error| format!("invalid inline edit action config: {error}"))?;

    let candidates = update_config
        .property_mappings
        .into_iter()
        .filter(|mapping| mapping.property_name == property_name)
        .filter_map(|mapping| mapping.input_name)
        .collect::<Vec<_>>();

    if let Some(input_name) = inline_edit_config.input_name.as_deref() {
        if candidates.iter().any(|candidate| candidate == input_name) {
            return Ok(input_name.to_string());
        }
        return Err(format!(
            "inline edit action does not map property '{property_name}' from input '{input_name}'"
        ));
    }

    let unique_candidates = candidates.into_iter().collect::<HashSet<_>>();
    match unique_candidates.len() {
        0 => Err(format!(
            "inline edit action must map property '{property_name}' from an input field"
        )),
        1 => Ok(unique_candidates.into_iter().next().unwrap_or_default()),
        _ => Err(format!(
            "inline edit action maps property '{property_name}' from multiple input fields; configure inline_edit_config.input_name explicitly"
        )),
    }
}

async fn validate_inline_edit_config(
    state: &AppState,
    object_type_id: Uuid,
    property_name: &str,
    property_type: &str,
    inline_edit_config: &PropertyInlineEditConfig,
) -> Result<(), String> {
    let row = sqlx::query_as::<_, ActionTypeRow>(
        r#"SELECT id, name, display_name, description, object_type_id, operation_kind, input_schema,
                  form_schema, config, confirmation_required, permission_key, authorization_policy, owner_id,
                  created_at, updated_at
           FROM action_types
           WHERE id = $1"#,
    )
    .bind(inline_edit_config.action_type_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|error| format!("failed to load inline edit action type: {error}"))?
    .ok_or_else(|| "configured inline edit action type was not found".to_string())?;

    let action = ActionType::try_from(row)
        .map_err(|error| format!("failed to decode inline edit action type: {error}"))?;

    if action.object_type_id != object_type_id {
        return Err(
            "inline edit action must belong to the same object type as the property".to_string(),
        );
    }

    if action.operation_kind != "update_object" {
        return Err("inline edit action must use the update_object operation".to_string());
    }

    let input_name = resolve_inline_edit_input_name(&action, property_name, inline_edit_config)?;
    let Some(input_field) = action
        .input_schema
        .iter()
        .find(|field| field.name == input_name)
    else {
        return Err(format!(
            "inline edit action input field '{input_name}' was not found in the action schema"
        ));
    };

    if input_field.property_type != property_type {
        return Err(format!(
            "inline edit action input '{input_name}' has type '{}' but property '{property_name}' has type '{property_type}'",
            input_field.property_type
        ));
    }

    Ok(())
}

pub async fn list_properties(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path(type_id): Path<Uuid>,
) -> impl IntoResponse {
    if let Err(error) = ensure_object_type_view_access(&state, &claims, type_id).await {
        return forbidden(error);
    }
    match sqlx::query_as::<_, Property>(
        r#"SELECT id, object_type_id, name, display_name, description, property_type, required,
                  unique_constraint, time_dependent, default_value, validation_rules,
                  inline_edit_config, created_at, updated_at
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
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path(type_id): Path<Uuid>,
    Json(body): Json<CreatePropertyRequest>,
) -> impl IntoResponse {
    if let Err(error) = ensure_object_type_manage_access(&state, &claims, type_id).await {
        return if error == "object type not found" {
            StatusCode::NOT_FOUND.into_response()
        } else {
            forbidden(error)
        };
    }
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
    if let Some(inline_edit_config) = &body.inline_edit_config {
        if let Err(error) = validate_inline_edit_config(
            &state,
            type_id,
            &body.name,
            &body.property_type,
            inline_edit_config,
        )
        .await
        {
            return (StatusCode::BAD_REQUEST, Json(json!({ "error": error }))).into_response();
        }
    }

    let id = Uuid::now_v7();
    let display_name = body.display_name.unwrap_or_else(|| body.name.clone());
    let inline_edit_config = body
        .inline_edit_config
        .map(|config| serde_json::to_value(config).unwrap_or(Value::Null));
    let result = sqlx::query_as::<_, Property>(
        r#"INSERT INTO properties (
               id, object_type_id, name, display_name, description, property_type,
               required, unique_constraint, time_dependent, default_value, validation_rules,
               inline_edit_config
           )
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
           RETURNING id, object_type_id, name, display_name, description, property_type, required,
                     unique_constraint, time_dependent, default_value, validation_rules,
                     inline_edit_config, created_at, updated_at"#,
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
    .bind(inline_edit_config)
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
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path((_type_id, property_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<UpdatePropertyRequest>,
) -> impl IntoResponse {
    let existing = match sqlx::query_as::<_, Property>(
        r#"SELECT id, object_type_id, name, display_name, description, property_type, required,
                  unique_constraint, time_dependent, default_value, validation_rules,
                  inline_edit_config, created_at, updated_at
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

    if let Err(error) =
        ensure_object_type_manage_access(&state, &claims, existing.object_type_id).await
    {
        return if error == "object type not found" {
            StatusCode::NOT_FOUND.into_response()
        } else {
            forbidden(error)
        };
    }

    let next_default = body.default_value.or(existing.default_value.clone());
    if let Some(default_value) = &next_default {
        if let Err(error) = validate_property_value(&existing.property_type, default_value) {
            return (StatusCode::BAD_REQUEST, Json(json!({ "error": error }))).into_response();
        }
    }
    let next_inline_edit_config = match body.inline_edit_config.clone() {
        Some(next) => next,
        None => existing.inline_edit_config.clone(),
    };
    if let Some(inline_edit_config) = &next_inline_edit_config {
        if let Err(error) = validate_inline_edit_config(
            &state,
            existing.object_type_id,
            &existing.name,
            &existing.property_type,
            inline_edit_config,
        )
        .await
        {
            return (StatusCode::BAD_REQUEST, Json(json!({ "error": error }))).into_response();
        }
    }
    let next_inline_edit_config_value =
        next_inline_edit_config.map(|config| serde_json::to_value(config).unwrap_or(Value::Null));

    match sqlx::query_as::<_, Property>(
        r#"UPDATE properties
           SET display_name = COALESCE($2, display_name),
               description = COALESCE($3, description),
               required = COALESCE($4, required),
               unique_constraint = COALESCE($5, unique_constraint),
               time_dependent = COALESCE($6, time_dependent),
               default_value = $7,
               validation_rules = $8,
               inline_edit_config = $9,
               updated_at = NOW()
           WHERE id = $1
           RETURNING id, object_type_id, name, display_name, description, property_type, required,
                     unique_constraint, time_dependent, default_value, validation_rules,
                     inline_edit_config, created_at, updated_at"#,
    )
    .bind(property_id)
    .bind(body.display_name)
    .bind(body.description)
    .bind(body.required)
    .bind(body.unique_constraint)
    .bind(body.time_dependent)
    .bind(next_default)
    .bind(body.validation_rules.or(existing.validation_rules.clone()))
    .bind(next_inline_edit_config_value)
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
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path((_type_id, property_id)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
    let existing_type_id = match sqlx::query_scalar::<_, Uuid>(
        "SELECT object_type_id FROM properties WHERE id = $1",
    )
    .bind(property_id)
    .fetch_optional(&state.db)
    .await
    {
        Ok(Some(object_type_id)) => object_type_id,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(error) => {
            tracing::error!("delete property lookup failed: {error}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    if let Err(error) = ensure_object_type_manage_access(&state, &claims, existing_type_id).await {
        return if error == "object type not found" {
            StatusCode::NOT_FOUND.into_response()
        } else {
            forbidden(error)
        };
    }

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
