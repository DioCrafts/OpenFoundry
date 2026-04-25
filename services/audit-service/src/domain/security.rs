use auth_middleware::Claims;
use serde_json::Value;
use uuid::Uuid;

use crate::models::{audit_event::AuditEvent, data_classification::ClassificationLevel};

pub fn filter_events_for_claims(events: Vec<AuditEvent>, claims: &Claims) -> Vec<AuditEvent> {
    events
        .into_iter()
        .filter(|event| can_access_event(event, claims))
        .collect()
}

pub fn can_access_event(event: &AuditEvent, claims: &Claims) -> bool {
    if claims.has_role("admin") {
        return true;
    }
    if classification_rank(event.classification) > clearance_rank(claims) {
        return false;
    }
    if !claims.allows_subject_id(event.subject_id.as_deref()) {
        return false;
    }

    let allowed_org_ids = claims.allowed_org_ids();
    if allowed_org_ids.is_empty() {
        return true;
    }

    match metadata_org_id(&event.metadata) {
        Some(org_id) => allowed_org_ids.contains(&org_id),
        None => !claims.is_guest_session() && event.classification == ClassificationLevel::Public,
    }
}

pub fn can_access_subject(claims: &Claims, subject_id: &str) -> bool {
    claims.has_role("admin") || claims.allows_subject_id(Some(subject_id))
}

fn clearance_rank(claims: &Claims) -> u8 {
    claims
        .classification_clearance()
        .and_then(marking_rank)
        .unwrap_or(0)
}

fn classification_rank(value: ClassificationLevel) -> u8 {
    marking_rank(value.as_str()).unwrap_or(0)
}

fn marking_rank(value: &str) -> Option<u8> {
    match value {
        "public" => Some(0),
        "confidential" => Some(1),
        "pii" => Some(2),
        _ => None,
    }
}

fn metadata_org_id(metadata: &Value) -> Option<Uuid> {
    metadata
        .get("organization_id")
        .or_else(|| metadata.get("org_id"))
        .and_then(Value::as_str)
        .and_then(|value| Uuid::parse_str(value).ok())
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use serde_json::json;

    use super::*;
    use crate::models::audit_event::{AuditEvent, AuditEventStatus, AuditSeverity};

    fn claims(clearance: &str) -> Claims {
        Claims {
            sub: Uuid::nil(),
            iat: 0,
            exp: i64::MAX,
            iss: None,
            aud: None,
            jti: Uuid::nil(),
            email: "user@example.com".to_string(),
            name: "User".to_string(),
            roles: vec!["viewer".to_string()],
            permissions: vec![],
            org_id: Some(Uuid::nil()),
            attributes: json!({ "classification_clearance": clearance }),
            auth_methods: vec!["password".to_string()],
            token_use: Some("access".to_string()),
            api_key_id: None,
            session_kind: None,
            session_scope: None,
        }
    }

    fn event(classification: ClassificationLevel) -> AuditEvent {
        AuditEvent {
            id: Uuid::now_v7(),
            sequence: 1,
            previous_hash: "prev".to_string(),
            entry_hash: "hash".to_string(),
            source_service: "gateway".to_string(),
            channel: "api".to_string(),
            actor: "user".to_string(),
            action: "read".to_string(),
            resource_type: "dataset".to_string(),
            resource_id: "123".to_string(),
            status: AuditEventStatus::Success,
            severity: AuditSeverity::Low,
            classification,
            subject_id: Some("subject-1".to_string()),
            ip_address: None,
            location: None,
            metadata: json!({ "organization_id": Uuid::nil() }),
            labels: vec![],
            retention_until: Utc::now(),
            occurred_at: Utc::now(),
            ingested_at: Utc::now(),
        }
    }

    #[test]
    fn classification_clearance_filters_events() {
        assert!(can_access_event(
            &event(ClassificationLevel::Public),
            &claims("public")
        ));
        assert!(!can_access_event(
            &event(ClassificationLevel::Pii),
            &claims("confidential")
        ));
    }
}
