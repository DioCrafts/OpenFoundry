use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::{Map, Value, json};
use uuid::Uuid;

use auth_middleware::layer::AuthUser;

use crate::{
    AppState,
    domain::{
        access::ensure_object_access,
        rules::{
            apply_rule_effect, build_rule_evaluate_response, enqueue_rule_schedule,
            evaluate_rule_against_object, load_recent_rule_runs, load_rule,
            load_rules_for_object_type, machinery_insights, machinery_queue, record_rule_run,
            transition_machinery_queue_item, validate_rule_definition,
        },
    },
    handlers::objects::load_object_instance,
    models::rule::{
        CreateRuleRequest, ListRulesQuery, ListRulesResponse, MachineryInsightsResponse,
        MachineryQueueResponse, OntologyRule, OntologyRuleRow, RuleEvaluateRequest,
        RuleEvaluateResponse, RuleEvaluationMode, UpdateMachineryQueueItemRequest,
        UpdateRuleRequest,
    },
};

fn invalid(message: impl Into<String>) -> Response {
    (
        StatusCode::BAD_REQUEST,
        Json(json!({ "error": message.into() })),
    )
        .into_response()
}

fn db_error(message: impl Into<String>) -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({ "error": message.into() })),
    )
        .into_response()
}

fn parse_properties_patch(value: &Value) -> Result<Option<Map<String, Value>>, String> {
    if value.is_null() {
        return Ok(None);
    }
    let Some(value) = value.as_object() else {
        return Err("properties_patch must be a JSON object".to_string());
    };
    Ok(Some(value.clone()))
}

async fn load_rule_row(state: &AppState, id: Uuid) -> Result<Option<OntologyRule>, String> {
    load_rule(state, id).await
}

pub async fn list_rules(
    State(state): State<AppState>,
    Query(query): Query<ListRulesQuery>,
) -> impl IntoResponse {
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(20).clamp(1, 100);
    let search = query.search.unwrap_or_default();

    let rows = if let Some(object_type_id) = query.object_type_id {
        sqlx::query_as::<_, OntologyRuleRow>(
            r#"SELECT id, name, display_name, description, object_type_id, evaluation_mode,
                      trigger_spec, effect_spec, owner_id, created_at, updated_at
               FROM ontology_rules
               WHERE object_type_id = $1
                 AND ($2 = '' OR name ILIKE '%' || $2 || '%' OR display_name ILIKE '%' || $2 || '%')
               ORDER BY created_at DESC"#,
        )
        .bind(object_type_id)
        .bind(search)
        .fetch_all(&state.db)
        .await
    } else {
        sqlx::query_as::<_, OntologyRuleRow>(
            r#"SELECT id, name, display_name, description, object_type_id, evaluation_mode,
                      trigger_spec, effect_spec, owner_id, created_at, updated_at
               FROM ontology_rules
               WHERE ($1 = '' OR name ILIKE '%' || $1 || '%' OR display_name ILIKE '%' || $1 || '%')
               ORDER BY created_at DESC"#,
        )
        .bind(search)
        .fetch_all(&state.db)
        .await
    };

    let rows = match rows {
        Ok(rows) => rows,
        Err(error) => return db_error(format!("failed to list rules: {error}")),
    };

    let rules = match rows
        .into_iter()
        .map(OntologyRule::try_from)
        .collect::<Result<Vec<_>, _>>()
    {
        Ok(rules) => rules,
        Err(error) => return db_error(format!("failed to decode rules: {error}")),
    };

    let total = rules.len() as i64;
    let offset = ((page - 1) * per_page) as usize;
    let data = rules
        .into_iter()
        .skip(offset)
        .take(per_page as usize)
        .collect::<Vec<_>>();

    Json(ListRulesResponse {
        data,
        total,
        page,
        per_page,
    })
    .into_response()
}

