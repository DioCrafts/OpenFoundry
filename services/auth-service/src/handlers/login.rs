use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use serde::{Deserialize, Serialize};

use crate::AppState;
use crate::domain::jwt::issue_tokens;
use crate::domain::mfa;
use crate::models::mfa::TotpConfiguration;
use crate::models::user::User;

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: i64,
}

#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum LoginResponse {
    Authenticated {
        access_token: String,
        refresh_token: String,
        token_type: String,
        expires_in: i64,
    },
    MfaRequired {
        challenge_token: String,
        methods: Vec<String>,
        expires_in: i64,
    },
}

pub async fn login(
    State(state): State<AppState>,
    Json(body): Json<LoginRequest>,
) -> impl IntoResponse {
    let user = sqlx::query_as::<_, User>(
        "SELECT id, email, name, password_hash, is_active, organization_id, attributes, mfa_enforced, auth_source, created_at, updated_at FROM users WHERE email = $1",
    )
    .bind(&body.email)
    .fetch_optional(&state.db)
    .await;

    let user = match user {
        Ok(Some(u)) if u.is_active => u,
        Ok(Some(_)) => {
            return (
                StatusCode::FORBIDDEN,
                Json(serde_json::json!({ "error": "account disabled" })),
            )
                .into_response();
        }
        _ => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({ "error": "invalid credentials" })),
            )
                .into_response();
        }
    };

    if !verify_password(&body.password, &user.password_hash) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({ "error": "invalid credentials" })),
        )
            .into_response();
    }

    let mfa_configuration = sqlx::query_as::<_, TotpConfiguration>(
        "SELECT user_id, secret, recovery_code_hashes, enabled, verified_at, created_at, updated_at FROM user_mfa_totp WHERE user_id = $1",
    )
    .bind(user.id)
    .fetch_optional(&state.db)
    .await;

    let mfa_configuration = match mfa_configuration {
        Ok(config) => config,
        Err(e) => {
            tracing::error!("failed to load MFA configuration: {e}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "login failed" })),
            )
                .into_response();
        }
    };

    if let Some(configuration) = mfa_configuration {
        if configuration.enabled {
            return match mfa::issue_challenge(&state.jwt_config, &user, "password") {
                Ok(challenge_token) => Json(LoginResponse::MfaRequired {
                    challenge_token,
                    methods: vec!["totp".to_string()],
                    expires_in: 300,
                })
                .into_response(),
                Err(e) => {
                    tracing::error!("failed to issue MFA challenge: {e}");
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::json!({ "error": "login failed" })),
                    )
                        .into_response()
                }
            };
        }
    } else if user.mfa_enforced {
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({ "error": "mfa setup required by administrator" })),
        )
            .into_response();
    }

    match issue_tokens(
        &state.db,
        &state.jwt_config,
        &user,
        vec!["password".to_string()],
    )
    .await
    {
        Ok((access_token, refresh_token)) => {
            tracing::info!(user_id = %user.id, "user logged in");
            Json(LoginResponse::Authenticated {
                access_token,
                refresh_token,
                token_type: "Bearer".to_string(),
                expires_in: state.jwt_config.access_ttl_secs,
            })
            .into_response()
        }
        Err(e) => {
            tracing::error!("token generation failed: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "token generation failed" })),
            )
                .into_response()
        }
    }
}

fn verify_password(password: &str, hash: &str) -> bool {
    use argon2::PasswordHash;
    use argon2::{Argon2, PasswordVerifier};

    let parsed = match PasswordHash::new(hash) {
        Ok(h) => h,
        Err(_) => return false,
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok()
}
