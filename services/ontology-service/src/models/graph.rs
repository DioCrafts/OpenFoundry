use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize)]
pub struct GraphQuery {
    pub root_object_id: Option<Uuid>,
    pub root_type_id: Option<Uuid>,
    pub depth: Option<usize>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub kind: String,
    pub label: String,
    pub secondary_label: Option<String>,
    pub color: Option<String>,
    pub route: Option<String>,
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub id: String,
    pub kind: String,
    pub source: String,
    pub target: String,
    pub label: String,
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct GraphResponse {
    pub mode: String,
    pub root_object_id: Option<Uuid>,
    pub root_type_id: Option<Uuid>,
    pub depth: usize,
    pub total_nodes: usize,
    pub total_edges: usize,
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}
