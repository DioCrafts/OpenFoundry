//! REST API connector — reads data from HTTP endpoints.

use serde_json::Value;

pub fn validate_config(config: &Value) -> Result<(), String> {
    if config.get("base_url").is_none() {
        return Err("rest_api connector requires 'base_url'".to_string());
    }
    Ok(())
}
