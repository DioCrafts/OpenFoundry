//! CSV connector — reads CSV files from URLs or storage paths.

use serde_json::Value;

pub fn validate_config(config: &Value) -> Result<(), String> {
    if config.get("url").is_none() && config.get("path").is_none() {
        return Err("csv connector requires either 'url' or 'path'".to_string());
    }
    Ok(())
}
