use std::collections::BTreeMap;

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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GraphSummary {
    pub scope: String,
    pub node_kinds: BTreeMap<String, usize>,
    pub edge_kinds: BTreeMap<String, usize>,
    pub object_types: BTreeMap<String, usize>,
    pub markings: BTreeMap<String, usize>,
    pub root_neighbor_count: usize,
    pub max_hops_reached: usize,
    pub boundary_crossings: usize,
    pub sensitive_objects: usize,
    pub sensitive_markings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GraphResponse {
    pub mode: String,
    pub root_object_id: Option<Uuid>,
    pub root_type_id: Option<Uuid>,
    pub depth: usize,
    pub total_nodes: usize,
    pub total_edges: usize,
    pub summary: GraphSummary,
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}
