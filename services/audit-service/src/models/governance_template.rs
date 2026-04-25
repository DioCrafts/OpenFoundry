use serde::{Deserialize, Serialize};

use crate::models::data_classification::ClassificationLevel;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceTemplatePolicy {
    pub name: String,
    pub description: String,
    pub scope: String,
    pub classification: ClassificationLevel,
    pub retention_days: i32,
    pub legal_hold: bool,
    pub purge_mode: String,
    pub rules: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceTemplate {
    pub slug: String,
    pub name: String,
    pub summary: String,
    pub standards: Vec<String>,
    pub policies: Vec<GovernanceTemplatePolicy>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApplyGovernanceTemplateRequest {
    pub scope: Option<String>,
    pub updated_by: String,
}
