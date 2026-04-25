use serde_json::{Map, Value, json};
use uuid::Uuid;

use crate::models::{
    control_panel::{
        IdentityProviderMapping, IdentityProviderMappingPreviewResponse,
        IdentityProviderOrganizationRule, IdentityProviderRuleMatchType, ResourceManagementPolicy,
        ResourceQuotaSettings,
    },
    sso::SsoProvider,
};

#[derive(Debug, Clone)]
pub struct IdentityProviderAssignment {
    pub provider_slug: String,
    pub mapping_found: bool,
    pub matched_rule_name: Option<String>,
    pub organization_id: Option<Uuid>,
    pub workspace: Option<String>,
    pub classification_clearance: Option<String>,
    pub role_names: Vec<String>,
    pub tenant_tier: Option<String>,
    pub resource_policy_name: Option<String>,
    pub quota: Option<ResourceQuotaSettings>,
    pub notes: Vec<String>,
}

impl IdentityProviderAssignment {
    pub fn into_preview_response(self, email: String) -> IdentityProviderMappingPreviewResponse {
        IdentityProviderMappingPreviewResponse {
            provider_slug: self.provider_slug,
            email,
            mapping_found: self.mapping_found,
            matched_rule_name: self.matched_rule_name,
            organization_id: self.organization_id,
            workspace: self.workspace,
            classification_clearance: self.classification_clearance,
            role_names: self.role_names,
            tenant_tier: self.tenant_tier,
            resource_policy_name: self.resource_policy_name,
            quota: self.quota,
            notes: self.notes,
        }
    }

    pub fn to_attributes(&self, raw_claims: &Value) -> Result<Value, String> {
        let mut attributes = match raw_claims {
            Value::Object(object) => object.clone(),
            _ => Map::new(),
        };
        attributes.insert("identity_provider".to_string(), json!(self.provider_slug));

        if let Some(workspace) = self.workspace.as_deref() {
            attributes.insert("workspace".to_string(), json!(workspace));
        }
        if let Some(classification_clearance) = self.classification_clearance.as_deref() {
            attributes.insert(
                "classification_clearance".to_string(),
                json!(classification_clearance),
            );
        }
        if let Some(tenant_tier) = self.tenant_tier.as_deref() {
            attributes.insert("tenant_tier".to_string(), json!(tenant_tier));
        }
        if let Some(resource_policy_name) = self.resource_policy_name.as_deref() {
            attributes.insert("resource_policy".to_string(), json!(resource_policy_name));
        }
        if let Some(quota) = self.quota.as_ref() {
            attributes.insert(
                "tenant_quotas".to_string(),
                serde_json::to_value(quota)
                    .map_err(|cause| format!("invalid resource quota policy: {cause}"))?,
            );
        }
        if let Some(rule_name) = self.matched_rule_name.as_deref() {
            attributes.insert("matched_idp_rule".to_string(), json!(rule_name));
        }

        Ok(Value::Object(attributes))
    }
}

