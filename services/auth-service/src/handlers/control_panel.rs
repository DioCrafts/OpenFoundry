use std::collections::HashSet;

use auth_middleware::layer::AuthUser;
use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use chrono::Utc;
use sqlx::types::Json as SqlJson;

use crate::{
    AppState,
    domain::idp_mapping,
    models::{
        control_panel::{
            ControlPanelRow, ControlPanelSettings, IdentityProviderMapping,
            IdentityProviderMappingPreviewRequest, ResourceManagementPolicy,
            UpdateControlPanelRequest, UpgradeAssistantCheck, UpgradeAssistantSettings,
            UpgradeAssistantStage, UpgradeReadinessCheck, UpgradeReadinessResponse,
        },
        sso::SsoProvider,
    },
};

use super::common::{json_error, require_permission};

#[derive(Debug, Clone)]
struct UpgradeReadinessMetrics {
    completed_stage_count: usize,
    total_stage_count: usize,
    preflight_ready_count: usize,
    preflight_total_count: usize,
    completed_rollout_percentage: u32,
    next_stage: Option<UpgradeAssistantStage>,
    blockers: Vec<String>,
    recommended_actions: Vec<String>,
}

pub async fn get_control_panel(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> impl IntoResponse {
    if let Err(response) = require_permission(&claims, "control_panel", "read") {
        return response;
    }

    match load_control_panel(&state.db).await {
        Ok(Some(settings)) => Json(settings).into_response(),
        Ok(None) => json_error(StatusCode::NOT_FOUND, "control panel settings not found"),
        Err(error) => {
            tracing::error!("failed to load control panel settings: {error}");
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to load control panel settings",
            )
        }
    }
}

pub async fn update_control_panel(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(body): Json<UpdateControlPanelRequest>,
) -> impl IntoResponse {
    if let Err(response) = require_permission(&claims, "control_panel", "write") {
        return response;
    }

    let Some(current) = (match load_control_panel(&state.db).await {
        Ok(value) => value,
        Err(error) => {
            tracing::error!("failed to load control panel settings for update: {error}");
            return json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to update control panel settings",
            );
        }
    }) else {
        return json_error(StatusCode::NOT_FOUND, "control panel settings not found");
    };

    let platform_name = body.platform_name.unwrap_or(current.platform_name);
    let support_email = body.support_email.unwrap_or(current.support_email);
    let docs_url = body.docs_url.unwrap_or(current.docs_url);
    let status_page_url = body.status_page_url.unwrap_or(current.status_page_url);
    let announcement_banner = body
        .announcement_banner
        .unwrap_or(current.announcement_banner);
    let maintenance_mode = body.maintenance_mode.unwrap_or(current.maintenance_mode);
    let release_channel = body.release_channel.unwrap_or(current.release_channel);
    let default_region = body.default_region.unwrap_or(current.default_region);
    let deployment_mode = body.deployment_mode.unwrap_or(current.deployment_mode);
    let allow_self_signup = body.allow_self_signup.unwrap_or(current.allow_self_signup);
    let allowed_email_domains = body
        .allowed_email_domains
        .unwrap_or(current.allowed_email_domains);
    let default_app_branding = body
        .default_app_branding
        .unwrap_or(current.default_app_branding);
    let restricted_operations = body
        .restricted_operations
        .unwrap_or(current.restricted_operations);
    let identity_provider_mappings = body
        .identity_provider_mappings
        .unwrap_or(current.identity_provider_mappings);
    let resource_management_policies = body
        .resource_management_policies
        .unwrap_or(current.resource_management_policies);
    let upgrade_assistant = body.upgrade_assistant.unwrap_or(current.upgrade_assistant);

    if let Err(error) = validate_control_panel_inputs(
        &identity_provider_mappings,
        &resource_management_policies,
        &upgrade_assistant,
    ) {
        return json_error(StatusCode::BAD_REQUEST, error);
    }

    let updated_by =
        match sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM users WHERE id = $1)")
            .bind(claims.sub)
            .fetch_one(&state.db)
            .await
        {
            Ok(true) => Some(claims.sub),
            Ok(false) => None,
            Err(error) => {
                tracing::error!("failed to validate control panel updater: {error}");
                return json_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "failed to update control panel settings",
                );
            }
        };

    let result = sqlx::query(
        r#"UPDATE control_panel_settings
           SET platform_name = $1,
               support_email = $2,
               docs_url = $3,
               status_page_url = $4,
               announcement_banner = $5,
               maintenance_mode = $6,
               release_channel = $7,
               default_region = $8,
               deployment_mode = $9,
               allow_self_signup = $10,
               allowed_email_domains = $11::jsonb,
               default_app_branding = $12::jsonb,
               restricted_operations = $13::jsonb,
               identity_provider_mappings = $14::jsonb,
               resource_management_policies = $15::jsonb,
               upgrade_assistant = $16::jsonb,
               updated_by = $17,
               updated_at = NOW()
           WHERE singleton_id = TRUE"#,
    )
    .bind(platform_name)
    .bind(support_email)
    .bind(docs_url)
    .bind(status_page_url)
    .bind(announcement_banner)
    .bind(maintenance_mode)
    .bind(release_channel)
    .bind(default_region)
    .bind(deployment_mode)
    .bind(allow_self_signup)
    .bind(SqlJson(allowed_email_domains))
    .bind(SqlJson(default_app_branding))
    .bind(SqlJson(restricted_operations))
    .bind(SqlJson(identity_provider_mappings))
    .bind(SqlJson(resource_management_policies))
    .bind(SqlJson(upgrade_assistant))
    .bind(updated_by)
    .execute(&state.db)
    .await;

    match result {
        Ok(_) => match load_control_panel(&state.db).await {
            Ok(Some(settings)) => Json(settings).into_response(),
            Ok(None) => json_error(
                StatusCode::NOT_FOUND,
                "control panel settings not found after update",
            ),
            Err(error) => {
                tracing::error!("failed to reload control panel settings: {error}");
                json_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "failed to reload control panel settings",
                )
            }
        },
        Err(error) => {
            tracing::error!("failed to update control panel settings: {error}");
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to update control panel settings",
            )
        }
    }
}

