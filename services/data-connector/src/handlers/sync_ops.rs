use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use uuid::Uuid;

use crate::AppState;
use crate::models::{connection::Connection, sync_job::SyncJob, sync_status::SyncStatus};

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
    let scheduled_at = body.schedule_at.unwrap_or_else(chrono::Utc::now);
    let max_attempts = body.max_attempts.unwrap_or(3).clamp(1, 10);
    let inserted = sqlx::query(
        r#"INSERT INTO sync_jobs (
               id, connection_id, target_dataset_id, table_name, status, scheduled_at, max_attempts, sync_metadata
           )
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8::jsonb)"#,
    )
    .bind(job_id)
    .bind(connection_id)
    .bind(body.target_dataset_id)
    .bind(&body.table_name)
    .bind(SyncStatus::Pending.as_str())
    .bind(scheduled_at)
    .bind(max_attempts)
    .bind(serde_json::json!({
        "selector": body.table_name,
        "connector_type": conn.connector_type,
    }))
    .execute(&state.db)
    .await;

    if let Err(error) = inserted {
        tracing::error!("sync job insert failed: {error}");
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    tracing::info!(
        connection_id = %connection_id,
        job_id = %job_id,
        connector_type = %conn.connector_type,
        target_dataset_id = ?body.target_dataset_id,
        "sync job queued"
    );

    let scheduler_state = state.clone();
    tokio::spawn(async move {
        if let Err(error) = crate::domain::scheduler::tick(&scheduler_state).await {
            tracing::warn!("sync scheduler trigger failed: {error}");
        }
    });

    (
        StatusCode::ACCEPTED,
        Json(serde_json::json!({
            "job_id": job_id,
            "status": "pending",
            "connection_id": connection_id,
            "scheduled_at": scheduled_at,
            "max_attempts": max_attempts,
        })),
    )
        .into_response()
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
