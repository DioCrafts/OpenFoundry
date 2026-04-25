use serde_json::{Value, json};

use crate::models::{run::MetricValue, training_job::TrainingTrial};

use super::hyperparameter;

#[derive(Debug, Clone)]
pub struct TrialOutcome {
    pub trial: TrainingTrial,
    pub metrics: Vec<MetricValue>,
    pub schema: Value,
}

#[derive(Debug, Clone)]
struct TrainingDataset {
    feature_names: Vec<String>,
    feature_means: Vec<f64>,
    feature_scales: Vec<f64>,
    rows: Vec<Vec<f64>>,
    labels: Vec<f64>,
    label_field: String,
    positive_label: String,
    negative_label: String,
}

pub fn has_inline_training_data(training_config: &Value) -> bool {
    training_config
        .get("records")
        .and_then(Value::as_array)
        .map(|records| !records.is_empty())
        .unwrap_or(false)
}

pub fn train_trial(
    training_config: &Value,
    hyperparameters: &Value,
    objective_metric_name: &str,
    trial_index: usize,
) -> Result<TrialOutcome, String> {
    let dataset = parse_dataset(training_config)?;
    let learning_rate = hyperparameter::value_as_f64(hyperparameters.get("learning_rate"), 0.08);
    let epochs = hyperparameter::value_as_u64(hyperparameters.get("epochs"), 350) as usize;
    let l2 = hyperparameter::value_as_f64(hyperparameters.get("l2"), 0.0);

    let (weights, bias) =
        fit_logistic_regression(&dataset.rows, &dataset.labels, learning_rate, epochs, l2);
    let metrics = evaluate_metrics(&dataset, &weights, bias);
    let objective_metric =
        select_metric(&metrics, objective_metric_name).unwrap_or_else(|| metrics[0].clone());

    let schema = json!({
        "signature": "tabular-binary",
        "engine": training_config
            .get("engine")
            .and_then(Value::as_str)
            .unwrap_or("tabular-logistic"),
        "model_adapter": {
            "kind": "native",
            "framework": training_config
                .get("engine")
                .and_then(Value::as_str)
                .unwrap_or("tabular-logistic"),
            "runtime": "in-process",
        },
        "model_state": {
            "feature_names": dataset.feature_names,
            "feature_means": dataset.feature_means,
            "feature_scales": dataset.feature_scales,
            "weights": weights,
            "bias": bias,
            "threshold": 0.5,
            "label_field": dataset.label_field,
            "positive_label": dataset.positive_label,
            "negative_label": dataset.negative_label,
        },
        "training_summary": {
            "row_count": dataset.rows.len(),
            "feature_count": dataset.rows.first().map(|row| row.len()).unwrap_or(0),
            "objective_metric": objective_metric.name,
            "objective_value": objective_metric.value,
            "framework": training_config
                .get("engine")
                .and_then(Value::as_str)
                .unwrap_or("tabular-logistic"),
        }
    });

    Ok(TrialOutcome {
        trial: TrainingTrial {
            id: format!("trial-{}", trial_index + 1),
            status: "completed".to_string(),
            hyperparameters: hyperparameters.clone(),
            objective_metric,
        },
        metrics,
        schema,
    })
}