pub async fn preview_identity_provider_mapping(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(body): Json<IdentityProviderMappingPreviewRequest>,
) -> impl IntoResponse {
    if let Err(response) = require_permission(&claims, "control_panel", "read") {
        return response;
    }

    let provider_slug = body.provider_slug.trim();
    if provider_slug.is_empty() {
        return json_error(StatusCode::BAD_REQUEST, "provider_slug is required");
    }
    let email = body.email.trim().to_string();
    if email.is_empty() {
        return json_error(StatusCode::BAD_REQUEST, "email is required");
    }

    let settings = match load_control_panel(&state.db).await {
        Ok(Some(settings)) => settings,
        Ok(None) => {
            return json_error(StatusCode::NOT_FOUND, "control panel settings not found");
        }
        Err(error) => {
            tracing::error!("failed to load control panel settings for preview: {error}");
            return json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to load identity provider mapping preview",
            );
        }
    };
    let provider = match load_provider_by_slug(&state.db, provider_slug).await {
        Ok(Some(provider)) => provider,
        Ok(None) => {
            return json_error(
                StatusCode::NOT_FOUND,
                format!("provider '{provider_slug}' not found"),
            );
        }
        Err(error) => {
            tracing::error!("failed to load provider for preview: {error}");
            return json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to load identity provider mapping preview",
            );
        }
    };

    let mapping = settings
        .identity_provider_mappings
        .iter()
        .find(|entry| entry.provider_slug == provider.slug);
    match idp_mapping::resolve_identity_provider_assignment(
        &provider,
        mapping,
        &email,
        &body.raw_claims,
        &settings.resource_management_policies,
    ) {
        Ok(assignment) => Json(assignment.into_preview_response(email)).into_response(),
        Err(error) => json_error(StatusCode::BAD_REQUEST, error),
    }
}

