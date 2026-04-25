use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use std::collections::BTreeSet;
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
    handlers::actions::{ensure_action_actor_permission, ensure_action_target_permission},
    models::{
        action_type::ActionType,
        graph::{GraphQuery, GraphResponse},
        object_view::{
            ObjectSimulationImpactSummary, ObjectSimulationRequest, ObjectSimulationResponse,
            ObjectViewResponse,
        },
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
    let data = objects
        .into_iter()
        .skip(offset)
        .take(per_page)
        .collect::<Vec<_>>();

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
    claims: &auth_middleware::claims::Claims,
    object: &ObjectInstance,
) -> Result<Vec<ActionType>, String> {
    let rows = sqlx::query_as::<_, crate::models::action_type::ActionTypeRow>(
        r#"SELECT id, name, display_name, description, object_type_id, operation_kind,
                  input_schema, config, confirmation_required, permission_key, authorization_policy,
                  owner_id,
                  created_at, updated_at
           FROM action_types
           WHERE object_type_id = $1
           ORDER BY created_at DESC"#,
    )
    .bind(object.object_type_id)
    .fetch_all(&state.db)
    .await
    .map_err(|error| format!("failed to load actions: {error}"))?;

    let mut actions = Vec::new();
    for row in rows {
        let action = ActionType::try_from(row)
            .map_err(|error| format!("failed to decode actions: {error}"))?;
        if ensure_action_actor_permission(claims, &action).is_ok()
            && ensure_action_target_permission(&action, Some(object)).is_ok()
        {
            actions.push(action);
        }
    }

    Ok(actions)
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

fn collect_changed_properties(
    manual_patch: &Map<String, Value>,
    action_preview: Option<&Value>,
) -> Vec<String> {
    let mut properties = BTreeSet::new();
    for key in manual_patch.keys() {
        properties.insert(key.clone());
    }

    if let Some(action_patch) = action_preview
        .and_then(|preview| preview.get("patch"))
        .and_then(Value::as_object)
    {
        for key in action_patch.keys() {
            properties.insert(key.clone());
        }
    }

    properties.into_iter().collect()
}

fn extract_graph_object_ids(graph: &GraphResponse) -> Vec<Uuid> {
    let mut impacted = BTreeSet::new();
    for node in &graph.nodes {
        if node.kind != "object_instance" {
            continue;
        }
        let Some(value) = node.id.strip_prefix("object:") else {
            continue;
        };
        if let Ok(object_id) = Uuid::parse_str(value) {
            impacted.insert(object_id);
        }
    }

    let mut ordered = Vec::new();
    if let Some(root_object_id) = graph.root_object_id {
        ordered.push(root_object_id);
        impacted.remove(&root_object_id);
    }
    ordered.extend(impacted);
    ordered
}

fn build_simulation_impact_summary(
    graph: &GraphResponse,
    action_preview: &Value,
    matching_rules: usize,
    changed_properties: &[String],
    impacted_object_count: usize,
    predicted_delete: bool,
) -> ObjectSimulationImpactSummary {
    let impacted_types = graph
        .summary
        .object_types
        .keys()
        .cloned()
        .collect::<Vec<_>>();

    ObjectSimulationImpactSummary {
        scope: graph.summary.scope.clone(),
        action_kind: action_preview
            .get("kind")
            .and_then(Value::as_str)
            .unwrap_or("manual_patch")
            .to_string(),
        predicted_delete,
        impacted_object_count,
        impacted_type_count: impacted_types.len(),
        impacted_types,
        direct_neighbors: graph.summary.root_neighbor_count,
        max_hops_reached: graph.summary.max_hops_reached,
        boundary_crossings: graph.summary.boundary_crossings,
        sensitive_objects: graph.summary.sensitive_objects,
        sensitive_markings: graph.summary.sensitive_markings.clone(),
        matching_rules,
        changed_properties: changed_properties.to_vec(),
    }
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
    let actions = match load_applicable_actions(&state, &claims, &object).await {
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
            "graph_scope": graph.summary.scope.clone(),
            "sensitive_objects": graph.summary.sensitive_objects,
            "boundary_crossings": graph.summary.boundary_crossings,
            "max_hops_reached": graph.summary.max_hops_reached,
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

    let simulated = match simulate_object_state(
        &state,
        &object,
        &manual_patch,
        action_preview.as_ref(),
    )
    .await
    {
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

    let changed_properties = collect_changed_properties(&manual_patch, action_preview.as_ref());
    let predicted_delete = action_preview
        .as_ref()
        .and_then(|preview| preview.get("kind"))
        .and_then(Value::as_str)
        == Some("delete_object");

    let mut impacted_objects = extract_graph_object_ids(&graph);
    if let Some(counterpart) = action_preview
        .as_ref()
        .and_then(|preview| preview.get("counterpart_object_id"))
        .and_then(Value::as_str)
        .and_then(|value| Uuid::parse_str(value).ok())
    {
        if !impacted_objects.contains(&counterpart) {
            impacted_objects.push(counterpart);
        }
    }
    if impacted_objects.is_empty() {
        impacted_objects.push(obj_id);
    }

    let recent_rule_runs = match load_recent_rule_runs(&state, obj_id, 8).await {
        Ok(runs) => runs,
        Err(error) => return db_error(error),
    };
    let timeline = build_object_timeline(&object, &recent_rule_runs, action_preview.as_ref());
    let action_preview = action_preview.unwrap_or_else(|| {
        json!({
            "kind": "manual_patch",
            "patch": manual_patch,
        })
    });
    let impact_summary = build_simulation_impact_summary(
        &graph,
        &action_preview,
        matching_rules.len(),
        &changed_properties,
        impacted_objects.len(),
        predicted_delete,
    );

    Json(ObjectSimulationResponse {
        before: object_to_json(object.clone()),
        after: simulated.map(object_to_json),
        deleted: predicted_delete,
        action_preview,
        matching_rules,
        graph,
        impact_summary,
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

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{build_simulation_impact_summary, extract_graph_object_ids};
    use crate::models::graph::{GraphEdge, GraphNode, GraphResponse, GraphSummary};
    use uuid::Uuid;

    fn graph_response() -> GraphResponse {
        GraphResponse {
            mode: "object".to_string(),
            root_object_id: Some(Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap()),
            root_type_id: None,
            depth: 2,
            total_nodes: 2,
            total_edges: 1,
            summary: GraphSummary {
                scope: "sensitive_connected".to_string(),
                node_kinds: Default::default(),
                edge_kinds: Default::default(),
                object_types: [("Case".to_string(), 1), ("Customer".to_string(), 1)]
                    .into_iter()
                    .collect(),
                markings: [("public".to_string(), 1), ("pii".to_string(), 1)]
                    .into_iter()
                    .collect(),
                root_neighbor_count: 1,
                max_hops_reached: 1,
                boundary_crossings: 1,
                sensitive_objects: 1,
                sensitive_markings: vec!["pii".to_string()],
            },
            nodes: vec![
                GraphNode {
                    id: "object:00000000-0000-0000-0000-000000000001".to_string(),
                    kind: "object_instance".to_string(),
                    label: "Root".to_string(),
                    secondary_label: Some("Case".to_string()),
                    color: None,
                    route: None,
                    metadata: json!({}),
                },
                GraphNode {
                    id: "object:00000000-0000-0000-0000-000000000002".to_string(),
                    kind: "object_instance".to_string(),
                    label: "Neighbor".to_string(),
                    secondary_label: Some("Customer".to_string()),
                    color: None,
                    route: None,
                    metadata: json!({}),
                },
            ],
            edges: vec![GraphEdge {
                id: "link:1".to_string(),
                kind: "link_instance".to_string(),
                source: "object:00000000-0000-0000-0000-000000000001".to_string(),
                target: "object:00000000-0000-0000-0000-000000000002".to_string(),
                label: "linked".to_string(),
                metadata: json!({}),
            }],
        }
    }

    #[test]
    fn graph_object_ids_keep_root_first() {
        let graph = graph_response();

        let impacted = extract_graph_object_ids(&graph);

        assert_eq!(
            impacted,
            vec![
                Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
                Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap(),
            ]
        );
    }

    #[test]
    fn simulation_impact_summary_reuses_graph_summary() {
        let graph = graph_response();

        let summary = build_simulation_impact_summary(
            &graph,
            &json!({ "kind": "delete_object" }),
            2,
            &["status".to_string(), "risk_score".to_string()],
            2,
            true,
        );

        assert_eq!(summary.scope, "sensitive_connected");
        assert_eq!(summary.action_kind, "delete_object");
        assert!(summary.predicted_delete);
        assert_eq!(summary.impacted_object_count, 2);
        assert_eq!(summary.impacted_type_count, 2);
        assert_eq!(summary.direct_neighbors, 1);
        assert_eq!(summary.matching_rules, 2);
        assert_eq!(summary.changed_properties.len(), 2);
    }
}
