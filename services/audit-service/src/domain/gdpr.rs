use chrono::{Duration, Utc};

use crate::models::{
    audit_event::AuditEvent,
    compliance_report::{
        GdprEraseRequest, GdprEraseResponse, GdprExportPayload, GdprExportRequest,
    },
};

pub fn export_payload(request: &GdprExportRequest, events: &[AuditEvent]) -> GdprExportPayload {
    let matching = matching_events(&request.subject_id, events);
    GdprExportPayload {
        subject_id: request.subject_id.clone(),
        generated_at: Utc::now(),
        portable_format: request.portable_format.clone(),
        event_count: matching.len(),
        resources: matching
            .iter()
            .map(|event| format!("{}:{}", event.resource_type, event.resource_id))
            .collect(),
        audit_excerpt: matching.into_iter().take(12).collect(),
    }
}

pub fn erase_response(request: &GdprEraseRequest, events: &[AuditEvent]) -> GdprEraseResponse {
    let matching = matching_events(&request.subject_id, events);
    let affected_resources = matching
        .iter()
        .map(|event| format!("{}:{}", event.resource_type, event.resource_id))
        .collect();

    GdprEraseResponse {
        subject_id: request.subject_id.clone(),
        requested_at: Utc::now(),
        completed_at: Some(Utc::now() + Duration::minutes(2)),
        status: if request.hard_delete {
            "scheduled-redaction".to_string()
        } else {
            "masked".to_string()
        },
        masked_event_count: matching.len(),
        affected_resources,
        legal_hold: request.legal_hold,
    }
}

fn matching_events<'a>(subject_id: &str, events: &'a [AuditEvent]) -> Vec<AuditEvent> {
    events
        .iter()
        .filter(|event| event.subject_id.as_deref() == Some(subject_id))
        .cloned()
        .collect()
}
