use std::time::Duration;

use auth_middleware::jwt::{build_access_claims, encode_token};
use reqwest::multipart::{Form, Part};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::{
    AppState,
    connectors::{self, SyncPayload},
    models::{connection::Connection, sync_job::SyncJob, sync_status::SyncStatus},
};

pub async fn run_due_jobs(state: &AppState) -> Result<usize, String> {
    let due_job_ids = sqlx::query_scalar::<_, Uuid>(
        r#"SELECT id
           FROM sync_jobs
           WHERE status IN ('pending', 'retrying')
             AND COALESCE(next_retry_at, scheduled_at, created_at) <= NOW()
           ORDER BY COALESCE(next_retry_at, scheduled_at, created_at) ASC
           LIMIT 8"#,
    )
    .fetch_all(&state.db)
    .await
    .map_err(|error| error.to_string())?;

    for job_id in &due_job_ids {
        if let Some(job) = claim_job(state, *job_id).await? {
            let state = state.clone();
            tokio::spawn(async move {
                if let Err(error) = process_job(&state, &job).await {
                    tracing::warn!(job_id = %job.id, "sync job failed: {error}");
                    if let Err(mark_error) = mark_job_failure(&state, &job, &error).await {
                        tracing::error!(job_id = %job.id, "sync failure bookkeeping failed: {mark_error}");
                    }
                }
            });
        }
    }

    Ok(due_job_ids.len())
}

async fn claim_job(state: &AppState, job_id: Uuid) -> Result<Option<SyncJob>, String> {
    sqlx::query_as::<_, SyncJob>(
        r#"UPDATE sync_jobs
           SET status = 'running',
               attempts = attempts + 1,
               started_at = NOW(),
               error = NULL
           WHERE id = $1
             AND status IN ('pending', 'retrying')
           RETURNING *"#,
    )
    .bind(job_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|error| error.to_string())
}

async fn process_job(state: &AppState, job: &SyncJob) -> Result<(), String> {
    let connection = sqlx::query_as::<_, Connection>("SELECT * FROM connections WHERE id = $1")
        .bind(job.connection_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("connection {} not found", job.connection_id))?;

    let target_dataset_id = job
        .target_dataset_id
        .ok_or_else(|| "target_dataset_id is required for sync jobs".to_string())?;

    let payload = fetch_source_payload(&connection, job).await?;
    let upload_result = upload_dataset(state, &connection, target_dataset_id, &payload).await?;

    sqlx::query(
        r#"UPDATE sync_jobs
           SET status = 'completed',
               rows_synced = $2,
               error = NULL,
               result_dataset_version = $3,
               next_retry_at = NULL,
               sync_metadata = $4::jsonb,
               completed_at = NOW()
           WHERE id = $1"#,
    )
    .bind(job.id)
    .bind(payload.rows_synced)
    .bind(upload_result.version)
    .bind(json!({
        "source": payload.metadata,
        "upload": upload_result.metadata,
    }))
    .execute(&state.db)
    .await
    .map_err(|error| error.to_string())?;

    sqlx::query(
        "UPDATE connections SET status = 'connected', last_sync_at = NOW(), updated_at = NOW() WHERE id = $1",
    )
    .bind(connection.id)
    .execute(&state.db)
    .await
    .map_err(|error| error.to_string())?;

    tracing::info!(
        job_id = %job.id,
        connection_id = %connection.id,
        rows_synced = payload.rows_synced,
        dataset_version = ?upload_result.version,
        "sync job completed"
    );

    Ok(())
}

async fn fetch_source_payload(
    connection: &Connection,
    job: &SyncJob,
) -> Result<SyncPayload, String> {
    match connection.connector_type.as_str() {
        "postgresql" => {
            connectors::postgres::fetch_dataset(&connection.config, &job.table_name).await
        }
        "csv" => connectors::csv::fetch_dataset(&connection.config, &job.table_name).await,
        "json" => connectors::json::fetch_dataset(&connection.config, &job.table_name).await,
        "rest_api" => {
            connectors::rest_api::fetch_dataset(&connection.config, &job.table_name).await
        }
        "salesforce" => {
            connectors::salesforce::fetch_dataset(&connection.config, &job.table_name).await
        }
        other => Err(format!("unsupported connector type for sync: {other}")),
    }
}

