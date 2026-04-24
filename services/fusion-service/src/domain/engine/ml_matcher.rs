use crate::models::cluster::{EntityRecord, MatchEvidence};

use super::comparator::{compare_values, normalize_phone};

pub fn score_candidate(left: &EntityRecord, right: &EntityRecord, evidence: &MatchEvidence) -> f32 {
    let name_score = compare_values("fuzzy", &left.display_name, &right.display_name);
    let email_score = compare_values(
        "email_exact",
        &attribute(left, "email"),
        &attribute(right, "email"),
    );
    let phone_score = if normalize_phone(&attribute(left, "phone"))
        == normalize_phone(&attribute(right, "phone"))
    {
        1.0
    } else {
        0.0
    };
    let company_score = compare_values(
        "fuzzy",
        &attribute(left, "company"),
        &attribute(right, "company"),
    );
    let city_score = compare_values("exact", &attribute(left, "city"), &attribute(right, "city"));

    (0.45 * evidence.rule_score
        + 0.2 * name_score
        + 0.15 * email_score
        + 0.1 * phone_score
        + 0.05 * company_score
        + 0.05 * city_score)
        .clamp(0.0, 1.0)
}

pub fn blend_scores(rule_score: f32, ml_score: f32) -> f32 {
    (0.65 * rule_score + 0.35 * ml_score).clamp(0.0, 1.0)
}

fn attribute(record: &EntityRecord, field: &str) -> String {
    record
        .attributes
        .get(field)
        .and_then(|value| value.as_str())
        .unwrap_or_default()
        .to_string()
}
