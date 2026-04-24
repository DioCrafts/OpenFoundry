use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use uuid::Uuid;

use crate::AppState;
use crate::models::session::*;
use auth_middleware::layer::AuthUser;

pub async fn create_session(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path(notebook_id): Path<Uuid>,
    Json(body): Json<CreateSessionRequest>,
) -> impl IntoResponse {
    let id = Uuid::now_v7();
    let kernel = body.kernel.unwrap_or_else(|| "python".to_string());

    if let Err(error) = state.kernel_manager.ensure_session(id, &kernel).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, error).into_response();
    }

    let result = sqlx::query_as::<_, Session>(
        r#"INSERT INTO sessions (id, notebook_id, kernel, status, started_by)
           VALUES ($1, $2, $3, 'idle', $4)
           RETURNING *"#,
    )
    .bind(id)
    .bind(notebook_id)
    .bind(&kernel)
    .bind(claims.sub)
    .fetch_one(&state.db)
    .await;

    match result {
        Ok(s) => (StatusCode::CREATED, Json(serde_json::json!(s))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub async fn list_sessions(
    _user: AuthUser,
    State(state): State<AppState>,
    Path(notebook_id): Path<Uuid>,
) -> impl IntoResponse {
    let sessions = sqlx::query_as::<_, Session>(
        "SELECT * FROM sessions WHERE notebook_id = $1 ORDER BY created_at DESC",
    )
    .bind(notebook_id)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    Json(serde_json::json!({ "data": sessions }))
}

pub async fn stop_session(
    _user: AuthUser,
    State(state): State<AppState>,
    Path((_notebook_id, session_id)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
    let result = sqlx::query_as::<_, Session>(
        "UPDATE sessions SET status = 'dead', last_activity = NOW() WHERE id = $1 RETURNING *",
    )
    .bind(session_id)
    .fetch_optional(&state.db)
    .await;

    match result {
        Ok(Some(s)) => {
            state.kernel_manager.drop_session(session_id).await;
            Json(serde_json::json!(s)).into_response()
        }
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}
