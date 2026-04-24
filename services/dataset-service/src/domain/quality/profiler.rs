use std::{path::PathBuf, str::from_utf8};

use arrow::util::display::array_value_to_string;
use bytes::Bytes;
use chrono::Utc;
use datafusion::prelude::NdJsonReadOptions;
use query_engine::context::QueryContext;
use tokio::fs;
use uuid::Uuid;

use crate::{
    AppState,
    domain::quality::{alerts, rules, scorer},
    models::{
        dataset::Dataset,
        quality::{
            DatasetColumnProfile, DatasetProfileRecord, DatasetQualityAlert,
            DatasetQualityHistoryEntry, DatasetQualityProfile, DatasetQualityResponse,
            DatasetQualityRule, DatasetRuleResult, DatasetValueCount,
        },
        schema::SchemaField,
    },
};

pub async fn fetch_dataset_quality(
    state: &AppState,
    dataset_id: Uuid,
) -> Result<DatasetQualityResponse, String> {
    let profile_record = sqlx::query_as::<_, DatasetProfileRecord>(
        "SELECT profile, score, profiled_at FROM dataset_profiles WHERE dataset_id = $1",
    )
    .bind(dataset_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|error| error.to_string())?;

    let rules = sqlx::query_as::<_, DatasetQualityRule>(
        "SELECT * FROM dataset_quality_rules WHERE dataset_id = $1 ORDER BY created_at ASC",
    )
    .bind(dataset_id)
    .fetch_all(&state.db)
    .await
    .map_err(|error| error.to_string())?;

    let alerts = sqlx::query_as::<_, DatasetQualityAlert>(
        "SELECT * FROM dataset_quality_alerts WHERE dataset_id = $1 ORDER BY created_at DESC",
    )
    .bind(dataset_id)
    .fetch_all(&state.db)
    .await
    .map_err(|error| error.to_string())?;

    let history = sqlx::query_as::<_, DatasetQualityHistoryEntry>(
        "SELECT * FROM dataset_quality_history WHERE dataset_id = $1 ORDER BY created_at ASC",
    )
    .bind(dataset_id)
    .fetch_all(&state.db)
    .await
    .map_err(|error| error.to_string())?;

    let (profile, score, profiled_at) = if let Some(record) = profile_record {
        (
            serde_json::from_value(record.profile).ok(),
            Some(record.score),
            Some(record.profiled_at),
        )
    } else {
        (None, None, None)
    };

    Ok(DatasetQualityResponse {
        profile,
        score,
        history,
        alerts,
        rules,
        profiled_at,
    })
}

pub async fn refresh_dataset_quality(
    state: &AppState,
    dataset: &Dataset,
    data_override: Option<Bytes>,
) -> Result<DatasetQualityResponse, String> {
    let rules = sqlx::query_as::<_, DatasetQualityRule>(
        "SELECT * FROM dataset_quality_rules WHERE dataset_id = $1 ORDER BY created_at ASC",
    )
    .bind(dataset.id)
    .fetch_all(&state.db)
    .await
    .map_err(|error| error.to_string())?;

    let version_path = format!("{}/v{}", dataset.storage_path, dataset.current_version);
    let data = if let Some(data) = data_override {
        data
    } else {
        state
            .storage
            .get(&version_path)
            .await
            .map_err(|error| error.to_string())?
    };

    let prepared = prepare_query_context(&dataset.format, &data).await?;
    let generated = generate_profile(&prepared.ctx, &rules).await?;
    let previous_score = sqlx::query_scalar::<_, Option<f64>>(
		"SELECT score FROM dataset_quality_history WHERE dataset_id = $1 ORDER BY created_at DESC LIMIT 1",
	)
	.bind(dataset.id)
	.fetch_optional(&state.db)
	.await
	.map_err(|error| error.to_string())?
	.flatten();

    let profile = DatasetQualityProfile {
        row_count: generated.row_count,
        column_count: generated.columns.len() as i64,
        duplicate_rows: generated.duplicate_rows,
        completeness_ratio: generated.completeness_ratio,
        uniqueness_ratio: generated.uniqueness_ratio,
        generated_at: Utc::now(),
        columns: generated.columns,
        rule_results: generated.rule_results,
    };
    let score = scorer::compute_quality_score(
        profile.row_count,
        profile.duplicate_rows,
        &profile.columns,
        &profile.rule_results,
    );
    let new_alerts = alerts::build_quality_alerts(previous_score, score, &profile.rule_results);

    persist_quality_snapshot(state, dataset, &profile, score, &new_alerts).await?;
    cleanup_temp_path(prepared.path).await;

    fetch_dataset_quality(state, dataset.id).await
}

