use serde_json::Value;

const VALID_TYPES: &[&str] = &[
    "string",
    "integer",
    "float",
    "boolean",
    "date",
    "timestamp",
    "json",
    "array",
    "reference",
];

pub fn validate_property_type(property_type: &str) -> Result<(), String> {
    if VALID_TYPES.contains(&property_type) {
        Ok(())
    } else {
        Err(format!(
            "invalid property type '{property_type}', valid types: {VALID_TYPES:?}"
        ))
    }
}

pub fn validate_property_value(property_type: &str, value: &Value) -> Result<(), String> {
    match property_type {
        "string" => {
            if value.is_string() {
                Ok(())
            } else {
                Err("expected string value".into())
            }
        }
        "integer" => {
            if value.is_i64() || value.is_u64() {
                Ok(())
            } else {
                Err("expected integer value".into())
            }
        }
        "float" => {
            if value.is_f64() || value.is_i64() {
                Ok(())
            } else {
                Err("expected numeric value".into())
            }
        }
        "boolean" => {
            if value.is_boolean() {
                Ok(())
            } else {
                Err("expected boolean value".into())
            }
        }
        "json" | "array" => Ok(()),
        "date" | "timestamp" => {
            if value.is_string() {
                Ok(())
            } else {
                Err("expected string date value".into())
            }
        }
        "reference" => {
            if value.is_string() {
                Ok(())
            } else {
                Err("expected UUID string for reference".into())
            }
        }
        _ => Err(format!("unknown type: {property_type}")),
    }
}

pub fn validate_cardinality(cardinality: &str) -> Result<(), String> {
    match cardinality {
        "one_to_one" | "one_to_many" | "many_to_one" | "many_to_many" => Ok(()),
        _ => Err(format!(
            "invalid cardinality '{cardinality}', valid: one_to_one, one_to_many, many_to_one, many_to_many"
        )),
    }
}
