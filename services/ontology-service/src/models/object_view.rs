use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::models::{
    action_type::ActionType,
    graph::GraphResponse,
    rule::{OntologyRuleRun, RuleMatchResponse},
};

#[derive(Debug, Serialize)]
pub struct ObjectViewResponse {
    pub object: Value,
    pub summary: Value,
    pub neighbors: Vec<Value>,
    pub graph: GraphResponse,
    pub applicable_actions: Vec<ActionType>,
    pub matching_rules: Vec<RuleMatchResponse>,
    pub recent_rule_runs: Vec<OntologyRuleRun>,
    pub timeline: Vec<Value>,
}

#[derive(Debug, Deserialize)]
pub struct ObjectSimulationRequest {
    pub action_id: Option<Uuid>,
    #[serde(default)]
    pub action_parameters: Value,
    #[serde(default)]
    pub properties_patch: Value,
    pub depth: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct ObjectSimulationImpactSummary {
    pub scope: String,
    pub action_kind: String,
    pub predicted_delete: bool,
    pub impacted_object_count: usize,
    pub impacted_type_count: usize,
    pub impacted_types: Vec<String>,
    pub direct_neighbors: usize,
    pub max_hops_reached: usize,
    pub boundary_crossings: usize,
    pub sensitive_objects: usize,
    pub sensitive_markings: Vec<String>,
    pub matching_rules: usize,
    pub changed_properties: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ObjectSimulationResponse {
    pub before: Value,
    pub after: Option<Value>,
    pub deleted: bool,
    pub action_preview: Value,
    pub matching_rules: Vec<RuleMatchResponse>,
    pub graph: GraphResponse,
    pub impact_summary: ObjectSimulationImpactSummary,
    pub impacted_objects: Vec<Uuid>,
    pub timeline: Vec<Value>,
}
