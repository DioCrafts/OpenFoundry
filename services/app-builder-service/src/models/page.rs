use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::widget::WidgetDefinition;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PageLayout {
    #[serde(default = "default_layout_kind")]
    pub kind: String,
    #[serde(default = "default_layout_columns")]
    pub columns: u8,
    #[serde(default = "default_layout_gap")]
    pub gap: String,
    #[serde(default = "default_layout_max_width")]
    pub max_width: String,
}

impl Default for PageLayout {
    fn default() -> Self {
        Self {
            kind: default_layout_kind(),
            columns: default_layout_columns(),
            gap: default_layout_gap(),
            max_width: default_layout_max_width(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppPage {
    #[serde(default = "default_page_id")]
    pub id: String,
    #[serde(default = "default_page_name")]
    pub name: String,
    #[serde(default = "default_page_path")]
    pub path: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub layout: PageLayout,
    #[serde(default)]
    pub widgets: Vec<WidgetDefinition>,
    #[serde(default = "default_visible")]
    pub visible: bool,
}

impl Default for AppPage {
    fn default() -> Self {
        Self {
            id: default_page_id(),
            name: default_page_name(),
            path: default_page_path(),
            description: String::new(),
            layout: PageLayout::default(),
            widgets: Vec::new(),
            visible: default_visible(),
        }
    }
}

impl AppPage {
    pub fn widget_count(&self) -> usize {
        self.widgets
            .iter()
            .map(WidgetDefinition::count_recursive)
            .sum()
    }
}

fn default_page_id() -> String {
    Uuid::now_v7().to_string()
}

fn default_page_name() -> String {
    "Overview".to_string()
}

fn default_page_path() -> String {
    "/".to_string()
}

fn default_layout_kind() -> String {
    "grid".to_string()
}

fn default_layout_columns() -> u8 {
    12
}

fn default_layout_gap() -> String {
    "1.25rem".to_string()
}

fn default_layout_max_width() -> String {
    "1440px".to_string()
}

fn default_visible() -> bool {
    true
}
