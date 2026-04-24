use std::{path::PathBuf, str::from_utf8};

use arrow::util::display::array_value_to_string;
use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use datafusion::prelude::NdJsonReadOptions;
use query_engine::context::QueryContext;
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::fs;
use uuid::Uuid;

use crate::{
    AppState,
    models::{
        branch::DatasetBranch,
        dataset::Dataset,
        schema::{DatasetSchema, SchemaField},
        version::DatasetVersion,
    },
};

#[derive(Debug, Deserialize)]
pub struct PreviewQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub version: Option<i32>,
    pub branch: Option<String>,
}

struct PreparedPreview {
    ctx: QueryContext,
    path: PathBuf,
}

struct PreviewSource {
    dataset: Dataset,
    branch: Option<String>,
    version: i32,
    size_bytes: i64,
    storage_path: String,
}

/// GET /api/v1/datasets/:id/preview
pub async fn preview_data(
    State(state): State<AppState>,
    Path(dataset_id): Path<Uuid>,
    Query(params): Query<PreviewQuery>,
) -> impl IntoResponse {
    let limit = params.limit.unwrap_or(50).clamp(1, 1_000);
    let offset = params.offset.unwrap_or(0).max(0);

    let source = match resolve_preview_source(&state, dataset_id, &params).await {
        Ok(Some(source)) => source,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(PreviewLookupError::Invalid(message)) => {
            return (StatusCode::BAD_REQUEST, Json(json!({ "error": message }))).into_response();
        }
        Err(PreviewLookupError::Database(error)) => {
            tracing::error!("preview lookup failed: {error}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let mut warnings = Vec::new();
    let mut errors = Vec::new();

    match state.storage.get(&source.storage_path).await {
        Ok(bytes) => match prepare_query_context(&source.dataset.format, &bytes).await {
            Ok(prepared) => {
                let schema = load_schema_fields(&prepared.ctx).await;
                let total_rows =
                    fetch_scalar_i64(&prepared.ctx, "SELECT COUNT(*) AS value FROM dataset").await;
                let rows = collect_object_rows(
                    &prepared.ctx,
                    &format!("SELECT * FROM dataset LIMIT {limit} OFFSET {offset}"),
                )
                .await;

                let columns = match schema {
                    Ok(columns) => columns,
                    Err(error) => {
                        errors.push(format!("schema inference failed: {error}"));
                        Vec::new()
                    }
                };
                let total_rows = match total_rows {
                    Ok(total_rows) => total_rows,
                    Err(error) => {
                        errors.push(format!("row counting failed: {error}"));
                        0
                    }
                };
                let rows = match rows {
                    Ok(rows) => rows,
                    Err(error) => {
                        errors.push(format!("row sampling failed: {error}"));
                        Vec::new()
                    }
                };

                if rows.is_empty() && total_rows > 0 {
                    warnings.push("requested page returned no rows".to_string());
                }

                cleanup_temp_path(prepared.path).await;

                return Json(json!({
                    "dataset_id": dataset_id,
                    "branch": source.branch,
                    "version": source.version,
                    "format": source.dataset.format,
                    "size_bytes": source.size_bytes,
                    "storage_path": source.storage_path,
                    "limit": limit,
                    "offset": offset,
                    "total_rows": total_rows,
                    "row_count": rows.len(),
                    "columns": columns,
                    "rows": rows,
                    "warnings": warnings,
                    "errors": errors,
                }))
                .into_response();
            }
            Err(error) => {
                errors.push(format!("preview preparation failed: {error}"));
            }
        },
        Err(error) => {
            warnings.push("dataset has metadata but no readable storage object yet".to_string());
            errors.push(format!("storage read failed: {error}"));
        }
    }

    Json(json!({
        "dataset_id": dataset_id,
        "branch": source.branch,
        "version": source.version,
        "format": source.dataset.format,
        "size_bytes": source.size_bytes,
        "storage_path": source.storage_path,
        "limit": limit,
        "offset": offset,
        "total_rows": 0,
        "row_count": 0,
        "columns": [],
        "rows": [],
        "warnings": warnings,
        "errors": errors,
    }))
    .into_response()
}

/// GET /api/v1/datasets/:id/schema
pub async fn get_schema(
    State(state): State<AppState>,
    Path(dataset_id): Path<Uuid>,
) -> impl IntoResponse {
    let schema =
        sqlx::query_as::<_, DatasetSchema>("SELECT * FROM dataset_schemas WHERE dataset_id = $1")
            .bind(dataset_id)
            .fetch_optional(&state.db)
            .await;

    match schema {
        Ok(Some(s)) => Json(s).into_response(),
        Ok(None) => match derive_schema(&state, dataset_id).await {
            Ok(Some(schema)) => Json(schema).into_response(),
            Ok(None) => (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": "no schema found" })),
            )
                .into_response(),
            Err(error) => {
                tracing::error!("derive schema failed: {error}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        },
        Err(error) => {
            tracing::error!("get schema failed: {error}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

async fn derive_schema(
    state: &AppState,
    dataset_id: Uuid,
) -> Result<Option<DatasetSchema>, String> {
    let dataset = sqlx::query_as::<_, Dataset>("SELECT * FROM datasets WHERE id = $1")
        .bind(dataset_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|error| error.to_string())?;

    let Some(dataset) = dataset else {
        return Ok(None);
    };

    let storage_path = format!("{}/v{}", dataset.storage_path, dataset.current_version);
    let bytes = match state.storage.get(&storage_path).await {
        Ok(bytes) => bytes,
        Err(_) => return Ok(None),
    };
    let prepared = prepare_query_context(&dataset.format, &bytes).await?;
    let fields = load_schema_fields(&prepared.ctx).await?;
    cleanup_temp_path(prepared.path).await;

    Ok(Some(DatasetSchema {
        id: Uuid::now_v7(),
        dataset_id,
        fields: serde_json::to_value(fields).map_err(|error| error.to_string())?,
        created_at: chrono::Utc::now(),
    }))
}

async fn resolve_preview_source(
    state: &AppState,
    dataset_id: Uuid,
    params: &PreviewQuery,
) -> Result<Option<PreviewSource>, PreviewLookupError> {
    let dataset = sqlx::query_as::<_, Dataset>("SELECT * FROM datasets WHERE id = $1")
        .bind(dataset_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|error| PreviewLookupError::Database(error.to_string()))?;

    let Some(dataset) = dataset else {
        return Ok(None);
    };

    if let Some(version) = params.version {
        if version < 1 {
            return Err(PreviewLookupError::Invalid(
                "version must be greater than zero".to_string(),
            ));
        }
    }

    let branch = params
        .branch
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());

    let branch_record = if let Some(branch_name) = branch {
        sqlx::query_as::<_, DatasetBranch>(
            "SELECT * FROM dataset_branches WHERE dataset_id = $1 AND name = $2",
        )
        .bind(dataset_id)
        .bind(branch_name)
        .fetch_optional(&state.db)
        .await
        .map_err(|error| PreviewLookupError::Database(error.to_string()))?
    } else {
        None
    };

    if branch.is_some() && branch_record.is_none() {
        return Ok(None);
    }

    let version = params
        .version
        .or_else(|| branch_record.as_ref().map(|record| record.version))
        .unwrap_or(dataset.current_version);

    let version_record = sqlx::query_as::<_, DatasetVersion>(
        "SELECT * FROM dataset_versions WHERE dataset_id = $1 AND version = $2",
    )
    .bind(dataset_id)
    .bind(version)
    .fetch_optional(&state.db)
    .await
    .map_err(|error| PreviewLookupError::Database(error.to_string()))?;

    let (storage_path, size_bytes) = if version == dataset.current_version {
        (
            format!("{}/v{}", dataset.storage_path, dataset.current_version),
            dataset.size_bytes,
        )
    } else if let Some(version_record) = version_record {
        (version_record.storage_path, version_record.size_bytes)
    } else {
        return Ok(None);
    };

    Ok(Some(PreviewSource {
        dataset,
        branch: branch_record.map(|record| record.name),
        version,
        size_bytes,
        storage_path,
    }))
}

async fn prepare_query_context(format: &str, data: &[u8]) -> Result<PreparedPreview, String> {
    let extension = match format {
        "csv" => "csv",
        "json" => "json",
        _ => "parquet",
    };
    let path = std::env::temp_dir().join(format!(
        "openfoundry-preview-{}.{}",
        Uuid::now_v7(),
        extension
    ));
    let bytes = if format == "json" {
        normalize_json_bytes(data)?
    } else {
        data.to_vec()
    };

    fs::write(&path, bytes)
        .await
        .map_err(|error| error.to_string())?;

    let ctx = QueryContext::new();
    let file_path = path.to_string_lossy().to_string();
    match format {
        "csv" => ctx
            .register_csv("dataset", &file_path)
            .await
            .map_err(|error| error.to_string())?,
        "json" => ctx
            .inner()
            .register_json("dataset", &file_path, NdJsonReadOptions::default())
            .await
            .map_err(|error| error.to_string())?,
        _ => ctx
            .register_parquet("dataset", &file_path)
            .await
            .map_err(|error| error.to_string())?,
    }

    Ok(PreparedPreview { ctx, path })
}

async fn load_schema_fields(ctx: &QueryContext) -> Result<Vec<SchemaField>, String> {
    let dataframe = ctx
        .sql("SELECT * FROM dataset LIMIT 1")
        .await
        .map_err(|error| error.to_string())?;

    Ok(dataframe
        .schema()
        .fields()
        .iter()
        .map(|field| SchemaField {
            name: field.name().to_string(),
            field_type: field.data_type().to_string(),
            nullable: field.is_nullable(),
        })
        .collect())
}

async fn collect_object_rows(ctx: &QueryContext, sql: &str) -> Result<Vec<Value>, String> {
    let batches = ctx
        .execute_sql(sql)
        .await
        .map_err(|error| error.to_string())?;
    let mut rows = Vec::new();

    for batch in batches {
        let field_names = batch
            .schema()
            .fields()
            .iter()
            .map(|field| field.name().to_string())
            .collect::<Vec<_>>();
        for row_index in 0..batch.num_rows() {
            let mut row = serde_json::Map::new();
            for (column_index, field_name) in field_names.iter().enumerate() {
                let raw = array_value_to_string(batch.column(column_index), row_index)
                    .unwrap_or_else(|_| "null".to_string());
                row.insert(field_name.clone(), json_scalar_or_string(&raw));
            }
            rows.push(Value::Object(row));
        }
    }

    Ok(rows)
}

async fn fetch_scalar_i64(ctx: &QueryContext, sql: &str) -> Result<i64, String> {
    let rows = collect_object_rows(ctx, sql).await?;
    Ok(rows
        .first()
        .and_then(|row| row.as_object())
        .and_then(|row| row.values().next())
        .and_then(|value| {
            value
                .as_i64()
                .or_else(|| value.as_str()?.parse::<i64>().ok())
        })
        .unwrap_or(0))
}

fn normalize_json_bytes(data: &[u8]) -> Result<Vec<u8>, String> {
    let text = from_utf8(data).map_err(|error| error.to_string())?;
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    if trimmed.starts_with('[') || trimmed.starts_with('{') {
        let parsed: Value = serde_json::from_slice(data).map_err(|error| error.to_string())?;
        let mut lines = String::new();
        match parsed {
            Value::Array(rows) => {
                for row in rows {
                    lines
                        .push_str(&serde_json::to_string(&row).map_err(|error| error.to_string())?);
                    lines.push('\n');
                }
            }
            Value::Object(_) => {
                lines.push_str(&serde_json::to_string(&parsed).map_err(|error| error.to_string())?);
                lines.push('\n');
            }
            _ => return Err("JSON uploads must contain objects or arrays of objects".to_string()),
        }
        return Ok(lines.into_bytes());
    }

    Ok(data.to_vec())
}

fn json_scalar_or_string(raw: &str) -> Value {
    if raw == "null" {
        Value::Null
    } else {
        serde_json::from_str(raw).unwrap_or_else(|_| Value::String(raw.to_string()))
    }
}

async fn cleanup_temp_path(path: PathBuf) {
    let _ = fs::remove_file(path).await;
}

enum PreviewLookupError {
    Invalid(String),
    Database(String),
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::json_scalar_or_string;

    #[test]
    fn parses_json_scalars_when_possible() {
        assert_eq!(json_scalar_or_string("12"), json!(12));
        assert_eq!(json_scalar_or_string("true"), json!(true));
        assert_eq!(json_scalar_or_string("ready"), json!("ready"));
        assert_eq!(json_scalar_or_string("null"), json!(null));
    }
}
