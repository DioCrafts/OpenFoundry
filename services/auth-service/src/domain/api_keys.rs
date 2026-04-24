use auth_middleware::jwt::{self, JwtConfig};
use chrono::{DateTime, Utc};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::rbac;
use crate::models::api_key::{ApiKey, ApiKeyWithSecret};
use crate::models::user::User;

#[derive(Debug)]
pub enum ApiKeyError {
    Database(sqlx::Error),
    Token(auth_middleware::JwtError),
    InvalidExpiration,
}

impl From<sqlx::Error> for ApiKeyError {
    fn from(value: sqlx::Error) -> Self {
        Self::Database(value)
    }
}

impl From<auth_middleware::JwtError> for ApiKeyError {
    fn from(value: auth_middleware::JwtError) -> Self {
        Self::Token(value)
    }
}

pub async fn list_api_keys(pool: &PgPool, user_id: Uuid) -> Result<Vec<ApiKey>, sqlx::Error> {
    sqlx::query_as::<_, ApiKey>(
		r#"SELECT id, user_id, name, prefix, scopes, expires_at, last_used_at, revoked_at, created_at
		   FROM api_keys
		   WHERE user_id = $1
		   ORDER BY created_at DESC"#,
	)
	.bind(user_id)
	.fetch_all(pool)
	.await
}

pub async fn create_api_key(
    pool: &PgPool,
    config: &JwtConfig,
    user: &User,
    name: &str,
    scopes: Vec<String>,
    expires_at: Option<DateTime<Utc>>,
) -> Result<ApiKeyWithSecret, ApiKeyError> {
    let now = Utc::now();
    if expires_at.is_some_and(|candidate| candidate <= now) {
        return Err(ApiKeyError::InvalidExpiration);
    }

    let access_bundle = rbac::get_user_access_bundle(pool, user.id)
        .await
        .unwrap_or_default();
    let granted_scopes = if scopes.is_empty() {
        access_bundle.permissions.clone()
    } else {
        scopes
    };

    let api_key_id = Uuid::now_v7();
    let prefix = format!("ofk_{}", &api_key_id.to_string()[..8]);
    let expires_in_secs = expires_at
        .map(|candidate| (candidate - now).num_seconds())
        .unwrap_or(60 * 60 * 24 * 180);

    let token = jwt::encode_token(
        config,
        &jwt::build_api_key_claims(
            config,
            user.id,
            &user.email,
            &user.name,
            access_bundle.roles,
            granted_scopes.clone(),
            user.organization_id,
            user.attributes.clone(),
            api_key_id,
            expires_in_secs,
        ),
    )?;

    sqlx::query(
        r#"INSERT INTO api_keys (id, user_id, name, prefix, scopes, expires_at)
		   VALUES ($1, $2, $3, $4, $5, $6)"#,
    )
    .bind(api_key_id)
    .bind(user.id)
    .bind(name)
    .bind(&prefix)
    .bind(json!(granted_scopes))
    .bind(expires_at)
    .execute(pool)
    .await?;

    Ok(ApiKeyWithSecret {
        id: api_key_id,
        name: name.to_string(),
        prefix,
        token,
        scopes: json!(granted_scopes),
        expires_at,
        created_at: now,
    })
}

pub async fn revoke_api_key(
    pool: &PgPool,
    api_key_id: Uuid,
    user_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
		"UPDATE api_keys SET revoked_at = NOW() WHERE id = $1 AND user_id = $2 AND revoked_at IS NULL",
	)
	.bind(api_key_id)
	.bind(user_id)
	.execute(pool)
	.await?;

    Ok(result.rows_affected() > 0)
}
