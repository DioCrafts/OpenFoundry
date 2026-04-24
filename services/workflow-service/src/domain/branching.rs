use serde_json::Value;

use crate::models::workflow::{WorkflowBranchCondition, WorkflowStep};

pub fn resolve_next_step(step: &WorkflowStep, context: &Value) -> Option<String> {
    for branch in &step.branches {
        if evaluate_condition(&branch.condition, context) {
            return Some(branch.next_step_id.clone());
        }
    }

    step.next_step_id.clone()
}

pub fn evaluate_condition(condition: &WorkflowBranchCondition, context: &Value) -> bool {
    let Some(current) = value_for_path(context, &condition.field) else {
        return false;
    };

    match condition.operator.as_str() {
        "eq" => current == &condition.value,
        "ne" => current != &condition.value,
        "contains" => current
            .as_str()
            .zip(condition.value.as_str())
            .map(|(value, needle)| value.contains(needle))
            .unwrap_or(false),
        "gt" => number_value(current) > number_value(&condition.value),
        "gte" => number_value(current) >= number_value(&condition.value),
        "lt" => number_value(current) < number_value(&condition.value),
        "lte" => number_value(current) <= number_value(&condition.value),
        _ => false,
    }
}

fn value_for_path<'a>(context: &'a Value, path: &str) -> Option<&'a Value> {
    let mut current = context;
    for segment in path.split('.') {
        current = current.get(segment)?;
    }
    Some(current)
}

fn number_value(value: &Value) -> f64 {
    value
        .as_f64()
        .or_else(|| value.as_i64().map(|raw| raw as f64))
        .or_else(|| value.as_u64().map(|raw| raw as f64))
        .or_else(|| value.as_str().and_then(|raw| raw.parse::<f64>().ok()))
        .unwrap_or(0.0)
}
