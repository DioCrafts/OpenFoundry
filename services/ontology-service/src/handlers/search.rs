use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;
use uuid::Uuid;

use auth_middleware::layer::AuthUser;

use crate::{
    AppState,
    domain::{graph, search, time_series},
    models::{
        graph::{GraphQuery, GraphResponse},
        quiver::{
            CreateQuiverVisualFunctionRequest, ListQuiverVisualFunctionsQuery,
            ListQuiverVisualFunctionsResponse, QuiverVisualFunction, QuiverVisualFunctionDraft,
            UpdateQuiverVisualFunctionRequest,
        },
        search::{SearchRequest, SearchResponse},
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

fn forbidden(message: impl Into<String>) -> Response {
    (
        StatusCode::FORBIDDEN,
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

fn validate_visual_function_draft(draft: &QuiverVisualFunctionDraft) -> Result<(), String> {
    if draft.name.trim().is_empty() {
        return Err("name is required".to_string());
    }
    if draft.join_field.trim().is_empty() {
        return Err("join_field is required".to_string());
    }
    if draft.date_field.trim().is_empty() {
        return Err("date_field is required".to_string());
    }
    if draft.metric_field.trim().is_empty() {
        return Err("metric_field is required".to_string());
    }
    if draft.group_field.trim().is_empty() {
        return Err("group_field is required".to_string());
    }
    time_series::normalize_chart_kind(&draft.chart_kind)?;
    Ok(())
}

#[cfg(test)]
fn draft_from_record(record: &QuiverVisualFunction) -> QuiverVisualFunctionDraft {
    QuiverVisualFunctionDraft {
        name: record.name.clone(),
        description: record.description.clone(),
        primary_type_id: record.primary_type_id,
        secondary_type_id: record.secondary_type_id,
        join_field: record.join_field.clone(),
        secondary_join_field: record.secondary_join_field.clone(),
        date_field: record.date_field.clone(),
        metric_field: record.metric_field.clone(),
        group_field: record.group_field.clone(),
        selected_group: record.selected_group.clone(),
        chart_kind: record.chart_kind.clone(),
        shared: record.shared,
    }
}

fn apply_quiver_update(
    current: &QuiverVisualFunction,
    update: UpdateQuiverVisualFunctionRequest,
) -> QuiverVisualFunctionDraft {
    QuiverVisualFunctionDraft {
        name: update.name.unwrap_or_else(|| current.name.clone()),
        description: update
            .description
            .unwrap_or_else(|| current.description.clone()),
        primary_type_id: update.primary_type_id.unwrap_or(current.primary_type_id),
        secondary_type_id: update.secondary_type_id.or(current.secondary_type_id),
        join_field: update
            .join_field
            .unwrap_or_else(|| current.join_field.clone()),
        secondary_join_field: update
            .secondary_join_field
            .unwrap_or_else(|| current.secondary_join_field.clone()),
        date_field: update
            .date_field
            .unwrap_or_else(|| current.date_field.clone()),
        metric_field: update
            .metric_field
            .unwrap_or_else(|| current.metric_field.clone()),
        group_field: update
            .group_field
            .unwrap_or_else(|| current.group_field.clone()),
        selected_group: update
            .selected_group
            .unwrap_or_else(|| current.selected_group.clone()),
        chart_kind: update
            .chart_kind
            .unwrap_or_else(|| current.chart_kind.clone()),
        shared: update.shared.unwrap_or(current.shared),
    }
}

async fn load_quiver_visual_function(
    state: &AppState,
    id: Uuid,
) -> Result<Option<QuiverVisualFunction>, String> {
    sqlx::query_as::<_, QuiverVisualFunction>(
        r#"SELECT id, name, description, primary_type_id, secondary_type_id, join_field,
                  secondary_join_field, date_field, metric_field, group_field, selected_group,
                  chart_kind, shared, vega_spec, owner_id, created_at, updated_at
           FROM ontology_quiver_visual_functions
           WHERE id = $1"#,
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await
    .map_err(|error| format!("failed to load quiver visual function: {error}"))
}

pub async fn search_ontology(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Json(body): Json<SearchRequest>,
) -> impl IntoResponse {
    if body.query.trim().is_empty() {
        return bad_request("query is required");
    }

    match search::search_ontology(&state, &claims, &body).await {
        Ok(results) => Json(SearchResponse {
            query: body.query,
            total: results.len(),
            data: results,
        })
        .into_response(),
        Err(error) => {
            tracing::error!("ontology search failed: {error}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn get_graph(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Query(query): Query<GraphQuery>,
) -> impl IntoResponse {
    match graph::build_graph(&state, &claims, &query).await {
        Ok(graph) => Json::<GraphResponse>(graph).into_response(),
        Err(error) if error.contains("forbidden") => {
            (StatusCode::FORBIDDEN, Json(json!({ "error": error }))).into_response()
        }
        Err(error) if error.contains("not found") => {
            (StatusCode::NOT_FOUND, Json(json!({ "error": error }))).into_response()
        }
        Err(error) => {
            tracing::error!("ontology graph failed: {error}");
            (StatusCode::BAD_REQUEST, Json(json!({ "error": error }))).into_response()
        }
    }
}

pub async fn list_quiver_visual_functions(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Query(query): Query<ListQuiverVisualFunctionsQuery>,
) -> impl IntoResponse {
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * per_page;
    let search = query.search.unwrap_or_default();
    let search_pattern = format!("%{search}%");
    let include_shared = query.include_shared.unwrap_or(true);

    let total = match sqlx::query_scalar::<_, i64>(
        r#"SELECT COUNT(*)
           FROM ontology_quiver_visual_functions
           WHERE (owner_id = $1 OR ($2 AND shared = TRUE))
             AND ($3 = '%%' OR name ILIKE $3 OR description ILIKE $3)"#,
    )
    .bind(claims.sub)
    .bind(include_shared)
    .bind(&search_pattern)
    .fetch_one(&state.db)
    .await
    {
        Ok(total) => total,
        Err(error) => {
            tracing::error!("list quiver visual functions total failed: {error}");
            return internal_error("failed to list quiver visual functions");
        }
    };

    let records = match sqlx::query_as::<_, QuiverVisualFunction>(
        r#"SELECT id, name, description, primary_type_id, secondary_type_id, join_field,
                  secondary_join_field, date_field, metric_field, group_field, selected_group,
                  chart_kind, shared, vega_spec, owner_id, created_at, updated_at
           FROM ontology_quiver_visual_functions
           WHERE (owner_id = $1 OR ($2 AND shared = TRUE))
             AND ($3 = '%%' OR name ILIKE $3 OR description ILIKE $3)
           ORDER BY updated_at DESC
           LIMIT $4 OFFSET $5"#,
    )
    .bind(claims.sub)
    .bind(include_shared)
    .bind(&search_pattern)
    .bind(per_page)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    {
        Ok(records) => records,
        Err(error) => {
            tracing::error!("list quiver visual functions failed: {error}");
            return internal_error("failed to list quiver visual functions");
        }
    };

    Json(ListQuiverVisualFunctionsResponse {
        data: records,
        total,
        page,
        per_page,
    })
    .into_response()
}

pub async fn create_quiver_visual_function(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Json(body): Json<CreateQuiverVisualFunctionRequest>,
) -> impl IntoResponse {
    let draft = body.into_draft();
    if let Err(error) = validate_visual_function_draft(&draft) {
        return bad_request(error);
    }

    let vega_spec = match time_series::build_quiver_vega_spec(&draft) {
        Ok(spec) => spec,
        Err(error) => return bad_request(error),
    };

    let id = Uuid::now_v7();
    let created = match sqlx::query_as::<_, QuiverVisualFunction>(
        r#"INSERT INTO ontology_quiver_visual_functions (
               id, name, description, primary_type_id, secondary_type_id, join_field,
               secondary_join_field, date_field, metric_field, group_field, selected_group,
               chart_kind, shared, vega_spec, owner_id
           )
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14::jsonb, $15)
           RETURNING id, name, description, primary_type_id, secondary_type_id, join_field,
                     secondary_join_field, date_field, metric_field, group_field, selected_group,
                     chart_kind, shared, vega_spec, owner_id, created_at, updated_at"#,
    )
    .bind(id)
    .bind(&draft.name)
    .bind(&draft.description)
    .bind(draft.primary_type_id)
    .bind(draft.secondary_type_id)
    .bind(&draft.join_field)
    .bind(&draft.secondary_join_field)
    .bind(&draft.date_field)
    .bind(&draft.metric_field)
    .bind(&draft.group_field)
    .bind(&draft.selected_group)
    .bind(&draft.chart_kind)
    .bind(draft.shared)
    .bind(vega_spec)
    .bind(claims.sub)
    .fetch_one(&state.db)
    .await
    {
        Ok(created) => created,
        Err(error) => {
            tracing::error!("create quiver visual function failed: {error}");
            return internal_error("failed to create quiver visual function");
        }
    };

    (StatusCode::CREATED, Json(created)).into_response()
}

pub async fn get_quiver_visual_function(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let Some(record) = (match load_quiver_visual_function(&state, id).await {
        Ok(record) => record,
        Err(error) => {
            tracing::error!("get quiver visual function failed: {error}");
            return internal_error("failed to load quiver visual function");
        }
    }) else {
        return not_found("quiver visual function not found");
    };

    if record.owner_id != claims.sub && !record.shared {
        return forbidden("you do not have access to this quiver visual function");
    }

    Json(record).into_response()
}

pub async fn update_quiver_visual_function(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateQuiverVisualFunctionRequest>,
) -> impl IntoResponse {
    let Some(current) = (match load_quiver_visual_function(&state, id).await {
        Ok(record) => record,
        Err(error) => {
            tracing::error!("update quiver visual function load failed: {error}");
            return internal_error("failed to load quiver visual function");
        }
    }) else {
        return not_found("quiver visual function not found");
    };

    if current.owner_id != claims.sub {
        return forbidden("only the owner can update this quiver visual function");
    }

    let draft = apply_quiver_update(&current, body);
    if let Err(error) = validate_visual_function_draft(&draft) {
        return bad_request(error);
    }

    let vega_spec = match time_series::build_quiver_vega_spec(&draft) {
        Ok(spec) => spec,
        Err(error) => return bad_request(error),
    };

    let updated = match sqlx::query_as::<_, QuiverVisualFunction>(
        r#"UPDATE ontology_quiver_visual_functions
           SET name = $2,
               description = $3,
               primary_type_id = $4,
               secondary_type_id = $5,
               join_field = $6,
               secondary_join_field = $7,
               date_field = $8,
               metric_field = $9,
               group_field = $10,
               selected_group = $11,
               chart_kind = $12,
               shared = $13,
               vega_spec = $14::jsonb,
               updated_at = NOW()
           WHERE id = $1
           RETURNING id, name, description, primary_type_id, secondary_type_id, join_field,
                     secondary_join_field, date_field, metric_field, group_field, selected_group,
                     chart_kind, shared, vega_spec, owner_id, created_at, updated_at"#,
    )
    .bind(id)
    .bind(&draft.name)
    .bind(&draft.description)
    .bind(draft.primary_type_id)
    .bind(draft.secondary_type_id)
    .bind(&draft.join_field)
    .bind(&draft.secondary_join_field)
    .bind(&draft.date_field)
    .bind(&draft.metric_field)
    .bind(&draft.group_field)
    .bind(&draft.selected_group)
    .bind(&draft.chart_kind)
    .bind(draft.shared)
    .bind(vega_spec)
    .fetch_one(&state.db)
    .await
    {
        Ok(updated) => updated,
        Err(error) => {
            tracing::error!("update quiver visual function failed: {error}");
            return internal_error("failed to update quiver visual function");
        }
    };

    Json(updated).into_response()
}

pub async fn delete_quiver_visual_function(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let Some(record) = (match load_quiver_visual_function(&state, id).await {
        Ok(record) => record,
        Err(error) => {
            tracing::error!("delete quiver visual function load failed: {error}");
            return internal_error("failed to load quiver visual function");
        }
    }) else {
        return not_found("quiver visual function not found");
    };

    if record.owner_id != claims.sub {
        return forbidden("only the owner can delete this quiver visual function");
    }

    match sqlx::query("DELETE FROM ontology_quiver_visual_functions WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await
    {
        Ok(result) if result.rows_affected() > 0 => StatusCode::NO_CONTENT.into_response(),
        Ok(_) => not_found("quiver visual function not found"),
        Err(error) => {
            tracing::error!("delete quiver visual function failed: {error}");
            internal_error("failed to delete quiver visual function")
        }
    }
}

pub async fn get_quiver_vega_spec(
    State(_state): State<AppState>,
    Json(body): Json<CreateQuiverVisualFunctionRequest>,
) -> impl IntoResponse {
    let draft = body.into_draft();
    if let Err(error) = validate_visual_function_draft(&draft) {
        return bad_request(error);
    }

    match time_series::build_quiver_vega_spec(&draft) {
        Ok(spec) => Json(json!({ "spec": spec })).into_response(),
        Err(error) => bad_request(error),
    }
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::*;

    fn sample_record() -> QuiverVisualFunction {
        QuiverVisualFunction {
            id: Uuid::nil(),
            name: "Daily Orders".to_string(),
            description: "Orders by region".to_string(),
            primary_type_id: Uuid::nil(),
            secondary_type_id: Some(Uuid::now_v7()),
            join_field: "order_id".to_string(),
            secondary_join_field: "order_id".to_string(),
            date_field: "event_date".to_string(),
            metric_field: "gmv".to_string(),
            group_field: "region".to_string(),
            selected_group: Some("EMEA".to_string()),
            chart_kind: "line".to_string(),
            shared: false,
            vega_spec: json!({}),
            owner_id: Uuid::nil(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn apply_update_overrides_selected_fields() {
        let record = sample_record();
        let draft = apply_quiver_update(
            &record,
            UpdateQuiverVisualFunctionRequest {
                name: Some("Executive Orders".to_string()),
                description: None,
                primary_type_id: None,
                secondary_type_id: None,
                join_field: None,
                secondary_join_field: None,
                date_field: None,
                metric_field: Some("net_revenue".to_string()),
                group_field: None,
                selected_group: Some(None),
                chart_kind: Some("area".to_string()),
                shared: Some(true),
            },
        );

        assert_eq!(draft.name, "Executive Orders");
        assert_eq!(draft.metric_field, "net_revenue");
        assert_eq!(draft.chart_kind, "area");
        assert_eq!(draft.selected_group, None);
        assert!(draft.shared);
        assert_eq!(draft.group_field, "region");
    }

    #[test]
    fn draft_from_record_preserves_core_fields() {
        let record = sample_record();
        let draft = draft_from_record(&record);

        assert_eq!(draft.name, record.name);
        assert_eq!(draft.secondary_type_id, record.secondary_type_id);
        assert_eq!(draft.selected_group, record.selected_group);
    }
}
