use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use uuid::Uuid;

use crate::{
    AppState,
    models::agent::{AgentHeartbeatRequest, ConnectorAgent, RegisterAgentRequest},
};

pub async fn register_agent(
    State(state): State<AppState>,
    auth_middleware::layer::AuthUser(claims): auth_middleware::layer::AuthUser,
    Json(body): Json<RegisterAgentRequest>,
) -> impl IntoResponse {
    let id = Uuid::now_v7();
    let result = sqlx::query_as::<_, ConnectorAgent>(
        r#"INSERT INTO connector_agents (
               id, name, agent_url, owner_id, status, capabilities, metadata, last_heartbeat_at
           )
           VALUES ($1, $2, $3, $4, 'online', $5::jsonb, $6::jsonb, NOW())
           ON CONFLICT (agent_url)
           DO UPDATE SET
               name = EXCLUDED.name,
               owner_id = EXCLUDED.owner_id,
               status = 'online',
               capabilities = EXCLUDED.capabilities,
               metadata = connector_agents.metadata || EXCLUDED.metadata,
               last_heartbeat_at = NOW(),
               updated_at = NOW()
           RETURNING *"#,
    )
    .bind(id)
    .bind(body.name)
    .bind(body.agent_url)
    .bind(claims.sub)
    .bind(body.capabilities)
    .bind(body.metadata)
    .fetch_one(&state.db)
    .await;

    match result {
        Ok(agent) => (StatusCode::CREATED, Json(agent)).into_response(),
        Err(error) => {
            tracing::error!("register connector agent failed: {error}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "register agent failed" })),
            )
                .into_response()
        }
    }
}

pub async fn list_agents(State(state): State<AppState>) -> impl IntoResponse {
    let result = sqlx::query_as::<_, ConnectorAgent>(
        "SELECT * FROM connector_agents ORDER BY updated_at DESC LIMIT 100",
    )
    .fetch_all(&state.db)
    .await;

    match result {
        Ok(agents) => Json(agents).into_response(),
        Err(error) => {
            tracing::error!("list connector agents failed: {error}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn heartbeat_agent(
    State(state): State<AppState>,
    Path(agent_id): Path<Uuid>,
    Json(body): Json<AgentHeartbeatRequest>,
) -> impl IntoResponse {
    let result = sqlx::query_as::<_, ConnectorAgent>(
        r#"UPDATE connector_agents
           SET status = 'online',
               capabilities = CASE
                   WHEN $2::jsonb = '{}'::jsonb THEN capabilities
                   ELSE $2::jsonb
               END,
               metadata = metadata || $3::jsonb,
               last_heartbeat_at = NOW(),
               updated_at = NOW()
           WHERE id = $1
           RETURNING *"#,
    )
    .bind(agent_id)
    .bind(body.capabilities)
    .bind(body.metadata)
    .fetch_optional(&state.db)
    .await;

    match result {
        Ok(Some(agent)) => Json(agent).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(error) => {
            tracing::error!("connector agent heartbeat failed: {error}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