pub async fn dataset_has_uploaded_data(state: &AppState, dataset: &Dataset) -> bool {
    let version_path = format!("{}/v{}", dataset.storage_path, dataset.current_version);
    state.storage.exists(&version_path).await.unwrap_or(false)
}

struct GeneratedProfile {
    row_count: i64,
    duplicate_rows: i64,
    completeness_ratio: f64,
    uniqueness_ratio: f64,
    columns: Vec<DatasetColumnProfile>,
    rule_results: Vec<DatasetRuleResult>,
}

struct PreparedQueryContext {
    ctx: QueryContext,
    path: PathBuf,
}

async fn generate_profile(
    ctx: &QueryContext,
    rules_config: &[DatasetQualityRule],
) -> Result<GeneratedProfile, String> {
    let schema_fields = load_schema_fields(ctx).await?;
    let row_count = fetch_scalar_i64(ctx, "SELECT COUNT(*) AS value FROM dataset").await?;
    let distinct_rows = fetch_scalar_i64(
        ctx,
        "SELECT COUNT(*) AS value FROM (SELECT DISTINCT * FROM dataset) AS distinct_rows",
    )
    .await?;
    let duplicate_rows = row_count.saturating_sub(distinct_rows);

    let mut columns = Vec::new();
    for field in &schema_fields {
        columns.push(build_column_profile(ctx, field, row_count).await?);
    }

    let completeness_ratio = if columns.is_empty() {
        1.0
    } else {
        columns
            .iter()
            .map(|column| 1.0 - column.null_rate)
            .sum::<f64>()
            / columns.len() as f64
    };
    let uniqueness_ratio = if columns.is_empty() {
        1.0
    } else {
        columns
            .iter()
            .map(|column| column.uniqueness_rate)
            .sum::<f64>()
            / columns.len() as f64
    };
    let rule_results = rules::evaluate_rules(ctx, rules_config, &columns).await?;

    Ok(GeneratedProfile {
        row_count,
        duplicate_rows,
        completeness_ratio,
        uniqueness_ratio,
        columns,
        rule_results,
    })
}

