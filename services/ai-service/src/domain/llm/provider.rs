use serde_json::{Map, Value};

pub fn interpolate_template(
    template: &str,
    variables: &Value,
    strict: bool,
) -> (String, Vec<String>) {
    let values = variables.as_object().cloned().unwrap_or_else(Map::new);
    let mut remaining = template;
    let mut rendered = String::new();
    let mut missing = Vec::new();

    while let Some(open_index) = remaining.find("{{") {
        rendered.push_str(&remaining[..open_index]);
        let after_open = &remaining[(open_index + 2)..];

        let Some(close_index) = after_open.find("}}") else {
            rendered.push_str(&remaining[open_index..]);
            remaining = "";
            break;
        };

        let key = after_open[..close_index].trim();
        if let Some(value) = values.get(key) {
            rendered.push_str(&value_to_string(value));
        } else {
            missing.push(key.to_string());
            if !strict {
                rendered.push_str("{{");
                rendered.push_str(key);
                rendered.push_str("}}");
            }
        }

        remaining = &after_open[(close_index + 2)..];
    }

    rendered.push_str(remaining);
    (rendered, missing)
}

fn value_to_string(value: &Value) -> String {
    match value {
        Value::String(inner) => inner.clone(),
        Value::Null => String::new(),
        _ => value.to_string(),
    }
}
