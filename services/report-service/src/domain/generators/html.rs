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
        kind: crate::models::report::GeneratorKind::Html,
        display_name: "HTML preview".to_string(),
        engine: "server-side renderer".to_string(),
        extensions: vec!["html".to_string()],
        capabilities: vec![
            "interactive preview".to_string(),
            "embedded charts".to_string(),
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
        "html",
        "text/html",
        "html renderer",
    )
}