pub async fn get_upgrade_readiness(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> impl IntoResponse {
    if let Err(response) = require_permission(&claims, "control_panel", "read") {
        return response;
    }

    let settings = match load_control_panel(&state.db).await {
        Ok(Some(settings)) => settings,
        Ok(None) => {
            return json_error(StatusCode::NOT_FOUND, "control panel settings not found");
        }
        Err(error) => {
            tracing::error!("failed to load control panel settings for readiness: {error}");
            return json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to load upgrade readiness",
            );
        }
    };

    let enabled_provider_slugs = match sqlx::query_scalar::<_, String>(
        "SELECT slug FROM sso_providers WHERE enabled = true",
    )
    .fetch_all(&state.db)
    .await
    {
        Ok(value) => value,
        Err(error) => {
            tracing::error!("failed to list enabled SSO providers: {error}");
            return json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to load upgrade readiness",
            );
        }
    };
    let enabled_policies = match sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM abac_policies WHERE enabled = true",
    )
    .fetch_one(&state.db)
    .await
    {
        Ok(value) => value,
        Err(error) => {
            tracing::error!("failed to count enabled policies: {error}");
            return json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to load upgrade readiness",
            );
        }
    };
    let active_scoped_sessions = match sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM scoped_sessions WHERE revoked_at IS NULL AND expires_at > NOW()",
    )
    .fetch_one(&state.db)
    .await
    {
        Ok(value) => value,
        Err(error) => {
            tracing::error!("failed to count active scoped sessions: {error}");
            return json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to load upgrade readiness",
            );
        }
    };

    Json(build_upgrade_readiness_response(
        &settings,
        &enabled_provider_slugs,
        enabled_policies,
        active_scoped_sessions,
    ))
    .into_response()
}

async fn load_control_panel(pool: &sqlx::PgPool) -> Result<Option<ControlPanelSettings>, String> {
    let row = sqlx::query_as::<_, ControlPanelRow>(
        r#"SELECT
               platform_name,
               support_email,
               docs_url,
               status_page_url,
               announcement_banner,
               maintenance_mode,
               release_channel,
               default_region,
               deployment_mode,
               allow_self_signup,
               allowed_email_domains,
               default_app_branding,
               restricted_operations,
               identity_provider_mappings,
               resource_management_policies,
               upgrade_assistant,
               updated_by,
               updated_at
           FROM control_panel_settings
           WHERE singleton_id = TRUE"#,
    )
    .fetch_optional(pool)
    .await
    .map_err(|cause| cause.to_string())?;

    row.map(TryInto::try_into).transpose()
}

async fn load_provider_by_slug(
    pool: &sqlx::PgPool,
    slug: &str,
) -> Result<Option<SsoProvider>, sqlx::Error> {
    sqlx::query_as::<_, SsoProvider>(
        "SELECT id, slug, name, provider_type, enabled, client_id, client_secret, issuer_url, authorization_url, token_url, userinfo_url, scopes, saml_metadata_url, saml_entity_id, saml_sso_url, saml_certificate, attribute_mapping, created_at, updated_at FROM sso_providers WHERE slug = $1",
    )
    .bind(slug)
    .fetch_optional(pool)
    .await
}

fn validate_control_panel_inputs(
    identity_provider_mappings: &[IdentityProviderMapping],
    resource_management_policies: &[ResourceManagementPolicy],
    upgrade_assistant: &UpgradeAssistantSettings,
) -> Result<(), String> {
    validate_identity_provider_mappings(identity_provider_mappings)?;
    validate_resource_management_policies(resource_management_policies)?;
    validate_upgrade_assistant(upgrade_assistant)?;
    Ok(())
}

fn validate_identity_provider_mappings(mappings: &[IdentityProviderMapping]) -> Result<(), String> {
    let mut provider_slugs = HashSet::new();
    for mapping in mappings {
        if mapping.provider_slug.trim().is_empty() {
            return Err("identity_provider_mappings[].provider_slug is required".to_string());
        }
        if !provider_slugs.insert(mapping.provider_slug.to_ascii_lowercase()) {
            return Err(format!(
                "duplicate identity provider mapping for provider '{}'",
                mapping.provider_slug
            ));
        }

        for domain in &mapping.allowed_email_domains {
            let trimmed = domain.trim();
            if trimmed.is_empty() || trimmed.contains('@') {
                return Err(format!(
                    "provider '{}' has an invalid allowed_email_domains entry '{}'",
                    mapping.provider_slug, domain
                ));
            }
        }

        let mut rule_names = HashSet::new();
        for rule in &mapping.organization_rules {
            if rule.name.trim().is_empty() {
                return Err(format!(
                    "provider '{}' has an organization rule without a name",
                    mapping.provider_slug
                ));
            }
            if !rule_names.insert(rule.name.to_ascii_lowercase()) {
                return Err(format!(
                    "provider '{}' defines duplicate organization rule '{}'",
                    mapping.provider_slug, rule.name
                ));
            }
            if rule.match_value.trim().is_empty() {
                return Err(format!(
                    "organization rule '{}' must define a non-empty match_value",
                    rule.name
                ));
            }
            if matches!(
                rule.match_type,
                crate::models::control_panel::IdentityProviderRuleMatchType::ClaimEquals
            ) && rule
                .claim
                .as_deref()
                .map(|claim| claim.trim().is_empty())
                .unwrap_or(true)
            {
                return Err(format!(
                    "organization rule '{}' must define claim when match_type is claim_equals",
                    rule.name
                ));
            }
        }
    }
    Ok(())
}

