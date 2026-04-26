use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use uuid::Uuid;

use auth_middleware::layer::AuthUser;

use crate::{
    AppState,
    domain::{
        access::{ensure_object_access, validate_marking},
        function_runtime::{load_accessible_object_set, load_linked_objects, object_to_json},
        graph,
        rules::{
            evaluate_rule_against_object, evaluate_rules_for_object, load_recent_rule_runs,
            load_rules_for_object_type,
        },
        schema::{load_effective_properties, validate_object_properties},
        search::semantic::cosine_similarity,
    },
    handlers::actions::preview_action_for_simulation,
    handlers::actions::{ensure_action_actor_permission, ensure_action_target_permission},
    models::{
        action_type::ActionType,
        graph::{GraphEdge, GraphNode, GraphQuery, GraphResponse},
        object_type::ObjectType,
        object_view::{
            ObjectScenarioSimulationRequest, ObjectScenarioSimulationResponse,
            ObjectSimulationImpactSummary, ObjectSimulationRequest, ObjectSimulationResponse,
            ObjectViewResponse, ScenarioGoalSpec, ScenarioLinkPreview, ScenarioMetricEvaluation,
            ScenarioMetricSpec, ScenarioObjectChange, ScenarioRuleOutcome,
            ScenarioSimulationCandidate, ScenarioSimulationOperation, ScenarioSimulationResult,
            ScenarioSummary, ScenarioSummaryDelta,
        },
        rule::{OntologyRule, RuleEvaluationMode, RuleMatchResponse},
        search::{KnnObjectResult, KnnObjectsRequest, KnnObjectsResponse},
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

#[derive(Debug, Clone)]
struct ScenarioObjectState {
    original: ObjectInstance,
    current: Option<ObjectInstance>,
    changed_properties: BTreeSet<String>,
    sources: BTreeSet<String>,
}

#[derive(Debug, Clone)]
struct ScenarioRuntimeState {
    object_states: BTreeMap<Uuid, ScenarioObjectState>,
    rule_outcomes: Vec<ScenarioRuleOutcome>,
    link_previews: Vec<ScenarioLinkPreview>,
    graph: GraphResponse,
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

pub async fn knn_objects(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path(type_id): Path<Uuid>,
    Json(body): Json<KnnObjectsRequest>,
) -> impl IntoResponse {
    let limit = body.limit.unwrap_or(10).clamp(1, 100);
    let metric = body
        .metric
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("cosine")
        .to_ascii_lowercase();
    if !matches!(metric.as_str(), "cosine" | "dot_product" | "euclidean") {
        return invalid("metric must be one of: cosine, dot_product, euclidean");
    }

    let definitions = match load_effective_properties(&state.db, type_id).await {
        Ok(definitions) => definitions,
        Err(error) => {
            tracing::error!("load effective properties failed for KNN query: {error}");
            return db_error("failed to load object type schema");
        }
    };

    let Some(property_definition) = definitions
        .iter()
        .find(|definition| definition.name == body.property_name)
    else {
        return invalid(format!("unknown property '{}'", body.property_name));
    };

    if property_definition.property_type != "vector" {
        return invalid(format!(
            "property '{}' must be of type vector to run a KNN query",
            body.property_name
        ));
    }

    let query_vector = match (&body.anchor_object_id, &body.query_vector) {
        (Some(_), Some(_)) => {
            return invalid("provide either anchor_object_id or query_vector, but not both");
        }
        (None, None) => return invalid("provide anchor_object_id or query_vector"),
        (None, Some(vector)) => {
            if vector.is_empty() {
                return invalid("query_vector cannot be empty");
            }
            vector.clone()
        }
        (Some(anchor_object_id), None) => {
            let anchor = match load_object_instance(&state.db, *anchor_object_id).await {
                Ok(Some(object)) => object,
                Ok(None) => return StatusCode::NOT_FOUND.into_response(),
                Err(error) => {
                    tracing::error!("anchor object lookup failed for KNN query: {error}");
                    return db_error("failed to load anchor object");
                }
            };

            if anchor.object_type_id != type_id {
                return invalid("anchor_object_id must belong to the object type being queried");
            }

            if let Err(error) = ensure_object_access(&claims, &anchor) {
                return (StatusCode::FORBIDDEN, Json(json!({ "error": error }))).into_response();
            }

            let Some(vector) =
                extract_vector_from_properties(&anchor.properties, &body.property_name)
            else {
                return invalid(format!(
                    "anchor object does not have a valid vector value for '{}'",
                    body.property_name
                ));
            };
            vector
        }
    };

    let exclude_anchor = body
        .exclude_anchor
        .unwrap_or(body.anchor_object_id.is_some());

    let objects = match load_accessible_object_set(&state, &claims, type_id).await {
        Ok(objects) => objects,
        Err(error) => {
            tracing::error!("failed to load candidate object set for KNN query: {error}");
            return db_error("failed to load candidate objects");
        }
    };

    let mut data = objects
        .into_iter()
        .filter_map(|object| {
            if exclude_anchor
                && body.anchor_object_id.is_some()
                && object_json_id(&object) == body.anchor_object_id
            {
                return None;
            }

            let candidate_vector = extract_vector_from_object_json(&object, &body.property_name)?;
            let (score, distance) = knn_score(&metric, &query_vector, &candidate_vector)?;

            Some(KnnObjectResult {
                object,
                score,
                distance,
            })
        })
        .collect::<Vec<_>>();

    data.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    data.truncate(limit);

    Json(json!(KnnObjectsResponse {
        property_name: body.property_name,
        metric,
        total: data.len(),
        data,
    }))
    .into_response()
}

fn extract_vector_from_object_json(object: &Value, property_name: &str) -> Option<Vec<f32>> {
    object
        .get("properties")
        .and_then(|properties| extract_vector_from_properties(properties, property_name))
}

fn extract_vector_from_properties(properties: &Value, property_name: &str) -> Option<Vec<f32>> {
    properties
        .as_object()
        .and_then(|properties| properties.get(property_name))
        .and_then(extract_vector_from_value)
}

fn extract_vector_from_value(value: &Value) -> Option<Vec<f32>> {
    let values = value.as_array()?;
    if values.is_empty() {
        return None;
    }

    values
        .iter()
        .map(|entry| entry.as_f64().map(|value| value as f32))
        .collect()
}

fn object_json_id(object: &Value) -> Option<Uuid> {
    object
        .get("id")
        .and_then(Value::as_str)
        .and_then(|value| Uuid::parse_str(value).ok())
}

fn knn_score(metric: &str, query: &[f32], candidate: &[f32]) -> Option<(f32, Option<f32>)> {
    if query.is_empty() || candidate.is_empty() || query.len() != candidate.len() {
        return None;
    }

    match metric {
        "cosine" => {
            let similarity = cosine_similarity(query, candidate);
            Some((similarity, Some((1.0 - similarity).max(0.0))))
        }
        "dot_product" => Some((dot_product(query, candidate)?, None)),
        "euclidean" => {
            let distance = euclidean_distance(query, candidate)?;
            Some((1.0 / (1.0 + distance), Some(distance)))
        }
        _ => None,
    }
}

fn dot_product(left: &[f32], right: &[f32]) -> Option<f32> {
    if left.is_empty() || right.is_empty() || left.len() != right.len() {
        return None;
    }

    Some(
        left.iter()
            .zip(right.iter())
            .map(|(left, right)| left * right)
            .sum(),
    )
}

fn euclidean_distance(left: &[f32], right: &[f32]) -> Option<f32> {
    if left.is_empty() || right.is_empty() || left.len() != right.len() {
        return None;
    }

    Some(
        left.iter()
            .zip(right.iter())
            .map(|(left, right)| {
                let delta = left - right;
                delta * delta
            })
            .sum::<f32>()
            .sqrt(),
    )
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
                  input_schema, form_schema, config, confirmation_required, permission_key, authorization_policy,
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

pub async fn simulate_object_scenarios(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path((type_id, obj_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<ObjectScenarioSimulationRequest>,
) -> impl IntoResponse {
    if body.scenarios.is_empty() {
        return invalid("scenarios must contain at least one candidate scenario");
    }

    let root_object = match load_object_instance(&state.db, obj_id).await {
        Ok(Some(object)) => object,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(error) => {
            tracing::error!("scenario simulation lookup failed: {error}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };
    if root_object.object_type_id != type_id {
        return StatusCode::NOT_FOUND.into_response();
    }
    if let Err(error) = ensure_object_access(&claims, &root_object) {
        return (StatusCode::FORBIDDEN, Json(json!({ "error": error }))).into_response();
    }

    let depth = body.depth.unwrap_or(2).clamp(1, 4);
    let max_iterations = body.max_iterations.unwrap_or(6).clamp(1, 24);
    let (base_graph, base_object_states, mut type_labels) =
        match load_scenario_context(&state, &claims, &root_object, depth, &body.scenarios).await {
            Ok(context) => context,
            Err(error) => return invalid(error),
        };

    let baseline = if body.include_baseline {
        match run_object_scenario(
            &state,
            &claims,
            &root_object,
            &base_graph,
            &base_object_states,
            &mut type_labels,
            None,
            &body.constraints,
            &body.goals,
            max_iterations,
        )
        .await
        {
            Ok(result) => Some(result),
            Err(error) => return invalid(error),
        }
    } else {
        None
    };

    let mut scenarios = Vec::new();
    for candidate in &body.scenarios {
        let mut result = match run_object_scenario(
            &state,
            &claims,
            &root_object,
            &base_graph,
            &base_object_states,
            &mut type_labels,
            Some(candidate),
            &body.constraints,
            &body.goals,
            max_iterations,
        )
        .await
        {
            Ok(result) => result,
            Err(error) => return invalid(error),
        };
        if let Some(baseline) = baseline.as_ref() {
            result.delta_from_baseline =
                Some(build_summary_delta(&result.summary, &baseline.summary));
        }
        scenarios.push(result);
    }

    Json(ObjectScenarioSimulationResponse {
        root_object_id: root_object.id,
        root_type_id: root_object.object_type_id,
        compared_at: chrono::Utc::now(),
        baseline,
        scenarios,
    })
    .into_response()
}

async fn load_scenario_context(
    state: &AppState,
    claims: &auth_middleware::claims::Claims,
    root_object: &ObjectInstance,
    depth: usize,
    scenarios: &[ScenarioSimulationCandidate],
) -> Result<
    (
        GraphResponse,
        BTreeMap<Uuid, ScenarioObjectState>,
        HashMap<Uuid, String>,
    ),
    String,
> {
    let base_graph = graph::build_graph(
        state,
        claims,
        &GraphQuery {
            root_object_id: Some(root_object.id),
            root_type_id: None,
            depth: Some(depth),
            limit: Some(80),
        },
    )
    .await?;
    let mut object_ids = extract_graph_object_ids(&base_graph)
        .into_iter()
        .collect::<BTreeSet<_>>();
    object_ids.insert(root_object.id);
    for scenario in scenarios {
        for operation in &scenario.operations {
            if let Some(target_object_id) = operation.target_object_id {
                object_ids.insert(target_object_id);
            }
        }
    }

    let mut object_states = BTreeMap::new();
    let mut object_type_ids = BTreeSet::new();
    for object_id in object_ids {
        let object = load_object_instance(&state.db, object_id)
            .await
            .map_err(|error| format!("failed to load scenario object {object_id}: {error}"))?
            .ok_or_else(|| format!("scenario object {object_id} was not found"))?;
        ensure_object_access(claims, &object)?;
        object_type_ids.insert(object.object_type_id);
        object_states.insert(
            object_id,
            ScenarioObjectState {
                original: object.clone(),
                current: Some(object),
                changed_properties: BTreeSet::new(),
                sources: BTreeSet::new(),
            },
        );
    }

    let type_labels =
        load_object_type_labels(state, &object_type_ids.into_iter().collect::<Vec<_>>()).await?;
    Ok((base_graph, object_states, type_labels))
}

async fn run_object_scenario(
    state: &AppState,
    claims: &auth_middleware::claims::Claims,
    root_object: &ObjectInstance,
    base_graph: &GraphResponse,
    base_object_states: &BTreeMap<Uuid, ScenarioObjectState>,
    type_labels: &mut HashMap<Uuid, String>,
    candidate: Option<&ScenarioSimulationCandidate>,
    constraints: &[ScenarioMetricSpec],
    goals: &[ScenarioGoalSpec],
    max_iterations: usize,
) -> Result<ScenarioSimulationResult, String> {
    let mut runtime = ScenarioRuntimeState {
        object_states: base_object_states.clone(),
        rule_outcomes: Vec::new(),
        link_previews: Vec::new(),
        graph: base_graph.clone(),
    };
    let mut property_cache = HashMap::new();
    let mut rule_cache = HashMap::new();

    if let Some(candidate) = candidate {
        for (index, operation) in candidate.operations.iter().enumerate() {
            apply_scenario_operation(
                state,
                claims,
                &mut runtime,
                type_labels,
                root_object.id,
                operation,
                index,
                &mut property_cache,
            )
            .await?;
        }
    }

    let mut queue = runtime
        .object_states
        .iter()
        .map(|(object_id, object_state)| (*object_id, object_state.original.clone()))
        .collect::<VecDeque<_>>();
    let max_steps = max_iterations
        .saturating_mul(runtime.object_states.len().max(1))
        .saturating_mul(8);
    let mut steps = 0usize;
    let mut auto_applied_rules = HashSet::<(Uuid, Uuid)>::new();

    while let Some((object_id, previous_snapshot)) = queue.pop_front() {
        steps += 1;
        if steps > max_steps {
            return Err("scenario propagation reached the configured iteration limit".to_string());
        }

        let Some(current_snapshot) = runtime
            .object_states
            .get(&object_id)
            .and_then(|entry| entry.current.clone())
        else {
            continue;
        };

        let properties_patch =
            diff_properties_between_objects(&previous_snapshot, &current_snapshot);
        let rules =
            load_cached_rules(state, &mut rule_cache, current_snapshot.object_type_id).await?;
        for rule in rules {
            let match_result = evaluate_rule_against_object(
                &rule,
                &previous_snapshot,
                if properties_patch.is_empty() {
                    None
                } else {
                    Some(&properties_patch)
                },
            );
            if !match_result.matched {
                continue;
            }

            let auto_applied = if rule.evaluation_mode == RuleEvaluationMode::Automatic
                && auto_applied_rules.insert((rule.id, object_id))
            {
                let before = current_snapshot.clone();
                let applied = apply_rule_effect_preview_to_runtime(
                    state,
                    &mut runtime,
                    type_labels,
                    &before,
                    &rule,
                    &match_result.effect_preview,
                    &mut property_cache,
                )
                .await?;
                if applied {
                    queue.push_back((object_id, before));
                }
                true
            } else {
                false
            };

            runtime.rule_outcomes.push(ScenarioRuleOutcome {
                object_id,
                rule_id: rule.id,
                rule_name: rule.name.clone(),
                rule_display_name: rule.display_name.clone(),
                evaluation_mode: rule.evaluation_mode.to_string(),
                matched: true,
                auto_applied,
                trigger_payload: match_result.trigger_payload,
                effect_preview: match_result.effect_preview,
            });
        }
    }

    runtime.graph = build_scenario_graph(
        &runtime.graph,
        &runtime.object_states,
        &runtime.link_previews,
        type_labels,
    );
    let object_changes = build_scenario_object_changes(&runtime.object_states, type_labels);
    let impacted_ids = collect_impacted_object_ids(
        &object_changes,
        &runtime.rule_outcomes,
        &runtime.link_previews,
    );
    let constraint_results = evaluate_scenario_constraints(
        &runtime.object_states,
        &runtime.rule_outcomes,
        &runtime.graph,
        &runtime.link_previews,
        constraints,
        &impacted_ids,
    )?;
    let goal_results = evaluate_scenario_goals(
        &runtime.object_states,
        &runtime.rule_outcomes,
        &runtime.graph,
        &runtime.link_previews,
        goals,
        &impacted_ids,
    )?;
    let summary = build_scenario_summary(
        &runtime.object_states,
        &runtime.rule_outcomes,
        &runtime.link_previews,
        &runtime.graph,
        &object_changes,
        &constraint_results,
        &goal_results,
        &impacted_ids,
        type_labels,
    );

    Ok(ScenarioSimulationResult {
        scenario_id: candidate
            .map(|_| {
                format!(
                    "scenario-{}",
                    chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
                )
            })
            .unwrap_or_else(|| "baseline".to_string()),
        name: candidate
            .map(|candidate| candidate.name.clone())
            .unwrap_or_else(|| "Baseline".to_string()),
        description: candidate.and_then(|candidate| candidate.description.clone()),
        graph: runtime.graph,
        object_changes,
        rule_outcomes: runtime.rule_outcomes,
        link_previews: runtime.link_previews,
        constraints: constraint_results,
        goals: goal_results,
        summary,
        delta_from_baseline: None,
    })
}

async fn apply_scenario_operation(
    state: &AppState,
    claims: &auth_middleware::claims::Claims,
    runtime: &mut ScenarioRuntimeState,
    type_labels: &mut HashMap<Uuid, String>,
    root_object_id: Uuid,
    operation: &ScenarioSimulationOperation,
    index: usize,
    property_cache: &mut HashMap<Uuid, Vec<crate::domain::schema::EffectivePropertyDefinition>>,
) -> Result<(), String> {
    let target_object_id = operation.target_object_id.unwrap_or(root_object_id);
    ensure_scenario_object_loaded(state, claims, runtime, type_labels, target_object_id).await?;
    let source_label = operation
        .label
        .clone()
        .unwrap_or_else(|| format!("scenario_operation_{}", index + 1));

    if let Some(action_id) = operation.action_id {
        let preview = preview_action_for_simulation(
            state,
            claims,
            action_id,
            Some(target_object_id),
            operation.action_parameters.clone(),
        )
        .await?;
        apply_action_preview_to_runtime(
            state,
            claims,
            runtime,
            type_labels,
            &preview,
            &source_label,
            property_cache,
        )
        .await?;
    }

    let patch = match operation.properties_patch.as_object() {
        Some(patch) => patch.clone(),
        None if operation.properties_patch.is_null() => Map::new(),
        None => return Err("scenario operation properties_patch must be a JSON object".to_string()),
    };
    if patch.is_empty() {
        if let Some(entry) = runtime.object_states.get_mut(&target_object_id) {
            entry.sources.insert(source_label);
        }
        return Ok(());
    }

    let current = runtime
        .object_states
        .get(&target_object_id)
        .and_then(|entry| entry.current.clone())
        .ok_or_else(|| {
            format!("scenario target object {target_object_id} was deleted earlier in the scenario")
        })?;
    let next = apply_validated_patch_to_object(state, property_cache, &current, &patch).await?;
    update_runtime_object_state(runtime, &current, Some(next), &source_label);
    Ok(())
}

async fn apply_action_preview_to_runtime(
    state: &AppState,
    claims: &auth_middleware::claims::Claims,
    runtime: &mut ScenarioRuntimeState,
    type_labels: &mut HashMap<Uuid, String>,
    preview: &Value,
    source_label: &str,
    property_cache: &mut HashMap<Uuid, Vec<crate::domain::schema::EffectivePropertyDefinition>>,
) -> Result<(), String> {
    let kind = preview
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    match kind {
        "update_object" => {
            let target_object_id = preview
                .get("target_object_id")
                .and_then(Value::as_str)
                .and_then(|value| Uuid::parse_str(value).ok())
                .ok_or_else(|| "action preview is missing target_object_id".to_string())?;
            let patch = preview
                .get("patch")
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_default();
            if patch.is_empty() {
                if let Some(entry) = runtime.object_states.get_mut(&target_object_id) {
                    entry.sources.insert(source_label.to_string());
                }
                return Ok(());
            }
            let current = runtime
                .object_states
                .get(&target_object_id)
                .and_then(|entry| entry.current.clone())
                .ok_or_else(|| "target object was deleted earlier in the scenario".to_string())?;
            let next =
                apply_validated_patch_to_object(state, property_cache, &current, &patch).await?;
            update_runtime_object_state(runtime, &current, Some(next), source_label);
        }
        "delete_object" => {
            let target_object_id = preview
                .get("target_object_id")
                .and_then(Value::as_str)
                .and_then(|value| Uuid::parse_str(value).ok())
                .ok_or_else(|| "action preview is missing target_object_id".to_string())?;
            let current = runtime
                .object_states
                .get(&target_object_id)
                .and_then(|entry| entry.current.clone())
                .ok_or_else(|| {
                    "target object was already deleted earlier in the scenario".to_string()
                })?;
            update_runtime_object_state(runtime, &current, None, source_label);
        }
        "create_link" => {
            let source_object_id = preview
                .get("source_object_id")
                .and_then(Value::as_str)
                .and_then(|value| Uuid::parse_str(value).ok());
            let target_object_id = preview
                .get("linked_object_id")
                .and_then(Value::as_str)
                .and_then(|value| Uuid::parse_str(value).ok());
            if let Some(source_object_id) = source_object_id {
                ensure_scenario_object_loaded(
                    state,
                    claims,
                    runtime,
                    type_labels,
                    source_object_id,
                )
                .await?;
                if let Some(entry) = runtime.object_states.get_mut(&source_object_id) {
                    entry.sources.insert(source_label.to_string());
                }
            }
            if let Some(target_object_id) = target_object_id {
                ensure_scenario_object_loaded(
                    state,
                    claims,
                    runtime,
                    type_labels,
                    target_object_id,
                )
                .await?;
                if let Some(entry) = runtime.object_states.get_mut(&target_object_id) {
                    entry.sources.insert(source_label.to_string());
                }
            }
            runtime.link_previews.push(ScenarioLinkPreview {
                source_object_id,
                target_object_id,
                link_type_id: preview
                    .get("link_type_id")
                    .and_then(Value::as_str)
                    .and_then(|value| Uuid::parse_str(value).ok()),
                preview: preview.clone(),
            });
        }
        _ => {
            if let Some(target_object_id) = preview
                .get("target_object_id")
                .and_then(Value::as_str)
                .and_then(|value| Uuid::parse_str(value).ok())
            {
                ensure_scenario_object_loaded(
                    state,
                    claims,
                    runtime,
                    type_labels,
                    target_object_id,
                )
                .await?;
                if let Some(entry) = runtime.object_states.get_mut(&target_object_id) {
                    entry.sources.insert(source_label.to_string());
                }
            }
        }
    }

    Ok(())
}

async fn apply_rule_effect_preview_to_runtime(
    state: &AppState,
    runtime: &mut ScenarioRuntimeState,
    type_labels: &mut HashMap<Uuid, String>,
    current: &ObjectInstance,
    rule: &OntologyRule,
    effect_preview: &Value,
    property_cache: &mut HashMap<Uuid, Vec<crate::domain::schema::EffectivePropertyDefinition>>,
) -> Result<bool, String> {
    let source_label = format!("rule:{}", rule.display_name);
    if let Some(patch) = effect_preview
        .get("object_patch")
        .and_then(Value::as_object)
        .cloned()
    {
        if !patch.is_empty() {
            let next =
                apply_validated_patch_to_object(state, property_cache, current, &patch).await?;
            update_runtime_object_state(runtime, current, Some(next), &source_label);
            return Ok(true);
        }
    }

    if let Some(entry) = runtime.object_states.get_mut(&current.id) {
        entry.sources.insert(source_label);
    }
    ensure_object_type_label(type_labels, state, current.object_type_id).await?;
    Ok(false)
}

fn update_runtime_object_state(
    runtime: &mut ScenarioRuntimeState,
    previous: &ObjectInstance,
    next: Option<ObjectInstance>,
    source_label: &str,
) {
    let changed = match &next {
        Some(next) => changed_properties_between(previous, next),
        None => previous
            .properties
            .as_object()
            .map(|properties| {
                let mut keys = properties.keys().cloned().collect::<BTreeSet<_>>();
                keys.insert("_deleted".to_string());
                keys
            })
            .unwrap_or_else(|| BTreeSet::from(["_deleted".to_string()])),
    };

    let entry = runtime
        .object_states
        .entry(previous.id)
        .or_insert_with(|| ScenarioObjectState {
            original: previous.clone(),
            current: Some(previous.clone()),
            changed_properties: BTreeSet::new(),
            sources: BTreeSet::new(),
        });
    entry.current = next;
    entry.changed_properties.extend(changed);
    entry.sources.insert(source_label.to_string());
}

async fn ensure_scenario_object_loaded(
    state: &AppState,
    claims: &auth_middleware::claims::Claims,
    runtime: &mut ScenarioRuntimeState,
    type_labels: &mut HashMap<Uuid, String>,
    object_id: Uuid,
) -> Result<(), String> {
    if runtime.object_states.contains_key(&object_id) {
        return Ok(());
    }

    let object = load_object_instance(&state.db, object_id)
        .await
        .map_err(|error| format!("failed to load scenario object {object_id}: {error}"))?
        .ok_or_else(|| format!("scenario object {object_id} was not found"))?;
    ensure_object_access(claims, &object)?;
    ensure_object_type_label(type_labels, state, object.object_type_id).await?;
    runtime.object_states.insert(
        object_id,
        ScenarioObjectState {
            original: object.clone(),
            current: Some(object),
            changed_properties: BTreeSet::new(),
            sources: BTreeSet::new(),
        },
    );
    Ok(())
}

async fn ensure_object_type_label(
    type_labels: &mut HashMap<Uuid, String>,
    state: &AppState,
    object_type_id: Uuid,
) -> Result<(), String> {
    if type_labels.contains_key(&object_type_id) {
        return Ok(());
    }

    let display_name =
        sqlx::query_scalar::<_, String>("SELECT display_name FROM object_types WHERE id = $1")
            .bind(object_type_id)
            .fetch_optional(&state.db)
            .await
            .map_err(|error| format!("failed to load object type label: {error}"))?
            .unwrap_or_else(|| object_type_id.to_string());
    type_labels.insert(object_type_id, display_name);
    Ok(())
}

async fn load_object_type_labels(
    state: &AppState,
    type_ids: &[Uuid],
) -> Result<HashMap<Uuid, String>, String> {
    if type_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let labels = sqlx::query_as::<_, ObjectType>("SELECT * FROM object_types WHERE id = ANY($1)")
        .bind(type_ids)
        .fetch_all(&state.db)
        .await
        .map_err(|error| format!("failed to load object type labels: {error}"))?
        .into_iter()
        .map(|object_type| (object_type.id, object_type.display_name))
        .collect::<HashMap<_, _>>();
    Ok(labels)
}

async fn load_cached_rules(
    state: &AppState,
    cache: &mut HashMap<Uuid, Vec<OntologyRule>>,
    object_type_id: Uuid,
) -> Result<Vec<OntologyRule>, String> {
    if let Some(rules) = cache.get(&object_type_id) {
        return Ok(rules.clone());
    }

    let rules = load_rules_for_object_type(state, object_type_id).await?;
    cache.insert(object_type_id, rules.clone());
    Ok(rules)
}

async fn load_cached_effective_properties(
    state: &AppState,
    cache: &mut HashMap<Uuid, Vec<crate::domain::schema::EffectivePropertyDefinition>>,
    object_type_id: Uuid,
) -> Result<Vec<crate::domain::schema::EffectivePropertyDefinition>, String> {
    if let Some(definitions) = cache.get(&object_type_id) {
        return Ok(definitions.clone());
    }

    let definitions = load_effective_properties(&state.db, object_type_id)
        .await
        .map_err(|error| format!("failed to load property definitions: {error}"))?;
    cache.insert(object_type_id, definitions.clone());
    Ok(definitions)
}

async fn apply_validated_patch_to_object(
    state: &AppState,
    property_cache: &mut HashMap<Uuid, Vec<crate::domain::schema::EffectivePropertyDefinition>>,
    current: &ObjectInstance,
    patch: &Map<String, Value>,
) -> Result<ObjectInstance, String> {
    let definitions =
        load_cached_effective_properties(state, property_cache, current.object_type_id).await?;
    let mut merged = current.properties.as_object().cloned().unwrap_or_default();
    for (key, value) in patch {
        merged.insert(key.clone(), value.clone());
    }
    let normalized = validate_object_properties(&definitions, &Value::Object(merged))
        .map_err(|error| format!("invalid scenario patch: {error}"))?;
    let mut next = current.clone();
    next.properties = normalized;
    next.updated_at = chrono::Utc::now();
    Ok(next)
}

fn diff_properties_between_objects(
    previous: &ObjectInstance,
    current: &ObjectInstance,
) -> Map<String, Value> {
    let previous_properties = previous.properties.as_object().cloned().unwrap_or_default();
    let current_properties = current.properties.as_object().cloned().unwrap_or_default();
    let mut diff = Map::new();

    for (key, value) in &current_properties {
        if previous_properties.get(key) != Some(value) {
            diff.insert(key.clone(), value.clone());
        }
    }
    for key in previous_properties.keys() {
        if !current_properties.contains_key(key) {
            diff.insert(key.clone(), Value::Null);
        }
    }

    diff
}

fn changed_properties_between(
    previous: &ObjectInstance,
    current: &ObjectInstance,
) -> BTreeSet<String> {
    diff_properties_between_objects(previous, current)
        .into_iter()
        .map(|(key, _)| key)
        .collect::<BTreeSet<_>>()
}

fn build_scenario_object_changes(
    object_states: &BTreeMap<Uuid, ScenarioObjectState>,
    type_labels: &HashMap<Uuid, String>,
) -> Vec<ScenarioObjectChange> {
    object_states
        .values()
        .filter(|entry| {
            !entry.changed_properties.is_empty()
                || entry.current.is_none()
                || !entry.sources.is_empty()
        })
        .map(|entry| ScenarioObjectChange {
            object_id: entry.original.id,
            object_type_id: entry.original.object_type_id,
            object_type_label: type_labels
                .get(&entry.original.object_type_id)
                .cloned()
                .unwrap_or_else(|| entry.original.object_type_id.to_string()),
            deleted: entry.current.is_none(),
            changed_properties: entry.changed_properties.iter().cloned().collect(),
            sources: entry.sources.iter().cloned().collect(),
            before: object_to_json(entry.original.clone()),
            after: entry.current.clone().map(object_to_json),
        })
        .collect()
}

fn collect_impacted_object_ids(
    object_changes: &[ScenarioObjectChange],
    rule_outcomes: &[ScenarioRuleOutcome],
    link_previews: &[ScenarioLinkPreview],
) -> BTreeSet<Uuid> {
    let mut impacted = object_changes
        .iter()
        .map(|change| change.object_id)
        .collect::<BTreeSet<_>>();
    for outcome in rule_outcomes {
        impacted.insert(outcome.object_id);
    }
    for link_preview in link_previews {
        if let Some(source_object_id) = link_preview.source_object_id {
            impacted.insert(source_object_id);
        }
        if let Some(target_object_id) = link_preview.target_object_id {
            impacted.insert(target_object_id);
        }
    }
    impacted
}

fn build_scenario_graph(
    base_graph: &GraphResponse,
    object_states: &BTreeMap<Uuid, ScenarioObjectState>,
    link_previews: &[ScenarioLinkPreview],
    type_labels: &HashMap<Uuid, String>,
) -> GraphResponse {
    let mut graph = base_graph.clone();
    graph.edges.retain(|edge| {
        let source_deleted = edge
            .source
            .strip_prefix("object:")
            .and_then(|value| Uuid::parse_str(value).ok())
            .and_then(|object_id| object_states.get(&object_id))
            .is_some_and(|entry| entry.current.is_none());
        let target_deleted = edge
            .target
            .strip_prefix("object:")
            .and_then(|value| Uuid::parse_str(value).ok())
            .and_then(|object_id| object_states.get(&object_id))
            .is_some_and(|entry| entry.current.is_none());
        !source_deleted && !target_deleted
    });

    let mut node_ids = graph
        .nodes
        .iter()
        .map(|node| node.id.clone())
        .collect::<HashSet<_>>();
    for entry in object_states.values() {
        let node_id = scenario_object_node_id(entry.original.id);
        if let Some(node) = graph.nodes.iter_mut().find(|node| node.id == node_id) {
            if let Some(metadata) = node.metadata.as_object_mut() {
                metadata.insert(
                    "scenario_changed".to_string(),
                    json!(!entry.changed_properties.is_empty()),
                );
                metadata.insert(
                    "scenario_deleted".to_string(),
                    json!(entry.current.is_none()),
                );
                metadata.insert(
                    "scenario_changed_properties".to_string(),
                    json!(entry.changed_properties.iter().cloned().collect::<Vec<_>>()),
                );
                metadata.insert(
                    "scenario_sources".to_string(),
                    json!(entry.sources.iter().cloned().collect::<Vec<_>>()),
                );
                if let Some(current) = &entry.current {
                    metadata.insert("properties".to_string(), current.properties.clone());
                    metadata.insert("marking".to_string(), json!(current.marking.clone()));
                } else {
                    metadata.insert("marking".to_string(), Value::Null);
                }
            }
            if entry.current.is_none() {
                node.kind = "deleted_object_instance".to_string();
            }
        } else if let Some(object) = entry.current.as_ref().or(Some(&entry.original)) {
            graph.nodes.push(build_scenario_graph_node(
                object,
                type_labels
                    .get(&object.object_type_id)
                    .cloned()
                    .unwrap_or_else(|| object.object_type_id.to_string()),
                1,
                entry.current.is_none(),
                &entry.changed_properties,
                &entry.sources,
            ));
            node_ids.insert(node_id);
        }
    }

    let mut edge_ids = graph
        .edges
        .iter()
        .map(|edge| edge.id.clone())
        .collect::<HashSet<_>>();
    for (index, link_preview) in link_previews.iter().enumerate() {
        let (Some(source_object_id), Some(target_object_id)) =
            (link_preview.source_object_id, link_preview.target_object_id)
        else {
            continue;
        };
        let edge = GraphEdge {
            id: format!("scenario_link:{index}"),
            kind: "scenario_link".to_string(),
            source: scenario_object_node_id(source_object_id),
            target: scenario_object_node_id(target_object_id),
            label: "Simulated link".to_string(),
            metadata: json!({
                "simulated": true,
                "link_type_id": link_preview.link_type_id,
                "preview": link_preview.preview,
            }),
        };
        if edge_ids.insert(edge.id.clone()) {
            graph.edges.push(edge);
        }
    }

    graph.nodes.sort_by(|left, right| left.id.cmp(&right.id));
    graph.edges.sort_by(|left, right| left.id.cmp(&right.id));
    graph.summary = graph::summarize_graph("object", &graph.nodes, &graph.edges);
    graph.total_nodes = graph.nodes.len();
    graph.total_edges = graph.edges.len();
    graph
}

fn build_scenario_graph_node(
    object: &ObjectInstance,
    type_label: String,
    distance_from_root: usize,
    deleted: bool,
    changed_properties: &BTreeSet<String>,
    sources: &BTreeSet<String>,
) -> GraphNode {
    GraphNode {
        id: scenario_object_node_id(object.id),
        kind: if deleted {
            "deleted_object_instance".to_string()
        } else {
            "object_instance".to_string()
        },
        label: scenario_object_label(object),
        secondary_label: Some(type_label),
        color: None,
        route: Some(format!(
            "/ontology/{}#object-{}",
            object.object_type_id, object.id
        )),
        metadata: json!({
            "object_type_id": object.object_type_id,
            "distance_from_root": distance_from_root,
            "role": "scenario",
            "organization_id": object.organization_id,
            "marking": if deleted { Value::Null } else { json!(object.marking.clone()) },
            "properties": object.properties,
            "scenario_changed": !changed_properties.is_empty(),
            "scenario_deleted": deleted,
            "scenario_changed_properties": changed_properties.iter().cloned().collect::<Vec<_>>(),
            "scenario_sources": sources.iter().cloned().collect::<Vec<_>>(),
        }),
    }
}

fn scenario_object_node_id(object_id: Uuid) -> String {
    format!("object:{object_id}")
}

fn scenario_object_label(object: &ObjectInstance) -> String {
    object
        .properties
        .get("name")
        .or_else(|| object.properties.get("title"))
        .or_else(|| object.properties.get("display_name"))
        .and_then(|value| match value {
            Value::String(value) if !value.trim().is_empty() => Some(value.clone()),
            other => serde_json::to_string(other).ok(),
        })
        .unwrap_or_else(|| object.id.to_string())
}

fn evaluate_scenario_constraints(
    object_states: &BTreeMap<Uuid, ScenarioObjectState>,
    rule_outcomes: &[ScenarioRuleOutcome],
    graph: &GraphResponse,
    link_previews: &[ScenarioLinkPreview],
    constraints: &[ScenarioMetricSpec],
    impacted_ids: &BTreeSet<Uuid>,
) -> Result<Vec<ScenarioMetricEvaluation>, String> {
    constraints
        .iter()
        .map(|constraint| {
            evaluate_metric_spec(
                object_states,
                rule_outcomes,
                graph,
                link_previews,
                impacted_ids,
                &constraint.name,
                &constraint.metric,
                &constraint.comparator,
                &constraint.target,
                &constraint.config,
                None,
            )
        })
        .collect()
}

fn evaluate_scenario_goals(
    object_states: &BTreeMap<Uuid, ScenarioObjectState>,
    rule_outcomes: &[ScenarioRuleOutcome],
    graph: &GraphResponse,
    link_previews: &[ScenarioLinkPreview],
    goals: &[ScenarioGoalSpec],
    impacted_ids: &BTreeSet<Uuid>,
) -> Result<Vec<ScenarioMetricEvaluation>, String> {
    goals
        .iter()
        .map(|goal| {
            evaluate_metric_spec(
                object_states,
                rule_outcomes,
                graph,
                link_previews,
                impacted_ids,
                &goal.name,
                &goal.metric,
                &goal.comparator,
                &goal.target,
                &goal.config,
                Some(goal.weight),
            )
        })
        .collect()
}

fn evaluate_metric_spec(
    object_states: &BTreeMap<Uuid, ScenarioObjectState>,
    rule_outcomes: &[ScenarioRuleOutcome],
    graph: &GraphResponse,
    link_previews: &[ScenarioLinkPreview],
    impacted_ids: &BTreeSet<Uuid>,
    name: &str,
    metric: &str,
    comparator: &str,
    target: &Value,
    config: &Value,
    goal_weight: Option<f64>,
) -> Result<ScenarioMetricEvaluation, String> {
    let observed = compute_scenario_metric(
        metric,
        config,
        object_states,
        rule_outcomes,
        graph,
        link_previews,
        impacted_ids,
    )?;
    let passed = compare_metric_values(&observed, comparator, target)?;
    let score =
        goal_weight.map(|weight| metric_score(&observed, comparator, target, passed, weight));
    Ok(ScenarioMetricEvaluation {
        name: name.to_string(),
        metric: metric.to_string(),
        comparator: comparator.to_string(),
        target: target.clone(),
        observed: observed.clone(),
        passed,
        score,
        message: format!(
            "Observed {} for {} against {} {}",
            observed, metric, comparator, target
        ),
    })
}

fn compute_scenario_metric(
    metric: &str,
    config: &Value,
    object_states: &BTreeMap<Uuid, ScenarioObjectState>,
    rule_outcomes: &[ScenarioRuleOutcome],
    graph: &GraphResponse,
    _link_previews: &[ScenarioLinkPreview],
    impacted_ids: &BTreeSet<Uuid>,
) -> Result<Value, String> {
    match metric {
        "impacted_object_count" => Ok(json!(impacted_ids.len())),
        "changed_object_count" => Ok(json!(
            object_states
                .values()
                .filter(|entry| !entry.changed_properties.is_empty() || entry.current.is_none())
                .count()
        )),
        "deleted_object_count" => Ok(json!(
            object_states
                .values()
                .filter(|entry| entry.current.is_none())
                .count()
        )),
        "automatic_rule_matches" => Ok(json!(
            rule_outcomes
                .iter()
                .filter(|outcome| outcome.evaluation_mode == "automatic")
                .count()
        )),
        "automatic_rule_applications" => Ok(json!(
            rule_outcomes
                .iter()
                .filter(|outcome| outcome.auto_applied)
                .count()
        )),
        "advisory_rule_matches" => Ok(json!(
            rule_outcomes
                .iter()
                .filter(|outcome| outcome.evaluation_mode == "advisory")
                .count()
        )),
        "schedule_count" => Ok(json!(
            filtered_schedule_outcomes(rule_outcomes, config).len()
        )),
        "boundary_crossings" => Ok(json!(graph.summary.boundary_crossings)),
        "sensitive_objects" => Ok(json!(graph.summary.sensitive_objects)),
        "property_equals_count" => {
            let property_name = config
                .get("property")
                .and_then(Value::as_str)
                .ok_or_else(|| "property_equals_count requires config.property".to_string())?;
            let expected = config
                .get("value")
                .ok_or_else(|| "property_equals_count requires config.value".to_string())?;
            Ok(json!(
                selected_metric_objects(object_states, config)
                    .into_iter()
                    .filter(|object| object.properties.get(property_name) == Some(expected))
                    .count()
            ))
        }
        "property_exists_count" => {
            let property_name = config
                .get("property")
                .and_then(Value::as_str)
                .ok_or_else(|| "property_exists_count requires config.property".to_string())?;
            Ok(json!(
                selected_metric_objects(object_states, config)
                    .into_iter()
                    .filter(|object| object.properties.get(property_name).is_some())
                    .count()
            ))
        }
        "property_numeric_sum" => {
            let property_name = config
                .get("property")
                .and_then(Value::as_str)
                .ok_or_else(|| "property_numeric_sum requires config.property".to_string())?;
            let sum = selected_metric_objects(object_states, config)
                .into_iter()
                .filter_map(|object| object.properties.get(property_name).and_then(Value::as_f64))
                .sum::<f64>();
            Ok(json!(sum))
        }
        "property_numeric_average" => {
            let property_name = config
                .get("property")
                .and_then(Value::as_str)
                .ok_or_else(|| "property_numeric_average requires config.property".to_string())?;
            let values = selected_metric_objects(object_states, config)
                .into_iter()
                .filter_map(|object| object.properties.get(property_name).and_then(Value::as_f64))
                .collect::<Vec<_>>();
            let average = if values.is_empty() {
                0.0
            } else {
                values.iter().sum::<f64>() / values.len() as f64
            };
            Ok(json!(average))
        }
        other => Err(format!("unsupported scenario metric '{other}'")),
    }
}

fn selected_metric_objects<'a>(
    object_states: &'a BTreeMap<Uuid, ScenarioObjectState>,
    config: &Value,
) -> Vec<&'a ObjectInstance> {
    let filtered_ids = config
        .get("object_ids")
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .filter_map(|value| Uuid::parse_str(value).ok())
                .collect::<HashSet<_>>()
        });
    let filtered_type_id = config
        .get("object_type_id")
        .and_then(Value::as_str)
        .and_then(|value| Uuid::parse_str(value).ok());
    let only_changed = config
        .get("only_changed")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let scope = config
        .get("scope")
        .and_then(Value::as_str)
        .unwrap_or("current");

    object_states
        .iter()
        .filter(|(object_id, entry)| {
            filtered_ids
                .as_ref()
                .is_none_or(|ids| ids.contains(object_id))
                && filtered_type_id.is_none_or(|type_id| entry.original.object_type_id == type_id)
                && (!only_changed
                    || !entry.changed_properties.is_empty()
                    || entry.current.is_none())
        })
        .filter_map(|(_, entry)| match scope {
            "original" => Some(&entry.original),
            _ => entry.current.as_ref(),
        })
        .collect()
}

fn filtered_schedule_outcomes<'a>(
    rule_outcomes: &'a [ScenarioRuleOutcome],
    config: &Value,
) -> Vec<&'a ScenarioRuleOutcome> {
    let required_tag = config.get("constraint_tag").and_then(Value::as_str);
    let required_capability = config.get("required_capability").and_then(Value::as_str);

    rule_outcomes
        .iter()
        .filter(|outcome| outcome.auto_applied)
        .filter(|outcome| outcome.effect_preview.get("schedule").is_some())
        .filter(|outcome| {
            let schedule = outcome.effect_preview.get("schedule");
            let tag_matches = required_tag.is_none_or(|required_tag| {
                schedule
                    .and_then(|schedule| schedule.get("constraint_tags"))
                    .and_then(Value::as_array)
                    .is_some_and(|tags| {
                        tags.iter()
                            .filter_map(Value::as_str)
                            .any(|tag| tag == required_tag)
                    })
            });
            let capability_matches = required_capability.is_none_or(|required_capability| {
                schedule
                    .and_then(|schedule| schedule.get("required_capability"))
                    .and_then(Value::as_str)
                    == Some(required_capability)
            });
            tag_matches && capability_matches
        })
        .collect()
}

fn compare_metric_values(
    observed: &Value,
    comparator: &str,
    target: &Value,
) -> Result<bool, String> {
    match comparator {
        "eq" => Ok(observed == target),
        "ne" => Ok(observed != target),
        "gte" | "gt" | "lte" | "lt" => {
            let observed = observed.as_f64().ok_or_else(|| {
                format!("metric comparator '{comparator}' requires numeric observed values")
            })?;
            let target = target.as_f64().ok_or_else(|| {
                format!("metric comparator '{comparator}' requires numeric target values")
            })?;
            Ok(match comparator {
                "gte" => observed >= target,
                "gt" => observed > target,
                "lte" => observed <= target,
                "lt" => observed < target,
                _ => false,
            })
        }
        "contains" => match (observed, target) {
            (Value::Array(values), _) => Ok(values.iter().any(|value| value == target)),
            (Value::String(observed), Value::String(target)) => Ok(observed.contains(target)),
            _ => Err("contains comparator requires arrays or strings".to_string()),
        },
        other => Err(format!("unsupported comparator '{other}'")),
    }
}

fn metric_score(
    observed: &Value,
    comparator: &str,
    target: &Value,
    passed: bool,
    weight: f64,
) -> f64 {
    let weight = weight.max(0.0);
    if weight == 0.0 {
        return 0.0;
    }

    match comparator {
        "gte" | "gt" => {
            let observed = observed.as_f64().unwrap_or(0.0);
            let target = target.as_f64().unwrap_or(0.0);
            if target <= 0.0 {
                return if passed { weight } else { 0.0 };
            }
            (observed / target).clamp(0.0, 1.0) * weight
        }
        "lte" | "lt" => {
            let observed = observed.as_f64().unwrap_or(0.0);
            let target = target.as_f64().unwrap_or(0.0);
            if observed <= 0.0 || target <= 0.0 {
                return if passed { weight } else { 0.0 };
            }
            if passed {
                weight
            } else {
                (target / observed).clamp(0.0, 1.0) * weight
            }
        }
        _ => {
            if passed {
                weight
            } else {
                0.0
            }
        }
    }
}

fn build_scenario_summary(
    object_states: &BTreeMap<Uuid, ScenarioObjectState>,
    rule_outcomes: &[ScenarioRuleOutcome],
    _link_previews: &[ScenarioLinkPreview],
    graph: &GraphResponse,
    object_changes: &[ScenarioObjectChange],
    constraints: &[ScenarioMetricEvaluation],
    goals: &[ScenarioMetricEvaluation],
    impacted_ids: &BTreeSet<Uuid>,
    type_labels: &HashMap<Uuid, String>,
) -> ScenarioSummary {
    let impacted_types = impacted_ids
        .iter()
        .filter_map(|object_id| object_states.get(object_id))
        .map(|entry| {
            type_labels
                .get(&entry.original.object_type_id)
                .cloned()
                .unwrap_or_else(|| entry.original.object_type_id.to_string())
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let changed_properties = object_changes
        .iter()
        .flat_map(|change| change.changed_properties.iter().cloned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();

    ScenarioSummary {
        impacted_object_count: impacted_ids.len(),
        changed_object_count: object_changes
            .iter()
            .filter(|change| !change.changed_properties.is_empty())
            .count(),
        deleted_object_count: object_changes
            .iter()
            .filter(|change| change.deleted)
            .count(),
        automatic_rule_matches: rule_outcomes
            .iter()
            .filter(|outcome| outcome.evaluation_mode == "automatic")
            .count(),
        automatic_rule_applications: rule_outcomes
            .iter()
            .filter(|outcome| outcome.auto_applied)
            .count(),
        advisory_rule_matches: rule_outcomes
            .iter()
            .filter(|outcome| outcome.evaluation_mode == "advisory")
            .count(),
        schedule_count: rule_outcomes
            .iter()
            .filter(|outcome| {
                outcome.auto_applied && outcome.effect_preview.get("schedule").is_some()
            })
            .count(),
        impacted_types,
        changed_properties,
        boundary_crossings: graph.summary.boundary_crossings,
        sensitive_objects: graph.summary.sensitive_objects,
        failed_constraints: constraints
            .iter()
            .filter(|constraint| !constraint.passed)
            .count(),
        achieved_goals: goals.iter().filter(|goal| goal.passed).count(),
        total_goals: goals.len(),
        goal_score: goals.iter().filter_map(|goal| goal.score).sum::<f64>(),
    }
}

fn build_summary_delta(
    summary: &ScenarioSummary,
    baseline: &ScenarioSummary,
) -> ScenarioSummaryDelta {
    ScenarioSummaryDelta {
        impacted_object_count: summary.impacted_object_count as i64
            - baseline.impacted_object_count as i64,
        changed_object_count: summary.changed_object_count as i64
            - baseline.changed_object_count as i64,
        deleted_object_count: summary.deleted_object_count as i64
            - baseline.deleted_object_count as i64,
        automatic_rule_matches: summary.automatic_rule_matches as i64
            - baseline.automatic_rule_matches as i64,
        automatic_rule_applications: summary.automatic_rule_applications as i64
            - baseline.automatic_rule_applications as i64,
        advisory_rule_matches: summary.advisory_rule_matches as i64
            - baseline.advisory_rule_matches as i64,
        schedule_count: summary.schedule_count as i64 - baseline.schedule_count as i64,
        failed_constraints: summary.failed_constraints as i64 - baseline.failed_constraints as i64,
        goal_score: summary.goal_score - baseline.goal_score,
    }
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

    use super::{
        build_simulation_impact_summary, extract_graph_object_ids, extract_vector_from_object_json,
        knn_score,
    };
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

    #[test]
    fn vector_extraction_reads_numeric_property_arrays() {
        let object = json!({
            "id": Uuid::nil(),
            "properties": {
                "embedding": [0.2, 0.4, 0.8]
            }
        });

        let vector = extract_vector_from_object_json(&object, "embedding").expect("vector");

        assert_eq!(vector, vec![0.2, 0.4, 0.8]);
    }

    #[test]
    fn cosine_knn_score_prefers_closer_vectors() {
        let query = [1.0, 0.0, 0.0];
        let close = [0.98, 0.02, 0.0];
        let far = [0.0, 1.0, 0.0];

        let close_score = knn_score("cosine", &query, &close).expect("close score").0;
        let far_score = knn_score("cosine", &query, &far).expect("far score").0;

        assert!(close_score > far_score);
    }

    #[test]
    fn euclidean_knn_score_reports_distance() {
        let query = [1.0, 1.0];
        let candidate = [2.0, 1.0];

        let (score, distance) = knn_score("euclidean", &query, &candidate).expect("score");

        assert!(score > 0.0);
        assert_eq!(distance, Some(1.0));
    }
}
