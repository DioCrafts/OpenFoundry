use crate::models::quality::{DatasetColumnProfile, DatasetRuleResult};

pub fn compute_quality_score(
    row_count: i64,
    duplicate_rows: i64,
    columns: &[DatasetColumnProfile],
    rule_results: &[DatasetRuleResult],
) -> f64 {
    let completeness = if columns.is_empty() {
        1.0
    } else {
        columns
            .iter()
            .map(|column| 1.0 - column.null_rate)
            .sum::<f64>()
            / columns.len() as f64
    };

    let uniqueness = if columns.is_empty() {
        1.0
    } else {
        columns
            .iter()
            .map(|column| column.uniqueness_rate)
            .sum::<f64>()
            / columns.len() as f64
    };

    let duplicate_penalty = if row_count <= 0 {
        0.0
    } else {
        (duplicate_rows.max(0) as f64 / row_count as f64).min(1.0)
    };

    let failed_rules = rule_results.iter().filter(|result| !result.passed).count() as f64;
    let rule_penalty = if rule_results.is_empty() {
        0.0
    } else {
        failed_rules / rule_results.len() as f64
    };

    let weighted = (completeness * 0.45)
        + (uniqueness * 0.25)
        + ((1.0 - duplicate_penalty) * 0.15)
        + ((1.0 - rule_penalty) * 0.15);

    (weighted.clamp(0.0, 1.0) * 1000.0).round() / 10.0
}
