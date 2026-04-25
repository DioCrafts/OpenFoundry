use std::collections::{HashSet, VecDeque};

use auth_middleware::claims::Claims;
use chrono::Utc;
use serde_json::{Map, Value, json};
use uuid::Uuid;

use crate::{
    AppState,
    domain::{
        access::{clearance_rank, marking_rank, validate_marking},
        function_runtime::{load_accessible_object_set, load_linked_objects},
    },
    models::object_set::{
        ObjectSetDefinition, ObjectSetEvaluationResponse, ObjectSetFilter, ObjectSetJoin,
        ObjectSetPolicy, ObjectSetTraversal,
    },
};

pub fn validate_object_set_definition(definition: &ObjectSetDefinition) -> Result<(), String> {
    if definition.name.trim().is_empty() {
        return Err("name is required".to_string());
    }
    for filter in &definition.filters {
        validate_filter(filter)?;
    }
    for traversal in &definition.traversals {
        validate_traversal(traversal)?;
    }
    if let Some(join) = definition.join.as_ref() {
        validate_join(join)?;
    }
    for projection in &definition.projections {
        if projection.trim().is_empty() {
            return Err("projections cannot contain empty values".to_string());
        }
    }
    validate_policy(&definition.policy)?;
    Ok(())
}

pub async fn evaluate_object_set(
    state: &AppState,
    claims: &Claims,
    definition: &ObjectSetDefinition,
    limit: usize,
    materialized: bool,
) -> Result<ObjectSetEvaluationResponse, String> {
    validate_object_set_definition(definition)?;
    enforce_object_set_policy(claims, &definition.policy)?;

    let base_objects =
        load_accessible_object_set(state, claims, definition.base_object_type_id).await?;
    let filtered = base_objects
        .into_iter()
        .filter(|object| allows_marking(claims, &definition.policy, object))
        .filter(|object| matches_filters(object, &definition.filters))
        .collect::<Vec<_>>();

    let mut traversal_neighbor_count = 0usize;
    let secondary_rows = if let Some(join) = definition.join.as_ref() {
        Some(load_accessible_object_set(state, claims, join.secondary_object_type_id).await?)
    } else {
        None
    };

    let mut rows = Vec::new();
    for base in &filtered {
        let neighbors = resolve_traversals(state, claims, base, &definition.traversals).await?;
        traversal_neighbor_count += neighbors.len();
        let seed_row = json!({
            "base": base,
            "neighbors": neighbors,
            "what_if_label": definition.what_if_label,
        });

        if let Some(join) = definition.join.as_ref() {
            let joined = secondary_rows
                .as_ref()
                .into_iter()
                .flat_map(|rows| rows.iter())
                .filter(|candidate| join_matches(base, candidate, join))
                .cloned()
                .collect::<Vec<_>>();

            if joined.is_empty() {
                if join.join_kind.eq_ignore_ascii_case("left") {
                    rows.push(augment_row_with_join(seed_row.clone(), Value::Null));
                }
                continue;
            }

            for item in joined {
                rows.push(augment_row_with_join(seed_row.clone(), item));
            }
        } else {
            rows.push(seed_row);
        }
    }

    let total_rows = rows.len();
    let limited_rows = rows
        .into_iter()
        .take(limit)
        .map(|row| project_row(row, &definition.projections))
        .collect::<Vec<_>>();

    Ok(ObjectSetEvaluationResponse {
        object_set: definition.clone(),
        total_base_matches: filtered.len(),
        total_rows,
        traversal_neighbor_count,
        rows: limited_rows,
        generated_at: Utc::now(),
        materialized,
    })
}

fn validate_filter(filter: &ObjectSetFilter) -> Result<(), String> {
    if filter.field.trim().is_empty() {
        return Err("filters require a field".to_string());
    }
    match filter.operator.as_str() {
        "equals" | "not_equals" | "contains" | "in" | "exists" | "gte" | "lte" => Ok(()),
        other => Err(format!("unsupported filter operator '{other}'")),
    }
}

fn validate_traversal(traversal: &ObjectSetTraversal) -> Result<(), String> {
    match traversal.direction.as_str() {
        "outbound" | "inbound" | "both" => {}
        other => return Err(format!("unsupported traversal direction '{other}'")),
    }
    if traversal.max_hops <= 0 || traversal.max_hops > 4 {
        return Err("traversal.max_hops must be between 1 and 4".to_string());
    }
    Ok(())
}

fn validate_join(join: &ObjectSetJoin) -> Result<(), String> {
    if join.left_field.trim().is_empty() || join.right_field.trim().is_empty() {
        return Err("join fields cannot be empty".to_string());
    }
    match join.join_kind.as_str() {
        "inner" | "left" => Ok(()),
        other => Err(format!("unsupported join kind '{other}'")),
    }
}

