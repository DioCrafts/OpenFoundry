//! Parquet reading/writing utilities — to be expanded with Arrow/DataFusion integration.
//!
//! For now, provides basic helpers for the dataset-service.

/// Infer schema from raw bytes (CSV detection by extension).
#[allow(dead_code)]
pub fn detect_format(filename: &str) -> &'static str {
    let ext = filename.rsplit('.').next().unwrap_or("").to_lowercase();
    match ext.as_str() {
        "parquet" | "pq" => "parquet",
        "csv" | "tsv" => "csv",
        "json" | "jsonl" | "ndjson" => "json",
        _ => "unknown",
    }
}
