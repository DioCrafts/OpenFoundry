use std::collections::{HashMap, HashSet};

use chrono::{Duration, Utc};
use serde_json::{Map, Value, json};
use uuid::Uuid;

use crate::{
    AppState,
    domain::{
        schema::{load_effective_properties, validate_object_properties},
        type_system::validate_property_value,
    },
    handlers::objects::ObjectInstance,
    models::rule::{
        MachineryInsight, OntologyRule, OntologyRuleRow, OntologyRuleRun, RuleEffectSpec,
        RuleEvaluateResponse, RuleMatchResponse, RuleTriggerSpec,
    },
};

fn invalid_rule(message: impl Into<String>) -> String {
    message.into()
}

async fn ensure_object_type_exists(state: &AppState, object_type_id: Uuid) -> Result<bool, String> {
    sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM object_types WHERE id = $1)")
        .bind(object_type_id)
        .fetch_one(&state.db)
        .await
        .map_err(|error| format!("failed to validate object type: {error}"))
}

async fn property_type_map(
    state: &AppState,
    object_type_id: Uuid,
) -> Result<HashMap<String, String>, String> {
    Ok(load_effective_properties(&state.db, object_type_id)
        .await
        .map_err(|error| format!("failed to load property definitions: {error}"))?
        .into_iter()
        .map(|property| (property.name, property.property_type))
        .collect())
}

pub async fn validate_rule_definition(
    state: &AppState,
    object_type_id: Uuid,
    trigger_spec: &RuleTriggerSpec,
    effect_spec: &RuleEffectSpec,
) -> Result<(), String> {
    if !ensure_object_type_exists(state, object_type_id).await? {
        return Err(invalid_rule("referenced object type does not exist"));
    }

    let property_types = property_type_map(state, object_type_id).await?;

    for property_name in trigger_spec.equals.keys() {
        if !property_types.contains_key(property_name) {
            return Err(invalid_rule(format!(
                "unknown property '{property_name}' in trigger equals"
            )));
        }
    }

    for (property_name, threshold) in &trigger_spec.numeric_gte {
        let Some(property_type) = property_types.get(property_name) else {
            return Err(invalid_rule(format!(
                "unknown property '{property_name}' in numeric_gte"
            )));
        };
        if !matches!(property_type.as_str(), "integer" | "float") {
            return Err(invalid_rule(format!(
                "property '{property_name}' must be numeric for numeric_gte"
            )));
        }
        if !threshold.is_finite() {
            return Err(invalid_rule(format!(
                "numeric_gte threshold for '{property_name}' must be finite"
            )));
        }
    }

    for (property_name, threshold) in &trigger_spec.numeric_lte {
        let Some(property_type) = property_types.get(property_name) else {
            return Err(invalid_rule(format!(
                "unknown property '{property_name}' in numeric_lte"
            )));
        };
        if !matches!(property_type.as_str(), "integer" | "float") {
            return Err(invalid_rule(format!(
                "property '{property_name}' must be numeric for numeric_lte"
            )));
        }
        if !threshold.is_finite() {
            return Err(invalid_rule(format!(
                "numeric_lte threshold for '{property_name}' must be finite"
            )));
        }
    }

    for property_name in trigger_spec.exists.iter().chain(trigger_spec.changed_properties.iter()) {
        if !property_types.contains_key(property_name) {
            return Err(invalid_rule(format!(
                "unknown property '{property_name}' in trigger specification"
            )));
        }
    }

    if !trigger_spec.markings.is_empty()
        && trigger_spec
            .markings
            .iter()
            .any(|value| !matches!(value.as_str(), "public" | "confidential" | "pii"))
    {
        return Err(invalid_rule(
            "markings must only contain public, confidential, or pii",
        ));
    }

    let has_effect = effect_spec.object_patch.is_some()
        || effect_spec.schedule.is_some()
        || effect_spec.alert.is_some();
    if !has_effect {
        return Err(invalid_rule(
            "rule effect must define object_patch, schedule, or alert",
        ));
    }

    if let Some(object_patch) = &effect_spec.object_patch {
        let Some(values) = object_patch.as_object() else {
            return Err(invalid_rule("object_patch must be a JSON object"));
        };

        for (property_name, value) in values {
            let Some(property_type) = property_types.get(property_name) else {
                return Err(invalid_rule(format!(
                    "unknown property '{property_name}' in object_patch"
                )));
            };
            validate_property_value(property_type, value)
                .map_err(|error| format!("{property_name}: {error}"))?;
        }
    }

    if let Some(schedule) = &effect_spec.schedule {
        let Some(property_type) = property_types.get(&schedule.property_name) else {
            return Err(invalid_rule(format!(
                "unknown property '{}' in schedule",
                schedule.property_name
            )));
        };

        if !matches!(property_type.as_str(), "timestamp" | "date" | "string") {
            return Err(invalid_rule(format!(
                "schedule property '{}' must be timestamp, date, or string",
                schedule.property_name
            )));
        }

        if schedule.offset_hours == 0 {
            return Err(invalid_rule(
                "schedule.offset_hours must not be zero so the schedule can move in time",
            ));
        }
    }

    if let Some(alert) = &effect_spec.alert {
        if alert.title.trim().is_empty() {
            return Err(invalid_rule("alert.title is required when alert is configured"));
        }
        if !matches!(alert.severity.as_str(), "low" | "medium" | "high" | "critical") {
            return Err(invalid_rule(
                "alert.severity must be one of low, medium, high, critical",
            ));
        }
    }

    Ok(())
}

