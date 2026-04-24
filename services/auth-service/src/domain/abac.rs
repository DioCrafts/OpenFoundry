use auth_middleware::Claims;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::policy::Policy;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationResult {
    pub allowed: bool,
    pub matched_policy_ids: Vec<Uuid>,
    pub deny_policy_ids: Vec<Uuid>,
    pub row_filter: Option<String>,
}

pub async fn list_policies(pool: &PgPool) -> Result<Vec<Policy>, sqlx::Error> {
    sqlx::query_as::<_, Policy>(
		r#"SELECT id, name, description, effect, resource, action, conditions, row_filter, enabled, created_by, created_at, updated_at
		   FROM abac_policies
		   ORDER BY created_at DESC"#,
	)
	.fetch_all(pool)
	.await
}

pub async fn evaluate(
    pool: &PgPool,
    claims: &Claims,
    resource: &str,
    action: &str,
    resource_attributes: &Value,
) -> Result<EvaluationResult, sqlx::Error> {
    let policies = sqlx::query_as::<_, Policy>(
		r#"SELECT id, name, description, effect, resource, action, conditions, row_filter, enabled, created_by, created_at, updated_at
		   FROM abac_policies
		   WHERE enabled = true
			 AND (resource = $1 OR resource = '*')
			 AND (action = $2 OR action = '*')
		   ORDER BY created_at ASC"#,
	)
	.bind(resource)
	.bind(action)
	.fetch_all(pool)
	.await?;

    let subject_context = build_subject_context(claims);
    let mut matched_policy_ids = Vec::new();
    let mut deny_policy_ids = Vec::new();
    let mut allow_filters = Vec::new();

    for policy in policies {
        if !policy_matches(&policy.conditions, &subject_context, resource_attributes) {
            continue;
        }

        if policy.effect.eq_ignore_ascii_case("deny") {
            deny_policy_ids.push(policy.id);
            continue;
        }

        matched_policy_ids.push(policy.id);

        if let Some(filter) = policy.row_filter.as_deref() {
            let rendered = render_row_filter(filter, &subject_context, resource_attributes);
            if !rendered.is_empty() {
                allow_filters.push(rendered);
            }
        }
    }

    let row_filter = if allow_filters.is_empty() {
        None
    } else {
        Some(
            allow_filters
                .into_iter()
                .map(|fragment| format!("({fragment})"))
                .collect::<Vec<_>>()
                .join(" OR "),
        )
    };

    Ok(EvaluationResult {
        allowed: deny_policy_ids.is_empty() && !matched_policy_ids.is_empty(),
        matched_policy_ids,
        deny_policy_ids,
        row_filter,
    })
}

fn build_subject_context(claims: &Claims) -> Value {
    let mut base = Map::new();
    base.insert("user_id".to_string(), Value::String(claims.sub.to_string()));
    base.insert(
        "organization_id".to_string(),
        claims
            .org_id
            .map(|org_id| Value::String(org_id.to_string()))
            .unwrap_or(Value::Null),
    );
    base.insert("roles".to_string(), json!(claims.roles));
    base.insert("permissions".to_string(), json!(claims.permissions));

    if let Some(attributes) = claims.attributes.as_object() {
        for (key, value) in attributes {
            base.insert(key.clone(), value.clone());
        }
    }

    Value::Object(base)
}

fn policy_matches(conditions: &Value, subject: &Value, resource: &Value) -> bool {
    let Some(root) = conditions.as_object() else {
        return true;
    };

    match_selector(root.get("subject"), subject, resource)
        && match_selector(root.get("resource"), resource, subject)
}

fn match_selector(selector: Option<&Value>, context: &Value, other_context: &Value) -> bool {
    let Some(selector) = selector.and_then(Value::as_object) else {
        return true;
    };

    selector.iter().all(|(key, expected)| {
        value_matches(
            context.get(key).unwrap_or(&Value::Null),
            expected,
            other_context,
        )
    })
}

fn value_matches(actual: &Value, expected: &Value, other_context: &Value) -> bool {
    match expected {
        Value::String(pointer) if pointer.starts_with("$other.") => {
            let key = pointer.trim_start_matches("$other.");
            actual == other_context.get(key).unwrap_or(&Value::Null)
        }
        Value::Array(expected_values) => {
            if let Some(actual_array) = actual.as_array() {
                actual_array.iter().any(|actual_item| {
                    expected_values
                        .iter()
                        .any(|candidate| candidate == actual_item)
                })
            } else {
                expected_values.iter().any(|candidate| candidate == actual)
            }
        }
        _ => actual == expected,
    }
}

fn render_row_filter(template: &str, subject: &Value, resource: &Value) -> String {
    let mut rendered = template.to_string();

    rendered = replace_context_tokens(rendered, "subject", subject);
    replace_context_tokens(rendered, "resource", resource)
}

fn replace_context_tokens(mut input: String, prefix: &str, context: &Value) -> String {
    let Some(map) = context.as_object() else {
        return input;
    };

    for (key, value) in map {
        let token = format!("{{{{{prefix}.{key}}}}}");
        let replacement = match value {
            Value::Null => "NULL".to_string(),
            Value::String(inner) => inner.clone(),
            _ => value.to_string(),
        };
        input = input.replace(&token, &replacement);
    }

    input
}
