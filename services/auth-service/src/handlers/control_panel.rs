use auth_middleware::layer::AuthUser;
use axum::{Json, extract::State, response::IntoResponse};
use chrono::Utc;
use sqlx::types::Json as SqlJson;

use crate::{
    AppState,
    models::control_panel::{
        ControlPanelRow, ControlPanelSettings, UpdateControlPanelRequest,
        UpgradeReadinessCheck, UpgradeReadinessResponse,
    },
};

use super::common::{json_error, require_permission};

pub async fn get_control_panel(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> impl IntoResponse {
    if let Err(response) = require_permission(&claims, "control_panel", "read") {
        return response;
    }

    match load_control_panel(&state.db).await {
        Ok(Some(settings)) => Json(settings).into_response(),
        Ok(None) => json_error(
            axum::http::StatusCode::NOT_FOUND,
            "control panel settings not found",
        ),
        Err(error) => {
            tracing::error!("failed to load control panel settings: {error}");
            json_error(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
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
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "failed to update control panel settings",
            );
        }
    }) else {
        return json_error(
            axum::http::StatusCode::NOT_FOUND,
            "control panel settings not found",
        );
    };
    let updated_by = match sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM users WHERE id = $1)")
        .bind(claims.sub)
        .fetch_one(&state.db)
        .await
    {
        Ok(true) => Some(claims.sub),
        Ok(false) => None,
        Err(error) => {
            tracing::error!("failed to validate control panel updater: {error}");
            return json_error(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
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
    .bind(body.platform_name.unwrap_or(current.platform_name))
    .bind(body.support_email.unwrap_or(current.support_email))
    .bind(body.docs_url.unwrap_or(current.docs_url))
    .bind(body.status_page_url.unwrap_or(current.status_page_url))
    .bind(body.announcement_banner.unwrap_or(current.announcement_banner))
    .bind(body.maintenance_mode.unwrap_or(current.maintenance_mode))
    .bind(body.release_channel.unwrap_or(current.release_channel))
    .bind(body.default_region.unwrap_or(current.default_region))
    .bind(body.deployment_mode.unwrap_or(current.deployment_mode))
    .bind(body.allow_self_signup.unwrap_or(current.allow_self_signup))
    .bind(SqlJson(
        body.allowed_email_domains
            .unwrap_or(current.allowed_email_domains),
    ))
    .bind(SqlJson(
        body.default_app_branding
            .unwrap_or(current.default_app_branding),
    ))
    .bind(SqlJson(
        body.restricted_operations
            .unwrap_or(current.restricted_operations),
    ))
    .bind(SqlJson(
        body.identity_provider_mappings
            .unwrap_or(current.identity_provider_mappings),
    ))
    .bind(SqlJson(
        body.resource_management_policies
            .unwrap_or(current.resource_management_policies),
    ))
    .bind(SqlJson(
        body.upgrade_assistant.unwrap_or(current.upgrade_assistant),
    ))
    .bind(updated_by)
    .execute(&state.db)
    .await;

    match result {
        Ok(_) => match load_control_panel(&state.db).await {
            Ok(Some(settings)) => Json(settings).into_response(),
            Ok(None) => json_error(
                axum::http::StatusCode::NOT_FOUND,
                "control panel settings not found after update",
            ),
            Err(error) => {
                tracing::error!("failed to reload control panel settings: {error}");
                json_error(
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    "failed to reload control panel settings",
                )
            }
        },
        Err(error) => {
            tracing::error!("failed to update control panel settings: {error}");
            json_error(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "failed to update control panel settings",
            )
        }
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
            return json_error(
                axum::http::StatusCode::NOT_FOUND,
                "control panel settings not found",
            );
        }
        Err(error) => {
            tracing::error!("failed to load control panel settings for readiness: {error}");
            return json_error(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "failed to load upgrade readiness",
            );
        }
    };

    let enabled_providers = match sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM sso_providers WHERE enabled = true",
    )
    .fetch_one(&state.db)
    .await
    {
        Ok(value) => value,
        Err(error) => {
            tracing::error!("failed to count enabled SSO providers: {error}");
            return json_error(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
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
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
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
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "failed to load upgrade readiness",
            );
        }
    };

    let checks = build_upgrade_readiness_checks(
        &settings,
        enabled_providers,
        enabled_policies,
        active_scoped_sessions,
    );
    let readiness = if checks.iter().any(|check| check.status == "blocked") {
        "blocked"
    } else if checks.iter().any(|check| check.status == "warning") {
        "attention"
    } else {
        "ready"
    };

    Json(UpgradeReadinessResponse {
        current_version: settings.upgrade_assistant.current_version.clone(),
        target_version: settings.upgrade_assistant.target_version.clone(),
        release_channel: settings.release_channel.clone(),
        readiness: readiness.to_string(),
        checks,
        generated_at: Utc::now(),
    })
    .into_response()
}

async fn load_control_panel(
    pool: &sqlx::PgPool,
) -> Result<Option<ControlPanelSettings>, String> {
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

fn build_upgrade_readiness_checks(
    settings: &ControlPanelSettings,
    enabled_providers: i64,
    enabled_policies: i64,
    active_scoped_sessions: i64,
) -> Vec<UpgradeReadinessCheck> {
    let mapping_count = settings.identity_provider_mappings.len() as i64;
    let resource_policy_count = settings.resource_management_policies.len() as i64;
    let maintenance_ready =
        settings.maintenance_mode || !settings.restricted_operations.is_empty();
    let rollback_ready = !settings.upgrade_assistant.rollback_channel.trim().is_empty()
        && !settings.upgrade_assistant.rollback_steps.is_empty();

    vec![
        UpgradeReadinessCheck {
            id: "identity_providers".to_string(),
            label: "Identity provider mapping".to_string(),
            status: if mapping_count == 0 {
                "blocked".to_string()
            } else if enabled_providers < mapping_count {
                "warning".to_string()
            } else {
                "ready".to_string()
            },
            detail: format!(
                "{mapping_count} mapping(s) configured for {enabled_providers} enabled provider(s)"
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
            detail: format!("{resource_policy_count} policy/policies available for tenant assignment"),
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
                    "maintenance_mode={} and {} restricted operation(s)",
                    settings.maintenance_mode,
                    settings.restricted_operations.len()
                )
            } else {
                "no maintenance window or restricted operation freeze is configured".to_string()
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
                if settings.upgrade_assistant.rollback_channel.trim().is_empty() {
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
            detail: format!("{enabled_policies} ABAC policy/policies enabled for rollout guardrails"),
        },
        UpgradeReadinessCheck {
            id: "temporary_sessions".to_string(),
            label: "Temporary access review".to_string(),
            status: if active_scoped_sessions <= 25 {
                "ready".to_string()
            } else {
                "warning".to_string()
            },
            detail: format!("{active_scoped_sessions} active scoped/guest session(s) require review"),
        },
    ]
}