pub async fn create_rule(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Json(body): Json<CreateRuleRequest>,
) -> impl IntoResponse {
    if body.name.trim().is_empty() {
        return invalid("rule name is required");
    }

    let display_name = body.display_name.unwrap_or_else(|| body.name.clone());
    let description = body.description.unwrap_or_default();
    let evaluation_mode = body.evaluation_mode.unwrap_or(RuleEvaluationMode::Advisory);
    let trigger_spec = body.trigger_spec.unwrap_or_default();
    let effect_spec = body.effect_spec.unwrap_or_default();

    if let Err(error) =
        validate_rule_definition(&state, body.object_type_id, &trigger_spec, &effect_spec).await
    {
        return invalid(error);
    }

    let row = match sqlx::query_as::<_, OntologyRuleRow>(
        r#"INSERT INTO ontology_rules (
               id, name, display_name, description, object_type_id, evaluation_mode,
               trigger_spec, effect_spec, owner_id
           )
           VALUES ($1, $2, $3, $4, $5, $6, $7::jsonb, $8::jsonb, $9)
           RETURNING id, name, display_name, description, object_type_id, evaluation_mode,
                     trigger_spec, effect_spec, owner_id, created_at, updated_at"#,
    )
    .bind(Uuid::now_v7())
    .bind(body.name.trim())
    .bind(display_name)
    .bind(description)
    .bind(body.object_type_id)
    .bind(evaluation_mode.to_string())
    .bind(json!(trigger_spec))
    .bind(json!(effect_spec))
    .bind(claims.sub)
    .fetch_one(&state.db)
    .await
    {
        Ok(row) => row,
        Err(error) => return db_error(format!("failed to create rule: {error}")),
    };

    match OntologyRule::try_from(row) {
        Ok(rule) => (StatusCode::CREATED, Json(rule)).into_response(),
        Err(error) => db_error(format!("failed to decode rule: {error}")),
    }
}

pub async fn get_rule(State(state): State<AppState>, Path(id): Path<Uuid>) -> impl IntoResponse {
    match load_rule_row(&state, id).await {
        Ok(Some(rule)) => Json(rule).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(error) => db_error(error),
    }
}

pub async fn update_rule(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateRuleRequest>,
) -> impl IntoResponse {
    let Some(existing) = (match load_rule_row(&state, id).await {
        Ok(rule) => rule,
        Err(error) => return db_error(error),
    }) else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let evaluation_mode = body
        .evaluation_mode
        .unwrap_or(existing.evaluation_mode.clone());
    let trigger_spec = body.trigger_spec.unwrap_or(existing.trigger_spec.clone());
    let effect_spec = body.effect_spec.unwrap_or(existing.effect_spec.clone());

    if let Err(error) =
        validate_rule_definition(&state, existing.object_type_id, &trigger_spec, &effect_spec).await
    {
        return invalid(error);
    }

    let row = match sqlx::query_as::<_, OntologyRuleRow>(
        r#"UPDATE ontology_rules
           SET display_name = COALESCE($2, display_name),
               description = COALESCE($3, description),
               evaluation_mode = $4,
               trigger_spec = $5::jsonb,
               effect_spec = $6::jsonb,
               updated_at = NOW()
           WHERE id = $1
           RETURNING id, name, display_name, description, object_type_id, evaluation_mode,
                     trigger_spec, effect_spec, owner_id, created_at, updated_at"#,
    )
    .bind(id)
    .bind(body.display_name)
    .bind(body.description)
    .bind(evaluation_mode.to_string())
    .bind(json!(trigger_spec))
    .bind(json!(effect_spec))
    .fetch_optional(&state.db)
    .await
    {
        Ok(Some(row)) => row,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(error) => return db_error(format!("failed to update rule: {error}")),
    };

    match OntologyRule::try_from(row) {
        Ok(rule) => Json(rule).into_response(),
        Err(error) => db_error(format!("failed to decode rule: {error}")),
    }
}

