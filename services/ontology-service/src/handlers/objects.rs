use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use uuid::Uuid;

use auth_middleware::layer::AuthUser;

use crate::{
    AppState,
    domain::{
        access::{ensure_object_access, validate_marking},
        function_runtime::{load_accessible_object_set, load_linked_objects, object_to_json},
        graph,
        rules::{evaluate_rules_for_object, load_recent_rule_runs},
        schema::{load_effective_properties, validate_object_properties},
    },
    handlers::actions::preview_action_for_simulation,
    models::{
        action_type::ActionType,
        graph::GraphQuery,
        object_view::{ObjectSimulationRequest, ObjectSimulationResponse, ObjectViewResponse},
        rule::RuleMatchResponse,
    },
};

fn invalid(message: impl Into<String>) -> axum::response::Response {
    (
        StatusCode::BAD_REQUEST,
        Json(json!({ "error": message.into() })),
    )
        .into_response()
}

fn db_error(message: impl Into<String>) -> axum::response::Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({ "error": message.into() })),
    )
        .into_response()
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct ObjectInstance {
    pub id: Uuid,
    pub object_type_id: Uuid,
    pub properties: Value,
    pub created_by: Uuid,
    pub organization_id: Option<Uuid>,
    pub marking: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateObjectRequest {
    pub properties: Value,
    pub marking: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateObjectRequest {
    pub properties: Value,
    pub replace: Option<bool>,
    pub marking: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListObjectsQuery {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct QueryObjectsRequest {
    #[serde(default)]
    pub equals: Value,
    pub limit: Option<usize>,
}

pub async fn create_object(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path(type_id): Path<Uuid>,
    Json(body): Json<CreateObjectRequest>,
) -> impl IntoResponse {
    let marking = body.marking.unwrap_or_else(|| "public".to_string());
    if let Err(error) = validate_marking(&marking) {
        return (StatusCode::BAD_REQUEST, Json(json!({ "error": error }))).into_response();
    }

    let definitions = match load_effective_properties(&state.db, type_id).await {
        Ok(definitions) => definitions,
        Err(error) => {
            tracing::error!("load effective properties failed: {error}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };
    let properties = match validate_object_properties(&definitions, &body.properties) {
        Ok(properties) => properties,
        Err(error) => {
            return (StatusCode::BAD_REQUEST, Json(json!({ "error": error }))).into_response();
        }
    };

    let id = Uuid::now_v7();
    let result = sqlx::query_as::<_, ObjectInstance>(
        r#"INSERT INTO object_instances (id, object_type_id, properties, created_by, organization_id, marking)
           VALUES ($1, $2, $3, $4, $5, $6)
           RETURNING id, object_type_id, properties, created_by, organization_id, marking, created_at, updated_at"#,
    )
    .bind(id)
    .bind(type_id)
    .bind(&properties)
    .bind(claims.sub)
    .bind(claims.org_id)
    .bind(&marking)
    .fetch_one(&state.db)
    .await;

    match result {
        Ok(obj) => (StatusCode::CREATED, Json(json!(obj))).into_response(),
        Err(error) => {
            tracing::error!("create object failed: {error}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn list_objects(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path(type_id): Path<Uuid>,
    Query(params): Query<ListObjectsQuery>,
) -> impl IntoResponse {
    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(20).clamp(1, 100) as usize;

    let objects = match sqlx::query_as::<_, ObjectInstance>(
        r#"SELECT id, object_type_id, properties, created_by, organization_id, marking, created_at, updated_at
           FROM object_instances
           WHERE object_type_id = $1
           ORDER BY created_at DESC"#,
    )
    .bind(type_id)
    .fetch_all(&state.db)
    .await
    {
        Ok(objects) => objects
            .into_iter()
            .filter(|object| ensure_object_access(&claims, object).is_ok())
            .collect::<Vec<_>>(),
        Err(error) => {
            tracing::error!("list objects failed: {error}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let total = objects.len();
    let offset = (page.saturating_sub(1) as usize) * per_page;
    let data = objects.into_iter().skip(offset).take(per_page).collect::<Vec<_>>();

    Json(json!({
        "data": data,
        "total": total,
        "page": page,
        "per_page": per_page,
    }))
    .into_response()
}

pub async fn get_object(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path((_type_id, obj_id)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
    match load_object_instance(&state.db, obj_id).await {
        Ok(Some(object)) => match ensure_object_access(&claims, &object) {
            Ok(_) => Json(json!(object)).into_response(),
            Err(error) => (StatusCode::FORBIDDEN, Json(json!({ "error": error }))).into_response(),
        },
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(error) => {
            tracing::error!("get object failed: {error}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn update_object(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path((_type_id, obj_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<UpdateObjectRequest>,
) -> impl IntoResponse {
    let object = match load_object_instance(&state.db, obj_id).await {
        Ok(Some(object)) => object,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(error) => {
            tracing::error!("update object lookup failed: {error}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    if let Err(error) = ensure_object_access(&claims, &object) {
        return (StatusCode::FORBIDDEN, Json(json!({ "error": error }))).into_response();
    }

    if let Some(marking) = &body.marking {
        if let Err(error) = validate_marking(marking) {
            return (StatusCode::BAD_REQUEST, Json(json!({ "error": error }))).into_response();
        }
    }

    let definitions = match load_effective_properties(&state.db, object.object_type_id).await {
        Ok(definitions) => definitions,
        Err(error) => {
            tracing::error!("load effective properties failed: {error}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let next_properties = if body.replace.unwrap_or(false) {
        body.properties.clone()
    } else {
        let mut merged = object.properties.as_object().cloned().unwrap_or_default();
        let Some(patch) = body.properties.as_object() else {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "properties must be a JSON object when replace=false" })),
            )
                .into_response();
        };
        for (key, value) in patch {
            merged.insert(key.clone(), value.clone());
        }
        Value::Object(merged)
    };

    let normalized = match validate_object_properties(&definitions, &next_properties) {
        Ok(normalized) => normalized,
        Err(error) => {
            return (StatusCode::BAD_REQUEST, Json(json!({ "error": error }))).into_response();
        }
    };

    match sqlx::query_as::<_, ObjectInstance>(
        r#"UPDATE object_instances
           SET properties = $2::jsonb,
               marking = COALESCE($3, marking),
               updated_at = NOW()
           WHERE id = $1
           RETURNING id, object_type_id, properties, created_by, organization_id, marking, created_at, updated_at"#,
    )
    .bind(obj_id)
    .bind(normalized)
    .bind(body.marking)
    .fetch_optional(&state.db)
    .await
    {
        Ok(Some(object)) => Json(json!(object)).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(error) => {
            tracing::error!("update object failed: {error}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn delete_object(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path((_type_id, obj_id)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
    let object = match load_object_instance(&state.db, obj_id).await {
        Ok(Some(object)) => object,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(error) => {
            tracing::error!("delete object lookup failed: {error}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    if let Err(error) = ensure_object_access(&claims, &object) {
        return (StatusCode::FORBIDDEN, Json(json!({ "error": error }))).into_response();
    }

    match sqlx::query("DELETE FROM object_instances WHERE id = $1")
        .bind(obj_id)
        .execute(&state.db)
        .await
    {
        Ok(result) if result.rows_affected() > 0 => StatusCode::NO_CONTENT.into_response(),
        Ok(_) => StatusCode::NOT_FOUND.into_response(),
        Err(error) => {
            tracing::error!("delete object failed: {error}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn query_objects(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path(type_id): Path<Uuid>,
    Json(body): Json<QueryObjectsRequest>,
) -> impl IntoResponse {
    let Some(equals) = body.equals.as_object() else {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "equals must be a JSON object" })),
        )
            .into_response();
    };

    let limit = body.limit.unwrap_or(50).clamp(1, 500);
    let objects = match load_accessible_object_set(&state, &claims, type_id).await {
        Ok(objects) => objects,
        Err(error) => {
            tracing::error!("object query failed: {error}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let data = objects
        .into_iter()
        .filter(|object| {
            object
                .get("properties")
                .and_then(Value::as_object)
                .map(|properties| {
                    equals
                        .iter()
                        .all(|(key, expected)| properties.get(key) == Some(expected))
                })
                .unwrap_or(false)
        })
        .take(limit)
        .collect::<Vec<_>>();

    Json(json!({
        "data": data,
        "total": data.len(),
    }))
    .into_response()
}

pub async fn list_neighbors(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path((_type_id, obj_id)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
    let object = match load_object_instance(&state.db, obj_id).await {
        Ok(Some(object)) => object,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(error) => {
            tracing::error!("neighbor lookup failed: {error}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    if let Err(error) = ensure_object_access(&claims, &object) {
        return (StatusCode::FORBIDDEN, Json(json!({ "error": error }))).into_response();
    }

    match load_linked_objects(&state, &claims, obj_id).await {
        Ok(data) => Json(json!({ "data": data })).into_response(),
        Err(error) => {
            tracing::error!("list neighbors failed: {error}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

async fn load_applicable_actions(
    state: &AppState,
    object_type_id: Uuid,
) -> Result<Vec<ActionType>, String> {
    let rows = sqlx::query_as::<_, crate::models::action_type::ActionTypeRow>(
        r#"SELECT id, name, display_name, description, object_type_id, operation_kind,
                  input_schema, config, confirmation_required, permission_key, owner_id,
                  created_at, updated_at
           FROM action_types
           WHERE object_type_id = $1
           ORDER BY created_at DESC"#,
    )
    .bind(object_type_id)
    .fetch_all(&state.db)
    .await
    .map_err(|error| format!("failed to load actions: {error}"))?;

    rows.into_iter()
        .map(ActionType::try_from)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("failed to decode actions: {error}"))
}

fn build_object_timeline(
    object: &ObjectInstance,
    recent_rule_runs: &[crate::models::rule::OntologyRuleRun],
    action_preview: Option<&Value>,
) -> Vec<Value> {
    let mut timeline = vec![
        json!({
            "kind": "created",
            "at": object.created_at,
            "object_id": object.id,
        }),
        json!({
            "kind": "updated",
            "at": object.updated_at,
            "object_id": object.id,
        }),
    ];

    for run in recent_rule_runs {
        timeline.push(json!({
            "kind": if run.simulated { "rule_simulated" } else { "rule_applied" },
            "at": run.created_at,
            "rule_id": run.rule_id,
            "matched": run.matched,
            "effect_preview": run.effect_preview,
        }));
    }

    if let Some(action_preview) = action_preview {
        timeline.push(json!({
            "kind": "simulated_action",
            "at": chrono::Utc::now(),
            "preview": action_preview,
        }));
    }

    timeline.sort_by(|left, right| {
        right["at"]
            .as_str()
            .unwrap_or_default()
            .cmp(left["at"].as_str().unwrap_or_default())
    });
    timeline
}

async fn simulate_object_state(
    state: &AppState,
    object: &ObjectInstance,
    manual_patch: &Map<String, Value>,
    action_preview: Option<&Value>,
) -> Result<Option<ObjectInstance>, String> {
    let mut merged = object.properties.as_object().cloned().unwrap_or_default();
    for (key, value) in manual_patch {
        merged.insert(key.clone(), value.clone());
    }

    if let Some(action_patch) = action_preview
        .and_then(|preview| preview.get("patch"))
        .and_then(Value::as_object)
    {
        for (key, value) in action_patch {
            merged.insert(key.clone(), value.clone());
        }
    }

    if action_preview
        .and_then(|preview| preview.get("kind"))
        .and_then(Value::as_str)
        == Some("delete_object")
    {
        return Ok(None);
    }

    let definitions = load_effective_properties(&state.db, object.object_type_id)
        .await
        .map_err(|error| format!("failed to load property definitions: {error}"))?;
    let normalized = validate_object_properties(&definitions, &Value::Object(merged))
        .map_err(|error| format!("invalid simulated object patch: {error}"))?;

    let mut simulated = object.clone();
    simulated.properties = normalized;
    simulated.updated_at = chrono::Utc::now();
    Ok(Some(simulated))
}

pub async fn get_object_view(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path((type_id, obj_id)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
    let object = match load_object_instance(&state.db, obj_id).await {
        Ok(Some(object)) => object,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(error) => {
            tracing::error!("object view lookup failed: {error}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    if object.object_type_id != type_id {
        return StatusCode::NOT_FOUND.into_response();
    }
    if let Err(error) = ensure_object_access(&claims, &object) {
        return (StatusCode::FORBIDDEN, Json(json!({ "error": error }))).into_response();
    }

    let neighbors = match load_linked_objects(&state, &claims, obj_id).await {
        Ok(neighbors) => neighbors,
        Err(error) => return db_error(error),
    };
    let graph = match graph::build_graph(
        &state,
        &claims,
        &GraphQuery {
            root_object_id: Some(obj_id),
            root_type_id: None,
            depth: Some(2),
            limit: Some(40),
        },
    )
    .await
    {
        Ok(graph) => graph,
        Err(error) => return db_error(error),
    };
    let actions = match load_applicable_actions(&state, type_id).await {
        Ok(actions) => actions,
        Err(error) => return db_error(error),
    };
    let matching_rules = match evaluate_rules_for_object(&state, &object, None).await {
        Ok(matches) => matches
            .into_iter()
            .map(|(_, match_result)| match_result)
            .filter(|match_result| match_result.matched)
            .collect::<Vec<RuleMatchResponse>>(),
        Err(error) => return db_error(error),
    };
    let recent_rule_runs = match load_recent_rule_runs(&state, obj_id, 12).await {
        Ok(runs) => runs,
        Err(error) => return db_error(error),
    };
    let timeline = build_object_timeline(&object, &recent_rule_runs, None);

    Json(ObjectViewResponse {
        object: object_to_json(object.clone()),
        summary: json!({
            "neighbor_count": neighbors.len(),
            "graph_nodes": graph.total_nodes,
            "graph_edges": graph.total_edges,
            "matching_rules": matching_rules.len(),
            "recent_rule_runs": recent_rule_runs.len(),
        }),
        neighbors,
        graph,
        applicable_actions: actions,
        matching_rules,
        recent_rule_runs,
        timeline,
    })
    .into_response()
}

pub async fn simulate_object(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path((type_id, obj_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<ObjectSimulationRequest>,
) -> impl IntoResponse {
    let object = match load_object_instance(&state.db, obj_id).await {
        Ok(Some(object)) => object,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(error) => {
            tracing::error!("object simulation lookup failed: {error}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    if object.object_type_id != type_id {
        return StatusCode::NOT_FOUND.into_response();
    }
    if let Err(error) = ensure_object_access(&claims, &object) {
        return (StatusCode::FORBIDDEN, Json(json!({ "error": error }))).into_response();
    }

    let manual_patch = match body.properties_patch.as_object() {
        Some(patch) => patch.clone(),
        None if body.properties_patch.is_null() => Map::new(),
        None => return invalid("properties_patch must be a JSON object"),
    };

    let action_preview = match body.action_id {
        Some(action_id) => match preview_action_for_simulation(
            &state,
            &claims,
            action_id,
            Some(obj_id),
            body.action_parameters.clone(),
        )
        .await
        {
            Ok(preview) => Some(preview),
            Err(error) => return invalid(error),
        },
        None => None,
    };

    let simulated = match simulate_object_state(&state, &object, &manual_patch, action_preview.as_ref()).await {
        Ok(simulated) => simulated,
        Err(error) => return invalid(error),
    };

    let mut combined_patch = manual_patch.clone();
    if let Some(action_patch) = action_preview
        .as_ref()
        .and_then(|preview| preview.get("patch"))
        .and_then(Value::as_object)
    {
        for (key, value) in action_patch {
            combined_patch.insert(key.clone(), value.clone());
        }
    }

    let matching_rules = match evaluate_rules_for_object(
        &state,
        &object,
        if combined_patch.is_empty() {
            None
        } else {
            Some(&combined_patch)
        },
    )
    .await
    {
        Ok(matches) => matches
            .into_iter()
            .map(|(_, match_result)| match_result)
            .filter(|match_result| match_result.matched)
            .collect::<Vec<RuleMatchResponse>>(),
        Err(error) => return db_error(error),
    };

    let graph = match graph::build_graph(
        &state,
        &claims,
        &GraphQuery {
            root_object_id: Some(obj_id),
            root_type_id: None,
            depth: Some(body.depth.unwrap_or(2).clamp(1, 4)),
            limit: Some(50),
        },
    )
    .await
    {
        Ok(graph) => graph,
        Err(error) => return db_error(error),
    };

    let mut impacted_objects = vec![obj_id];
    if let Some(counterpart) = action_preview
        .as_ref()
        .and_then(|preview| preview.get("counterpart_object_id"))
        .and_then(Value::as_str)
        .and_then(|value| Uuid::parse_str(value).ok())
    {
        impacted_objects.push(counterpart);
    }

    let recent_rule_runs = match load_recent_rule_runs(&state, obj_id, 8).await {
        Ok(runs) => runs,
        Err(error) => return db_error(error),
    };
    let timeline = build_object_timeline(&object, &recent_rule_runs, action_preview.as_ref());

    Json(ObjectSimulationResponse {
        before: object_to_json(object.clone()),
        after: simulated.map(object_to_json),
        deleted: action_preview
            .as_ref()
            .and_then(|preview| preview.get("kind"))
            .and_then(Value::as_str)
            == Some("delete_object"),
        action_preview: action_preview.unwrap_or_else(|| json!({
            "kind": "manual_patch",
            "patch": manual_patch,
        })),
        matching_rules,
        graph,
        impacted_objects,
        timeline,
    })
    .into_response()
}

pub async fn load_object_instance(
    db: &sqlx::PgPool,
    obj_id: Uuid,
) -> Result<Option<ObjectInstance>, sqlx::Error> {
    sqlx::query_as::<_, ObjectInstance>(
        r#"SELECT id, object_type_id, properties, created_by, organization_id, marking, created_at, updated_at
           FROM object_instances
           WHERE id = $1"#,
    )
    .bind(obj_id)
    .fetch_optional(db)
    .await
}
