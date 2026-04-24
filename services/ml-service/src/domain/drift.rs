use chrono::Utc;

use crate::models::deployment::{DriftMetric, DriftReport, GenerateDriftReportRequest};

fn metric_status(score: f64, threshold: f64) -> String {
    if score >= threshold {
        "alert".to_string()
    } else if score >= threshold * 0.7 {
        "warning".to_string()
    } else {
        "healthy".to_string()
    }
}

fn round_score(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

pub fn generate_report(request: &GenerateDriftReportRequest, variant_count: usize) -> DriftReport {
    let baseline = request.baseline_rows.unwrap_or(10_000).max(1) as f64;
    let observed = request
        .observed_rows
        .unwrap_or((baseline * 1.12) as i64)
        .max(1) as f64;
    let volume_shift = ((observed - baseline).abs() / baseline).min(1.5);

    let dataset_score = round_score((0.12 + volume_shift + variant_count as f64 * 0.04).min(1.5));
    let concept_score =
        round_score((0.09 + volume_shift * 0.7 + variant_count as f64 * 0.03).min(1.5));
    let recommend_retraining = dataset_score >= 0.25 || concept_score >= 0.18;

    DriftReport {
        generated_at: Utc::now(),
        dataset_metrics: vec![DriftMetric {
            name: "psi".to_string(),
            score: dataset_score,
            threshold: 0.25,
            status: metric_status(dataset_score, 0.25),
        }],
        concept_metrics: vec![DriftMetric {
            name: "prediction_target_gap".to_string(),
            score: concept_score,
            threshold: 0.18,
            status: metric_status(concept_score, 0.18),
        }],
        recommend_retraining,
        auto_retraining_job_id: None,
        notes: if recommend_retraining {
            "Observed drift exceeded the configured threshold; retraining is recommended."
                .to_string()
        } else {
            "Observed drift remains within the configured guardrails.".to_string()
        },
    }
}
