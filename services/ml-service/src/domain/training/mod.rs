pub mod hyperparameter;
pub mod runner;

use serde_json::Value;

use crate::models::{run::MetricValue, training_job::TrainingTrial};

#[derive(Debug, Clone)]
pub struct TrainingExecution {
    pub trials: Vec<TrainingTrial>,
    pub best_hyperparameters: Option<Value>,
    pub best_metrics: Vec<MetricValue>,
    pub best_schema: Option<Value>,
}

pub fn execute_training(
    training_config: &Value,
    search: Option<&Value>,
    objective_metric_name: &str,
) -> Result<TrainingExecution, String> {
    if !runner::has_inline_training_data(training_config) {
        let trials = synthetic_trials(search, objective_metric_name);
        let best_hyperparameters = trials.first().map(|trial| trial.hyperparameters.clone());
        return Ok(TrainingExecution {
            trials,
            best_hyperparameters,
            best_metrics: Vec::new(),
            best_schema: None,
        });
    }

    let mut outcomes = hyperparameter::candidate_sets(search)
        .into_iter()
        .enumerate()
        .map(|(index, candidate)| {
            runner::train_trial(training_config, &candidate, objective_metric_name, index)
        })
        .collect::<Result<Vec<_>, _>>()?;
    outcomes.sort_by(|left, right| {
        right
            .trial
            .objective_metric
            .value
            .total_cmp(&left.trial.objective_metric.value)
    });

    let trials = outcomes.iter().map(|outcome| outcome.trial.clone()).collect();
    let best = outcomes.first();

    Ok(TrainingExecution {
        trials,
        best_hyperparameters: best.map(|outcome| outcome.trial.hyperparameters.clone()),
        best_metrics: best.map(|outcome| outcome.metrics.clone()).unwrap_or_default(),
        best_schema: best.map(|outcome| outcome.schema.clone()),
    })
}

fn synthetic_trials(search: Option<&Value>, objective_metric_name: &str) -> Vec<TrainingTrial> {
    hyperparameter::candidate_sets(search)
        .into_iter()
        .enumerate()
        .map(|(index, hyperparameters)| TrainingTrial {
            id: format!("trial-{}", index + 1),
            status: "completed".to_string(),
            hyperparameters,
            objective_metric: MetricValue {
                name: objective_metric_name.to_string(),
                value: 0.5 + index as f64 * 0.05,
            },
        })
        .collect()
}