fn validate_resource_management_policies(
    policies: &[ResourceManagementPolicy],
) -> Result<(), String> {
    let mut names = HashSet::new();
    for policy in policies {
        if policy.name.trim().is_empty() {
            return Err("resource_management_policies[].name is required".to_string());
        }
        if !names.insert(policy.name.to_ascii_lowercase()) {
            return Err(format!(
                "duplicate resource management policy '{}'",
                policy.name
            ));
        }
        if policy.tenant_tier.trim().is_empty() {
            return Err(format!(
                "resource management policy '{}' must define tenant_tier",
                policy.name
            ));
        }
    }
    Ok(())
}

fn validate_upgrade_assistant(settings: &UpgradeAssistantSettings) -> Result<(), String> {
    if settings.current_version.trim().is_empty() {
        return Err("upgrade_assistant.current_version is required".to_string());
    }
    if settings.target_version.trim().is_empty() {
        return Err("upgrade_assistant.target_version is required".to_string());
    }
    if settings.maintenance_window.trim().is_empty() {
        return Err("upgrade_assistant.maintenance_window is required".to_string());
    }
    if settings.rollback_channel.trim().is_empty() {
        return Err("upgrade_assistant.rollback_channel is required".to_string());
    }
    if settings.rollback_steps.is_empty() {
        return Err("upgrade_assistant.rollback_steps must not be empty".to_string());
    }

    let mut check_ids = HashSet::new();
    for check in &settings.preflight_checks {
        validate_upgrade_check(check)?;
        if !check_ids.insert(check.id.to_ascii_lowercase()) {
            return Err(format!("duplicate preflight check id '{}'", check.id));
        }
    }

    let mut stage_ids = HashSet::new();
    let mut previous_percentage = 0;
    for stage in &settings.rollout_stages {
        validate_upgrade_stage(stage)?;
        if !stage_ids.insert(stage.id.to_ascii_lowercase()) {
            return Err(format!("duplicate rollout stage id '{}'", stage.id));
        }
        if stage.rollout_percentage < previous_percentage {
            return Err(
                "upgrade_assistant.rollout_stages must be sorted by rollout_percentage".to_string(),
            );
        }
        previous_percentage = stage.rollout_percentage;
    }

    if let Some(last_stage) = settings.rollout_stages.last() {
        if last_stage.rollout_percentage != 100 {
            return Err(
                "upgrade_assistant.rollout_stages must end with rollout_percentage 100".to_string(),
            );
        }
    } else {
        return Err("upgrade_assistant.rollout_stages must contain at least one stage".to_string());
    }

    Ok(())
}

fn validate_upgrade_check(check: &UpgradeAssistantCheck) -> Result<(), String> {
    if check.id.trim().is_empty() {
        return Err("upgrade_assistant.preflight_checks[].id is required".to_string());
    }
    if check.label.trim().is_empty() {
        return Err(format!(
            "upgrade_assistant preflight check '{}' must define label",
            check.id
        ));
    }
    if check.owner.trim().is_empty() {
        return Err(format!(
            "upgrade_assistant preflight check '{}' must define owner",
            check.id
        ));
    }
    if !matches!(
        check.status.trim(),
        "pending" | "ready" | "warning" | "blocked" | "completed"
    ) {
        return Err(format!(
            "upgrade_assistant preflight check '{}' has unsupported status '{}'",
            check.id, check.status
        ));
    }
    Ok(())
}

fn validate_upgrade_stage(stage: &UpgradeAssistantStage) -> Result<(), String> {
    if stage.id.trim().is_empty() {
        return Err("upgrade_assistant.rollout_stages[].id is required".to_string());
    }
    if stage.label.trim().is_empty() {
        return Err(format!(
            "upgrade_assistant rollout stage '{}' must define label",
            stage.id
        ));
    }
    if stage.rollout_percentage == 0 || stage.rollout_percentage > 100 {
        return Err(format!(
            "upgrade_assistant rollout stage '{}' must use rollout_percentage between 1 and 100",
            stage.id
        ));
    }
    if !matches!(
        stage.status.trim(),
        "pending" | "in_progress" | "completed" | "blocked" | "paused"
    ) {
        return Err(format!(
            "upgrade_assistant rollout stage '{}' has unsupported status '{}'",
            stage.id, stage.status
        ));
    }
    Ok(())
}

