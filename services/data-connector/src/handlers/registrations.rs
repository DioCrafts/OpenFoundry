use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use uuid::Uuid;

use crate::{
    AppState,
    domain::discovery,
    models::{
        connection::Connection,
        registration::{
            AutoRegisterRequest, BulkRegisterRequest, ConnectionRegistration, DiscoveredSource,
            VirtualTableQueryRequest,
        },
    },
};

pub async fn discover_connection_sources(
    State(state): State<AppState>,
    Path(connection_id): Path<Uuid>,
) -> impl IntoResponse {
    let connection = match load_connection(&state, connection_id).await {
        Ok(connection) => connection,
        Err(response) => return response,
    };

    match discovery::discover_sources(&state, &connection).await {
        Ok(sources) => Json(serde_json::json!({ "data": sources })).into_response(),
        Err(error) => (
            StatusCode::BAD_GATEWAY,
            Json(serde_json::json!({ "error": error })),
        )
            .into_response(),
    }
}

pub async fn list_registrations(
    State(state): State<AppState>,
    Path(connection_id): Path<Uuid>,
) -> impl IntoResponse {
    let result = sqlx::query_as::<_, ConnectionRegistration>(
        "SELECT * FROM connection_registrations WHERE connection_id = $1 ORDER BY created_at DESC",
    )
    .bind(connection_id)
    .fetch_all(&state.db)
    .await;

    match result {
        Ok(registrations) => Json(serde_json::json!({ "data": registrations })).into_response(),
        Err(error) => {
            tracing::error!("list registrations failed: {error}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn auto_register_sources(
    State(state): State<AppState>,
    Path(connection_id): Path<Uuid>,
    Json(body): Json<AutoRegisterRequest>,
) -> impl IntoResponse {
    let connection = match load_connection(&state, connection_id).await {
        Ok(connection) => connection,
        Err(response) => return response,
    };

    let discovered = match discovery::discover_sources(&state, &connection).await {
        Ok(sources) => sources,
        Err(error) => {
            return (
                StatusCode::BAD_GATEWAY,
                Json(serde_json::json!({ "error": error })),
            )
                .into_response();
        }
    };

    let mode = match discovery::normalize_registration_mode(body.registration_mode.as_deref()) {
        Ok(mode) => mode,
        Err(error) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": error })),
            )
                .into_response();
        }
    };

    let selected = discovery::select_sources(&discovered, &body);
    let mut registrations = Vec::new();
    for source in selected {
        match discovery::upsert_registration(
            &state,
            connection_id,
            source,
            mode,
            body.auto_sync.unwrap_or(false),
            body.update_detection.unwrap_or(true),
            body.default_target_dataset_id,
            serde_json::json!({
                "auto_registration": true,
            }),
        )
        .await
        {
            Ok(registration) => registrations.push(registration),
            Err(error) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": error })),
                )
                    .into_response();
            }
        }
    }

    Json(serde_json::json!({
        "registered_count": registrations.len(),
        "data": registrations,
    }))
    .into_response()
}

pub async fn bulk_register_sources(
    State(state): State<AppState>,
    Path(connection_id): Path<Uuid>,
    Json(body): Json<BulkRegisterRequest>,
) -> impl IntoResponse {
    let connection = match load_connection(&state, connection_id).await {
        Ok(connection) => connection,
        Err(response) => return response,
    };

    let discovered = discovery::discover_sources(&state, &connection)
        .await
        .unwrap_or_default();
    let mut by_selector = discovered
        .into_iter()
        .map(|source| (source.selector.clone(), source))
        .collect::<std::collections::BTreeMap<_, _>>();

    let mut registrations = Vec::new();
    for item in body.registrations {
        let source = by_selector
            .remove(&item.selector)
            .unwrap_or_else(|| DiscoveredSource {
                selector: item.selector.clone(),
                display_name: item
                    .display_name
                    .clone()
                    .unwrap_or_else(|| item.selector.clone()),
                source_kind: item
                    .source_kind
                    .clone()
                    .unwrap_or_else(|| connection.connector_type.clone()),
                supports_sync: true,
                supports_zero_copy: true,
                source_signature: None,
                metadata: item.metadata.clone(),
            });
        let mode = match discovery::normalize_registration_mode(item.registration_mode.as_deref()) {
            Ok(mode) => mode,
            Err(error) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({ "error": error })),
                )
                    .into_response();
            }
        };
        match discovery::upsert_registration(
            &state,
            connection_id,
            &source,
            mode,
            item.auto_sync.unwrap_or(false),
            item.update_detection.unwrap_or(true),
            item.target_dataset_id,
            serde_json::json!({
                "bulk_registration": true,
            }),
        )
        .await
        {
            Ok(registration) => registrations.push(registration),
            Err(error) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": error })),
                )
                    .into_response();
            }
        }
    }

    Json(serde_json::json!({
        "registered_count": registrations.len(),
        "data": registrations,
    }))
    .into_response()
}

pub async fn query_virtual_table(
    State(state): State<AppState>,
    Path(connection_id): Path<Uuid>,
    Json(body): Json<VirtualTableQueryRequest>,
) -> impl IntoResponse {
    let connection = match load_connection(&state, connection_id).await {
        Ok(connection) => connection,
        Err(response) => return response,
    };

    match discovery::query_virtual_table(&state, &connection, &body).await {
        Ok(response) => Json(response).into_response(),
        Err(error) => (
            StatusCode::BAD_GATEWAY,
            Json(serde_json::json!({ "error": error })),
        )
            .into_response(),
    }
}

async fn load_connection(
    state: &AppState,
    connection_id: Uuid,
) -> Result<Connection, axum::response::Response> {
    match sqlx::query_as::<_, Connection>("SELECT * FROM connections WHERE id = $1")
        .bind(connection_id)
        .fetch_optional(&state.db)
        .await
    {
        Ok(Some(connection)) => Ok(connection),
        Ok(None) => Err(StatusCode::NOT_FOUND.into_response()),
        Err(error) => {
            tracing::error!("connection lookup failed: {error}");
            Err(StatusCode::INTERNAL_SERVER_ERROR.into_response())
        }
    }
}
