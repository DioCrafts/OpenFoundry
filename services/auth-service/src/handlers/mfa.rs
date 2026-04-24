use auth_middleware::layer::AuthUser;
use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::AppState;
use crate::domain::{jwt, mfa};
use crate::models::mfa::TotpConfiguration;
use crate::models::user::User;

use super::common::json_error;
use super::login::TokenResponse;

#[derive(Debug, Serialize)]
pub struct MfaStatusResponse {
    pub configured: bool,
    pub enabled: bool,
    pub recovery_codes_remaining: usize,
}

#[derive(Debug, Serialize)]
pub struct EnrollMfaResponse {
    pub secret: String,
    pub recovery_codes: Vec<String>,
    pub otpauth_uri: String,
}

#[derive(Debug, Deserialize)]
pub struct VerifyMfaRequest {
    pub code: String,
}

#[derive(Debug, Deserialize)]
pub struct CompleteLoginRequest {
    pub challenge_token: String,
    pub code: String,
}

pub async fn status(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> impl IntoResponse {
    if !claims.has_permission("mfa", "self") && !claims.has_permission("users", "write") {
        return json_error(StatusCode::FORBIDDEN, "missing permission mfa:self");
    }

    match load_mfa_configuration(&state.db, claims.sub).await {
        Ok(Some(configuration)) => Json(MfaStatusResponse {
            configured: true,
            enabled: configuration.enabled,
            recovery_codes_remaining: configuration
                .recovery_code_hashes
                .as_array()
                .map_or(0, Vec::len),
        })
        .into_response(),
        Ok(None) => Json(MfaStatusResponse {
            configured: false,
            enabled: false,
            recovery_codes_remaining: 0,
        })
        .into_response(),
        Err(e) => {
            tracing::error!("failed to load MFA status: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn enroll(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> impl IntoResponse {
    if !claims.has_permission("mfa", "self") && !claims.has_permission("users", "write") {
        return json_error(StatusCode::FORBIDDEN, "missing permission mfa:self");
    }

    let enrollment = mfa::create_enrollment(&claims.email);
    let recovery_code_hashes = mfa::hash_recovery_codes(&enrollment.recovery_codes);

    match sqlx::query(
        r#"INSERT INTO user_mfa_totp (user_id, secret, recovery_code_hashes, enabled)
           VALUES ($1, $2, $3, false)
           ON CONFLICT (user_id) DO UPDATE
           SET secret = EXCLUDED.secret,
               recovery_code_hashes = EXCLUDED.recovery_code_hashes,
               enabled = false,
               verified_at = NULL,
               updated_at = NOW()"#,
    )
    .bind(claims.sub)
    .bind(&enrollment.secret)
    .bind(recovery_code_hashes)
    .execute(&state.db)
    .await
    {
        Ok(_) => Json(EnrollMfaResponse {
            secret: enrollment.secret,
            recovery_codes: enrollment.recovery_codes,
            otpauth_uri: enrollment.otpauth_uri,
        })
        .into_response(),
        Err(e) => {
            tracing::error!("failed to persist MFA enrollment: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn verify_setup(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(body): Json<VerifyMfaRequest>,
) -> impl IntoResponse {
    if !claims.has_permission("mfa", "self") && !claims.has_permission("users", "write") {
        return json_error(StatusCode::FORBIDDEN, "missing permission mfa:self");
    }

    let Some(configuration) = (match load_mfa_configuration(&state.db, claims.sub).await {
        Ok(configuration) => configuration,
        Err(e) => {
            tracing::error!("failed to load MFA configuration: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    }) else {
        return StatusCode::NOT_FOUND.into_response();
    };

    if !mfa::verify_totp(&configuration.secret, &body.code) {
        return json_error(StatusCode::UNAUTHORIZED, "invalid MFA code");
    }

    match sqlx::query(
        "UPDATE user_mfa_totp SET enabled = true, verified_at = NOW(), updated_at = NOW() WHERE user_id = $1",
    )
    .bind(claims.sub)
    .execute(&state.db)
    .await
    {
        Ok(_) => Json(json!({ "enabled": true })).into_response(),
        Err(e) => {
            tracing::error!("failed to enable MFA: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn disable(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(body): Json<VerifyMfaRequest>,
) -> impl IntoResponse {
    if !claims.has_permission("mfa", "self") && !claims.has_permission("users", "write") {
        return json_error(StatusCode::FORBIDDEN, "missing permission mfa:self");
    }

    let user = match load_user(&state.db, claims.sub).await {
        Ok(Some(user)) => user,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("failed to load user for MFA disable: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    if user.mfa_enforced && !claims.has_permission("users", "write") {
        return json_error(StatusCode::FORBIDDEN, "mfa is enforced by an administrator");
    }

    let Some(configuration) = (match load_mfa_configuration(&state.db, claims.sub).await {
        Ok(configuration) => configuration,
        Err(e) => {
            tracing::error!("failed to load MFA configuration: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    }) else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let valid_code = mfa::verify_totp(&configuration.secret, &body.code)
        || mfa::consume_recovery_code(&configuration.recovery_code_hashes, &body.code).is_some();
    if !valid_code {
        return json_error(StatusCode::UNAUTHORIZED, "invalid MFA code");
    }

    match sqlx::query("DELETE FROM user_mfa_totp WHERE user_id = $1")
        .bind(claims.sub)
        .execute(&state.db)
        .await
    {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => {
            tracing::error!("failed to disable MFA: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn complete_login(
    State(state): State<AppState>,
    Json(body): Json<CompleteLoginRequest>,
) -> impl IntoResponse {
    let challenge = match mfa::validate_challenge(&state.jwt_config, &body.challenge_token) {
        Ok(challenge) => challenge,
        Err(_) => return json_error(StatusCode::UNAUTHORIZED, "invalid MFA challenge"),
    };

    let user = match load_user(&state.db, challenge.sub).await {
        Ok(Some(user)) => user,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("failed to load user for MFA completion: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let Some(configuration) = (match load_mfa_configuration(&state.db, user.id).await {
        Ok(configuration) => configuration,
        Err(e) => {
            tracing::error!("failed to load MFA configuration: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    }) else {
        return json_error(StatusCode::UNAUTHORIZED, "mfa not configured");
    };

    let mut next_recovery_hashes = None;
    let valid_code = if mfa::verify_totp(&configuration.secret, &body.code) {
        true
    } else if let Some(updated_hashes) =
        mfa::consume_recovery_code(&configuration.recovery_code_hashes, &body.code)
    {
        next_recovery_hashes = Some(updated_hashes);
        true
    } else {
        false
    };

    if !valid_code {
        return json_error(StatusCode::UNAUTHORIZED, "invalid MFA code");
    }

    if let Some(updated_hashes) = next_recovery_hashes {
        if let Err(e) = sqlx::query(
            "UPDATE user_mfa_totp SET recovery_code_hashes = $2, updated_at = NOW() WHERE user_id = $1",
        )
        .bind(user.id)
        .bind(updated_hashes)
        .execute(&state.db)
        .await
        {
            tracing::error!("failed to consume recovery code: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    }

    let mut auth_methods = challenge.auth_methods;
    auth_methods.push("mfa".to_string());

    match jwt::issue_tokens(
        &state.db,
        &state.jwt_config,
        &user,
        mfa::normalize_scopes(&auth_methods),
    )
    .await
    {
        Ok((access_token, refresh_token)) => Json(TokenResponse {
            access_token,
            refresh_token,
            token_type: "Bearer".to_string(),
            expires_in: state.jwt_config.access_ttl_secs,
        })
        .into_response(),
        Err(e) => {
            tracing::error!("failed to issue tokens after MFA: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

async fn load_mfa_configuration(
    pool: &sqlx::PgPool,
    user_id: uuid::Uuid,
) -> Result<Option<TotpConfiguration>, sqlx::Error> {
    sqlx::query_as::<_, TotpConfiguration>(
        "SELECT user_id, secret, recovery_code_hashes, enabled, verified_at, created_at, updated_at FROM user_mfa_totp WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
}

async fn load_user(pool: &sqlx::PgPool, user_id: uuid::Uuid) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>(
        "SELECT id, email, name, password_hash, is_active, organization_id, attributes, mfa_enforced, auth_source, created_at, updated_at FROM users WHERE id = $1",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
}