fn validate_policy(policy: &ObjectSetPolicy) -> Result<(), String> {
    for marking in &policy.allowed_markings {
        validate_marking(marking)?;
    }
    if let Some(minimum_clearance) = policy.minimum_clearance.as_deref() {
        validate_marking(minimum_clearance)?;
    }
    Ok(())
}

fn enforce_object_set_policy(claims: &Claims, policy: &ObjectSetPolicy) -> Result<(), String> {
    if policy.deny_guest_sessions && claims.is_guest_session() {
        return Err("forbidden: object set blocks guest sessions".to_string());
    }
    if let Some(minimum_clearance) = policy.minimum_clearance.as_deref() {
        let required = marking_rank(minimum_clearance)
            .ok_or_else(|| format!("invalid minimum clearance '{minimum_clearance}'"))?;
        if clearance_rank(claims) < required {
            return Err(
                "forbidden: insufficient classification clearance for object set".to_string(),
            );
        }
    }
    if let Some(restricted_view_id) = policy.required_restricted_view_id {
        let allowed = claims.restricted_view_ids();
        if !allowed.contains(&restricted_view_id) && !claims.has_role("admin") {
            return Err(
                "forbidden: required restricted view is not present in the session".to_string(),
            );
        }
    }
    Ok(())
}

fn allows_marking(claims: &Claims, policy: &ObjectSetPolicy, object: &Value) -> bool {
    let Some(marking) = resolve_path(object, "marking").and_then(Value::as_str) else {
        return false;
    };
    if !claims.allows_marking(marking) {
        return false;
    }
    if policy.allowed_markings.is_empty() {
        return true;
    }
    policy
        .allowed_markings
        .iter()
        .any(|candidate| candidate.eq_ignore_ascii_case(marking))
}

fn matches_filters(object: &Value, filters: &[ObjectSetFilter]) -> bool {
    filters.iter().all(|filter| {
        let actual = resolve_path(object, &filter.field);
        match filter.operator.as_str() {
            "equals" => actual == Some(&filter.value),
            "not_equals" => actual != Some(&filter.value),
            "contains" => contains_value(actual, &filter.value),
            "in" => filter
                .value
                .as_array()
                .map(|values| {
                    actual.is_some_and(|current| values.iter().any(|item| item == current))
                })
                .unwrap_or(false),
            "exists" => actual.is_some() == filter.value.as_bool().unwrap_or(true),
            "gte" => compare_values(actual, Some(&filter.value)).is_some_and(|value| value >= 0),
            "lte" => compare_values(actual, Some(&filter.value)).is_some_and(|value| value <= 0),
            _ => false,
        }
    })
}

async fn resolve_traversals(
    state: &AppState,
    claims: &Claims,
    base: &Value,
    traversals: &[ObjectSetTraversal],
) -> Result<Vec<Value>, String> {
    if traversals.is_empty() {
        return Ok(Vec::new());
    }

    let Some(base_id) = resolve_path(base, "id")
        .and_then(Value::as_str)
        .and_then(|value| Uuid::parse_str(value).ok())
    else {
        return Ok(Vec::new());
    };

    let mut all_neighbors = Vec::new();
    for traversal in traversals {
        let mut seen = HashSet::new();
        let mut queue = VecDeque::from([(base_id, 0i32)]);
        seen.insert(base_id);
        while let Some((current_id, depth)) = queue.pop_front() {
            if depth >= traversal.max_hops {
                continue;
            }
            let linked = load_linked_objects(state, claims, current_id).await?;
            for link in linked {
                if !traversal_matches(&link, traversal) {
                    continue;
                }
                let Some(object_id) = resolve_path(&link, "object.id")
                    .and_then(Value::as_str)
                    .and_then(|value| Uuid::parse_str(value).ok())
                else {
                    continue;
                };
                if !seen.insert(object_id) {
                    continue;
                }
                queue.push_back((object_id, depth + 1));
                all_neighbors.push(link);
            }
        }
    }

    Ok(all_neighbors)
}

fn traversal_matches(link: &Value, traversal: &ObjectSetTraversal) -> bool {
    if traversal.direction != "both" {
        let direction = resolve_path(link, "direction").and_then(Value::as_str);
        if direction != Some(traversal.direction.as_str()) {
            return false;
        }
    }
    if let Some(link_type_id) = traversal.link_type_id {
        let actual = resolve_path(link, "link_type_id")
            .and_then(Value::as_str)
            .and_then(|value| Uuid::parse_str(value).ok());
        if actual != Some(link_type_id) {
            return false;
        }
    }
    if let Some(target_object_type_id) = traversal.target_object_type_id {
        let actual = resolve_path(link, "object.object_type_id")
            .and_then(Value::as_str)
            .and_then(|value| Uuid::parse_str(value).ok());
        if actual != Some(target_object_type_id) {
            return false;
        }
    }
    true
}

