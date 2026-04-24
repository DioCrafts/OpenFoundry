use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WidgetBinding {
    #[serde(default = "default_source_type")]
    pub source_type: String,
    #[serde(default)]
    pub source_id: Option<String>,
    #[serde(default)]
    pub query_text: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub fields: Vec<String>,
    #[serde(default = "default_parameters")]
    pub parameters: Value,
    #[serde(default)]
    pub limit: Option<u32>,
}

fn default_source_type() -> String {
    "dataset".to_string()
}

fn default_parameters() -> Value {
    Value::Object(Map::new())
}
