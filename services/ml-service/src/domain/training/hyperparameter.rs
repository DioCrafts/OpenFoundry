use serde_json::{Value, json};

pub fn candidate_sets(search: Option<&Value>) -> Vec<Value> {
    search
        .and_then(|value| value.get("candidates"))
        .and_then(Value::as_array)
        .cloned()
        .filter(|candidates| !candidates.is_empty())
        .unwrap_or_else(|| {
            vec![
                json!({ "learning_rate": 0.05, "epochs": 250, "l2": 0.0 }),
                json!({ "learning_rate": 0.08, "epochs": 350, "l2": 0.001 }),
                json!({ "learning_rate": 0.12, "epochs": 500, "l2": 0.01 }),
            ]
        })
}

pub fn value_as_f64(value: Option<&Value>, fallback: f64) -> f64 {
    value.and_then(Value::as_f64).unwrap_or(fallback)
}

pub fn value_as_u64(value: Option<&Value>, fallback: u64) -> u64 {
    value.and_then(Value::as_u64).unwrap_or(fallback)
}