fn join_matches(base: &Value, candidate: &Value, join: &ObjectSetJoin) -> bool {
    let left = resolve_path(base, &join.left_field);
    let right = resolve_path(candidate, &join.right_field);
    left.is_some() && left == right
}

fn augment_row_with_join(row: Value, joined: Value) -> Value {
    let mut object = row.as_object().cloned().unwrap_or_default();
    object.insert("joined".to_string(), joined);
    Value::Object(object)
}

fn project_row(row: Value, projections: &[String]) -> Value {
    if projections.is_empty() {
        return row;
    }

    let mut projected = Map::new();
    for projection in projections {
        projected.insert(
            projection.clone(),
            resolve_projection_value(&row, projection)
                .cloned()
                .unwrap_or(Value::Null),
        );
    }
    Value::Object(projected)
}

fn resolve_projection_value<'a>(row: &'a Value, projection: &str) -> Option<&'a Value> {
    if projection.starts_with("base.")
        || projection.starts_with("joined.")
        || projection.starts_with("neighbors.")
    {
        return resolve_path(row, projection);
    }

    resolve_path(row, projection).or_else(|| resolve_path(row, &format!("base.{projection}")))
}

fn resolve_path<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    if path.trim().is_empty() {
        return Some(value);
    }

    if !path.contains('.') {
        return value
            .get(path)
            .or_else(|| {
                value
                    .get("properties")
                    .and_then(|properties| properties.get(path))
            })
            .or_else(|| value.get("base").and_then(|base| resolve_path(base, path)));
    }

    let mut current = value;
    for segment in path.split('.') {
        current = current.get(segment)?;
    }
    Some(current)
}

fn contains_value(actual: Option<&Value>, expected: &Value) -> bool {
    match (actual, expected) {
        (Some(Value::String(current)), Value::String(candidate)) => current.contains(candidate),
        (Some(Value::Array(items)), candidate) => items.iter().any(|item| item == candidate),
        _ => false,
    }
}

fn compare_values(left: Option<&Value>, right: Option<&Value>) -> Option<i8> {
    let left = left?;
    let right = right?;
    if let (Some(l), Some(r)) = (left.as_f64(), right.as_f64()) {
        return Some(match l.partial_cmp(&r)? {
            std::cmp::Ordering::Less => -1,
            std::cmp::Ordering::Equal => 0,
            std::cmp::Ordering::Greater => 1,
        });
    }
    if let (Some(l), Some(r)) = (left.as_str(), right.as_str()) {
        return Some(match l.cmp(r) {
            std::cmp::Ordering::Less => -1,
            std::cmp::Ordering::Equal => 0,
            std::cmp::Ordering::Greater => 1,
        });
    }
    None
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{compare_values, matches_filters, project_row, resolve_path};
    use crate::models::object_set::ObjectSetFilter;

    #[test]
    fn object_set_filters_support_properties_and_marking() {
        let object = json!({
            "id": "00000000-0000-0000-0000-000000000001",
            "marking": "confidential",
            "properties": {
                "status": "active",
                "score": 99
            }
        });

        assert!(matches_filters(
            &object,
            &[
                ObjectSetFilter {
                    field: "status".to_string(),
                    operator: "equals".to_string(),
                    value: json!("active"),
                },
                ObjectSetFilter {
                    field: "marking".to_string(),
                    operator: "equals".to_string(),
                    value: json!("confidential"),
                },
                ObjectSetFilter {
                    field: "score".to_string(),
                    operator: "gte".to_string(),
                    value: json!(70),
                },
            ],
        ));
    }

    #[test]
    fn projections_prefer_wrapper_and_base_paths() {
        let row = json!({
            "base": {
                "id": "1",
                "properties": {
                    "name": "Case-7"
                }
            },
            "joined": {
                "properties": {
                    "owner": "Alice"
                }
            }
        });

        let projected = project_row(
            row,
            &[
                "id".to_string(),
                "base.properties.name".to_string(),
                "joined.properties.owner".to_string(),
            ],
        );

        assert_eq!(projected["id"], json!("1"));
        assert_eq!(projected["base.properties.name"], json!("Case-7"));
        assert_eq!(projected["joined.properties.owner"], json!("Alice"));
    }

    #[test]
    fn compare_values_handles_numbers_and_strings() {
        assert_eq!(compare_values(Some(&json!(3)), Some(&json!(7))), Some(-1));
        assert_eq!(
            compare_values(Some(&json!("b")), Some(&json!("a"))),
            Some(1)
        );
        assert_eq!(
            resolve_path(&json!({"properties": {"kind": "case"}}), "kind"),
            Some(&json!("case"))
        );
    }
}
