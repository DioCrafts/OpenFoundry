use axum::{
    Json,
    extract::{Multipart, Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use bytes::{Bytes, BytesMut};
use serde_json::json;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::{
    AppState,
    domain::{quality::profiler, runtime, transactions},
    models::dataset::Dataset,
};

/// POST /api/v1/datasets/:id/upload
pub async fn upload_data(
    State(state): State<AppState>,
    Path(dataset_id): Path<Uuid>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let dataset = match load_dataset(&state, dataset_id).await {
        Ok(Some(dataset)) => dataset,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(error) => {
            tracing::error!("upload lookup failed: {error}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let mut file_data = BytesMut::new();
    let mut message = String::new();
    while let Ok(Some(field)) = multipart.next_field().await {
        if field.name() == Some("file") {
            match field.bytes().await {
                Ok(data) => file_data.extend_from_slice(&data),
                Err(error) => {
                    tracing::error!("failed to read upload: {error}");
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(json!({ "error": "failed to read file" })),
                    )
                        .into_response();
                }
            }
        } else if field.name() == Some("message") {
            if let Ok(value) = field.text().await {
                message = value;
            }
        }
    }

    if file_data.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "no file provided" })),
        )
            .into_response();
    }

    let data = file_data.freeze();
    let size = data.len() as i64;
    let (row_count, schema_fields) = match infer_upload_metadata(&dataset.format, &data).await {
        Ok(metadata) => metadata,
        Err(error) => {
            return (StatusCode::BAD_REQUEST, Json(json!({ "error": error }))).into_response();
        }
    };

    let mut tx = match state.db.begin().await {
        Ok(tx) => tx,
        Err(error) => {
            tracing::error!("upload transaction begin failed: {error}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let locked = match lock_dataset(&mut tx, dataset_id).await {
        Ok(dataset) => dataset,
        Err(error) => {
            tracing::error!("upload dataset lock failed: {error}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let latest_version = match sqlx::query_scalar::<_, Option<i32>>(
        "SELECT MAX(version) FROM dataset_versions WHERE dataset_id = $1",
    )
    .bind(dataset_id)
    .fetch_one(&mut *tx)
    .await
    {
        Ok(value) => value.unwrap_or(locked.current_version.saturating_sub(1)),
        Err(error) => {
            tracing::error!("upload latest version lookup failed: {error}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };
    let new_version = latest_version + 1;
    let version_path = format!("{}/v{new_version}", locked.storage_path);

    if let Err(error) = state.storage.put(&version_path, data.clone()).await {
        tracing::error!("storage upload failed: {error}");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "storage upload failed" })),
        )
            .into_response();
    }

    let version_id = Uuid::now_v7();
    if let Err(error) = sqlx::query(
        r#"INSERT INTO dataset_versions (
               id, dataset_id, version, message, size_bytes, row_count, storage_path
           )
           VALUES ($1, $2, $3, $4, $5, $6, $7)"#,
    )
    .bind(version_id)
    .bind(dataset_id)
    .bind(new_version)
    .bind(message.trim())
    .bind(size)
    .bind(row_count)
    .bind(&version_path)
    .execute(&mut *tx)
    .await
    {
        let _ = state.storage.delete(&version_path).await;
        tracing::error!("insert dataset version failed: {error}");
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    if let Err(error) = sqlx::query_as::<_, Dataset>(
        r#"UPDATE datasets
           SET current_version = $2,
               size_bytes = $3,
               row_count = $4,
               updated_at = NOW()
           WHERE id = $1
           RETURNING *"#,
    )
    .bind(dataset_id)
    .bind(new_version)
    .bind(size)
    .bind(row_count)
    .fetch_one(&mut *tx)
    .await
    {
        let _ = state.storage.delete(&version_path).await;
        tracing::error!("update dataset after upload failed: {error}");
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    if let Err(error) = sqlx::query(
        r#"UPDATE dataset_branches
           SET version = $3,
               updated_at = NOW()
           WHERE dataset_id = $1 AND name = $2"#,
    )
    .bind(dataset_id)
    .bind(&locked.active_branch)
    .bind(new_version)
    .execute(&mut *tx)
    .await
    {
        let _ = state.storage.delete(&version_path).await;
        tracing::error!("update active branch after upload failed: {error}");
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    if let Err(error) = sqlx::query(
        r#"INSERT INTO dataset_schemas (id, dataset_id, fields)
           VALUES ($1, $2, $3::jsonb)
           ON CONFLICT (dataset_id)
           DO UPDATE SET fields = EXCLUDED.fields, created_at = NOW()"#,
    )
    .bind(Uuid::now_v7())
    .bind(dataset_id)
    .bind(&schema_fields)
    .execute(&mut *tx)
    .await
    {
        let _ = state.storage.delete(&version_path).await;
        tracing::error!("upsert dataset schema failed: {error}");
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    let transaction = match transactions::record_committed_transaction(
        &mut tx,
        dataset_id,
        transactions::TransactionRecord {
            view_id: None,
            operation: "upload".to_string(),
            branch_name: Some(locked.active_branch.clone()),
            summary: if message.trim().is_empty() {
                format!("Uploaded dataset version {new_version}")
            } else {
                message.trim().to_string()
            },
            metadata: json!({
                "version": new_version,
                "size_bytes": size,
                "row_count": row_count,
                "storage_path": version_path,
                "active_branch": locked.active_branch,
            }),
        },
    )
    .await
    {
        Ok(transaction) => transaction,
        Err(error) => {
            let _ = state.storage.delete(&version_path).await;
            tracing::error!("record dataset upload transaction failed: {error}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    if let Err(error) = sqlx::query("UPDATE dataset_versions SET transaction_id = $2 WHERE id = $1")
        .bind(version_id)
        .bind(transaction.id)
        .execute(&mut *tx)
        .await
    {
        let _ = state.storage.delete(&version_path).await;
        tracing::error!("attach transaction to dataset version failed: {error}");
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    if let Err(error) = tx.commit().await {
        let _ = state.storage.delete(&version_path).await;
        tracing::error!("commit dataset upload failed: {error}");
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    let refreshed_dataset = load_dataset(&state, dataset_id).await.ok().flatten();
    if let Some(dataset) = refreshed_dataset {
        if let Err(error) =
            profiler::refresh_dataset_quality(&state, &dataset, Some(data.clone())).await
        {
            tracing::warn!(dataset_id = %dataset_id, "quality refresh failed after upload: {error}");
        }
    }

    tracing::info!(dataset_id = %dataset_id, version = new_version, "data uploaded");
    (
        StatusCode::OK,
        Json(json!({
            "dataset_id": dataset_id,
            "version": new_version,
            "size_bytes": size,
            "row_count": row_count,
            "transaction_id": transaction.id,
        })),
    )
        .into_response()
}

async fn infer_upload_metadata(
    format: &str,
    data: &Bytes,
) -> Result<(i64, serde_json::Value), String> {
    let prepared = runtime::prepare_query_context(format, data).await?;
    let row_count =
        runtime::fetch_scalar_i64(&prepared.ctx, "SELECT COUNT(*) AS value FROM dataset").await?;
    let schema_fields = runtime::load_schema_fields(&prepared.ctx).await?;
    runtime::cleanup_temp_path(prepared.path).await;
    Ok((row_count, runtime::schema_to_value(&schema_fields)?))
}

async fn load_dataset(state: &AppState, dataset_id: Uuid) -> Result<Option<Dataset>, sqlx::Error> {
    sqlx::query_as::<_, Dataset>("SELECT * FROM datasets WHERE id = $1")
        .bind(dataset_id)
        .fetch_optional(&state.db)
        .await
}

async fn lock_dataset(
    tx: &mut Transaction<'_, Postgres>,
    dataset_id: Uuid,
) -> Result<Dataset, sqlx::Error> {
    sqlx::query_as::<_, Dataset>("SELECT * FROM datasets WHERE id = $1 FOR UPDATE")
        .bind(dataset_id)
        .fetch_one(&mut **tx)
        .await
}