fn parse_dataset(training_config: &Value) -> Result<TrainingDataset, String> {
    let records = training_config
        .get("records")
        .and_then(Value::as_array)
        .ok_or_else(|| "training_config.records must be a non-empty array".to_string())?;
    if records.is_empty() {
        return Err("training_config.records must contain at least one row".to_string());
    }

    let label_field = training_config
        .get("label_field")
        .and_then(Value::as_str)
        .unwrap_or("label")
        .to_string();
    let positive_label = training_config
        .get("positive_label")
        .and_then(Value::as_str)
        .unwrap_or("positive")
        .to_string();
    let negative_label = training_config
        .get("negative_label")
        .and_then(Value::as_str)
        .unwrap_or("negative")
        .to_string();

    let feature_names = training_config
        .get("features")
        .and_then(Value::as_array)
        .map(|features| {
            features
                .iter()
                .filter_map(Value::as_str)
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>()
        })
        .filter(|features| !features.is_empty())
        .unwrap_or_else(|| derive_feature_names(records, &label_field));

    if feature_names.is_empty() {
        return Err("training_config.features resolved to an empty set".to_string());
    }

    let mut raw_rows = Vec::with_capacity(records.len());
    let mut labels = Vec::with_capacity(records.len());

    for record in records {
        let object = record
            .as_object()
            .ok_or_else(|| "each training record must be a JSON object".to_string())?;
        let label_value = object
            .get(&label_field)
            .ok_or_else(|| format!("missing label field '{label_field}'"))?;
        labels.push(binary_label(label_value, &positive_label));
        raw_rows.push(
            feature_names
                .iter()
                .map(|feature| scalar_feature(object.get(feature)))
                .collect::<Vec<_>>(),
        );
    }

    let (feature_means, feature_scales, rows) = standardize_rows(&raw_rows);

    Ok(TrainingDataset {
        feature_names,
        feature_means,
        feature_scales,
        rows,
        labels,
        label_field,
        positive_label,
        negative_label,
    })
}

fn derive_feature_names(records: &[Value], label_field: &str) -> Vec<String> {
    let mut names = records
        .iter()
        .filter_map(Value::as_object)
        .flat_map(|object| object.keys().cloned())
        .filter(|name| name != label_field)
        .collect::<Vec<_>>();
    names.sort();
    names.dedup();
    names
}

fn scalar_feature(value: Option<&Value>) -> f64 {
    match value {
        Some(Value::Number(number)) => number.as_f64().unwrap_or(0.0),
        Some(Value::Bool(flag)) => {
            if *flag {
                1.0
            } else {
                0.0
            }
        }
        Some(Value::String(text)) => text.parse::<f64>().unwrap_or_else(|_| {
            let hash = text
                .bytes()
                .fold(0u64, |acc, byte| acc.wrapping_add(byte as u64));
            (hash % 1000) as f64 / 1000.0
        }),
        _ => 0.0,
    }
}

fn binary_label(value: &Value, positive_label: &str) -> f64 {
    match value {
        Value::Bool(flag) => {
            if *flag {
                1.0
            } else {
                0.0
            }
        }
        Value::Number(number) => {
            if number.as_f64().unwrap_or(0.0) >= 0.5 {
                1.0
            } else {
                0.0
            }
        }
        Value::String(text) => {
            if text == positive_label || text.eq_ignore_ascii_case("true") || text == "1" {
                1.0
            } else {
                0.0
            }
        }
        _ => 0.0,
    }
}

