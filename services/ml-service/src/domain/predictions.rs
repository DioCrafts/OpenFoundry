use serde_json::Value;

use crate::models::{
    deployment::TrafficSplitEntry,
    prediction::{FeatureContribution, PredictionOutput},
};

#[derive(Debug, Clone)]
pub struct ModelRuntime {
    pub version_number: i32,
    pub schema: Value,
}

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
    runtime: &ModelRuntime,
    explain: bool,
    ordinal: usize,
) -> PredictionOutput {
    if let Some(output) = predict_with_model_state(input, split, runtime, explain, ordinal) {
        return output;
    }

    fallback_predict(input, split, runtime.version_number, explain, ordinal)
}

fn predict_with_model_state(
    input: &Value,
    split: &TrafficSplitEntry,
    runtime: &ModelRuntime,
    explain: bool,
    ordinal: usize,
) -> Option<PredictionOutput> {
    let model_state = runtime.schema.get("model_state")?;
    let feature_names = model_state
        .get("feature_names")?
        .as_array()?
        .iter()
        .filter_map(Value::as_str)
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    let feature_means = model_state
        .get("feature_means")?
        .as_array()?
        .iter()
        .filter_map(Value::as_f64)
        .collect::<Vec<_>>();
    let feature_scales = model_state
        .get("feature_scales")?
        .as_array()?
        .iter()
        .filter_map(Value::as_f64)
        .collect::<Vec<_>>();
    let weights = model_state
        .get("weights")?
        .as_array()?
        .iter()
        .filter_map(Value::as_f64)
        .collect::<Vec<_>>();

    if feature_names.is_empty()
        || feature_names.len() != weights.len()
        || feature_names.len() != feature_means.len()
        || feature_names.len() != feature_scales.len()
    {
        return None;
    }

    let bias = model_state
        .get("bias")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let threshold = model_state
        .get("threshold")
        .and_then(Value::as_f64)
        .unwrap_or(0.5);
    let positive_label = model_state
        .get("positive_label")
        .and_then(Value::as_str)
        .unwrap_or("positive");
    let negative_label = model_state
        .get("negative_label")
        .and_then(Value::as_str)
        .unwrap_or("negative");

    let object = input.as_object()?;
    let mut standardized = Vec::with_capacity(feature_names.len());
    let mut contributions = Vec::new();
    for (index, feature_name) in feature_names.iter().enumerate() {
        let raw = scalar_score(object.get(feature_name)?).unwrap_or(0.0);
        let scale = if feature_scales[index] == 0.0 {
            1.0
        } else {
            feature_scales[index]
        };
        let standardized_value = (raw - feature_means[index]) / scale;
        standardized.push(standardized_value);
        if explain {
            contributions.push(FeatureContribution {
                name: feature_name.clone(),
                value: round_score((weights[index] * standardized_value).abs()),
            });
        }
    }

    contributions.sort_by(|left, right| right.value.total_cmp(&left.value));
    contributions.truncate(3);

    let raw_signal = weights
        .iter()
        .zip(standardized.iter())
        .map(|(weight, value)| weight * value)
        .sum::<f64>()
        + bias;
    let score = round_score(sigmoid(raw_signal).clamp(0.001, 0.999));
    let confidence = round_score((0.5 + (score - threshold).abs()).clamp(0.5, 0.99));

    Some(PredictionOutput {
        record_id: format!("record-{}", ordinal + 1),
        variant: split.label.clone(),
        model_version_id: split.model_version_id,
        predicted_label: if score >= threshold {
            positive_label.to_string()
        } else {
            negative_label.to_string()
        },
        score,
        confidence,
        contributions,
    })
}

fn fallback_predict(
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

fn sigmoid(value: f64) -> f64 {
    1.0 / (1.0 + (-value).exp())
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use uuid::Uuid;

    use crate::models::deployment::TrafficSplitEntry;

    use super::{ModelRuntime, predict_record};

    #[test]
    fn predicts_from_real_model_state_when_available() {
        let split = TrafficSplitEntry {
            model_version_id: Uuid::now_v7(),
            label: "champion".to_string(),
            allocation: 100,
        };
        let runtime = ModelRuntime {
            version_number: 3,
            schema: json!({
                "model_state": {
                    "feature_names": ["tickets_open", "usage_delta"],
                    "feature_means": [4.0, -0.2],
                    "feature_scales": [2.0, 0.2],
                    "weights": [1.8, -1.4],
                    "bias": 0.2,
                    "threshold": 0.5,
                    "positive_label": "positive",
                    "negative_label": "negative"
                }
            }),
        };

        let output = predict_record(
            &json!({ "tickets_open": 9, "usage_delta": -0.9 }),
            &split,
            &runtime,
            true,
            0,
        );

        assert_eq!(output.predicted_label, "positive");
        assert!(output.score >= 0.5);
        assert!(!output.contributions.is_empty());
    }
}
