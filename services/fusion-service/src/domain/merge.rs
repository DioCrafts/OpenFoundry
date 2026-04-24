use std::collections::BTreeMap;

use chrono::Utc;
use serde_json::{Map, Value};
use uuid::Uuid;

use crate::models::{
    cluster::{EntityRecord, ResolvedCluster},
    golden_record::{GoldenRecord, GoldenRecordProvenance},
    merge_strategy::{MergeStrategy, SurvivorshipRule},
};

use super::engine::comparator::normalize_text;

pub fn synthesize_golden_record(
    cluster: &ResolvedCluster,
    strategy: &MergeStrategy,
) -> GoldenRecord {
    let now = Utc::now();
    let field_rules = if strategy.rules.is_empty() {
        default_survivorship_rules()
    } else {
        strategy.rules.clone()
    };

    let mut canonical_values = Map::new();
    let mut provenance = Vec::new();

    for rule in &field_rules {
        if let Some(selected) = select_value(&cluster.records, rule, &strategy.default_strategy) {
            canonical_values.insert(rule.field.clone(), selected.value.clone());
            provenance.push(GoldenRecordProvenance {
                field: rule.field.clone(),
                source: selected.source,
                external_id: selected.external_id,
                strategy: selected.strategy,
            });
        }
    }

    if !canonical_values.contains_key("display_name") {
        if let Some(record) = cluster.records.first() {
            canonical_values.insert(
                "display_name".to_string(),
                Value::String(record.display_name.clone()),
            );
        }
    }

    let title = canonical_values
        .get("display_name")
        .or_else(|| canonical_values.get("name"))
        .and_then(|value| value.as_str())
        .unwrap_or("Golden Record")
        .to_string();

    let completeness_score =
        (canonical_values.len() as f32 / field_rules.len().max(1) as f32).clamp(0.0, 1.0);

    GoldenRecord {
        id: Uuid::now_v7(),
        cluster_id: cluster.id,
        title,
        canonical_values: Value::Object(canonical_values),
        provenance,
        completeness_score,
        confidence_score: cluster.confidence_score,
        status: if cluster.status == "rejected" {
            "rejected"
        } else {
            "active"
        }
        .to_string(),
        created_at: now,
        updated_at: now,
    }
}

#[derive(Debug)]
struct SelectedValue {
    value: Value,
    source: String,
    external_id: String,
    strategy: String,
}

fn select_value(
    records: &[EntityRecord],
    rule: &SurvivorshipRule,
    default_strategy: &str,
) -> Option<SelectedValue> {
    let strategy = if rule.strategy.trim().is_empty() {
        default_strategy.to_string()
    } else {
        rule.strategy.clone()
    };

    match strategy.as_str() {
        "source_priority" => select_source_priority(records, rule),
        "highest_confidence" => select_highest_confidence(records, &rule.field, &strategy),
        "most_common" => select_most_common(records, &rule.field, &strategy),
        _ => select_longest_non_empty(records, &rule.field, &strategy),
    }
}

fn select_source_priority(
    records: &[EntityRecord],
    rule: &SurvivorshipRule,
) -> Option<SelectedValue> {
    for source in &rule.source_priority {
        for record in records {
            if &record.source != source {
                continue;
            }

            if let Some(value) = extract_value(record, &rule.field) {
                return Some(SelectedValue {
                    value,
                    source: record.source.clone(),
                    external_id: record.external_id.clone(),
                    strategy: "source_priority".to_string(),
                });
            }
        }
    }

    select_longest_non_empty(records, &rule.field, &rule.fallback)
}

fn select_highest_confidence(
    records: &[EntityRecord],
    field: &str,
    strategy: &str,
) -> Option<SelectedValue> {
    records
        .iter()
        .filter_map(|record| extract_value(record, field).map(|value| (record, value)))
        .max_by(|left, right| left.0.confidence.total_cmp(&right.0.confidence))
        .map(|(record, value)| SelectedValue {
            value,
            source: record.source.clone(),
            external_id: record.external_id.clone(),
            strategy: strategy.to_string(),
        })
}

fn select_longest_non_empty(
    records: &[EntityRecord],
    field: &str,
    strategy: &str,
) -> Option<SelectedValue> {
    records
        .iter()
        .filter_map(|record| extract_value(record, field).map(|value| (record, value)))
        .max_by(|left, right| value_length(&left.1).cmp(&value_length(&right.1)))
        .map(|(record, value)| SelectedValue {
            value,
            source: record.source.clone(),
            external_id: record.external_id.clone(),
            strategy: strategy.to_string(),
        })
}

fn select_most_common(
    records: &[EntityRecord],
    field: &str,
    strategy: &str,
) -> Option<SelectedValue> {
    let mut counts = BTreeMap::<String, (usize, &EntityRecord, Value)>::new();

    for record in records {
        let Some(value) = extract_value(record, field) else {
            continue;
        };
        let normalized = normalize_text(value.as_str().unwrap_or_default());
        let entry = counts
            .entry(normalized)
            .or_insert((0, record, value.clone()));
        entry.0 += 1;
    }

    counts
        .into_values()
        .max_by(|left, right| left.0.cmp(&right.0))
        .map(|(_, record, value)| SelectedValue {
            value,
            source: record.source.clone(),
            external_id: record.external_id.clone(),
            strategy: strategy.to_string(),
        })
}

fn extract_value(record: &EntityRecord, field: &str) -> Option<Value> {
    match field {
        "display_name" | "name" => Some(Value::String(record.display_name.clone())),
        _ => record.attributes.get(field).cloned().and_then(|value| {
            if value.is_null() {
                None
            } else if value.as_str().is_some_and(|value| value.trim().is_empty()) {
                None
            } else {
                Some(value)
            }
        }),
    }
}

fn value_length(value: &Value) -> usize {
    value.as_str().map(str::len).unwrap_or(0)
}

fn default_survivorship_rules() -> Vec<SurvivorshipRule> {
    vec![
        SurvivorshipRule {
            field: "display_name".to_string(),
            strategy: "longest_non_empty".to_string(),
            source_priority: vec!["crm".to_string(), "erp".to_string(), "support".to_string()],
            fallback: "highest_confidence".to_string(),
        },
        SurvivorshipRule {
            field: "email".to_string(),
            strategy: "source_priority".to_string(),
            source_priority: vec!["crm".to_string(), "erp".to_string(), "support".to_string()],
            fallback: "most_common".to_string(),
        },
        SurvivorshipRule {
            field: "phone".to_string(),
            strategy: "most_common".to_string(),
            source_priority: vec![],
            fallback: "longest_non_empty".to_string(),
        },
        SurvivorshipRule {
            field: "company".to_string(),
            strategy: "most_common".to_string(),
            source_priority: vec![],
            fallback: "longest_non_empty".to_string(),
        },
    ]
}