pub fn resolve_identity_provider_assignment(
    provider: &SsoProvider,
    mapping: Option<&IdentityProviderMapping>,
    email: &str,
    raw_claims: &Value,
    policies: &[ResourceManagementPolicy],
) -> Result<IdentityProviderAssignment, String> {
    validate_allowed_email_domains(email, mapping)?;

    let matched_rule = mapping.and_then(|entry| match_organization_rule(entry, email, raw_claims));
    let organization_id = resolve_organization_id(provider, mapping, matched_rule, raw_claims)?;
    let workspace = matched_rule
        .and_then(|rule| rule.workspace.clone())
        .or_else(|| {
            resolve_string_claim(
                raw_claims,
                mapping.and_then(|entry| entry.workspace_claim.as_deref()),
                provider
                    .attribute_mapping
                    .get("workspace")
                    .and_then(Value::as_str),
            )
        })
        .or_else(|| mapping.and_then(|entry| entry.default_workspace.clone()));
    let classification_clearance = matched_rule
        .and_then(|rule| rule.classification_clearance.clone())
        .or_else(|| {
            resolve_string_claim(
                raw_claims,
                mapping.and_then(|entry| entry.classification_clearance_claim.as_deref()),
                provider
                    .attribute_mapping
                    .get("classification_clearance")
                    .and_then(Value::as_str),
            )
        })
        .or_else(|| mapping.and_then(|entry| entry.default_classification_clearance.clone()));

    let mut role_names = parse_role_claim(
        raw_claims,
        mapping.and_then(|entry| entry.role_claim.as_deref()),
    );
    if let Some(entry) = mapping {
        role_names.extend(entry.default_roles.clone());
    }
    if let Some(rule) = matched_rule {
        role_names.extend(rule.roles.clone());
    }
    role_names.sort();
    role_names.dedup();

    let matched_policy = match_resource_policy(policies, organization_id, workspace.as_deref());
    let mut notes = Vec::new();
    if mapping.is_none() {
        notes.push(
            "No control-panel mapping configured for this provider; using provider attribute mapping only."
                .to_string(),
        );
    }
    if let Some(rule) = matched_rule {
        notes.push(format!(
            "Matched organization rule '{}' for provider {}.",
            rule.name, provider.slug
        ));
    }
    if let Some(policy) = matched_policy {
        notes.push(format!(
            "Resource policy '{}' matched for tenant assignment.",
            policy.name
        ));
    } else {
        notes.push(
            "No resource management policy matched the resolved org/workspace scope.".to_string(),
        );
    }

    let rule_tenant_tier = matched_rule.and_then(|rule| rule.tenant_tier.clone());
    if let (Some(rule_tenant_tier), Some(policy)) = (rule_tenant_tier.as_deref(), matched_policy) {
        if policy.tenant_tier != rule_tenant_tier {
            notes.push(format!(
                "Organization rule overrides policy tenant tier '{}' with '{}'.",
                policy.tenant_tier, rule_tenant_tier
            ));
        }
    }

    Ok(IdentityProviderAssignment {
        provider_slug: provider.slug.clone(),
        mapping_found: mapping.is_some(),
        matched_rule_name: matched_rule.map(|rule| rule.name.clone()),
        organization_id,
        workspace,
        classification_clearance,
        role_names,
        tenant_tier: rule_tenant_tier
            .or_else(|| matched_policy.map(|policy| policy.tenant_tier.clone())),
        resource_policy_name: matched_policy.map(|policy| policy.name.clone()),
        quota: matched_policy.map(|policy| policy.quota.clone()),
        notes,
    })
}

pub fn validate_allowed_email_domains(
    email: &str,
    mapping: Option<&IdentityProviderMapping>,
) -> Result<(), String> {
    let Some(mapping) = mapping else {
        return Ok(());
    };
    if mapping.allowed_email_domains.is_empty() {
        return Ok(());
    }

    let domain = email_domain(email);
    if mapping
        .allowed_email_domains
        .iter()
        .any(|candidate| candidate.eq_ignore_ascii_case(&domain))
    {
        Ok(())
    } else {
        Err(format!(
            "email domain '{domain}' is not allowed for provider {}",
            mapping.provider_slug
        ))
    }
}

pub fn resolve_organization_id(
    provider: &SsoProvider,
    mapping: Option<&IdentityProviderMapping>,
    matched_rule: Option<&IdentityProviderOrganizationRule>,
    raw_claims: &Value,
) -> Result<Option<Uuid>, String> {
    if let Some(rule) = matched_rule {
        return Ok(Some(rule.organization_id));
    }

    if let Some(org_id) = resolve_string_claim(
        raw_claims,
        mapping.and_then(|entry| entry.organization_claim.as_deref()),
        provider
            .attribute_mapping
            .get("organization_id")
            .and_then(Value::as_str),
    ) {
        return Uuid::parse_str(&org_id)
            .map(Some)
            .map_err(|cause| format!("invalid organization_id claim: {cause}"));
    }

    Ok(mapping.and_then(|entry| entry.default_organization_id))
}

pub fn resolve_string_claim(
    raw_claims: &Value,
    preferred_key: Option<&str>,
    fallback_key: Option<&str>,
) -> Option<String> {
    preferred_key
        .and_then(|key| raw_claims.get(key))
        .and_then(claim_value_as_string)
        .or_else(|| {
            fallback_key
                .and_then(|key| raw_claims.get(key))
                .and_then(claim_value_as_string)
        })
}

pub fn parse_role_claim(raw_claims: &Value, key: Option<&str>) -> Vec<String> {
    let Some(value) = key.and_then(|claim| raw_claims.get(claim)) else {
        return Vec::new();
    };

    let mut roles = match value {
        Value::String(text) => text
            .split(',')
            .map(|role| role.trim().to_string())
            .filter(|role| !role.is_empty())
            .collect::<Vec<_>>(),
        Value::Array(items) => items
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_string)
            .collect::<Vec<_>>(),
        _ => Vec::new(),
    };
    roles.sort();
    roles.dedup();
    roles
}

