use auth_middleware::jwt::{self, JwtConfig};
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::{rbac, security};
use crate::models::session::RefreshToken;
use crate::models::user::User;

/// Issue an access + refresh token pair for a user.
pub async fn issue_tokens(
    pool: &PgPool,
    config: &JwtConfig,
    user: &User,
    auth_methods: Vec<String>,
) -> Result<(String, String), auth_middleware::JwtError> {
    let access_bundle = rbac::get_user_access_bundle(pool, user.id)
        .await
        .unwrap_or_default();

    let access_claims = jwt::build_access_claims(
        config,
        user.id,
        &user.email,
        &user.name,
        access_bundle.roles,
        access_bundle.permissions,
        user.organization_id,
        user.attributes.clone(),
        auth_methods,
    );
    let refresh_claims = jwt::build_refresh_claims(config, user.id);

    let access_token = jwt::encode_token(config, &access_claims)?;
    let refresh_token = jwt::encode_token(config, &refresh_claims)?;

    let _ = store_refresh_token(
        pool,
        user.id,
        refresh_claims.jti,
        &refresh_token,
        refresh_claims.exp,
    )
    .await;

    Ok((access_token, refresh_token))
}

pub async fn store_refresh_token(
    pool: &PgPool,
    user_id: Uuid,
    token_id: Uuid,
    refresh_token: &str,
    expires_at_ts: i64,
) -> Result<(), sqlx::Error> {
    let expires_at = chrono::DateTime::from_timestamp(expires_at_ts, 0)
        .unwrap_or_else(chrono::Utc::now)
        .with_timezone(&chrono::Utc);

    sqlx::query(
        r#"INSERT INTO refresh_tokens (id, user_id, token_hash, expires_at, revoked)
           VALUES ($1, $2, $3, $4, false)
           ON CONFLICT (id) DO UPDATE
           SET token_hash = EXCLUDED.token_hash,
               expires_at = EXCLUDED.expires_at,
               revoked = false"#,
    )
    .bind(token_id)
    .bind(user_id)
    .bind(security::hash_token(refresh_token))
    .bind(expires_at)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn revoke_refresh_token(pool: &PgPool, token_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE refresh_tokens SET revoked = true WHERE id = $1")
        .bind(token_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn get_refresh_token(
    pool: &PgPool,
    token_id: Uuid,
) -> Result<Option<RefreshToken>, sqlx::Error> {
    sqlx::query_as::<_, RefreshToken>(
        "SELECT id, user_id, token_hash, expires_at, revoked, created_at FROM refresh_tokens WHERE id = $1",
    )
    .bind(token_id)
    .fetch_optional(pool)
    .await
}

pub fn refresh_token_matches(token: &RefreshToken, raw_token: &str) -> bool {
    !token.revoked
        && token.token_hash == security::hash_token(raw_token)
        && token.expires_at > chrono::Utc::now()
}
