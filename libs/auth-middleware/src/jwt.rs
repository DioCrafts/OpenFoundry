use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde_json::Value;
use thiserror::Error;
use uuid::Uuid;

use crate::claims::Claims;

#[derive(Debug, Error)]
pub enum JwtError {
    #[error("token expired")]
    Expired,
    #[error("invalid token: {0}")]
    Invalid(String),
    #[error("encoding error: {0}")]
    Encoding(String),
}

/// Configuration for JWT token operations.
#[derive(Debug, Clone)]
pub struct JwtConfig {
    /// Secret key for HMAC-SHA256 signing.
    secret: Vec<u8>,
    /// Access token TTL in seconds (default: 3600).
    pub access_ttl_secs: i64,
    /// Refresh token TTL in seconds (default: 604800 = 7 days).
    pub refresh_ttl_secs: i64,
}

impl JwtConfig {
    pub fn new(secret: &str) -> Self {
        Self {
            secret: secret.as_bytes().to_vec(),
            access_ttl_secs: 3600,
            refresh_ttl_secs: 604_800,
        }
    }

    pub fn with_access_ttl(mut self, secs: i64) -> Self {
        self.access_ttl_secs = secs;
        self
    }

    pub fn with_refresh_ttl(mut self, secs: i64) -> Self {
        self.refresh_ttl_secs = secs;
        self
    }
}

/// Encode claims into a signed JWT string.
pub fn encode_token(config: &JwtConfig, claims: &Claims) -> Result<String, JwtError> {
    encode(
        &Header::default(),
        claims,
        &EncodingKey::from_secret(&config.secret),
    )
    .map_err(|e| JwtError::Encoding(e.to_string()))
}

/// Decode and validate a JWT string into Claims.
pub fn decode_token(config: &JwtConfig, token: &str) -> Result<Claims, JwtError> {
    let mut validation = Validation::default();
    validation.validate_exp = true;
    validation.leeway = 30; // 30 second leeway for clock skew

    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(&config.secret),
        &validation,
    )
    .map_err(|e| match e.kind() {
        jsonwebtoken::errors::ErrorKind::ExpiredSignature => JwtError::Expired,
        _ => JwtError::Invalid(e.to_string()),
    })?;

    Ok(data.claims)
}

/// Build a new Claims set for a user (access token).
pub fn build_access_claims(
    config: &JwtConfig,
    user_id: Uuid,
    email: &str,
    name: &str,
    roles: Vec<String>,
    permissions: Vec<String>,
    org_id: Option<Uuid>,
    attributes: Value,
    auth_methods: Vec<String>,
) -> Claims {
    let now = chrono::Utc::now().timestamp();
    Claims {
        sub: user_id,
        iat: now,
        exp: now + config.access_ttl_secs,
        jti: Uuid::now_v7(),
        email: email.to_string(),
        name: name.to_string(),
        roles,
        permissions,
        org_id,
        attributes,
        auth_methods,
        token_use: Some("access".to_string()),
        api_key_id: None,
    }
}

/// Build a minimal Claims set for a refresh token.
pub fn build_refresh_claims(config: &JwtConfig, user_id: Uuid) -> Claims {
    let now = chrono::Utc::now().timestamp();
    Claims {
        sub: user_id,
        iat: now,
        exp: now + config.refresh_ttl_secs,
        jti: Uuid::now_v7(),
        email: String::new(),
        name: String::new(),
        roles: vec![],
        permissions: vec![],
        org_id: None,
        attributes: Value::Object(Default::default()),
        auth_methods: vec![],
        token_use: Some("refresh".to_string()),
        api_key_id: None,
    }
}

/// Build claims for a long-lived API key.
pub fn build_api_key_claims(
    _config: &JwtConfig,
    user_id: Uuid,
    email: &str,
    name: &str,
    roles: Vec<String>,
    permissions: Vec<String>,
    org_id: Option<Uuid>,
    attributes: Value,
    api_key_id: Uuid,
    expires_in_secs: i64,
) -> Claims {
    let now = chrono::Utc::now().timestamp();
    Claims {
        sub: user_id,
        iat: now,
        exp: now + expires_in_secs,
        jti: api_key_id,
        email: email.to_string(),
        name: name.to_string(),
        roles,
        permissions,
        org_id,
        attributes,
        auth_methods: vec!["api_key".to_string()],
        token_use: Some("api_key".to_string()),
        api_key_id: Some(api_key_id),
    }
}