fn standardize_rows(rows: &[Vec<f64>]) -> (Vec<f64>, Vec<f64>, Vec<Vec<f64>>) {
    let feature_count = rows.first().map(|row| row.len()).unwrap_or(0);
    let mut means = vec![0.0; feature_count];
    let mut scales = vec![1.0; feature_count];

    if rows.is_empty() {
        return (means, scales, Vec::new());
    }

    for row in rows {
        for (index, value) in row.iter().enumerate() {
            means[index] += *value;
        }
    }
    for mean in &mut means {
        *mean /= rows.len() as f64;
    }

    for row in rows {
        for (index, value) in row.iter().enumerate() {
            let delta = *value - means[index];
            scales[index] += delta * delta;
        }
    }
    for scale in &mut scales {
        *scale = (*scale / rows.len() as f64).sqrt();
        if *scale == 0.0 {
            *scale = 1.0;
        }
    }

    let standardized = rows
        .iter()
        .map(|row| {
            row.iter()
                .enumerate()
                .map(|(index, value)| (*value - means[index]) / scales[index])
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    (means, scales, standardized)
}

fn fit_logistic_regression(
    rows: &[Vec<f64>],
    labels: &[f64],
    learning_rate: f64,
    epochs: usize,
    l2: f64,
) -> (Vec<f64>, f64) {
    let feature_count = rows.first().map(|row| row.len()).unwrap_or(0);
    let mut weights = vec![0.0; feature_count];
    let mut bias = 0.0;

    if rows.is_empty() {
        return (weights, bias);
    }

    for _ in 0..epochs {
        let mut gradient = vec![0.0; feature_count];
        let mut bias_gradient = 0.0;

        for (row, label) in rows.iter().zip(labels.iter()) {
            let prediction = sigmoid(dot(&weights, row) + bias);
            let error = prediction - *label;

            for (index, value) in row.iter().enumerate() {
                gradient[index] += error * value;
            }
            bias_gradient += error;
        }

        let row_count = rows.len() as f64;
        for (index, weight) in weights.iter_mut().enumerate() {
            let regularized = gradient[index] / row_count + l2 * *weight;
            *weight -= learning_rate * regularized;
        }
        bias -= learning_rate * (bias_gradient / row_count);
    }

    (weights, bias)
}

fn evaluate_metrics(dataset: &TrainingDataset, weights: &[f64], bias: f64) -> Vec<MetricValue> {
    let mut true_positive = 0.0_f64;
    let mut true_negative = 0.0_f64;
    let mut false_positive = 0.0_f64;
    let mut false_negative = 0.0_f64;
    let mut log_loss = 0.0_f64;

    for (row, label) in dataset.rows.iter().zip(dataset.labels.iter()) {
        let probability = sigmoid(dot(weights, row) + bias).clamp(1e-6, 1.0 - 1e-6);
        let predicted = if probability >= 0.5 { 1.0 } else { 0.0 };
        log_loss += -(*label * probability.ln() + (1.0 - *label) * (1.0 - probability).ln());

        match (predicted, *label) {
            (1.0, 1.0) => true_positive += 1.0,
            (0.0, 0.0) => true_negative += 1.0,
            (1.0, 0.0) => false_positive += 1.0,
            (0.0, 1.0) => false_negative += 1.0,
            _ => {}
        }
    }

    let total = (true_positive + true_negative + false_positive + false_negative).max(1.0_f64);
    let accuracy = round_metric((true_positive + true_negative) / total);
    let precision = round_metric(true_positive / (true_positive + false_positive).max(1.0));
    let recall = round_metric(true_positive / (true_positive + false_negative).max(1.0));
    let f1 = round_metric(if precision + recall == 0.0 {
        0.0
    } else {
        2.0 * precision * recall / (precision + recall)
    });

    vec![
        MetricValue {
            name: "accuracy".to_string(),
            value: accuracy,
        },
        MetricValue {
            name: "precision".to_string(),
            value: precision,
        },
        MetricValue {
            name: "recall".to_string(),
            value: recall,
        },
        MetricValue {
            name: "f1".to_string(),
            value: f1,
        },
        MetricValue {
            name: "log_loss".to_string(),
            value: round_metric(log_loss / total),
        },
    ]
}

fn select_metric(metrics: &[MetricValue], name: &str) -> Option<MetricValue> {
    metrics.iter().find(|metric| metric.name == name).cloned()
}

fn dot(left: &[f64], right: &[f64]) -> f64 {
    left.iter()
        .zip(right.iter())
        .map(|(l, r)| l * r)
        .sum::<f64>()
}

fn sigmoid(value: f64) -> f64 {
    1.0 / (1.0 + (-value).exp())
}

fn round_metric(value: f64) -> f64 {
    (value * 10_000.0).round() / 10_000.0
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{has_inline_training_data, train_trial};

    #[test]
    fn trains_real_logistic_trial_from_inline_records() {
        let config = json!({
            "engine": "tabular-logistic",
            "label_field": "label",
            "positive_label": "positive",
            "records": [
                { "label": "positive", "tickets_open": 9, "usage_delta": -0.8, "nps": 2 },
                { "label": "positive", "tickets_open": 7, "usage_delta": -0.6, "nps": 3 },
                { "label": "negative", "tickets_open": 1, "usage_delta": 0.2, "nps": 9 },
                { "label": "negative", "tickets_open": 2, "usage_delta": 0.4, "nps": 8 }
            ]
        });

        assert!(has_inline_training_data(&config));
        let outcome = train_trial(
            &config,
            &json!({ "learning_rate": 0.1, "epochs": 400, "l2": 0.0 }),
            "f1",
            0,
        )
        .unwrap();

        assert_eq!(outcome.trial.status, "completed");
        assert!(outcome.trial.objective_metric.value >= 0.8);
        assert!(outcome.schema.pointer("/model_state/weights").is_some());
    }
}
