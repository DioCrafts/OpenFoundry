use serde_json::Value;

use crate::models::{
    deployment::TrafficSplitEntry,
    prediction::{FeatureContribution, PredictionOutput},
};

fn round_score(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

fn scalar_score(value: &Value) -> Option<f64> {
    match value {
        Value::Number(number) => number.as_f64(),
        Value::String(text) => Some((text.len() as f64).min(100.0) / 100.0),
        Value::Bool(flag) => Some(if *flag { 0.65 } else { 0.35 }),
        _ => None,
    }
}

pub fn route_variant(splits: &[TrafficSplitEntry], ordinal: usize) -> Option<TrafficSplitEntry> {
    if splits.is_empty() {
        return None;
    }

    let bucket = ((ordinal as u64 * 37) % 100) as u8;
    let mut cumulative = 0u8;

    for split in splits {
        cumulative = cumulative.saturating_add(split.allocation);
        if bucket < cumulative {
            return Some(split.clone());
        }
    }

    splits.first().cloned()
}

pub fn predict_record(
    input: &Value,
    split: &TrafficSplitEntry,
    version_number: i32,
    explain: bool,
    ordinal: usize,
) -> PredictionOutput {
    let mut raw_signal = version_number as f64 * 0.08;
    let mut contributions = Vec::new();

    if let Some(object) = input.as_object() {
        for (key, value) in object {
            if let Some(score) = scalar_score(value) {
                raw_signal += score;
                if explain {
                    contributions.push(FeatureContribution {
                        name: key.clone(),
                        value: round_score(score),
                    });
                }
            }
        }
    } else if let Some(score) = scalar_score(input) {
        raw_signal += score;
        if explain {
            contributions.push(FeatureContribution {
                name: "input".to_string(),
                value: round_score(score),
            });
        }
    }

    if contributions.is_empty() && explain {
        contributions.push(FeatureContribution {
            name: "bias".to_string(),
            value: 0.42,
        });
    }

    contributions.sort_by(|left, right| right.value.total_cmp(&left.value));
    contributions.truncate(3);

    let score = round_score(((raw_signal.sin() + 1.0) / 2.0).clamp(0.02, 0.98));
    let confidence = round_score((0.58 + (score - 0.5).abs() * 0.8).clamp(0.51, 0.99));

    PredictionOutput {
        record_id: format!("record-{}", ordinal + 1),
        variant: split.label.clone(),
        model_version_id: split.model_version_id,
        predicted_label: if score >= 0.5 {
            "positive".to_string()
        } else {
            "negative".to_string()
        },
        score,
        confidence,
        contributions,
    }
}
