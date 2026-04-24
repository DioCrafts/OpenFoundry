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
        kind: crate::models::report::GeneratorKind::Excel,
        display_name: "Excel workbook".to_string(),
        engine: "rust_xlsxwriter".to_string(),
        extensions: vec!["xlsx".to_string()],
        capabilities: vec!["worksheet tabs".to_string(), "cell formatting".to_string()],
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
        "xlsx",
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "rust_xlsxwriter",
    )
}
