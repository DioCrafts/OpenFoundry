use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::models::{
    decode_json,
    recipient::{DistributionChannelCatalogEntry, DistributionResult},
    report::GeneratorKind,
    schedule::ScheduledRun,
    template::SectionKind,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportPreviewHighlight {
    pub label: String,
    pub value: String,
    pub delta: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportPreviewSection {
    pub section_id: String,
    pub title: String,
    pub kind: SectionKind,
    pub summary: String,
    pub rows: Vec<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportExecutionPreview {
    pub headline: String,
    pub generated_for: String,
    pub engine: String,
    pub highlights: Vec<ReportPreviewHighlight>,
    pub sections: Vec<ReportPreviewSection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportArtifact {
    pub file_name: String,
    pub mime_type: String,
    pub size_bytes: i64,
    pub storage_url: String,
    pub checksum: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportExecutionMetrics {
    pub duration_ms: i32,
    pub row_count: i64,
    pub section_count: usize,
    pub recipient_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportExecution {
    pub id: Uuid,
    pub report_id: Uuid,
    pub report_name: String,
    pub status: String,
    pub generator_kind: GeneratorKind,
    pub triggered_by: String,
    pub generated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub preview: ReportExecutionPreview,
    pub artifact: ReportArtifact,
    pub distributions: Vec<DistributionResult>,
    pub metrics: ReportExecutionMetrics,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ReportExecutionRow {
    pub id: Uuid,
    pub report_id: Uuid,
    pub report_name: String,
    pub status: String,
    pub generator_kind: String,
    pub triggered_by: String,
    pub generated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub preview: Value,
    pub artifact: Value,
    pub distributions: Value,
    pub metrics: Value,
}

impl TryFrom<ReportExecutionRow> for ReportExecution {
    type Error = String;

    fn try_from(row: ReportExecutionRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            report_id: row.report_id,
            report_name: row.report_name,
            status: row.status,
            generator_kind: row.generator_kind.parse()?,
            triggered_by: row.triggered_by,
            generated_at: row.generated_at,
            completed_at: row.completed_at,
            preview: decode_json(row.preview, "preview")?,
            artifact: decode_json(row.artifact, "artifact")?,
            distributions: decode_json(row.distributions, "distributions")?,
            metrics: decode_json(row.metrics, "metrics")?,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportOverview {
    pub report_count: usize,
    pub active_schedules: usize,
    pub executions_24h: usize,
    pub generator_mix: Vec<String>,
    pub latest_execution: Option<ReportExecution>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratorCatalogEntry {
    pub kind: GeneratorKind,
    pub display_name: String,
    pub engine: String,
    pub extensions: Vec<String>,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportCatalog {
    pub generators: Vec<GeneratorCatalogEntry>,
    pub delivery_channels: Vec<DistributionChannelCatalogEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleBoard {
    pub active_schedules: usize,
    pub paused_reports: usize,
    pub upcoming: Vec<ScheduledRun>,
    pub recent_executions: Vec<ReportExecution>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadPayload {
    pub file_name: String,
    pub mime_type: String,
    pub storage_url: String,
    pub preview_excerpt: String,
    pub report_name: String,
}