async fn persist_quality_snapshot(
    state: &AppState,
    dataset: &Dataset,
    profile: &DatasetQualityProfile,
    score: f64,
    new_alerts: &[alerts::NewQualityAlert],
) -> Result<(), String> {
    let schema_fields = profile
        .columns
        .iter()
        .map(|column| SchemaField {
            name: column.name.clone(),
            field_type: column.field_type.clone(),
            nullable: column.null_count > 0,
        })
        .collect::<Vec<_>>();

    sqlx::query(
        r#"INSERT INTO dataset_schemas (id, dataset_id, fields)
		   VALUES ($1, $2, $3)
		   ON CONFLICT (dataset_id)
		   DO UPDATE SET fields = EXCLUDED.fields, created_at = NOW()"#,
    )
    .bind(Uuid::now_v7())
    .bind(dataset.id)
    .bind(serde_json::to_value(&schema_fields).map_err(|error| error.to_string())?)
    .execute(&state.db)
    .await
    .map_err(|error| error.to_string())?;

    sqlx::query("UPDATE datasets SET row_count = $2, updated_at = NOW() WHERE id = $1")
        .bind(dataset.id)
        .bind(profile.row_count)
        .execute(&state.db)
        .await
        .map_err(|error| error.to_string())?;

    sqlx::query(
		r#"INSERT INTO dataset_profiles (id, dataset_id, profile, score, alerts, profiled_at, updated_at)
		   VALUES ($1, $2, $3, $4, $5, NOW(), NOW())
		   ON CONFLICT (dataset_id)
		   DO UPDATE SET profile = EXCLUDED.profile,
		                 score = EXCLUDED.score,
		                 alerts = EXCLUDED.alerts,
		                 profiled_at = NOW(),
		                 updated_at = NOW()"#,
	)
	.bind(Uuid::now_v7())
	.bind(dataset.id)
	.bind(serde_json::to_value(profile).map_err(|error| error.to_string())?)
	.bind(score)
	.bind(
		serde_json::to_value(
			new_alerts
				.iter()
				.map(|alert| {
					serde_json::json!({
						"level": alert.level,
						"kind": alert.kind,
						"message": alert.message,
						"details": alert.details,
					})
				})
				.collect::<Vec<_>>(),
		)
		.map_err(|error| error.to_string())?,
	)
	.execute(&state.db)
	.await
	.map_err(|error| error.to_string())?;

    let passed_rules = profile
        .rule_results
        .iter()
        .filter(|result| result.passed)
        .count() as i32;
    let failed_rules = profile.rule_results.len() as i32 - passed_rules;
    sqlx::query(
		"UPDATE dataset_quality_alerts SET status = 'resolved', resolved_at = NOW() WHERE dataset_id = $1 AND status = 'active'",
	)
	.bind(dataset.id)
	.execute(&state.db)
	.await
	.map_err(|error| error.to_string())?;

    sqlx::query(
		"INSERT INTO dataset_quality_history (id, dataset_id, score, passed_rules, failed_rules, alerts_count) VALUES ($1, $2, $3, $4, $5, $6)",
	)
	.bind(Uuid::now_v7())
	.bind(dataset.id)
	.bind(score)
	.bind(passed_rules)
	.bind(failed_rules)
	.bind(new_alerts.len() as i32)
	.execute(&state.db)
	.await
	.map_err(|error| error.to_string())?;

    for alert in new_alerts {
        sqlx::query(
			"INSERT INTO dataset_quality_alerts (id, dataset_id, level, kind, message, status, details) VALUES ($1, $2, $3, $4, $5, 'active', $6)",
		)
		.bind(Uuid::now_v7())
		.bind(dataset.id)
		.bind(&alert.level)
		.bind(&alert.kind)
		.bind(&alert.message)
		.bind(&alert.details)
		.execute(&state.db)
		.await
		.map_err(|error| error.to_string())?;
    }

    Ok(())
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

async fn build_column_profile(
    ctx: &QueryContext,
    field: &SchemaField,
    row_count: i64,
) -> Result<DatasetColumnProfile, String> {
    let quoted = quote_identifier(&field.name);
    let null_count = fetch_scalar_i64(
        ctx,
        &format!("SELECT COUNT(*) AS value FROM dataset WHERE {quoted} IS NULL"),
    )
    .await?;
    let non_null_count = row_count.saturating_sub(null_count);
    let distinct_count = if non_null_count == 0 {
        0
    } else {
        fetch_scalar_i64(
            ctx,
            &format!(
                "SELECT COUNT(DISTINCT {quoted}) AS value FROM dataset WHERE {quoted} IS NOT NULL"
            ),
        )
        .await?
    };

    let sample_values = fetch_value_counts(ctx, &quoted).await?;
    let (min_value, max_value, average_value) = if is_numeric_type(&field.field_type) {
        fetch_numeric_stats(ctx, &quoted).await?
    } else {
        (None, None, None)
    };

    Ok(DatasetColumnProfile {
        name: field.name.clone(),
        field_type: field.field_type.clone(),
        nullable: field.nullable,
        null_count,
        null_rate: ratio(null_count, row_count),
        distinct_count,
        uniqueness_rate: ratio(distinct_count, non_null_count),
        sample_values,
        min_value,
        max_value,
        average_value,
    })
}

async fn fetch_value_counts(
    ctx: &QueryContext,
    quoted: &str,
) -> Result<Vec<DatasetValueCount>, String> {
    let rows = collect_rows(
        ctx,
        &format!(
            "SELECT CAST({quoted} AS VARCHAR) AS value, COUNT(*) AS count
			 FROM dataset
			 WHERE {quoted} IS NOT NULL
			 GROUP BY CAST({quoted} AS VARCHAR)
			 ORDER BY count DESC
			 LIMIT 5"
        ),
    )
    .await?;

    Ok(rows
        .into_iter()
        .filter_map(|row| {
            let value = row.first()?.clone();
            let count = row.get(1)?.parse::<i64>().ok()?;
            Some(DatasetValueCount { value, count })
        })
        .collect())
}

async fn fetch_numeric_stats(
    ctx: &QueryContext,
    quoted: &str,
) -> Result<(Option<String>, Option<String>, Option<f64>), String> {
    let rows = collect_rows(
		ctx,
		&format!(
			"SELECT MIN({quoted}) AS min_value, MAX({quoted}) AS max_value, AVG({quoted}) AS avg_value FROM dataset WHERE {quoted} IS NOT NULL"
		),
	)
	.await?;

    let row = rows.first();
    let min_value = row
        .and_then(|values| values.first())
        .cloned()
        .filter(|value| value != "null");
    let max_value = row
        .and_then(|values| values.get(1))
        .cloned()
        .filter(|value| value != "null");
    let average_value = row
        .and_then(|values| values.get(2))
        .and_then(|value| value.parse::<f64>().ok());

    Ok((min_value, max_value, average_value))
}

async fn prepare_query_context(format: &str, data: &[u8]) -> Result<PreparedQueryContext, String> {
    let extension = match format {
        "csv" => "csv",
        "json" => "json",
        _ => "parquet",
    };
    let path = std::env::temp_dir().join(format!(
        "openfoundry-quality-{}.{}",
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

    Ok(PreparedQueryContext { ctx, path })
}

fn normalize_json_bytes(data: &[u8]) -> Result<Vec<u8>, String> {
    let text = from_utf8(data).map_err(|error| error.to_string())?;
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    if trimmed.starts_with('[') || trimmed.starts_with('{') {
        let parsed: serde_json::Value =
            serde_json::from_slice(data).map_err(|error| error.to_string())?;
        let mut lines = String::new();
        match parsed {
            serde_json::Value::Array(rows) => {
                for row in rows {
                    lines
                        .push_str(&serde_json::to_string(&row).map_err(|error| error.to_string())?);
                    lines.push('\n');
                }
            }
            serde_json::Value::Object(_) => {
                lines.push_str(&serde_json::to_string(&parsed).map_err(|error| error.to_string())?);
                lines.push('\n');
            }
            _ => return Err("JSON uploads must contain objects or arrays of objects".to_string()),
        }
        return Ok(lines.into_bytes());
    }

    Ok(data.to_vec())
}

async fn collect_rows(ctx: &QueryContext, sql: &str) -> Result<Vec<Vec<String>>, String> {
    let batches = ctx
        .execute_sql(sql)
        .await
        .map_err(|error| error.to_string())?;
    let mut rows = Vec::new();

    for batch in batches {
        for row_index in 0..batch.num_rows() {
            let mut row = Vec::new();
            for column_index in 0..batch.num_columns() {
                row.push(
                    array_value_to_string(batch.column(column_index), row_index)
                        .unwrap_or_else(|_| "null".to_string()),
                );
            }
            rows.push(row);
        }
    }

    Ok(rows)
}

async fn fetch_scalar_i64(ctx: &QueryContext, sql: &str) -> Result<i64, String> {
    let rows = collect_rows(ctx, sql).await?;
    Ok(rows
        .first()
        .and_then(|row| row.first())
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(0))
}

fn quote_identifier(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

fn is_numeric_type(field_type: &str) -> bool {
    let normalized = field_type.to_ascii_lowercase();
    normalized.contains("int")
        || normalized.contains("float")
        || normalized.contains("double")
        || normalized.contains("decimal")
        || normalized.contains("uint")
}

fn ratio(part: i64, total: i64) -> f64 {
    if total <= 0 {
        0.0
    } else {
        (part as f64 / total as f64).clamp(0.0, 1.0)
    }
}

async fn cleanup_temp_path(path: PathBuf) {
    let _ = fs::remove_file(path).await;
}