pub fn match_resource_policy<'a>(
    policies: &'a [ResourceManagementPolicy],
    organization_id: Option<Uuid>,
    workspace: Option<&str>,
) -> Option<&'a ResourceManagementPolicy> {
    policies.iter().find(|policy| {
        let org_matches = policy.applies_to_org_ids.is_empty()
            || organization_id.is_some_and(|org_id| policy.applies_to_org_ids.contains(&org_id));
        let workspace_matches = policy.applies_to_workspaces.is_empty()
            || workspace.is_some_and(|candidate| {
                policy
                    .applies_to_workspaces
                    .iter()
                    .any(|value| value.eq_ignore_ascii_case(candidate))
            });
        org_matches && workspace_matches
    })
}

fn match_organization_rule<'a>(
    mapping: &'a IdentityProviderMapping,
    email: &str,
    raw_claims: &Value,
) -> Option<&'a IdentityProviderOrganizationRule> {
    mapping
        .organization_rules
        .iter()
        .find(|rule| organization_rule_matches(rule, email, raw_claims))
}

fn organization_rule_matches(
    rule: &IdentityProviderOrganizationRule,
    email: &str,
    raw_claims: &Value,
) -> bool {
    match rule.match_type {
        IdentityProviderRuleMatchType::EmailDomain => {
            email_domain(email).eq_ignore_ascii_case(rule.match_value.trim())
        }
        IdentityProviderRuleMatchType::ClaimEquals => {
            let Some(claim) = rule.claim.as_deref() else {
                return false;
            };
            claim_matches_value(raw_claims.get(claim), rule.match_value.as_str())
        }
    }
}

fn claim_matches_value(value: Option<&Value>, expected: &str) -> bool {
    let expected = expected.trim();
    match value {
        Some(Value::String(text)) => text.eq_ignore_ascii_case(expected),
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(claim_value_as_string)
            .any(|candidate| candidate.eq_ignore_ascii_case(expected)),
        Some(other) => claim_value_as_string(other)
            .is_some_and(|candidate| candidate.eq_ignore_ascii_case(expected)),
        None => false,
    }
}

fn claim_value_as_string(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.to_string()),
        Value::Number(number) => Some(number.to_string()),
        Value::Bool(flag) => Some(flag.to_string()),
        _ => None,
    }
}

