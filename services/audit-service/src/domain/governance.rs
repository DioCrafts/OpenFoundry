use crate::models::{
    compliance_report::{ComplianceReport, ComplianceStandard},
    governance_posture::{
        CompliancePostureOverview, CompliancePostureStandard, GovernanceTemplateApplication,
    },
    governance_template::GovernanceTemplate,
    policy::AuditPolicy,
};

pub fn build_compliance_posture(
    templates: &[GovernanceTemplate],
    applications: &[GovernanceTemplateApplication],
    policies: &[AuditPolicy],
    reports: &[ComplianceReport],
) -> CompliancePostureOverview {
    let standards = [
        ComplianceStandard::Soc2,
        ComplianceStandard::Iso27001,
        ComplianceStandard::Hipaa,
        ComplianceStandard::Gdpr,
        ComplianceStandard::Itar,
    ]
    .into_iter()
    .map(|standard| {
        let template_matches = templates
            .iter()
            .filter(|template| {
                template
                    .standards
                    .iter()
                    .any(|candidate| candidate.eq_ignore_ascii_case(standard.as_str()))
            })
            .collect::<Vec<_>>();

        let application_matches = applications
            .iter()
            .filter(|application| application.default_report_standard == standard)
            .collect::<Vec<_>>();

        let active_policy_count = application_matches
            .iter()
            .flat_map(|application| application.policy_names.iter())
            .filter(|policy_name| {
                policies
                    .iter()
                    .any(|policy| policy.active && policy.name == **policy_name)
            })
            .count() as i64;

        let latest_report = reports
            .iter()
            .filter(|report| report.standard == standard)
            .max_by_key(|report| report.generated_at);

        let checkpoint_prompt_count = application_matches
            .iter()
            .map(|application| application.checkpoint_prompts.len() as i64)
            .sum::<i64>();
        let sds_remediation_count = application_matches
            .iter()
            .map(|application| application.sds_remediations.len() as i64)
            .sum::<i64>();

        let mut coverage_score = 0;
        if !template_matches.is_empty() {
            coverage_score += 25;
        }
        if !application_matches.is_empty() {
            coverage_score += 30;
        }
        if active_policy_count > 0 {
            coverage_score += 25;
        }
        if latest_report.is_some() {
            coverage_score += 20;
        }

        CompliancePostureStandard {
            standard,
            template_available: !template_matches.is_empty(),
            applied_scope_count: application_matches.len() as i64,
            active_policy_count,
            latest_report_status: latest_report.map(|report| report.status.clone()),
            latest_report_generated_at: latest_report.map(|report| report.generated_at),
            coverage_score,
            checkpoint_prompt_count,
            sds_remediation_count,
            evidence_summary: format!(
                "{} template(s), {} scope(s), {} active policy mapping(s), {} report(s)",
                template_matches.len(),
                application_matches.len(),
                active_policy_count,
                reports
                    .iter()
                    .filter(|report| report.standard == standard)
                    .count()
            ),
        }
    })
    .collect::<Vec<_>>();

    CompliancePostureOverview {
        standards,
        supported_capabilities: vec![
            "immutable_audit_log".to_string(),
            "approval_workflows".to_string(),
            "checkpoint_prompts".to_string(),
            "cipher_operations".to_string(),
            "sensitive_data_scanner".to_string(),
            "retention_policies".to_string(),
            "gdpr_subject_rights".to_string(),
            "governance_project_templates".to_string(),
        ],
        active_template_application_count: applications.len() as i64,
        active_legal_hold_policy_count: policies.iter().filter(|policy| policy.legal_hold).count()
            as i64,
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, Utc};

    use super::*;
    use crate::models::{
        compliance_report::{ComplianceArtifact, ComplianceFinding, ComplianceReport},
        data_classification::ClassificationLevel,
        governance_template::GovernanceTemplatePolicy,
    };

    #[test]
    fn posture_scores_rise_with_templates_policies_and_reports() {
        let templates = vec![GovernanceTemplate {
            slug: "gdpr-default".to_string(),
            name: "GDPR".to_string(),
            summary: "summary".to_string(),
            standards: vec!["gdpr".to_string()],
            default_report_standard: ComplianceStandard::Gdpr,
            checkpoint_prompts: vec!["justify export".to_string()],
            sds_remediations: vec!["mask pii".to_string()],
            policies: vec![GovernanceTemplatePolicy {
                name: "GDPR PII retention".to_string(),
                description: "desc".to_string(),
                scope: "datasets".to_string(),
                classification: ClassificationLevel::Pii,
                retention_days: 30,
                legal_hold: false,
                purge_mode: "mask".to_string(),
                rules: vec![],
            }],
        }];
        let applications = vec![GovernanceTemplateApplication {
            id: uuid::Uuid::nil(),
            template_slug: "gdpr-default".to_string(),
            template_name: "GDPR".to_string(),
            scope: "workspace-a".to_string(),
            standards: vec!["gdpr".to_string()],
            policy_names: vec!["GDPR PII retention".to_string()],
            checkpoint_prompts: vec!["justify export".to_string()],
            sds_remediations: vec!["mask pii".to_string()],
            default_report_standard: ComplianceStandard::Gdpr,
            applied_by: "security".to_string(),
            applied_at: Utc::now(),
            updated_at: Utc::now(),
        }];
        let policies = vec![AuditPolicy {
            id: uuid::Uuid::nil(),
            name: "GDPR PII retention".to_string(),
            description: "desc".to_string(),
            scope: "workspace-a".to_string(),
            classification: ClassificationLevel::Pii,
            retention_days: 30,
            legal_hold: false,
            purge_mode: "mask".to_string(),
            active: true,
            rules: vec![],
            updated_by: "security".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }];
        let reports = vec![ComplianceReport {
            id: uuid::Uuid::nil(),
            standard: ComplianceStandard::Gdpr,
            title: "GDPR".to_string(),
            scope: "workspace-a".to_string(),
            window_start: Utc::now() - Duration::days(30),
            window_end: Utc::now(),
            generated_at: Utc::now(),
            status: "ready".to_string(),
            findings: vec![ComplianceFinding::new("1", "title", "pass", "evidence")],
            artifact: ComplianceArtifact {
                file_name: "artifact.zip".to_string(),
                mime_type: "application/zip".to_string(),
                storage_url: "s3://artifact.zip".to_string(),
                checksum: "abc".to_string(),
                size_bytes: 1,
            },
            relevant_event_count: 1,
            policy_count: 1,
            control_summary: "summary".to_string(),
            expires_at: Utc::now(),
        }];

        let posture = build_compliance_posture(&templates, &applications, &policies, &reports);
        let gdpr = posture
            .standards
            .iter()
            .find(|entry| entry.standard == ComplianceStandard::Gdpr)
            .expect("gdpr posture");

        assert!(gdpr.coverage_score >= 100);
        assert_eq!(gdpr.applied_scope_count, 1);
        assert_eq!(gdpr.active_policy_count, 1);
    }
}
