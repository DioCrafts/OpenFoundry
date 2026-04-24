//! Schema inference from raw data samples.

use serde_json::Value;

#[allow(dead_code)]
#[derive(Debug, Clone, serde::Serialize)]
pub struct InferredColumn {
    pub name: String,
    pub source_type: String,
    pub inferred_type: String,
    pub nullable: bool,
}

/// Infer columns from a JSON array sample.
#[allow(dead_code)]
pub fn infer_from_json_sample(sample: &[Value]) -> Vec<InferredColumn> {
    let Some(first) = sample.first() else {
        return vec![];
    };

    let Value::Object(obj) = first else {
        return vec![];
    };

    obj.iter()
        .map(|(key, val)| {
            let inferred_type = match val {
                Value::Bool(_) => "boolean",
                Value::Number(n) if n.is_f64() => "float64",
                Value::Number(_) => "int64",
                Value::String(_) => "string",
                Value::Array(_) => "list",
                Value::Object(_) => "struct",
                Value::Null => "string",
            };
            InferredColumn {
                name: key.clone(),
                source_type: "json".to_string(),
                inferred_type: inferred_type.to_string(),
                nullable: sample
                    .iter()
                    .any(|row| row.get(key).is_none_or(|v| v.is_null())),
            }
        })
        .collect()
}
