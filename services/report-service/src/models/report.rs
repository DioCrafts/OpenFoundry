use std::{
    fmt::{Display, Formatter},
    str::FromStr,
};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::models::{
    decode_json, recipient::DistributionRecipient, schedule::ReportSchedule,
    template::ReportTemplate,
};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GeneratorKind {
    Pdf,
    Excel,
    Csv,
    Html,
    Pptx,
}

impl GeneratorKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pdf => "pdf",
            Self::Excel => "excel",
            Self::Csv => "csv",
            Self::Html => "html",
            Self::Pptx => "pptx",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Pdf => "PDF",
            Self::Excel => "Excel",
            Self::Csv => "CSV",
            Self::Html => "HTML",
            Self::Pptx => "PPTX",
        }
    }
}

impl Display for GeneratorKind {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for GeneratorKind {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "pdf" => Ok(Self::Pdf),
            "excel" => Ok(Self::Excel),
            "csv" => Ok(Self::Csv),
            "html" => Ok(Self::Html),
            "pptx" => Ok(Self::Pptx),
            _ => Err(format!("unsupported generator kind: {value}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportDefinition {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub owner: String,
    pub generator_kind: GeneratorKind,
    pub dataset_name: String,
    pub template: ReportTemplate,
    pub schedule: ReportSchedule,
    pub recipients: Vec<DistributionRecipient>,
    pub tags: Vec<String>,
    pub parameters: Value,
    pub active: bool,
    pub last_generated_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateReportRequest {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub owner: String,
    pub generator_kind: GeneratorKind,
    pub dataset_name: String,
    pub template: ReportTemplate,
    #[serde(default)]
    pub schedule: ReportSchedule,
    #[serde(default)]
    pub recipients: Vec<DistributionRecipient>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub parameters: Value,
    #[serde(default = "default_active")]
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateReportRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub owner: Option<String>,
    pub generator_kind: Option<GeneratorKind>,
    pub dataset_name: Option<String>,
    pub template: Option<ReportTemplate>,
    pub schedule: Option<ReportSchedule>,
    pub recipients: Option<Vec<DistributionRecipient>>,
    pub tags: Option<Vec<String>>,
    pub parameters: Option<Value>,
    pub active: Option<bool>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ReportRow {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub owner: String,
    pub generator_kind: String,
    pub dataset_name: String,
    pub template: Value,
    pub schedule: Value,
    pub recipients: Value,
    pub tags: Value,
    pub parameters: Value,
    pub active: bool,
    pub last_generated_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TryFrom<ReportRow> for ReportDefinition {
    type Error = String;

    fn try_from(row: ReportRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            name: row.name,
            description: row.description,
            owner: row.owner,
            generator_kind: GeneratorKind::from_str(&row.generator_kind)?,
            dataset_name: row.dataset_name,
            template: decode_json(row.template, "template")?,
            schedule: decode_json(row.schedule, "schedule")?,
            recipients: decode_json(row.recipients, "recipients")?,
            tags: decode_json(row.tags, "tags")?,
            parameters: row.parameters,
            active: row.active,
            last_generated_at: row.last_generated_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

fn default_active() -> bool {
    true
}