pub async fn load_rule(state: &AppState, rule_id: Uuid) -> Result<Option<OntologyRule>, String> {
    sqlx::query_as::<_, OntologyRuleRow>(
        r#"SELECT id, name, display_name, description, object_type_id, evaluation_mode,
                  trigger_spec, effect_spec, owner_id, created_at, updated_at
           FROM ontology_rules
           WHERE id = $1"#,
    )
    .bind(rule_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|error| format!("failed to load rule: {error}"))?
    .map(OntologyRule::try_from)
    .transpose()
    .map_err(|error| format!("failed to decode rule: {error}"))
}

pub async fn load_rules_for_object_type(
    state: &AppState,
    object_type_id: Uuid,
) -> Result<Vec<OntologyRule>, String> {
    sqlx::query_as::<_, OntologyRuleRow>(
        r#"SELECT id, name, display_name, description, object_type_id, evaluation_mode,
                  trigger_spec, effect_spec, owner_id, created_at, updated_at
           FROM ontology_rules
           WHERE object_type_id = $1
           ORDER BY created_at DESC"#,
    )
    .bind(object_type_id)
    .fetch_all(&state.db)
    .await
    .map_err(|error| format!("failed to load rules: {error}"))?
    .into_iter()
    .map(OntologyRule::try_from)
    .collect::<Result<Vec<_>, _>>()
    .map_err(|error| format!("failed to decode rules: {error}"))
}

fn merged_properties(
    object: &ObjectInstance,
    properties_patch: Option<&Map<String, Value>>,
) -> Map<String, Value> {
    let mut merged = object.properties.as_object().cloned().unwrap_or_default();
    if let Some(properties_patch) = properties_patch {
        for (key, value) in properties_patch {
            merged.insert(key.clone(), value.clone());
        }
    }
    merged
}

pub fn derive_changed_properties(
    before: Option<&ObjectInstance>,
    after_properties: &Map<String, Value>,
) -> HashSet<String> {
    let mut changed = HashSet::new();
    let before_properties = before.and_then(|object| object.properties.as_object());

    for (key, value) in after_properties {
        let previous = before_properties.and_then(|properties| properties.get(key));
        if previous != Some(value) {
            changed.insert(key.clone());
        }
    }

    if let Some(before_properties) = before_properties {
        for key in before_properties.keys() {
            if !after_properties.contains_key(key) {
                changed.insert(key.clone());
            }
        }
    }

    changed
}

fn matches_equals(
    equals: &HashMap<String, Value>,
    properties: &Map<String, Value>,
) -> Result<(), String> {
    for (key, expected) in equals {
        if properties.get(key) != Some(expected) {
            return Err(format!("property '{key}' does not match expected value"));
        }
    }
    Ok(())
}

