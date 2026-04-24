use serde_json::json;

use crate::models::quality::DatasetRuleResult;

#[derive(Debug, Clone)]
pub struct NewQualityAlert {
    pub level: String,
    pub kind: String,
    pub message: String,
    pub details: serde_json::Value,
}

pub fn build_quality_alerts(
    previous_score: Option<f64>,
    current_score: f64,
    rule_results: &[DatasetRuleResult],
) -> Vec<NewQualityAlert> {
    let mut alerts = Vec::new();

    if let Some(previous) = previous_score {
        let delta = current_score - previous;
        if delta <= -10.0 {
            alerts.push(NewQualityAlert {
                level: "high".to_string(),
                kind: "score_drop".to_string(),
                message: format!(
                    "Quality score dropped by {:.1} points since the previous run.",
                    delta.abs()
                ),
                details: json!({
                    "previous_score": previous,
                    "current_score": current_score,
                    "delta": delta,
                }),
            });
        }
    }

    for result in rule_results.iter().filter(|result| !result.passed) {
        alerts.push(NewQualityAlert {
            level: result.severity.clone(),
            kind: "rule_failed".to_string(),
            message: format!("Rule '{}' failed: {}", result.name, result.message),
            details: json!({
                "rule_id": result.rule_id,
                "rule_type": result.rule_type,
                "measured_value": result.measured_value,
            }),
        });
    }

    alerts
}
