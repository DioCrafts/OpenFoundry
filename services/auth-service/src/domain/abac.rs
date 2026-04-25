use auth_middleware::Claims;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    domain::access,
    models::{
        policy::Policy,
        restricted_view::{RestrictedView, RestrictedViewRow},
    },
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationResult {
    pub allowed: bool,
    pub matched_policy_ids: Vec<Uuid>,
    pub deny_policy_ids: Vec<Uuid>,
    pub row_filter: Option<String>,
    pub hidden_columns: Vec<String>,
    pub matched_restricted_view_ids: Vec<Uuid>,
    pub restricted_views: Vec<EvaluationRestrictedView>,
    pub deny_reasons: Vec<String>,
    pub allowed_org_ids: Vec<Uuid>,
    pub allowed_markings: Vec<String>,
    pub effective_clearance: Option<String>,
    pub consumer_mode: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationRestrictedView {
    pub id: Uuid,
    pub name: String,
    pub row_filter: Option<String>,
    pub hidden_columns: Vec<String>,
    pub allowed_org_ids: Vec<Uuid>,
    pub allowed_markings: Vec<String>,
    pub consumer_mode_enabled: bool,
    pub allow_guest_access: bool,
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
    let has_configured_policies = !policies.is_empty();

    let subject_context = build_subject_context(claims);
    let restricted_view_rows = sqlx::query_as::<_, RestrictedViewRow>(
        r#"SELECT id, name, description, resource, action, conditions, row_filter, hidden_columns,
		          allowed_org_ids, allowed_markings, consumer_mode_enabled, allow_guest_access,
		          enabled, created_by, created_at, updated_at
		   FROM restricted_views
		   WHERE enabled = true
		     AND (resource = $1 OR resource = '*')
		     AND (action = $2 OR action = '*')
		   ORDER BY created_at ASC"#,
    )
    .bind(resource)
    .bind(action)
    .fetch_all(pool)
    .await?;
    let has_configured_restricted_views = !restricted_view_rows.is_empty();

    let mut matched_policy_ids = Vec::new();
    let mut deny_policy_ids = Vec::new();
    let mut allow_filters = Vec::new();
    let mut restricted_filters = Vec::new();
    let mut hidden_columns = Vec::new();
    let mut matched_restricted_view_ids = Vec::new();
    let mut restricted_views = Vec::new();
    let mut deny_reasons = Vec::new();
    let mut consumer_mode = claims.consumer_mode_enabled();
    let resource_org_id = resource_org_id(resource_attributes);
    let resource_marking = resource_marking(resource_attributes);
    let scoped_restricted_view_ids = claims.restricted_view_ids();

    if !claims.has_role("admin") && !claims.allows_org_id(resource_org_id) {
        deny_reasons.push("organization isolation boundary denied access".to_string());
    }
    if let Some(marking) = resource_marking.as_deref() {
        if !claims.has_role("admin") && !claims.allows_marking(marking) {
            deny_reasons.push(format!(
                "classification boundary denied marking '{}'",
                marking
            ));
        }
    }

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

    for row in restricted_view_rows {
        let view = RestrictedView::try_from(row).map_err(|cause| {
            sqlx::Error::Decode(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                cause,
            )))
        })?;

        if !policy_matches(&view.conditions, &subject_context, resource_attributes) {
            continue;
        }
        if !claims.has_role("admin") && !scoped_restricted_view_ids.is_empty() {
            if !scoped_restricted_view_ids.contains(&view.id) {
                continue;
            }
        }
        if claims.is_guest_session() && !view.allow_guest_access {
            continue;
        }
        if !view.allowed_org_ids.is_empty() {
            let Some(org_id) = resource_org_id else {
                continue;
            };
            if !view.allowed_org_ids.contains(&org_id) {
                continue;
            }
        }
        if !view.allowed_markings.is_empty() {
            let marking = resource_marking.as_deref().unwrap_or("public");
            if !view
                .allowed_markings
                .iter()
                .any(|candidate| candidate.eq_ignore_ascii_case(marking))
            {
                continue;
            }
        }

        matched_restricted_view_ids.push(view.id);
        if let Some(filter) = view.row_filter.as_deref() {
            let rendered = render_row_filter(filter, &subject_context, resource_attributes);
            if !rendered.is_empty() {
                restricted_filters.push(rendered);
            }
        }
        hidden_columns.extend(view.hidden_columns.iter().cloned());
        consumer_mode |= view.consumer_mode_enabled;
        restricted_views.push(EvaluationRestrictedView {
            id: view.id,
            name: view.name,
            row_filter: view.row_filter,
            hidden_columns: view.hidden_columns,
            allowed_org_ids: view.allowed_org_ids,
            allowed_markings: view.allowed_markings,
            consumer_mode_enabled: view.consumer_mode_enabled,
            allow_guest_access: view.allow_guest_access,
        });
    }

    hidden_columns.sort();
    hidden_columns.dedup();

    let allow_policy_filter = join_with_or(allow_filters);
    let restricted_filter = join_with_or(restricted_filters);
    let row_filter = join_with_and([allow_policy_filter, restricted_filter]);
    let has_access_controls = has_configured_policies
        || has_configured_restricted_views
        || !deny_policy_ids.is_empty()
        || !deny_reasons.is_empty();

    Ok(EvaluationResult {
        allowed: deny_policy_ids.is_empty()
            && deny_reasons.is_empty()
            && (claims.has_role("admin")
                || !has_access_controls
                || !matched_policy_ids.is_empty()
                || !matched_restricted_view_ids.is_empty()),
        matched_policy_ids,
        deny_policy_ids,
        row_filter,
        hidden_columns,
        matched_restricted_view_ids,
        restricted_views,
        deny_reasons,
        allowed_org_ids: claims.allowed_org_ids(),
        allowed_markings: claims.allowed_markings(),
        effective_clearance: claims.classification_clearance().map(str::to_string),
        consumer_mode,
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
    base.insert(
        "allowed_org_ids".to_string(),
        json!(claims.allowed_org_ids()),
    );
    base.insert(
        "allowed_markings".to_string(),
        json!(claims.allowed_markings()),
    );
    base.insert(
        "restricted_view_ids".to_string(),
        json!(claims.restricted_view_ids()),
    );
    base.insert(
        "consumer_mode".to_string(),
        Value::Bool(claims.consumer_mode_enabled()),
    );

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

fn join_with_or(filters: Vec<String>) -> Option<String> {
    if filters.is_empty() {
        None
    } else {
        Some(
            filters
                .into_iter()
                .map(|fragment| format!("({fragment})"))
                .collect::<Vec<_>>()
                .join(" OR "),
        )
    }
}

fn join_with_and<const N: usize>(filters: [Option<String>; N]) -> Option<String> {
    let parts = filters
        .into_iter()
        .flatten()
        .filter(|candidate| !candidate.trim().is_empty())
        .collect::<Vec<_>>();

    match parts.len() {
        0 => None,
        1 => parts.into_iter().next(),
        _ => Some(
            parts
                .into_iter()
                .map(|fragment| format!("({fragment})"))
                .collect::<Vec<_>>()
                .join(" AND "),
        ),
    }
}

fn resource_org_id(resource: &Value) -> Option<Uuid> {
    resource
        .get("organization_id")
        .or_else(|| resource.get("org_id"))
        .and_then(Value::as_str)
        .and_then(|value| Uuid::parse_str(value).ok())
}

fn resource_marking(resource: &Value) -> Option<String> {
    resource
        .get("effective_marking")
        .or_else(|| resource.get("marking"))
        .and_then(Value::as_str)
        .map(|value| value.to_ascii_lowercase())
        .filter(|value| access::marking_rank(value).is_some())
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