fn matches_numeric_thresholds(
    thresholds: &HashMap<String, f64>,
    properties: &Map<String, Value>,
    comparator: impl Fn(f64, f64) -> bool,
    label: &str,
) -> Result<(), String> {
    for (key, threshold) in thresholds {
        let Some(value) = properties.get(key) else {
            return Err(format!("property '{key}' is missing for {label}"));
        };
        let Some(number) = value.as_f64().or_else(|| value.as_i64().map(|value| value as f64)) else {
            return Err(format!("property '{key}' is not numeric for {label}"));
        };
        if !comparator(number, *threshold) {
            return Err(format!(
                "property '{key}' does not satisfy {label} {threshold}"
            ));
        }
    }
    Ok(())
}

fn build_rule_effect_preview(
    effect_spec: &RuleEffectSpec,
    object: &ObjectInstance,
) -> Value {
    let mut object_patch = effect_spec
        .object_patch
        .as_ref()
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    let schedule_preview = effect_spec.schedule.as_ref().map(|schedule| {
        let scheduled_at = (Utc::now() + Duration::hours(schedule.offset_hours)).to_rfc3339();
        object_patch.insert(schedule.property_name.clone(), Value::String(scheduled_at.clone()));
        json!({
            "property_name": schedule.property_name,
            "scheduled_at": scheduled_at,
            "offset_hours": schedule.offset_hours,
        })
    });

    let effective_patch = if object_patch.is_empty() {
        Value::Null
    } else {
        Value::Object(object_patch)
    };

    json!({
        "object_patch": effective_patch,
        "schedule": schedule_preview,
        "alert": effect_spec.alert,
        "object_id": object.id,
    })
}

pub fn evaluate_rule_against_object(
    rule: &OntologyRule,
    object: &ObjectInstance,
    properties_patch: Option<&Map<String, Value>>,
) -> RuleMatchResponse {
    let properties = merged_properties(object, properties_patch);
    let changed_properties = derive_changed_properties(Some(object), &properties);
    let mut trigger_reasons = Vec::new();

    if let Err(error) = matches_equals(&rule.trigger_spec.equals, &properties) {
        trigger_reasons.push(error);
    }
    if let Err(error) = matches_numeric_thresholds(
        &rule.trigger_spec.numeric_gte,
        &properties,
        |value, threshold| value >= threshold,
        "numeric_gte",
    ) {
        trigger_reasons.push(error);
    }
    if let Err(error) = matches_numeric_thresholds(
        &rule.trigger_spec.numeric_lte,
        &properties,
        |value, threshold| value <= threshold,
        "numeric_lte",
    ) {
        trigger_reasons.push(error);
    }

    for property_name in &rule.trigger_spec.exists {
        if !properties.contains_key(property_name) {
            trigger_reasons.push(format!("property '{property_name}' is missing"));
        }
    }

    if !rule.trigger_spec.changed_properties.is_empty()
        && !rule
            .trigger_spec
            .changed_properties
            .iter()
            .any(|property_name| changed_properties.contains(property_name))
    {
        trigger_reasons.push("none of the configured changed_properties were updated".to_string());
    }

    if !rule.trigger_spec.markings.is_empty()
        && !rule
            .trigger_spec
            .markings
            .iter()
            .any(|marking| marking == &object.marking)
    {
        trigger_reasons.push("object marking does not match rule markings".to_string());
    }

    let matched = trigger_reasons.is_empty();
    let changed_properties = changed_properties.into_iter().collect::<Vec<_>>();
    let effect_preview = if matched {
        build_rule_effect_preview(&rule.effect_spec, object)
    } else {
        Value::Null
    };

    RuleMatchResponse {
        rule_id: rule.id,
        matched,
        trigger_payload: json!({
            "object_id": object.id,
            "changed_properties": changed_properties,
            "reasons": trigger_reasons,
        }),
        effect_preview,
    }
}

