//! PostgreSQL connector — connects to an external Postgres database
//! and reads tables/views for sync.

use serde_json::Value;

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