struct DatasetUploadResult {
    version: Option<i32>,
    metadata: Value,
}

async fn upload_dataset(
    state: &AppState,
    connection: &Connection,
    dataset_id: Uuid,
    payload: &SyncPayload,
) -> Result<DatasetUploadResult, String> {
    let token = issue_sync_token(state, connection.owner_id)?;
    let url = format!(
        "{}/api/v1/datasets/{dataset_id}/upload",
        state.dataset_service_url.trim_end_matches('/')
    );

    let part = Part::bytes(payload.bytes.clone())
        .file_name(payload.file_name.clone())
        .mime_str(mime_for_format(&payload.format))
        .map_err(|error| error.to_string())?;
    let form = Form::new().part("file", part);

    let response = state
        .http_client
        .post(url)
        .bearer_auth(token)
        .multipart(form)
        .send()
        .await
        .map_err(|error| error.to_string())?;

    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(format!("dataset upload failed with HTTP {status}: {body}"));
    }

    let payload_json =
        serde_json::from_str::<Value>(&body).unwrap_or_else(|_| json!({ "raw": body }));
    Ok(DatasetUploadResult {
        version: payload_json
            .get("version")
            .and_then(Value::as_i64)
            .map(|value| value as i32),
        metadata: payload_json,
    })
}

fn issue_sync_token(state: &AppState, owner_id: Uuid) -> Result<String, String> {
    let claims = build_access_claims(
        &state.jwt_config,
        owner_id,
        "sync@openfoundry.local",
        "OpenFoundry Sync",
        vec!["admin".to_string()],
        vec!["*:*".to_string()],
        None,
        json!({ "source": "data_connector_sync" }),
        vec!["service_sync".to_string()],
    );

    encode_token(&state.jwt_config, &claims).map_err(|error| error.to_string())
}

fn mime_for_format(format: &str) -> &'static str {
    match format {
        "csv" => "text/csv",
        "json" => "application/json",
        _ => "application/octet-stream",
    }
}

pub async fn mark_job_failure(state: &AppState, job: &SyncJob, error: &str) -> Result<(), String> {
    let retry_at = if job.attempts < job.max_attempts {
        Some(
            chrono::Utc::now()
                + chrono::Duration::from_std(retry_delay(job.attempts))
                    .map_err(|duration_error| duration_error.to_string())?,
        )
    } else {
        None
    };

    let status = if retry_at.is_some() {
        SyncStatus::Retrying.as_str()
    } else {
        SyncStatus::Failed.as_str()
    };

    sqlx::query(
        r#"UPDATE sync_jobs
           SET status = $2,
               error = $3,
               next_retry_at = $4,
               completed_at = CASE WHEN $2 = 'failed' THEN NOW() ELSE NULL END,
               sync_metadata = sync_metadata || $5::jsonb
           WHERE id = $1"#,
    )
    .bind(job.id)
    .bind(status)
    .bind(error)
    .bind(retry_at)
    .bind(json!({
        "last_error": error,
        "attempts": job.attempts,
        "max_attempts": job.max_attempts,
    }))
    .execute(&state.db)
    .await
    .map_err(|db_error| db_error.to_string())?;

    sqlx::query("UPDATE connections SET status = 'error', updated_at = NOW() WHERE id = $1")
        .bind(job.connection_id)
        .execute(&state.db)
        .await
        .map_err(|db_error| db_error.to_string())?;

    Ok(())
}

fn retry_delay(attempts: i32) -> Duration {
    Duration::from_secs((attempts.max(1) as u64 * 5).min(300))
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::retry_delay;

    #[test]
    fn retry_delay_is_bounded() {
        assert_eq!(retry_delay(1), Duration::from_secs(5));
        assert_eq!(retry_delay(3), Duration::from_secs(15));
        assert_eq!(retry_delay(100), Duration::from_secs(300));
    }
}
