use chrono::{Duration, Utc};

use crate::models::{audit_event::AuditEvent, policy::CollectorStatus};

pub fn collector_catalog(events: &[AuditEvent]) -> Vec<CollectorStatus> {
    let services = [
        ("gateway", "of.audit.gateway", true),
        ("auth-service", "of.audit.auth", false),
        ("dataset-service", "of.audit.datasets", false),
        ("workflow-service", "of.audit.workflows", false),
        ("notification-service", "of.audit.notifications", false),
    ];

    services
        .iter()
        .map(|(service, subject, connected)| {
            let service_events = events
                .iter()
                .filter(|event| event.source_service == *service)
                .count();
            CollectorStatus {
                service_name: (*service).to_string(),
                subject: (*subject).to_string(),
                connected: *connected || service_events > 0,
                last_event_at: events
                    .iter()
                    .filter(|event| event.source_service == *service)
                    .map(|event| event.occurred_at)
                    .max(),
                backlog_depth: if service_events == 0 {
                    2
                } else {
                    (service_events % 5) as i32
                },
                health: if service_events > 0 {
                    "healthy".to_string()
                } else {
                    "warming".to_string()
                },
                next_pull_at: Utc::now() + Duration::seconds(30),
            }
        })
        .collect()
}