fn build_upgrade_readiness_response(
    settings: &ControlPanelSettings,
    enabled_provider_slugs: &[String],
    enabled_policies: i64,
    active_scoped_sessions: i64,
) -> UpgradeReadinessResponse {
    let checks = build_upgrade_readiness_checks(
        settings,
        enabled_provider_slugs,
        enabled_policies,
        active_scoped_sessions,
    );
    let metrics = build_upgrade_readiness_metrics(settings, &checks);
    let readiness = if checks.iter().any(|check| check.status == "blocked") {
        "blocked"
    } else if checks.iter().any(|check| check.status == "warning") {
        "attention"
    } else {
        "ready"
    };

    UpgradeReadinessResponse {
        current_version: settings.upgrade_assistant.current_version.clone(),
        target_version: settings.upgrade_assistant.target_version.clone(),
        release_channel: settings.release_channel.clone(),
        readiness: readiness.to_string(),
        checks,
        blockers: metrics.blockers,
        recommended_actions: metrics.recommended_actions,
        next_stage: metrics.next_stage,
        completed_stage_count: metrics.completed_stage_count,
        total_stage_count: metrics.total_stage_count,
        preflight_ready_count: metrics.preflight_ready_count,
        preflight_total_count: metrics.preflight_total_count,
        completed_rollout_percentage: metrics.completed_rollout_percentage,
        generated_at: Utc::now(),
    }
}

