use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use uuid::Uuid;

use crate::AppState;
use crate::models::connection::Connection;
use crate::models::sync_job::SyncJob;

/// POST /api/v1/connections/:id/sync
pub async fn sync_connection(
    State(state): State<AppState>,
    Path(connection_id): Path<Uuid>,
    Json(body): Json<crate::models::sync_job::SyncRequest>,
) -> impl IntoResponse {
    // Verify connection exists
    let conn = match sqlx::query_as::<_, Connection>("SELECT * FROM connections WHERE id = $1")
        .bind(connection_id)
        .fetch_optional(&state.db)
        .await
    {
        Ok(Some(c)) => c,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("sync lookup failed: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let job_id = Uuid::now_v7();
    let _ = sqlx::query(
        r#"INSERT INTO sync_jobs (id, connection_id, target_dataset_id, table_name, status, started_at)
           VALUES ($1, $2, $3, $4, 'running', NOW())"#,
    )
    .bind(job_id)
    .bind(connection_id)
    .bind(body.target_dataset_id)
    .bind(&body.table_name)
    .execute(&state.db)
    .await;

    tracing::info!(
        connection_id = %connection_id,
        job_id = %job_id,
        connector_type = %conn.connector_type,
        "sync job started"
    );

    // For now, mark as completed immediately — real async sync is next iteration
    let _ = sqlx::query(
        "UPDATE sync_jobs SET status = 'completed', completed_at = NOW() WHERE id = $1",
    )
    .bind(job_id)
    .execute(&state.db)
    .await;

    (StatusCode::ACCEPTED, Json(serde_json::json!({
        "job_id": job_id,
        "status": "completed",
        "connection_id": connection_id,
    }))).into_response()
}

/// GET /api/v1/connections/:id/sync-jobs
pub async fn list_sync_jobs(
    State(state): State<AppState>,
    Path(connection_id): Path<Uuid>,
) -> impl IntoResponse {
    let jobs = sqlx::query_as::<_, SyncJob>(
        "SELECT * FROM sync_jobs WHERE connection_id = $1 ORDER BY created_at DESC LIMIT 50",
    )
    .bind(connection_id)
    .fetch_all(&state.db)
    .await;

    match jobs {
        Ok(j) => Json(j).into_response(),
        Err(e) => {
            tracing::error!("list sync jobs failed: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
