use crate::models::audit_event::AuditEvent;

pub fn next_sequence(previous_sequence: Option<i64>) -> i64 {
    previous_sequence.unwrap_or(0) + 1
}

pub fn previous_hash_value(previous_hash: Option<&str>) -> String {
    previous_hash.unwrap_or("GENESIS").to_string()
}

pub fn chain_hash(
    sequence: i64,
    previous_hash: &str,
    source_service: &str,
    action: &str,
) -> String {
    format!(
        "AUD-{sequence:08x}-{}-{}",
        normalize(previous_hash),
        normalize(&format!("{source_service}-{action}"))
    )
}

pub fn label_event(event: &AuditEvent) -> Vec<String> {
    let mut labels = event.labels.clone();
    if event.classification.requires_masking() {
        labels.push("contains-sensitive-data".to_string());
    }
    if event.subject_id.is_some() {
        labels.push("gdpr-subject-linked".to_string());
    }
    labels.sort();
    labels.dedup();
    labels
}

fn normalize(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .take(8)
        .collect::<String>()
        .to_uppercase()
}