fn build_upgrade_readiness_checks(
    settings: &ControlPanelSettings,
    enabled_provider_slugs: &[String],
    enabled_policies: i64,
    active_scoped_sessions: i64,
) -> Vec<UpgradeReadinessCheck> {
    let mapped_provider_slugs = settings
        .identity_provider_mappings
        .iter()
        .map(|entry| entry.provider_slug.to_ascii_lowercase())
        .collect::<HashSet<_>>();
    let enabled_provider_count = enabled_provider_slugs.len();
    let mapped_enabled_provider_count = enabled_provider_slugs
        .iter()
        .filter(|slug| mapped_provider_slugs.contains(&slug.to_ascii_lowercase()))
        .count();
    let orphan_mapping_count = settings
        .identity_provider_mappings
        .iter()
        .filter(|entry| {
            !enabled_provider_slugs
                .iter()
                .any(|slug| slug.eq_ignore_ascii_case(&entry.provider_slug))
        })
        .count();
    let organization_rule_count = settings
        .identity_provider_mappings
        .iter()
        .map(|entry| entry.organization_rules.len())
        .sum::<usize>();
    let resource_policy_count = settings.resource_management_policies.len();
    let maintenance_ready = !settings
        .upgrade_assistant
        .maintenance_window
        .trim()
        .is_empty()
        && (settings.maintenance_mode || !settings.restricted_operations.is_empty());
    let rollback_ready = !settings
        .upgrade_assistant
        .rollback_channel
        .trim()
        .is_empty()
        && !settings.upgrade_assistant.rollback_steps.is_empty();
    let preflight_blocked_count = settings
        .upgrade_assistant
        .preflight_checks
        .iter()
        .filter(|check| check.status == "blocked")
        .count();
    let preflight_warning_count = settings
        .upgrade_assistant
        .preflight_checks
        .iter()
        .filter(|check| check.status == "warning")
        .count();
    let preflight_ready_count = settings
        .upgrade_assistant
        .preflight_checks
        .iter()
        .filter(|check| is_ready_preflight_status(check.status.as_str()))
        .count();
    let completed_stage_count = settings
        .upgrade_assistant
        .rollout_stages
        .iter()
        .filter(|stage| is_completed_stage_status(stage.status.as_str()))
        .count();

    vec![
        UpgradeReadinessCheck {
            id: "identity_providers".to_string(),
            label: "Identity provider mapping".to_string(),
            status: if settings.identity_provider_mappings.is_empty() {
                "blocked".to_string()
            } else if mapped_enabled_provider_count < enabled_provider_count
                || organization_rule_count == 0
            {
                "warning".to_string()
            } else {
                "ready".to_string()
            },
            detail: format!(
                "{mapped_enabled_provider_count}/{enabled_provider_count} enabled provider(s) mapped, {organization_rule_count} advanced org rule(s), {orphan_mapping_count} stale mapping(s)"
            ),
        },
        UpgradeReadinessCheck {
            id: "resource_policies".to_string(),
            label: "Resource quotas and limits".to_string(),
            status: if resource_policy_count == 0 {
                "blocked".to_string()
            } else {
                "ready".to_string()
            },
            detail: format!(
                "{resource_policy_count} policy/policies available for tenant assignment and enrollment shaping"
            ),
        },
        UpgradeReadinessCheck {
            id: "preflight".to_string(),
            label: "Preflight gates".to_string(),
            status: if preflight_blocked_count > 0 {
                "blocked".to_string()
            } else if preflight_warning_count > 0
                || preflight_ready_count < settings.upgrade_assistant.preflight_checks.len()
            {
                "warning".to_string()
            } else {
                "ready".to_string()
            },
            detail: format!(
                "{preflight_ready_count}/{} checks ready, {preflight_warning_count} warning, {preflight_blocked_count} blocked",
                settings.upgrade_assistant.preflight_checks.len()
            ),
        },
        UpgradeReadinessCheck {
            id: "rollout_plan".to_string(),
            label: "Rollout progression".to_string(),
            status: if settings.upgrade_assistant.rollout_stages.is_empty() {
                "blocked".to_string()
            } else if completed_stage_count < settings.upgrade_assistant.rollout_stages.len() {
                "warning".to_string()
            } else {
                "ready".to_string()
            },
            detail: format!(
                "{completed_stage_count}/{} rollout stages completed",
                settings.upgrade_assistant.rollout_stages.len()
            ),
        },
        UpgradeReadinessCheck {
            id: "maintenance_window".to_string(),
            label: "Upgrade maintenance posture".to_string(),
            status: if maintenance_ready {
                "ready".to_string()
            } else {
                "warning".to_string()
            },
            detail: if maintenance_ready {
                format!(
                    "{} with {} restricted operation(s)",
                    settings.upgrade_assistant.maintenance_window,
                    settings.restricted_operations.len()
                )
            } else {
                "maintenance window exists but freeze controls are incomplete".to_string()
            },
        },
        UpgradeReadinessCheck {
            id: "rollback_plan".to_string(),
            label: "Rollback assistant".to_string(),
            status: if rollback_ready {
                "ready".to_string()
            } else {
                "blocked".to_string()
            },
            detail: format!(
                "{} rollback step(s) targeting channel {}",
                settings.upgrade_assistant.rollback_steps.len(),
                if settings
                    .upgrade_assistant
                    .rollback_channel
                    .trim()
                    .is_empty()
                {
                    "<unset>"
                } else {
                    settings.upgrade_assistant.rollback_channel.as_str()
                }
            ),
        },
        UpgradeReadinessCheck {
            id: "security_policies".to_string(),
            label: "Security policies".to_string(),
            status: if enabled_policies > 0 {
                "ready".to_string()
            } else {
                "warning".to_string()
            },
            detail: format!(
                "{enabled_policies} ABAC policy/policies enabled for rollout guardrails"
            ),
        },
        UpgradeReadinessCheck {
            id: "temporary_sessions".to_string(),
            label: "Temporary access review".to_string(),
            status: if active_scoped_sessions <= 25 {
                "ready".to_string()
            } else {
                "warning".to_string()
            },
            detail: format!(
                "{active_scoped_sessions} active scoped/guest session(s) require review"
            ),
        },
    ]
}

