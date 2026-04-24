use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WidgetEvent {
    #[serde(default = "default_event_id")]
    pub id: String,
    #[serde(default = "default_trigger")]
    pub trigger: String,
    #[serde(default = "default_action")]
    pub action: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default = "default_event_config")]
    pub config: Value,
}

fn default_event_id() -> String {
    Uuid::now_v7().to_string()
}

fn default_trigger() -> String {
    "click".to_string()
}

fn default_action() -> String {
    "navigate".to_string()
}

fn default_event_config() -> Value {
    Value::Object(Map::new())
}