pub async fn apply_rule_effect(
    state: &AppState,
    object: &ObjectInstance,
    effect_preview: &Value,
) -> Result<ObjectInstance, String> {
    let patch = effect_preview
        .get("object_patch")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    if patch.is_empty() {
        return Ok(object.clone());
    }

    let definitions = load_effective_properties(&state.db, object.object_type_id)
        .await
        .map_err(|error| format!("failed to load property definitions: {error}"))?;
    let property_types = definitions
        .iter()
        .map(|property| (property.name.as_str(), property.property_type.as_str()))
        .collect::<HashMap<_, _>>();

    let mut next_properties = object.properties.as_object().cloned().unwrap_or_default();
    for (property_name, value) in &patch {
        let property_type = property_types
            .get(property_name.as_str())
            .ok_or_else(|| format!("unknown property '{property_name}' in rule effect"))?;
        validate_property_value(property_type, value)
            .map_err(|error| format!("{property_name}: {error}"))?;
        next_properties.insert(property_name.clone(), value.clone());
    }

    let normalized = validate_object_properties(&definitions, &Value::Object(next_properties))
        .map_err(|error| format!("invalid rule effect patch: {error}"))?;

    sqlx::query_as::<_, ObjectInstance>(
        r#"UPDATE object_instances
           SET properties = $2::jsonb,
               updated_at = NOW()
           WHERE id = $1
           RETURNING id, object_type_id, properties, created_by, organization_id, marking, created_at, updated_at"#,
    )
    .bind(object.id)
    .bind(normalized)
    .fetch_one(&state.db)
    .await
    .map_err(|error| format!("failed to apply rule effect: {error}"))
}

pub async fn record_rule_run(
    state: &AppState,
    rule_id: Uuid,
    object_id: Uuid,
    matched: bool,
    simulated: bool,
    trigger_payload: &Value,
    effect_preview: Option<&Value>,
    created_by: Uuid,
) -> Result<OntologyRuleRun, String> {
    sqlx::query_as::<_, OntologyRuleRun>(
        r#"INSERT INTO ontology_rule_runs (
               id, rule_id, object_id, matched, simulated, trigger_payload, effect_preview, created_by
           )
           VALUES ($1, $2, $3, $4, $5, $6::jsonb, $7::jsonb, $8)
           RETURNING id, rule_id, object_id, matched, simulated, trigger_payload, effect_preview, created_by, created_at"#,
    )
    .bind(Uuid::now_v7())
    .bind(rule_id)
    .bind(object_id)
    .bind(matched)
    .bind(simulated)
    .bind(trigger_payload)
    .bind(effect_preview)
    .bind(created_by)
    .fetch_one(&state.db)
    .await
    .map_err(|error| format!("failed to record rule run: {error}"))
}

pub async fn evaluate_rules_for_object(
    state: &AppState,
    object: &ObjectInstance,
    properties_patch: Option<&Map<String, Value>>,
) -> Result<Vec<(OntologyRule, RuleMatchResponse)>, String> {
    let rules = load_rules_for_object_type(state, object.object_type_id).await?;
    Ok(rules
        .into_iter()
        .map(|rule| {
            let match_result = evaluate_rule_against_object(&rule, object, properties_patch);
            (rule, match_result)
        })
        .collect())
}

pub async fn load_recent_rule_runs(
    state: &AppState,
    object_id: Uuid,
    limit: usize,
) -> Result<Vec<OntologyRuleRun>, String> {
    sqlx::query_as::<_, OntologyRuleRun>(
        r#"SELECT id, rule_id, object_id, matched, simulated, trigger_payload, effect_preview,
                  created_by, created_at
           FROM ontology_rule_runs
           WHERE object_id = $1
           ORDER BY created_at DESC
           LIMIT $2"#,
    )
    .bind(object_id)
    .bind(limit as i64)
    .fetch_all(&state.db)
    .await
    .map_err(|error| format!("failed to load recent rule runs: {error}"))
}