fn build_upgrade_readiness_metrics(
    settings: &ControlPanelSettings,
    checks: &[UpgradeReadinessCheck],
) -> UpgradeReadinessMetrics {
    let completed_stage_count = settings
        .upgrade_assistant
        .rollout_stages
        .iter()
        .filter(|stage| is_completed_stage_status(stage.status.as_str()))
        .count();
    let total_stage_count = settings.upgrade_assistant.rollout_stages.len();
    let preflight_ready_count = settings
        .upgrade_assistant
        .preflight_checks
        .iter()
        .filter(|check| is_ready_preflight_status(check.status.as_str()))
        .count();
    let preflight_total_count = settings.upgrade_assistant.preflight_checks.len();
    let completed_rollout_percentage = settings
        .upgrade_assistant
        .rollout_stages
        .iter()
        .filter(|stage| is_completed_stage_status(stage.status.as_str()))
        .map(|stage| stage.rollout_percentage)
        .max()
        .unwrap_or(0);
    let next_stage = settings
        .upgrade_assistant
        .rollout_stages
        .iter()
        .find(|stage| !is_completed_stage_status(stage.status.as_str()))
        .cloned();

    let mut blockers = checks
        .iter()
        .filter(|check| check.status == "blocked")
        .map(|check| format!("{}: {}", check.label, check.detail))
        .collect::<Vec<_>>();
    blockers.extend(
        settings
            .upgrade_assistant
            .preflight_checks
            .iter()
            .filter(|check| check.status == "blocked")
            .map(|check| format!("{}: {}", check.label, check.notes)),
    );
    blockers.sort();
    blockers.dedup();

    let mut recommended_actions = settings
        .upgrade_assistant
        .preflight_checks
        .iter()
        .filter(|check| check.status != "ready" && check.status != "completed")
        .map(|check| format!("{}: {}", check.owner, check.notes))
        .collect::<Vec<_>>();
    if let Some(stage) = next_stage.as_ref() {
        recommended_actions.push(format!(
            "Prepare rollout stage '{}' for {}% of tenants.",
            stage.label, stage.rollout_percentage
        ));
    }
    if checks
        .iter()
        .any(|check| check.id == "temporary_sessions" && check.status == "warning")
    {
        recommended_actions.push(
            "Review and revoke stale scoped/guest sessions before progressing the rollout."
                .to_string(),
        );
    }
    if checks
        .iter()
        .any(|check| check.id == "identity_providers" && check.status != "ready")
    {
        recommended_actions.push(
            "Run IdP mapping previews for each enabled provider and close unmapped org-assignment gaps."
                .to_string(),
        );
    }
    recommended_actions.sort();
    recommended_actions.dedup();

    UpgradeReadinessMetrics {
        completed_stage_count,
        total_stage_count,
        preflight_ready_count,
        preflight_total_count,
        completed_rollout_percentage,
        next_stage,
        blockers,
        recommended_actions,
    }
}

fn is_ready_preflight_status(status: &str) -> bool {
    matches!(status, "ready" | "completed")
}

