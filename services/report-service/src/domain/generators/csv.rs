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
        kind: crate::models::report::GeneratorKind::Csv,
        display_name: "CSV export".to_string(),
        engine: "csv".to_string(),
        extensions: vec!["csv".to_string()],
        capabilities: vec![
            "flat export".to_string(),
            "downstream ingestion".to_string(),
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
        "csv",
        "text/csv",
        "csv",
    )
}
