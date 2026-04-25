//! PostgreSQL connector — connects to an external Postgres database
//! and reads tables/views for sync.

use std::time::Instant;

use serde_json::{Value, json};

use super::{
    ConnectionTestResult, SyncPayload, add_source_signature, basic_discovered_source,
    virtual_table_response,
};
use crate::models::registration::{
    DiscoveredSource, VirtualTableQueryRequest, VirtualTableQueryResponse,
};

/// Validate that the connection config has the required fields.
pub fn validate_config(config: &Value) -> Result<(), String> {
    let required = ["host", "port", "database", "user", "password"];
    for field in &required {
        if config.get(*field).is_none() {
            return Err(format!("missing required field: {field}"));
        }
    }
    Ok(())
}

/// Build a connection string from the JSON config.
pub fn build_connection_string(config: &Value) -> String {
    format!(
        "postgres://{}:{}@{}:{}/{}",
        config["user"].as_str().unwrap_or("postgres"),
        config["password"].as_str().unwrap_or(""),
        config["host"].as_str().unwrap_or("localhost"),
        config["port"].as_u64().unwrap_or(5432),
        config["database"].as_str().unwrap_or("postgres"),
    )
}

pub async fn test_connection(config: &Value) -> Result<ConnectionTestResult, String> {
    validate_config(config)?;

    let started = Instant::now();
    let pool = sqlx::PgPool::connect(&build_connection_string(config))
        .await
        .map_err(|error| error.to_string())?;
    let (database_name, current_user): (String, String) =
        sqlx::query_as("SELECT current_database(), current_user")
            .fetch_one(&pool)
            .await
            .map_err(|error| error.to_string())?;

    Ok(ConnectionTestResult {
        success: true,
        message: format!("connected to database '{database_name}' as '{current_user}'"),
        latency_ms: started.elapsed().as_millis(),
        details: Some(json!({
            "database": database_name,
            "user": current_user,
        })),
    })
}

pub async fn fetch_dataset(config: &Value, selector: &str) -> Result<SyncPayload, String> {
    validate_config(config)?;
    let query = source_query(config, selector)?;

    let pool = sqlx::PgPool::connect(&build_connection_string(config))
        .await
        .map_err(|error| error.to_string())?;

    let payload_query = format!(
        "SELECT COALESCE(json_agg(row_to_json(dataset_rows)), '[]'::json) FROM ({query}) AS dataset_rows"
    );
    let payload: Value = sqlx::query_scalar(&payload_query)
        .fetch_one(&pool)
        .await
        .map_err(|error| error.to_string())?;
    let rows_synced = payload
        .as_array()
        .map(|rows| rows.len() as i64)
        .unwrap_or(0);
    let file_name = format!("{}.json", sanitize_file_stem(selector));

    let mut payload = SyncPayload {
        bytes: serde_json::to_vec(&payload).map_err(|error| error.to_string())?,
        format: "json".to_string(),
        rows_synced,
        file_name,
        metadata: json!({
            "query": query,
            "selector": selector,
        }),
    };
    add_source_signature(&mut payload);
    Ok(payload)
}

pub async fn discover_sources(config: &Value) -> Result<Vec<DiscoveredSource>, String> {
    validate_config(config)?;

    let pool = sqlx::PgPool::connect(&build_connection_string(config))
        .await
        .map_err(|error| error.to_string())?;
    let rows = sqlx::query_as::<_, (String, String, String)>(
        r#"SELECT table_schema, table_name, table_type
           FROM information_schema.tables
           WHERE table_schema NOT IN ('pg_catalog', 'information_schema')
           ORDER BY table_schema, table_name
           LIMIT 200"#,
    )
    .fetch_all(&pool)
    .await
    .map_err(|error| error.to_string())?;

    Ok(rows
        .into_iter()
        .map(|(schema, table, table_type)| {
            let selector = format!("{schema}.{table}");
            basic_discovered_source(
                selector.clone(),
                selector,
                if table_type.contains("VIEW") {
                    "view"
                } else {
                    "table"
                },
                json!({
                    "schema": schema,
                    "table": table,
                    "table_type": table_type,
                }),
            )
        })
        .collect())
}

pub async fn query_virtual_table(
    config: &Value,
    request: &VirtualTableQueryRequest,
) -> Result<VirtualTableQueryResponse, String> {
    validate_config(config)?;
    let query = source_query(config, &request.selector)?;
    let limited = format!(
        "SELECT COALESCE(json_agg(row_to_json(dataset_rows)), '[]'::json) FROM ({query} LIMIT {}) AS dataset_rows",
        request.limit.unwrap_or(50).clamp(1, 500)
    );

    let pool = sqlx::PgPool::connect(&build_connection_string(config))
        .await
        .map_err(|error| error.to_string())?;
    let rows: Value = sqlx::query_scalar(&limited)
        .fetch_one(&pool)
        .await
        .map_err(|error| error.to_string())?;
    let rows = rows.as_array().cloned().unwrap_or_default();

    Ok(virtual_table_response(
        &request.selector,
        rows,
        json!({
            "query": query,
            "limit": request.limit.unwrap_or(50).clamp(1, 500),
        }),
    ))
}

fn source_query(config: &Value, selector: &str) -> Result<String, String> {
    if let Some(query) = config.get("query").and_then(Value::as_str) {
        let trimmed = query.trim();
        if !trimmed.is_empty() {
            return Ok(trimmed.to_string());
        }
    }

    let trimmed = selector.trim();
    if trimmed.is_empty() {
        return Err("table_name is required for PostgreSQL sync".to_string());
    }

    if trimmed.to_ascii_lowercase().starts_with("select ") {
        return Ok(trimmed.to_string());
    }

    let identifier = trimmed
        .split('.')
        .map(|part| {
            if part.is_empty()
                || !part
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '"')
            {
                return Err(format!("invalid PostgreSQL identifier: {trimmed}"));
            }
            Ok(part.to_string())
        })
        .collect::<Result<Vec<_>, _>>()?
        .join(".");

    Ok(format!("SELECT * FROM {identifier}"))
}

fn sanitize_file_stem(selector: &str) -> String {
    selector
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect::<String>()
        .trim_matches('_')
        .to_string()
        .chars()
        .take(64)
        .collect::<String>()
        .if_empty_then("postgres_sync")
}

trait StringFallback {
    fn if_empty_then(self, fallback: &str) -> String;
}

impl StringFallback for String {
    fn if_empty_then(self, fallback: &str) -> String {
        if self.is_empty() {
            fallback.to_string()
        } else {
            self
        }
    }
}
