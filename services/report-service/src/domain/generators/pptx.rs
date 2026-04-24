use chrono::{DateTime, Utc};

use crate::{
    domain::{
        data_fetcher::ReportDataSnapshot,
        generators::{GeneratedBundle, build_bundle},
    },
    models::{report::ReportDefinition, snapshot::GeneratorCatalogEntry},
};

pub fn catalog_entry() -> GeneratorCatalogEntry {
    GeneratorCatalogEntry {
        kind: crate::models::report::GeneratorKind::Pptx,
        display_name: "PowerPoint deck".to_string(),
        engine: "pptx composer".to_string(),
        extensions: vec!["pptx".to_string()],
        capabilities: vec!["slide deck".to_string(), "speaker notes".to_string()],
    }
}

pub fn generate(
    report: &ReportDefinition,
    snapshot: &ReportDataSnapshot,
    execution_id: uuid::Uuid,
    generated_at: DateTime<Utc>,
) -> GeneratedBundle {
    build_bundle(
        report,
        snapshot,
        execution_id,
        generated_at,
        "pptx",
        "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        "pptx composer",
    )
}
