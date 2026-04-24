mod csv;
mod excel;
mod html;
mod pdf;
mod pptx;

use chrono::{DateTime, Utc};

use crate::{
    domain::data_fetcher::ReportDataSnapshot,
    models::{
        report::{GeneratorKind, ReportDefinition},
        snapshot::{
            GeneratorCatalogEntry, ReportArtifact, ReportExecutionMetrics, ReportExecutionPreview,
            ReportPreviewHighlight, ReportPreviewSection,
        },
    },
};

#[derive(Debug, Clone)]
pub struct GeneratedBundle {
    pub preview: ReportExecutionPreview,
    pub artifact: ReportArtifact,
    pub metrics: ReportExecutionMetrics,
}

pub fn catalog() -> Vec<GeneratorCatalogEntry> {
    vec![
        pdf::catalog_entry(),
        excel::catalog_entry(),
        csv::catalog_entry(),
        html::catalog_entry(),
        pptx::catalog_entry(),
    ]
}

pub fn generate(
    report: &ReportDefinition,
    snapshot: &ReportDataSnapshot,
    execution_id: uuid::Uuid,
    generated_at: DateTime<Utc>,
) -> GeneratedBundle {
    match report.generator_kind {
        GeneratorKind::Pdf => pdf::generate(report, snapshot, execution_id, generated_at),
        GeneratorKind::Excel => excel::generate(report, snapshot, execution_id, generated_at),
        GeneratorKind::Csv => csv::generate(report, snapshot, execution_id, generated_at),
        GeneratorKind::Html => html::generate(report, snapshot, execution_id, generated_at),
        GeneratorKind::Pptx => pptx::generate(report, snapshot, execution_id, generated_at),
    }
}

fn build_bundle(
    report: &ReportDefinition,
    snapshot: &ReportDataSnapshot,
    execution_id: uuid::Uuid,
    generated_at: DateTime<Utc>,
    extension: &str,
    mime_type: &str,
    engine: &str,
) -> GeneratedBundle {
    let section_rows = snapshot
        .sections
        .iter()
        .map(|section| ReportPreviewSection {
            section_id: section.section_id.clone(),
            title: section.title.clone(),
            kind: section.kind,
            summary: section.summary.clone(),
            rows: section.rows.iter().take(3).cloned().collect(),
        })
        .collect::<Vec<_>>();

    let preview = ReportExecutionPreview {
        headline: format!("{} generated for {}", report.name, snapshot.audience_label),
        generated_for: report.dataset_name.clone(),
        engine: engine.to_string(),
        highlights: snapshot
            .highlights
            .iter()
            .map(|highlight| ReportPreviewHighlight {
                label: highlight.label.clone(),
                value: highlight.value.clone(),
                delta: highlight.delta.clone(),
            })
            .collect(),
        sections: section_rows,
    };

    let row_count = snapshot
        .sections
        .iter()
        .map(|section| section.rows.len() as i64)
        .sum::<i64>();

    let artifact = ReportArtifact {
        file_name: format!(
            "{}-{}.{}",
            report.name.to_lowercase().replace(' ', "-"),
            generated_at.format("%Y%m%d%H%M"),
            extension
        ),
        mime_type: mime_type.to_string(),
        size_bytes: 140_000 + row_count * 160,
        storage_url: format!("/api/v1/reports/executions/{execution_id}/download"),
        checksum: format!("{execution_id}-{extension}"),
    };

    let metrics = ReportExecutionMetrics {
        duration_ms: 850 + (row_count as i32 * 12),
        row_count,
        section_count: snapshot.sections.len(),
        recipient_count: report.recipients.len(),
    };

    GeneratedBundle {
        preview,
        artifact,
        metrics,
    }
}
