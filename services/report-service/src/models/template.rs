use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SectionKind {
    Kpi,
    Table,
    Chart,
    Narrative,
    Map,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportSection {
    pub id: String,
    pub title: String,
    pub kind: SectionKind,
    #[serde(default)]
    pub query: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub config: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportTemplate {
    pub title: String,
    pub subtitle: String,
    pub theme: String,
    pub layout: String,
    pub sections: Vec<ReportSection>,
}
