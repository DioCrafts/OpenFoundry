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
        kind: crate::models::report::GeneratorKind::Pdf,
        display_name: "PDF".to_string(),
        engine: "typst".to_string(),
        extensions: vec!["pdf".to_string()],
        capabilities: vec![
            "pixel-perfect layout".to_string(),
            "page headers".to_string(),
        ],
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
        "pdf",
        "application/pdf",
        "typst",
    )
}
