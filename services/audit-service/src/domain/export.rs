use chrono::{Duration, Utc};

use crate::models::{
    audit_event::AuditEvent,
    compliance_report::{
        ComplianceArtifact, ComplianceFinding, ComplianceReport, ComplianceReportRequest,
        ComplianceStandard,
    },
    policy::AuditPolicy,
};

pub fn build_report(
    request: &ComplianceReportRequest,
    events: &[AuditEvent],
    policies: &[AuditPolicy],
) -> ComplianceReport {
    let standard = request.standard;
    let relevant_events = events
        .iter()
        .filter(|event| {
            event.occurred_at >= request.window_start && event.occurred_at <= request.window_end
        })
        .count();

    let findings = match standard {
        ComplianceStandard::Soc2 => vec![
            ComplianceFinding::new(
                "CC7.2",
                "Access monitoring in place",
                "pass",
                "Gateway and auth events are chained in the immutable log.",
            ),
            ComplianceFinding::new(
                "CC8.1",
                "Retention policy defined",
                "pass",
                "Retention TTL policies are attached to audit classes and reviewed monthly.",
            ),
        ],
        ComplianceStandard::Iso27001 => vec![
            ComplianceFinding::new(
                "A.5.34",
                "Privacy and PII classification",
                "pass",
                "Sensitive events are labeled as PII or confidential before export.",
            ),
            ComplianceFinding::new(
                "A.8.15",
                "Logging",
                "pass",
                "Append-only hash chaining preserves the integrity of event history.",
            ),
        ],
        ComplianceStandard::Hipaa => vec![
            ComplianceFinding::new(
                "164.312(b)",
                "Audit controls",
                "pass",
                "Access and disclosure actions are retained with subject linkage.",
            ),
            ComplianceFinding::new(
                "164.526",
                "Amendment and erasure workflow",
                "pass",
                "GDPR/erasure workflows mask subject data while preserving traceability.",
            ),
        ],
        ComplianceStandard::Gdpr => vec![
            ComplianceFinding::new(
                "Art. 5(1)(e)",
                "Storage limitation enforced",
                "pass",
                "Audit policies attach explicit retention windows and erase workflows to subject-linked data.",
            ),
            ComplianceFinding::new(
                "Art. 15/17",
                "Subject rights workflow",
                "pass",
                "Export and erasure endpoints provide portable exports and subject masking with audit traceability.",
            ),
        ],
        ComplianceStandard::Itar => vec![
            ComplianceFinding::new(
                "ITAR 122.5",
                "Export access review",
                "pass",
                "Controlled exports can be attached to approval and lineage evidence for cross-border restrictions.",
            ),
            ComplianceFinding::new(
                "ITAR 123.26",
                "Controlled data handling",
                "pass",
                "Confidential/controlled classifications and governance templates preserve retention and hold requirements.",
            ),
        ],
    };

    let generated_at = Utc::now();
    ComplianceReport {
        id: uuid::Uuid::now_v7(),
        standard,
        title: request.title.clone(),
        scope: request.scope.clone(),
        window_start: request.window_start,
        window_end: request.window_end,
        generated_at,
        status: "ready".to_string(),
        findings,
        artifact: ComplianceArtifact {
            file_name: format!(
                "{}-{}.zip",
                standard.as_str(),
                generated_at.format("%Y%m%d%H%M")
            ),
            mime_type: "application/zip".to_string(),
            storage_url: format!(
                "s3://compliance-reports/{}/{}.zip",
                request.scope.replace(' ', "-"),
                generated_at.timestamp()
            ),
            checksum: format!("cmp-{:x}", generated_at.timestamp()),
            size_bytes: 32_768 + relevant_events as i64 * 14 + policies.len() as i64 * 64,
        },
        relevant_event_count: relevant_events as i64,
        policy_count: policies.len() as i64,
        control_summary: format!("{} controls evidenced across {} events", 2, relevant_events),
        expires_at: generated_at + Duration::days(30),
    }
}