pub async fn delete_rule(State(state): State<AppState>, Path(id): Path<Uuid>) -> impl IntoResponse {
    match sqlx::query("DELETE FROM ontology_rules WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await
    {
        Ok(result) if result.rows_affected() > 0 => StatusCode::NO_CONTENT.into_response(),
        Ok(_) => StatusCode::NOT_FOUND.into_response(),
        Err(error) => db_error(format!("failed to delete rule: {error}")),
    }
}

pub async fn simulate_rule(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<RuleEvaluateRequest>,
) -> impl IntoResponse {
    let Some(rule) = (match load_rule_row(&state, id).await {
        Ok(rule) => rule,
        Err(error) => return db_error(error),
    }) else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let object = match load_object_instance(&state.db, body.object_id).await {
        Ok(Some(object)) => object,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(error) => return db_error(format!("failed to load object: {error}")),
    };

    if object.object_type_id != rule.object_type_id {
        return invalid("rule object_type_id does not match the target object");
    }
    if let Err(error) = ensure_object_access(&claims, &object) {
        return (StatusCode::FORBIDDEN, Json(json!({ "error": error }))).into_response();
    }

    let properties_patch = match parse_properties_patch(&body.properties_patch) {
        Ok(properties_patch) => properties_patch,
        Err(error) => return invalid(error),
    };
    let match_result = evaluate_rule_against_object(&rule, &object, properties_patch.as_ref());

    if let Err(error) = record_rule_run(
        &state,
        rule.id,
        object.id,
        match_result.matched,
        true,
        &match_result.trigger_payload,
        Some(&match_result.effect_preview),
        claims.sub,
    )
    .await
    {
        tracing::warn!(%error, "failed to record ontology rule simulation");
    }

    Json(build_rule_evaluate_response(rule, &object, match_result)).into_response()
}

pub async fn apply_rule(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<RuleEvaluateRequest>,
) -> impl IntoResponse {
    let Some(rule) = (match load_rule_row(&state, id).await {
        Ok(rule) => rule,
        Err(error) => return db_error(error),
    }) else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let object = match load_object_instance(&state.db, body.object_id).await {
        Ok(Some(object)) => object,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(error) => return db_error(format!("failed to load object: {error}")),
    };

    if object.object_type_id != rule.object_type_id {
        return invalid("rule object_type_id does not match the target object");
    }
    if let Err(error) = ensure_object_access(&claims, &object) {
        return (StatusCode::FORBIDDEN, Json(json!({ "error": error }))).into_response();
    }

    let properties_patch = match parse_properties_patch(&body.properties_patch) {
        Ok(properties_patch) => properties_patch,
        Err(error) => return invalid(error),
    };
    let match_result = evaluate_rule_against_object(&rule, &object, properties_patch.as_ref());

    let updated_object = if match_result.matched {
        match apply_rule_effect(&state, &object, &match_result.effect_preview).await {
            Ok(updated) => updated,
            Err(error) => return db_error(error),
        }
    } else {
        object.clone()
    };

    let recorded_run = match record_rule_run(
        &state,
        rule.id,
        object.id,
        match_result.matched,
        false,
        &match_result.trigger_payload,
        Some(&match_result.effect_preview),
        claims.sub,
    )
    .await
    {
        Ok(run) => Some(run),
        Err(error) => {
            tracing::warn!(%error, "failed to record ontology rule execution");
            None
        }
    };

    if let (Some(recorded_run), true) = (recorded_run.as_ref(), match_result.matched) {
        if let Err(error) = enqueue_rule_schedule(
            &state,
            &rule,
            &updated_object,
            recorded_run.id,
            &match_result.effect_preview,
            claims.sub,
        )
        .await
        {
            tracing::warn!(%error, "failed to enqueue machinery schedule");
        }
    }

    let response = RuleEvaluateResponse {
        object: json!(updated_object),
        ..build_rule_evaluate_response(rule, &object, match_result)
    };

    Json(response).into_response()
}

pub async fn get_machinery_insights(
    State(state): State<AppState>,
    Query(query): Query<ListRulesQuery>,
) -> impl IntoResponse {
    match machinery_insights(&state, query.object_type_id).await {
        Ok(data) => Json(MachineryInsightsResponse {
            object_type_id: query.object_type_id,
            data,
        })
        .into_response(),
        Err(error) => db_error(error),
    }
}

pub async fn get_machinery_queue(
    State(state): State<AppState>,
    Query(query): Query<ListRulesQuery>,
) -> impl IntoResponse {
    match machinery_queue(&state, query.object_type_id).await {
        Ok(data) => Json::<MachineryQueueResponse>(data).into_response(),
        Err(error) => db_error(error),
    }
}

pub async fn update_machinery_queue_item(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateMachineryQueueItemRequest>,
) -> impl IntoResponse {
    match transition_machinery_queue_item(&state, id, body.status.as_str()).await {
        Ok(Some(item)) => Json(json!(item)).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(error) if error == "unsupported machinery queue status" => invalid(error),
        Err(error) => db_error(error),
    }
}

pub async fn list_object_rule_runs(
    State(state): State<AppState>,
    Path(object_id): Path<Uuid>,
) -> impl IntoResponse {
    match load_recent_rule_runs(&state, object_id, 20).await {
        Ok(runs) => Json(json!({ "data": runs })).into_response(),
        Err(error) => db_error(error),
    }
}

pub async fn list_rules_for_object_type(
    State(state): State<AppState>,
    Path(object_type_id): Path<Uuid>,
) -> impl IntoResponse {
    match load_rules_for_object_type(&state, object_type_id).await {
        Ok(rules) => Json(json!({ "data": rules })).into_response(),
        Err(error) => db_error(error),
    }
}
