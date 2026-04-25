use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use uuid::Uuid;

use crate::{
    AppState,
    models::{
        connection::Connection,
        connector_profile::{
            ConnectionCapabilityResponse, ConnectorContractCatalog, certification_summary,
            connector_profile, connector_profiles,
        },
    },
};

pub async fn get_connector_catalog() -> impl IntoResponse {
    Json(ConnectorContractCatalog {
        connectors: connector_profiles(),
        certification_summary: certification_summary(),
    })
    .into_response()
}

pub async fn get_connection_capabilities(
    State(state): State<AppState>,
    Path(connection_id): Path<Uuid>,
) -> impl IntoResponse {
    let connection =
        match sqlx::query_as::<_, Connection>("SELECT * FROM connections WHERE id = $1")
            .bind(connection_id)
            .fetch_optional(&state.db)
            .await
        {
            Ok(Some(connection)) => connection,
            Ok(None) => return StatusCode::NOT_FOUND.into_response(),
            Err(error) => {
                tracing::error!("connection capability lookup failed: {error}");
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
        };

    let Some(contract) = connector_profile(&connection.connector_type) else {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": format!("no connector contract available for {}", connection.connector_type)
            })),
        )
            .into_response();
    };

    Json(ConnectionCapabilityResponse {
        connection_id: connection.id,
        connector_type: connection.connector_type,
        status: connection.status,
        contract,
    })
    .into_response()
}
