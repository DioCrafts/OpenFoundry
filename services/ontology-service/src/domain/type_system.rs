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
    "vector",
    "reference",
    "geo_point",
    "media_reference",
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
        "vector" => {
            let Some(values) = value.as_array() else {
                return Err("expected numeric array value for vector".into());
            };
            if values.is_empty() {
                return Err("vector value cannot be empty".into());
            }
            if values
                .iter()
                .all(|entry| entry.is_f64() || entry.is_i64() || entry.is_u64())
            {
                Ok(())
            } else {
                Err("vector requires an array of numeric values".into())
            }
        }
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
        "geo_point" => {
            let Some(value) = value.as_object() else {
                return Err("expected object value with lat/lon for geo_point".into());
            };
            let latitude = value
                .get("lat")
                .or_else(|| value.get("latitude"))
                .and_then(Value::as_f64);
            let longitude = value
                .get("lon")
                .or_else(|| value.get("longitude"))
                .and_then(Value::as_f64);
            match (latitude, longitude) {
                (Some(latitude), Some(longitude))
                    if (-90.0..=90.0).contains(&latitude)
                        && (-180.0..=180.0).contains(&longitude) =>
                {
                    Ok(())
                }
                (Some(_), Some(_)) => Err("geo_point latitude/longitude out of range".into()),
                _ => Err("geo_point requires numeric lat/lon fields".into()),
            }
        }
        "media_reference" => {
            if value.is_string() {
                return Ok(());
            }

            let Some(value) = value.as_object() else {
                return Err("expected string or object for media_reference".into());
            };
            let uri = value
                .get("uri")
                .or_else(|| value.get("url"))
                .and_then(Value::as_str)
                .filter(|value| !value.trim().is_empty());
            if uri.is_none() {
                return Err("media_reference requires a non-empty uri or url".into());
            }
            Ok(())
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

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{validate_property_type, validate_property_value};

    #[test]
    fn accepts_geo_point_type_and_value() {
        assert!(validate_property_type("geo_point").is_ok());
        assert!(validate_property_value("geo_point", &json!({ "lat": 40.4, "lon": -3.7 })).is_ok());
    }

    #[test]
    fn accepts_media_reference_type_and_value() {
        assert!(validate_property_type("media_reference").is_ok());
        assert!(
            validate_property_value("media_reference", &json!({ "uri": "s3://bucket/file.png" }))
                .is_ok()
        );
    }

    #[test]
    fn accepts_vector_type_and_numeric_array_value() {
        assert!(validate_property_type("vector").is_ok());
        assert!(validate_property_value("vector", &json!([0.1, 0.2, 0.3])).is_ok());
    }
}
