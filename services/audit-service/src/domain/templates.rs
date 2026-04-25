use crate::models::{
    data_classification::ClassificationLevel,
    governance_template::{GovernanceTemplate, GovernanceTemplatePolicy},
};

pub fn governance_template_catalog() -> Vec<GovernanceTemplate> {
    vec![
        GovernanceTemplate {
            slug: "gdpr-default".to_string(),
            name: "GDPR Data Stewardship".to_string(),
            summary: "Retention, subject-rights handling, and PII controls for GDPR workloads."
                .to_string(),
            standards: vec!["gdpr".to_string()],
            policies: vec![
                GovernanceTemplatePolicy {
                    name: "GDPR PII retention".to_string(),
                    description: "Short-lived retention window for personal data processing."
                        .to_string(),
                    scope: "datasets".to_string(),
                    classification: ClassificationLevel::Pii,
                    retention_days: 30,
                    legal_hold: false,
                    purge_mode: "mask".to_string(),
                    rules: vec![
                        "require_subject_id".to_string(),
                        "enable_erasure_workflow".to_string(),
                    ],
                },
                GovernanceTemplatePolicy {
                    name: "GDPR disclosure logging".to_string(),
                    description: "Track exports and disclosure events with subject linkage."
                        .to_string(),
                    scope: "audit".to_string(),
                    classification: ClassificationLevel::Pii,
                    retention_days: 365,
                    legal_hold: true,
                    purge_mode: "retain".to_string(),
                    rules: vec![
                        "log_export_actions".to_string(),
                        "notify_privacy_officer".to_string(),
                    ],
                },
            ],
        },
        GovernanceTemplate {
            slug: "hipaa-baseline".to_string(),
            name: "HIPAA Security Baseline".to_string(),
            summary: "Audit retention and protected health information handling for HIPAA."
                .to_string(),
            standards: vec!["hipaa".to_string()],
            policies: vec![
                GovernanceTemplatePolicy {
                    name: "HIPAA PHI access logs".to_string(),
                    description: "Retain audit trails for access to protected health information."
                        .to_string(),
                    scope: "audit".to_string(),
                    classification: ClassificationLevel::Pii,
                    retention_days: 2190,
                    legal_hold: true,
                    purge_mode: "retain".to_string(),
                    rules: vec![
                        "log_all_phi_access".to_string(),
                        "require_mfa".to_string(),
                    ],
                },
                GovernanceTemplatePolicy {
                    name: "HIPAA incident review".to_string(),
                    description: "Classify and review anomalous PHI activity with escalation."
                        .to_string(),
                    scope: "security".to_string(),
                    classification: ClassificationLevel::Confidential,
                    retention_days: 365,
                    legal_hold: true,
                    purge_mode: "retain".to_string(),
                    rules: vec![
                        "monitor_anomalies".to_string(),
                        "escalate_phi_incidents".to_string(),
                    ],
                },
            ],
        },
        GovernanceTemplate {
            slug: "itar-export-control".to_string(),
            name: "ITAR Export Control".to_string(),
            summary: "Restrict export, legal hold, and controlled-data handling for ITAR."
                .to_string(),
            standards: vec!["itar".to_string()],
            policies: vec![
                GovernanceTemplatePolicy {
                    name: "ITAR export gate".to_string(),
                    description: "Block uncontrolled exports and preserve evidence for review."
                        .to_string(),
                    scope: "exports".to_string(),
                    classification: ClassificationLevel::Confidential,
                    retention_days: 2555,
                    legal_hold: true,
                    purge_mode: "retain".to_string(),
                    rules: vec![
                        "require_export_approval".to_string(),
                        "deny_cross_border_export".to_string(),
                    ],
                },
                GovernanceTemplatePolicy {
                    name: "ITAR controlled lineage".to_string(),
                    description: "Retain lineage and transformation history for controlled assets."
                        .to_string(),
                    scope: "lineage".to_string(),
                    classification: ClassificationLevel::Confidential,
                    retention_days: 2555,
                    legal_hold: true,
                    purge_mode: "retain".to_string(),
                    rules: vec![
                        "preserve_lineage".to_string(),
                        "require_clearance_for_builds".to_string(),
                    ],
                },
            ],
        },
    ]
}

pub fn find_governance_template(slug: &str) -> Option<GovernanceTemplate> {
    governance_template_catalog()
        .into_iter()
        .find(|template| template.slug == slug)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn governance_templates_cover_core_standards() {
        let catalog = governance_template_catalog();
        assert!(catalog.iter().any(|template| template.slug == "gdpr-default"));
        assert!(catalog.iter().any(|template| template.slug == "hipaa-baseline"));
        assert!(catalog.iter().any(|template| template.slug == "itar-export-control"));
    }
}