fn is_completed_stage_status(status: &str) -> bool {
    status == "completed"
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use uuid::Uuid;

    use crate::models::control_panel::{
        AppBrandingSettings, ControlPanelSettings, IdentityProviderMapping,
        IdentityProviderOrganizationRule, IdentityProviderRuleMatchType, ResourceManagementPolicy,
        ResourceQuotaSettings, UpgradeAssistantCheck, UpgradeAssistantSettings,
        UpgradeAssistantStage,
    };

    use super::{
        build_upgrade_readiness_response, validate_identity_provider_mappings,
        validate_upgrade_assistant,
    };

    fn sample_settings() -> ControlPanelSettings {
        ControlPanelSettings {
            platform_name: "OpenFoundry".to_string(),
            support_email: "support@openfoundry.dev".to_string(),
            docs_url: "https://docs.openfoundry.dev".to_string(),
            status_page_url: "https://status.openfoundry.dev".to_string(),
            announcement_banner: String::new(),
            maintenance_mode: false,
            release_channel: "stable".to_string(),
            default_region: "eu-west-1".to_string(),
            deployment_mode: "self-hosted".to_string(),
            allow_self_signup: false,
            allowed_email_domains: vec![],
            default_app_branding: AppBrandingSettings {
                display_name: "OpenFoundry".to_string(),
                primary_color: "#0f766e".to_string(),
                accent_color: "#d97706".to_string(),
                logo_url: None,
                favicon_url: None,
                show_powered_by: true,
            },
            restricted_operations: vec!["terraform-apply".to_string()],
            identity_provider_mappings: vec![IdentityProviderMapping {
                provider_slug: "enterprise-saml".to_string(),
                default_organization_id: None,
                organization_claim: Some("organization_id".to_string()),
                workspace_claim: Some("workspace".to_string()),
                default_workspace: Some("shared-enterprise".to_string()),
                classification_clearance_claim: Some("classification_clearance".to_string()),
                default_classification_clearance: Some("internal".to_string()),
                role_claim: Some("roles".to_string()),
                default_roles: vec!["viewer".to_string()],
                allowed_email_domains: vec!["openfoundry.dev".to_string()],
                organization_rules: vec![IdentityProviderOrganizationRule {
                    name: "Finance".to_string(),
                    match_type: IdentityProviderRuleMatchType::ClaimEquals,
                    claim: Some("department".to_string()),
                    match_value: "finance".to_string(),
                    organization_id: Uuid::now_v7(),
                    workspace: Some("finance-control".to_string()),
                    classification_clearance: Some("restricted".to_string()),
                    roles: vec!["operator".to_string()],
                    tenant_tier: Some("regulated".to_string()),
                }],
            }],
            resource_management_policies: vec![ResourceManagementPolicy {
                name: "enterprise-default".to_string(),
                tenant_tier: "enterprise".to_string(),
                applies_to_org_ids: vec![],
                applies_to_workspaces: vec![],
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
            }],
            upgrade_assistant: UpgradeAssistantSettings {
                current_version: "2026.04.0".to_string(),
                target_version: "2026.05.0".to_string(),
                maintenance_window: "Sun 02:00-04:00 UTC".to_string(),
                rollback_channel: "stable".to_string(),
                preflight_checks: vec![
                    UpgradeAssistantCheck {
                        id: "backups".to_string(),
                        label: "Backups verified".to_string(),
                        owner: "platform".to_string(),
                        status: "ready".to_string(),
                        notes: "Snapshots ready".to_string(),
                    },
                    UpgradeAssistantCheck {
                        id: "sso".to_string(),
                        label: "Identity providers aligned".to_string(),
                        owner: "identity".to_string(),
                        status: "warning".to_string(),
                        notes: "Preview partner org assignment".to_string(),
                    },
                ],
                rollout_stages: vec![
                    UpgradeAssistantStage {
                        id: "staging".to_string(),
                        label: "Stage staging".to_string(),
                        rollout_percentage: 10,
                        status: "completed".to_string(),
                    },
                    UpgradeAssistantStage {
                        id: "canary".to_string(),
                        label: "Promote canary tenants".to_string(),
                        rollout_percentage: 30,
                        status: "pending".to_string(),
                    },
                    UpgradeAssistantStage {
                        id: "full".to_string(),
                        label: "Full production rollout".to_string(),
                        rollout_percentage: 100,
                        status: "pending".to_string(),
                    },
                ],
                rollback_steps: vec!["restore previous release channel".to_string()],
            },
            updated_by: None,
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn rejects_claim_rule_without_claim_name() {
        let mappings = vec![IdentityProviderMapping {
            provider_slug: "enterprise-saml".to_string(),
            default_organization_id: None,
            organization_claim: None,
            workspace_claim: None,
            default_workspace: None,
            classification_clearance_claim: None,
            default_classification_clearance: None,
            role_claim: None,
            default_roles: vec![],
            allowed_email_domains: vec![],
            organization_rules: vec![IdentityProviderOrganizationRule {
                name: "Invalid".to_string(),
                match_type: IdentityProviderRuleMatchType::ClaimEquals,
                claim: None,
                match_value: "finance".to_string(),
                organization_id: Uuid::now_v7(),
                workspace: None,
                classification_clearance: None,
                roles: vec![],
                tenant_tier: None,
            }],
        }];

        assert!(validate_identity_provider_mappings(&mappings).is_err());
    }

    #[test]
    fn rejects_rollout_without_terminal_stage() {
        let mut assistant = sample_settings().upgrade_assistant;
        assistant.rollout_stages.pop();

        let error = validate_upgrade_assistant(&assistant).expect_err("validation should fail");
        assert!(error.contains("rollout_percentage 100"));
    }

    #[test]
    fn readiness_reports_stage_progress_and_recommended_actions() {
        let response = build_upgrade_readiness_response(
            &sample_settings(),
            &["enterprise-saml".to_string()],
            2,
            34,
        );

        assert_eq!(response.readiness, "attention");
        assert_eq!(response.completed_stage_count, 1);
        assert_eq!(response.completed_rollout_percentage, 10);
        assert_eq!(response.preflight_ready_count, 1);
        assert_eq!(
            response.next_stage.as_ref().map(|stage| stage.id.as_str()),
            Some("canary")
        );
        assert!(
            response
                .recommended_actions
                .iter()
                .any(|entry| entry.contains("Prepare rollout stage 'Promote canary tenants'"))
        );
    }
}
