use crate::models::{
    audit_event::{AuditEvent, AuditSeverity},
    data_classification::AnomalyAlert,
};

pub fn detect_anomalies(events: &[AuditEvent]) -> Vec<AnomalyAlert> {
    let mut alerts = Vec::new();

    for event in events {
        if event.severity == AuditSeverity::Critical
            || event
                .labels
                .iter()
                .any(|label| label == "contains-sensitive-data")
        {
            alerts.push(AnomalyAlert {
                id: uuid::Uuid::now_v7(),
                title: format!("Sensitive access pattern: {}", event.action),
                description: format!(
                    "{} touched {}:{} from {}",
                    event.actor, event.resource_type, event.resource_id, event.source_service
                ),
                severity: if event.severity == AuditSeverity::Critical {
                    "critical".to_string()
                } else {
                    "elevated".to_string()
                },
                detected_at: event.ingested_at,
                correlation_key: format!("{}:{}", event.source_service, event.action),
                linked_event_id: event.id,
                recommended_action:
                    "Review actor session, confirm data minimization, and verify policy coverage."
                        .to_string(),
            });
        }
    }

    alerts.truncate(8);
    alerts
}
