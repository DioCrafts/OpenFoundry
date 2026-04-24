use crate::models::{
    cluster::MatchEvidence,
    match_rule::{MatchCondition, MatchRule},
};

use super::{blocking::CandidatePair, comparator};

pub fn evaluate_candidate(rule: &MatchRule, pair: &CandidatePair) -> MatchEvidence {
    let total_weight = rule
        .conditions
        .iter()
        .map(|condition| condition.weight.max(0.0))
        .sum::<f32>()
        .max(1.0);
    let mut matched_weight = 0.0;
    let mut explanations = Vec::new();
    let mut required_miss = false;

    for condition in &rule.conditions {
        let explanation = score_condition(condition, pair);
        if explanation.0 >= condition.threshold {
            matched_weight += condition.weight.max(0.0);
        } else if condition.required {
            required_miss = true;
        }
        explanations.push(explanation.1);
    }

    let mut rule_score = (matched_weight / total_weight).clamp(0.0, 1.0);
    if required_miss {
        rule_score *= 0.45;
    }

    MatchEvidence {
        left_record_id: pair.left.record_id.clone(),
        right_record_id: pair.right.record_id.clone(),
        blocking_key: pair.blocking_key.clone(),
        rule_score,
        ml_score: 0.0,
        final_score: rule_score,
        comparators: explanations.clone(),
        explanation: explanations.join("; "),
        requires_review: false,
    }
}

fn score_condition(condition: &MatchCondition, pair: &CandidatePair) -> (f32, String) {
    let left_value = field_value(&pair.left, &condition.field);
    let right_value = field_value(&pair.right, &condition.field);
    let score = comparator::compare_values(&condition.comparator, &left_value, &right_value);
    let explanation = format!("{}:{}={score:.2}", condition.field, condition.comparator,);
    (score, explanation)
}

fn field_value(record: &crate::models::cluster::EntityRecord, field: &str) -> String {
    match field {
        "display_name" | "name" => record.display_name.clone(),
        _ => record
            .attributes
            .get(field)
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_string(),
    }
}
