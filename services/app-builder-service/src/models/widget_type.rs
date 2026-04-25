use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WidgetDefaultSize {
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WidgetCatalogItem {
    pub widget_type: String,
    pub label: String,
    pub description: String,
    pub category: String,
    pub default_props: Value,
    pub default_size: WidgetDefaultSize,
    pub supported_bindings: Vec<String>,
    pub supports_children: bool,
}

pub fn widget_catalog() -> Vec<WidgetCatalogItem> {
    vec![
        WidgetCatalogItem {
            widget_type: "table".to_string(),
            label: "Table".to_string(),
            description: "Paginated records from a dataset, ontology object list, or SQL query."
                .to_string(),
            category: "data".to_string(),
            default_props: json!({ "page_size": 10, "striped": true }),
            default_size: WidgetDefaultSize {
                width: 8,
                height: 5,
            },
            supported_bindings: vec![
                "dataset".to_string(),
                "query".to_string(),
                "ontology".to_string(),
            ],
            supports_children: false,
        },
        WidgetCatalogItem {
            widget_type: "form".to_string(),
            label: "Form".to_string(),
            description: "Editable form with submit button and event handlers.".to_string(),
            category: "input".to_string(),
            default_props: json!({
                "fields": [
                    { "name": "title", "label": "Title", "type": "text" },
                    { "name": "owner", "label": "Owner", "type": "text" }
                ],
                "submit_label": "Save"
            }),
            default_size: WidgetDefaultSize {
                width: 6,
                height: 5,
            },
            supported_bindings: vec![
                "dataset".to_string(),
                "ontology".to_string(),
                "static".to_string(),
            ],
            supports_children: false,
        },
        WidgetCatalogItem {
            widget_type: "chart".to_string(),
            label: "Chart".to_string(),
            description: "Line, bar, pie, or area chart backed by query or dataset bindings."
                .to_string(),
            category: "visualization".to_string(),
            default_props: json!({ "chart_type": "line", "x_field": "label", "y_field": "value" }),
            default_size: WidgetDefaultSize {
                width: 6,
                height: 4,
            },
            supported_bindings: vec!["dataset".to_string(), "query".to_string()],
            supports_children: false,
        },
        WidgetCatalogItem {
            widget_type: "map".to_string(),
            label: "Map".to_string(),
            description: "Marker or region map with geospatial bindings.".to_string(),
            category: "visualization".to_string(),
            default_props: json!({ "latitude_field": "lat", "longitude_field": "lon", "zoom": 3 }),
            default_size: WidgetDefaultSize {
                width: 6,
                height: 5,
            },
            supported_bindings: vec![
                "dataset".to_string(),
                "query".to_string(),
                "ontology".to_string(),
            ],
            supports_children: false,
        },
        WidgetCatalogItem {
            widget_type: "text".to_string(),
            label: "Text".to_string(),
            description: "Narrative copy, markdown, headings, and status labels.".to_string(),
            category: "content".to_string(),
            default_props: json!({ "content": "## Title\nAdd narrative context here.", "format": "markdown" }),
            default_size: WidgetDefaultSize {
                width: 4,
                height: 2,
            },
            supported_bindings: vec!["static".to_string(), "ontology".to_string()],
            supports_children: false,
        },
        WidgetCatalogItem {
            widget_type: "image".to_string(),
            label: "Image".to_string(),
            description: "Branding, illustrations, or dataset-driven image URLs.".to_string(),
            category: "content".to_string(),
            default_props: json!({ "url": "https://images.unsplash.com/photo-1516321497487-e288fb19713f?auto=format&fit=crop&w=1200&q=80", "alt": "OpenFoundry" }),
            default_size: WidgetDefaultSize {
                width: 4,
                height: 3,
            },
            supported_bindings: vec!["static".to_string(), "dataset".to_string()],
            supports_children: false,
        },
        WidgetCatalogItem {
            widget_type: "button".to_string(),
            label: "Button".to_string(),
            description:
                "Calls event handlers such as navigate, filter, open link, or execute query."
                    .to_string(),
            category: "actions".to_string(),
            default_props: json!({ "label": "Run action", "variant": "primary" }),
            default_size: WidgetDefaultSize {
                width: 3,
                height: 1,
            },
            supported_bindings: vec!["static".to_string(), "ontology".to_string()],
            supports_children: false,
        },
        WidgetCatalogItem {
            widget_type: "scenario".to_string(),
            label: "Scenario Lab".to_string(),
            description:
                "What-if controls that publish runtime parameters into the rest of the app."
                    .to_string(),
            category: "decision".to_string(),
            default_props: json!({
                "headline": "What-if controls",
                "parameters": [
                    {
                        "name": "demand_multiplier",
                        "label": "Demand multiplier",
                        "type": "number",
                        "default_value": "1.10",
                        "description": "Scale the primary operating assumption."
                    },
                    {
                        "name": "service_level",
                        "label": "Service level target",
                        "type": "number",
                        "default_value": "0.95",
                        "description": "Target fulfillment ratio for the plan."
                    }
                ],
                "apply_label": "Apply scenario",
                "reset_label": "Reset",
                "summary_template": "Scenario now set to {{demand_multiplier}} demand and {{service_level}} service level."
            }),
            default_size: WidgetDefaultSize {
                width: 6,
                height: 4,
            },
            supported_bindings: vec!["static".to_string()],
            supports_children: false,
        },
        WidgetCatalogItem {
            widget_type: "agent".to_string(),
            label: "Agent Widget".to_string(),
            description:
                "Embed an AI agent run surface with prompt, response, and tool trace output."
                    .to_string(),
            category: "ai".to_string(),
            default_props: json!({
                "agent_id": "",
                "placeholder": "Ask the embedded agent to summarize the current situation...",
                "empty_state": "Choose an active agent to turn this panel into an interactive copilot.",
                "welcome_message": "This widget can call a real OpenFoundry agent and bring the response back into the app.",
                "knowledge_base_id": "",
                "show_traces": true,
                "submit_label": "Run agent",
                "include_runtime_context": true,
                "runtime_context_intro": "Current Workshop scenario context:"
            }),
            default_size: WidgetDefaultSize {
                width: 6,
                height: 5,
            },
            supported_bindings: vec![],
            supports_children: false,
        },
        WidgetCatalogItem {
            widget_type: "container".to_string(),
            label: "Container".to_string(),
            description: "Layout wrapper for related widgets, sections, or cards.".to_string(),
            category: "layout".to_string(),
            default_props: json!({ "title": "Section", "variant": "card" }),
            default_size: WidgetDefaultSize {
                width: 12,
                height: 3,
            },
            supported_bindings: vec!["static".to_string()],
            supports_children: true,
        },
    ]
}