pub async fn machinery_insights(
    state: &AppState,
    object_type_id: Option<Uuid>,
) -> Result<Vec<MachineryInsight>, String> {
    let rows = if let Some(object_type_id) = object_type_id {
        sqlx::query_as::<_, OntologyRuleRow>(
            r#"SELECT id, name, display_name, description, object_type_id, evaluation_mode,
                      trigger_spec, effect_spec, owner_id, created_at, updated_at
               FROM ontology_rules
               WHERE object_type_id = $1
               ORDER BY created_at DESC"#,
        )
        .bind(object_type_id)
        .fetch_all(&state.db)
        .await
        .map_err(|error| format!("failed to load rules: {error}"))?
    } else {
        sqlx::query_as::<_, OntologyRuleRow>(
            r#"SELECT id, name, display_name, description, object_type_id, evaluation_mode,
                      trigger_spec, effect_spec, owner_id, created_at, updated_at
               FROM ontology_rules
               ORDER BY created_at DESC"#,
        )
        .fetch_all(&state.db)
        .await
        .map_err(|error| format!("failed to load rules: {error}"))?
    };

    let rules = rows
        .into_iter()
        .map(OntologyRule::try_from)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("failed to decode rules: {error}"))?;

    let rule_ids = rules.iter().map(|rule| rule.id).collect::<Vec<_>>();
    if rule_ids.is_empty() {
        return Ok(Vec::new());
    }

    let runs = sqlx::query_as::<_, OntologyRuleRun>(
        r#"SELECT id, rule_id, object_id, matched, simulated, trigger_payload, effect_preview,
                  created_by, created_at
           FROM ontology_rule_runs
           WHERE rule_id = ANY($1)
           ORDER BY created_at DESC"#,
    )
    .bind(&rule_ids)
    .fetch_all(&state.db)
    .await
    .map_err(|error| format!("failed to load rule runs: {error}"))?;

    let mut grouped_runs = HashMap::<Uuid, Vec<OntologyRuleRun>>::new();
    for run in runs {
        grouped_runs.entry(run.rule_id).or_default().push(run);
    }

    Ok(rules
        .into_iter()
        .map(|rule| {
            let runs = grouped_runs.remove(&rule.id).unwrap_or_default();
            let matched_runs = runs.iter().filter(|run| run.matched).count();
            let pending_schedules = runs
                .iter()
                .filter(|run| {
                    run.effect_preview
                        .as_ref()
                        .and_then(|preview| preview.get("schedule"))
                        .is_some()
                })
                .count();
            let last_matched = runs.iter().find(|run| run.matched);

            MachineryInsight {
                rule_id: rule.id,
                name: rule.name,
                display_name: rule.display_name,
                evaluation_mode: rule.evaluation_mode,
                matched_runs,
                total_runs: runs.len(),
                pending_schedules,
                last_matched_at: last_matched.map(|run| run.created_at),
                last_object_id: last_matched.map(|run| run.object_id),
            }
        })
        .collect())
}

pub fn build_rule_evaluate_response(
    rule: OntologyRule,
    object: &ObjectInstance,
    match_result: RuleMatchResponse,
) -> RuleEvaluateResponse {
    RuleEvaluateResponse {
        rule,
        matched: match_result.matched,
        trigger_payload: match_result.trigger_payload,
        effect_preview: match_result.effect_preview,
        object: json!(object),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use chrono::Utc;
    use serde_json::json;
    use uuid::Uuid;

    use crate::{
        handlers::objects::ObjectInstance,
        models::rule::{OntologyRule, RuleEffectSpec, RuleEvaluationMode, RuleTriggerSpec},
    };

    use super::evaluate_rule_against_object;

    fn sample_object() -> ObjectInstance {
        ObjectInstance {
            id: Uuid::nil(),
            object_type_id: Uuid::nil(),
            properties: json!({
                "status": "pending",
                "risk_score": 0.92,
            }),
            created_by: Uuid::nil(),
            organization_id: None,
            marking: "confidential".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn matches_rule_with_numeric_and_equals_conditions() {
        let rule = OntologyRule {
            id: Uuid::now_v7(),
            name: "high_risk_case".to_string(),
            display_name: "High risk case".to_string(),
            description: "".to_string(),
            object_type_id: Uuid::nil(),
            evaluation_mode: RuleEvaluationMode::Advisory,
            trigger_spec: RuleTriggerSpec {
                equals: [("status".to_string(), json!("pending"))].into_iter().collect(),
                numeric_gte: [("risk_score".to_string(), 0.8)].into_iter().collect(),
                numeric_lte: HashMap::new(),
                exists: vec![],
                changed_properties: vec![],
                markings: vec!["confidential".to_string()],
            },
            effect_spec: RuleEffectSpec {
                object_patch: Some(json!({ "priority": "high" })),
                schedule: None,
                alert: None,
            },
            owner_id: Uuid::nil(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let result = evaluate_rule_against_object(&rule, &sample_object(), None);
        assert!(result.matched);
        assert_eq!(
            result
                .effect_preview
                .get("object_patch")
                .and_then(|value| value.get("priority")),
            Some(&json!("high"))
        );
    }
}
