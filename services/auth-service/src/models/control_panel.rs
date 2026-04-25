use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppBrandingSettings {
    pub display_name: String,
    pub primary_color: String,
    pub accent_color: String,
    pub logo_url: Option<String>,
    pub favicon_url: Option<String>,
    pub show_powered_by: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IdentityProviderRuleMatchType {
    EmailDomain,
    ClaimEquals,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityProviderOrganizationRule {
    pub name: String,
    pub match_type: IdentityProviderRuleMatchType,
    pub claim: Option<String>,
    pub match_value: String,
    pub organization_id: Uuid,
    pub workspace: Option<String>,
    pub classification_clearance: Option<String>,
    #[serde(default)]
    pub roles: Vec<String>,
    pub tenant_tier: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityProviderMapping {
    pub provider_slug: String,
    pub default_organization_id: Option<Uuid>,
    pub organization_claim: Option<String>,
    pub workspace_claim: Option<String>,
    pub default_workspace: Option<String>,
    pub classification_clearance_claim: Option<String>,
    pub default_classification_clearance: Option<String>,
    pub role_claim: Option<String>,
    pub default_roles: Vec<String>,
    pub allowed_email_domains: Vec<String>,
    #[serde(default)]
    pub organization_rules: Vec<IdentityProviderOrganizationRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceQuotaSettings {
    pub max_query_limit: usize,
    pub max_distributed_query_workers: usize,
    pub max_pipeline_workers: usize,
    pub max_request_body_bytes: usize,
    pub requests_per_minute: u32,
    pub max_storage_gb: u32,
    pub max_shared_spaces: u32,
    pub max_guest_sessions: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceManagementPolicy {
    pub name: String,
    pub tenant_tier: String,
    pub applies_to_org_ids: Vec<Uuid>,
    pub applies_to_workspaces: Vec<String>,
    pub quota: ResourceQuotaSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpgradeAssistantCheck {
    pub id: String,
    pub label: String,
    pub owner: String,
    pub status: String,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpgradeAssistantStage {
    pub id: String,
    pub label: String,
    pub rollout_percentage: u32,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpgradeAssistantSettings {
    pub current_version: String,
    pub target_version: String,
    pub maintenance_window: String,
    pub rollback_channel: String,
    pub preflight_checks: Vec<UpgradeAssistantCheck>,
    pub rollout_stages: Vec<UpgradeAssistantStage>,
    pub rollback_steps: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpgradeReadinessCheck {
    pub id: String,
    pub label: String,
    pub status: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpgradeReadinessResponse {
    pub current_version: String,
    pub target_version: String,
    pub release_channel: String,
    pub readiness: String,
    pub checks: Vec<UpgradeReadinessCheck>,
    pub blockers: Vec<String>,
    pub recommended_actions: Vec<String>,
    pub next_stage: Option<UpgradeAssistantStage>,
    pub completed_stage_count: usize,
    pub total_stage_count: usize,
    pub preflight_ready_count: usize,
    pub preflight_total_count: usize,
    pub completed_rollout_percentage: u32,
    pub generated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct IdentityProviderMappingPreviewRequest {
    pub provider_slug: String,
    pub email: String,
    #[serde(default)]
    pub raw_claims: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityProviderMappingPreviewResponse {
    pub provider_slug: String,
    pub email: String,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlPanelSettings {
    pub platform_name: String,
    pub support_email: String,
    pub docs_url: String,
    pub status_page_url: String,
    pub announcement_banner: String,
    pub maintenance_mode: bool,
    pub release_channel: String,
    pub default_region: String,
    pub deployment_mode: String,
    pub allow_self_signup: bool,
    pub allowed_email_domains: Vec<String>,
    pub default_app_branding: AppBrandingSettings,
    pub restricted_operations: Vec<String>,
    pub identity_provider_mappings: Vec<IdentityProviderMapping>,
    pub resource_management_policies: Vec<ResourceManagementPolicy>,
    pub upgrade_assistant: UpgradeAssistantSettings,
    pub updated_by: Option<Uuid>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateControlPanelRequest {
    pub platform_name: Option<String>,
    pub support_email: Option<String>,
    pub docs_url: Option<String>,
    pub status_page_url: Option<String>,
    pub announcement_banner: Option<String>,
    pub maintenance_mode: Option<bool>,
    pub release_channel: Option<String>,
    pub default_region: Option<String>,
    pub deployment_mode: Option<String>,
    pub allow_self_signup: Option<bool>,
    pub allowed_email_domains: Option<Vec<String>>,
    pub default_app_branding: Option<AppBrandingSettings>,
    pub restricted_operations: Option<Vec<String>>,
    pub identity_provider_mappings: Option<Vec<IdentityProviderMapping>>,
    pub resource_management_policies: Option<Vec<ResourceManagementPolicy>>,
    pub upgrade_assistant: Option<UpgradeAssistantSettings>,
}

#[derive(Debug, Clone, FromRow)]
pub struct ControlPanelRow {
    pub platform_name: String,
    pub support_email: String,
    pub docs_url: String,
    pub status_page_url: String,
    pub announcement_banner: String,
    pub maintenance_mode: bool,
    pub release_channel: String,
    pub default_region: String,
    pub deployment_mode: String,
    pub allow_self_signup: bool,
    pub allowed_email_domains: Value,
    pub default_app_branding: Value,
    pub restricted_operations: Value,
    pub identity_provider_mappings: Value,
    pub resource_management_policies: Value,
    pub upgrade_assistant: Value,
    pub updated_by: Option<Uuid>,
    pub updated_at: DateTime<Utc>,
}

impl TryFrom<ControlPanelRow> for ControlPanelSettings {
    type Error = String;

    fn try_from(row: ControlPanelRow) -> Result<Self, Self::Error> {
        Ok(Self {
            platform_name: row.platform_name,
            support_email: row.support_email,
            docs_url: row.docs_url,
            status_page_url: row.status_page_url,
            announcement_banner: row.announcement_banner,
            maintenance_mode: row.maintenance_mode,
            release_channel: row.release_channel,
            default_region: row.default_region,
            deployment_mode: row.deployment_mode,
            allow_self_signup: row.allow_self_signup,
            allowed_email_domains: serde_json::from_value(row.allowed_email_domains)
                .map_err(|cause| format!("invalid allowed_email_domains: {cause}"))?,
            default_app_branding: serde_json::from_value(row.default_app_branding)
                .map_err(|cause| format!("invalid default_app_branding: {cause}"))?,
            restricted_operations: serde_json::from_value(row.restricted_operations)
                .map_err(|cause| format!("invalid restricted_operations: {cause}"))?,
            identity_provider_mappings: serde_json::from_value(row.identity_provider_mappings)
                .map_err(|cause| format!("invalid identity_provider_mappings: {cause}"))?,
            resource_management_policies: serde_json::from_value(row.resource_management_policies)
                .map_err(|cause| format!("invalid resource_management_policies: {cause}"))?,
            upgrade_assistant: serde_json::from_value(row.upgrade_assistant)
                .map_err(|cause| format!("invalid upgrade_assistant: {cause}"))?,
            updated_by: row.updated_by,
            updated_at: row.updated_at,
        })
    }
}