fn email_domain(email: &str) -> String {
    email
        .rsplit_once('@')
        .map(|(_, domain)| domain.to_ascii_lowercase())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use serde_json::json;
    use uuid::Uuid;

    use crate::models::{
        control_panel::{
            IdentityProviderMapping, IdentityProviderOrganizationRule,
            IdentityProviderRuleMatchType, ResourceManagementPolicy, ResourceQuotaSettings,
        },
        sso::SsoProvider,
    };

    use super::{
        match_resource_policy, parse_role_claim, resolve_identity_provider_assignment,
        resolve_organization_id, validate_allowed_email_domains,
    };

    fn provider() -> SsoProvider {
        SsoProvider {
            id: Uuid::now_v7(),
            slug: "enterprise-saml".to_string(),
            name: "Enterprise SAML".to_string(),
            provider_type: "saml".to_string(),
            enabled: true,
            client_id: None,
            client_secret: None,
            issuer_url: None,
            authorization_url: None,
            token_url: None,
            userinfo_url: None,
            scopes: vec![],
            saml_metadata_url: None,
            saml_entity_id: None,
            saml_sso_url: None,
            saml_certificate: None,
            attribute_mapping: json!({
                "organization_id": "org_id",
                "workspace": "workspace",
                "classification_clearance": "clearance"
            }),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn parses_roles_from_csv_claim() {
        let roles = parse_role_claim(&json!({ "roles": "viewer, editor, viewer" }), Some("roles"));
        assert_eq!(roles, vec!["editor".to_string(), "viewer".to_string()]);
    }

    #[test]
    fn validates_allowed_email_domains() {
        let mapping = IdentityProviderMapping {
            provider_slug: "enterprise-saml".to_string(),
            default_organization_id: None,
            organization_claim: None,
            workspace_claim: None,
            default_workspace: None,
            classification_clearance_claim: None,
            default_classification_clearance: None,
            role_claim: None,
            default_roles: vec![],
            allowed_email_domains: vec!["openfoundry.dev".to_string()],
            organization_rules: vec![],
        };

        assert!(validate_allowed_email_domains("operator@openfoundry.dev", Some(&mapping)).is_ok());
        assert!(validate_allowed_email_domains("outsider@example.com", Some(&mapping)).is_err());
    }

    #[test]
    fn resolves_organization_from_claim() {
        let org_id = Uuid::now_v7();
        let resolved = resolve_organization_id(
            &provider(),
            None,
            None,
            &json!({ "org_id": org_id.to_string() }),
        )
        .expect("organization claim should parse");

        assert_eq!(resolved, Some(org_id));
    }

    #[test]
    fn matches_resource_policy_by_workspace() {
        let policy = ResourceManagementPolicy {
            name: "shared".to_string(),
            tenant_tier: "team".to_string(),
            applies_to_org_ids: vec![],
            applies_to_workspaces: vec!["partner-portal".to_string()],
            quota: ResourceQuotaSettings {
                max_query_limit: 5000,
                max_distributed_query_workers: 4,
                max_pipeline_workers: 4,
                max_request_body_bytes: 20 * 1024 * 1024,
                requests_per_minute: 900,
                max_storage_gb: 120,
                max_shared_spaces: 8,
                max_guest_sessions: 20,
            },
        };

        let policies = [policy];
        let matched = match_resource_policy(&policies, None, Some("partner-portal"));
        assert!(matched.is_some());
        assert_eq!(
            matched.map(|entry| entry.tenant_tier.as_str()),
            Some("team")
        );
    }

    #[test]
    fn resolves_assignment_from_claim_rule() {
        let org_id = Uuid::now_v7();
        let policy = ResourceManagementPolicy {
            name: "finance-enterprise".to_string(),
            tenant_tier: "enterprise".to_string(),
            applies_to_org_ids: vec![org_id],
            applies_to_workspaces: vec!["finance-control".to_string()],
            quota: ResourceQuotaSettings {
                max_query_limit: 10000,
                max_distributed_query_workers: 8,
                max_pipeline_workers: 8,
                max_request_body_bytes: 50 * 1024 * 1024,
                requests_per_minute: 5000,
                max_storage_gb: 500,
                max_shared_spaces: 25,
                max_guest_sessions: 50,
            },
        };
        let mapping = IdentityProviderMapping {
            provider_slug: "enterprise-saml".to_string(),
            default_organization_id: None,
            organization_claim: None,
            workspace_claim: Some("workspace".to_string()),
            default_workspace: None,
            classification_clearance_claim: Some("clearance".to_string()),
            default_classification_clearance: Some("internal".to_string()),
            role_claim: Some("roles".to_string()),
            default_roles: vec!["viewer".to_string()],
            allowed_email_domains: vec!["openfoundry.dev".to_string()],
            organization_rules: vec![IdentityProviderOrganizationRule {
                name: "Finance org".to_string(),
                match_type: IdentityProviderRuleMatchType::ClaimEquals,
                claim: Some("department".to_string()),
                match_value: "finance".to_string(),
                organization_id: org_id,
                workspace: Some("finance-control".to_string()),
                classification_clearance: Some("restricted".to_string()),
                roles: vec!["operator".to_string()],
                tenant_tier: Some("regulated".to_string()),
            }],
        };

        let assignment = resolve_identity_provider_assignment(
            &provider(),
            Some(&mapping),
            "analyst@openfoundry.dev",
            &json!({
                "department": "finance",
                "workspace": "ignored",
                "clearance": "ignored",
                "roles": ["viewer", "editor"]
            }),
            &[policy],
        )
        .expect("assignment should resolve");

        assert_eq!(assignment.organization_id, Some(org_id));
        assert_eq!(assignment.workspace.as_deref(), Some("finance-control"));
        assert_eq!(
            assignment.classification_clearance.as_deref(),
            Some("restricted")
        );
        assert_eq!(
            assignment.role_names,
            vec![
                "editor".to_string(),
                "operator".to_string(),
                "viewer".to_string()
            ]
        );
        assert_eq!(assignment.tenant_tier.as_deref(), Some("regulated"));
        assert_eq!(
            assignment.resource_policy_name.as_deref(),
            Some("finance-enterprise")
        );
        assert_eq!(assignment.matched_rule_name.as_deref(), Some("Finance org"));
    }
}
