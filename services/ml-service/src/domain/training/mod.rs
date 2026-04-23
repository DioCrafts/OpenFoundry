use serde_json::{json, Value};

use crate::models::{run::MetricValue, training_job::TrainingTrial};

fn round_score(value: f64) -> f64 {
	(value * 100.0).round() / 100.0
}

fn value_as_f64(value: Option<&Value>, fallback: f64) -> f64 {
	value.and_then(Value::as_f64).unwrap_or(fallback)
}

pub fn generate_trials(search: Option<&Value>, objective_metric_name: &str) -> Vec<TrainingTrial> {
	let candidates = search
		.and_then(|value| value.get("candidates"))
		.and_then(Value::as_array)
		.cloned()
		.unwrap_or_else(|| {
			vec![
				json!({ "learning_rate": 0.01, "max_depth": 4, "subsample": 0.8 }),
				json!({ "learning_rate": 0.05, "max_depth": 6, "subsample": 0.9 }),
				json!({ "learning_rate": 0.08, "max_depth": 8, "subsample": 1.0 }),
			]
		});

	candidates
		.into_iter()
		.enumerate()
		.map(|(index, hyperparameters)| {
			let learning_rate = value_as_f64(hyperparameters.get("learning_rate"), 0.05);
			let max_depth = value_as_f64(hyperparameters.get("max_depth"), 6.0);
			let subsample = value_as_f64(hyperparameters.get("subsample"), 0.9);
			let score = round_score(
				(0.68
					+ index as f64 * 0.04
					+ learning_rate * 0.35
					+ max_depth * 0.007
					+ subsample * 0.05)
					.min(0.99),
			);

			TrainingTrial {
				id: format!("trial-{}", index + 1),
				status: "completed".to_string(),
				hyperparameters,
				objective_metric: MetricValue {
					name: objective_metric_name.to_string(),
					value: score,
				},
			}
		})
		.collect()
}

pub fn best_trial(trials: &[TrainingTrial]) -> Option<TrainingTrial> {
	trials
		.iter()
		.cloned()
		.max_by(|left, right| left.objective_metric.value.total_cmp(&right.objective_metric.value))
}
