//! JSON connector — reads JSON/NDJSON from URLs or storage paths.

use serde_json::Value;

pub fn validate_config(config: &Value) -> Result<(), String> {
    if config.get("url").is_none() && config.get("path").is_none() {
        return Err("json connector requires either 'url' or 'path'".to_string());
    }
    Ok(())
}
