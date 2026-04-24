use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use uuid::Uuid;

use crate::models::{binding::WidgetBinding, event::WidgetEvent};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WidgetPosition {
    #[serde(default)]
    pub x: i32,
    #[serde(default)]
    pub y: i32,
    #[serde(default = "default_widget_width")]
    pub width: i32,
    #[serde(default = "default_widget_height")]
    pub height: i32,
}

impl Default for WidgetPosition {
    fn default() -> Self {
        Self {
            x: 0,
            y: 0,
            width: default_widget_width(),
            height: default_widget_height(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WidgetDefinition {
    #[serde(default = "default_widget_id")]
    pub id: String,
    #[serde(default = "default_widget_type")]
    pub widget_type: String,
    #[serde(default = "default_widget_title")]
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub position: WidgetPosition,
    #[serde(default = "default_widget_props")]
    pub props: Value,
    #[serde(default)]
    pub binding: Option<WidgetBinding>,
    #[serde(default)]
    pub events: Vec<WidgetEvent>,
    #[serde(default)]
    pub children: Vec<WidgetDefinition>,
}

impl WidgetDefinition {
    pub fn count_recursive(&self) -> usize {
        1 + self
            .children
            .iter()
            .map(WidgetDefinition::count_recursive)
            .sum::<usize>()
    }
}

fn default_widget_id() -> String {
    Uuid::now_v7().to_string()
}

fn default_widget_type() -> String {
    "text".to_string()
}

fn default_widget_title() -> String {
    "Untitled widget".to_string()
}

fn default_widget_width() -> i32 {
    4
}

fn default_widget_height() -> i32 {
    3
}

fn default_widget_props() -> Value {
    Value::Object(Map::new())
}
