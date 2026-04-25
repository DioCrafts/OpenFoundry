import api from './client';

export interface AppBrandingSettings {
	display_name: string;
	primary_color: string;
	accent_color: string;
	logo_url: string | null;
	favicon_url: string | null;
	show_powered_by: boolean;
}

export type IdentityProviderRuleMatchType = 'email_domain' | 'claim_equals';

export interface IdentityProviderOrganizationRule {
	name: string;
	match_type: IdentityProviderRuleMatchType;
	claim: string | null;
	match_value: string;
	organization_id: string;
	workspace: string | null;
	classification_clearance: string | null;
	roles: string[];
	tenant_tier: string | null;
}

export interface IdentityProviderMapping {
	provider_slug: string;
	default_organization_id: string | null;
	organization_claim: string | null;
	workspace_claim: string | null;
	default_workspace: string | null;
	classification_clearance_claim: string | null;
	default_classification_clearance: string | null;
	role_claim: string | null;
	default_roles: string[];
	allowed_email_domains: string[];
	organization_rules: IdentityProviderOrganizationRule[];
}

export interface ResourceQuotaSettings {
	max_query_limit: number;
	max_distributed_query_workers: number;
	max_pipeline_workers: number;
	max_request_body_bytes: number;
	requests_per_minute: number;
	max_storage_gb: number;
	max_shared_spaces: number;
	max_guest_sessions: number;
}

export interface ResourceManagementPolicy {
	name: string;
	tenant_tier: string;
	applies_to_org_ids: string[];
	applies_to_workspaces: string[];
	quota: ResourceQuotaSettings;
}

export interface UpgradeAssistantCheck {
	id: string;
	label: string;
	owner: string;
	status: string;
	notes: string;
}

export interface UpgradeAssistantStage {
	id: string;
	label: string;
	rollout_percentage: number;
	status: string;
}

export interface UpgradeAssistantSettings {
	current_version: string;
	target_version: string;
	maintenance_window: string;
	rollback_channel: string;
	preflight_checks: UpgradeAssistantCheck[];
	rollout_stages: UpgradeAssistantStage[];
	rollback_steps: string[];
}

export interface UpgradeReadinessCheck {
	id: string;
	label: string;
	status: string;
	detail: string;
}

export interface UpgradeReadinessResponse {
	current_version: string;
	target_version: string;
	release_channel: string;
	readiness: string;
	checks: UpgradeReadinessCheck[];
	blockers: string[];
	recommended_actions: string[];
	next_stage: UpgradeAssistantStage | null;
	completed_stage_count: number;
	total_stage_count: number;
	preflight_ready_count: number;
	preflight_total_count: number;
	completed_rollout_percentage: number;
	generated_at: string;
}

export interface IdentityProviderMappingPreviewRequest {
	provider_slug: string;
	email: string;
	raw_claims: Record<string, unknown>;
}

export interface IdentityProviderMappingPreviewResponse {
	provider_slug: string;
	email: string;
	mapping_found: boolean;
	matched_rule_name: string | null;
	organization_id: string | null;
	workspace: string | null;
	classification_clearance: string | null;
	role_names: string[];
	tenant_tier: string | null;
	resource_policy_name: string | null;
	quota: ResourceQuotaSettings | null;
	notes: string[];
}

export interface ControlPanelSettings {
	platform_name: string;
	support_email: string;
	docs_url: string;
	status_page_url: string;
	announcement_banner: string;
	maintenance_mode: boolean;
	release_channel: string;
	default_region: string;
	deployment_mode: string;
	allow_self_signup: boolean;
	allowed_email_domains: string[];
	default_app_branding: AppBrandingSettings;
	restricted_operations: string[];
	identity_provider_mappings: IdentityProviderMapping[];
	resource_management_policies: ResourceManagementPolicy[];
	upgrade_assistant: UpgradeAssistantSettings;
	updated_by: string | null;
	updated_at: string;
}

export type UpdateControlPanelRequest = Partial<{
	platform_name: string;
	support_email: string;
	docs_url: string;
	status_page_url: string;
	announcement_banner: string;
	maintenance_mode: boolean;
	release_channel: string;
	default_region: string;
	deployment_mode: string;
	allow_self_signup: boolean;
	allowed_email_domains: string[];
	default_app_branding: AppBrandingSettings;
	restricted_operations: string[];
	identity_provider_mappings: IdentityProviderMapping[];
	resource_management_policies: ResourceManagementPolicy[];
	upgrade_assistant: UpgradeAssistantSettings;
}>;

export function getControlPanel() {
	return api.get<ControlPanelSettings>('/control-panel');
}

export function updateControlPanel(body: UpdateControlPanelRequest) {
	return api.put<ControlPanelSettings>('/control-panel', body);
}

export function getUpgradeReadiness() {
	return api.get<UpgradeReadinessResponse>('/control-panel/upgrade-readiness');
}

export function previewIdentityProviderMapping(body: IdentityProviderMappingPreviewRequest) {
	return api.post<IdentityProviderMappingPreviewResponse>(
		'/control-panel/identity-provider-mappings/preview',
		body,
	);
}
