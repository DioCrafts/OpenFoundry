use std::collections::HashSet;

use auth_middleware::Claims;
use auth_middleware::jwt::{self, JwtConfig};
use chrono::Utc;
use hmac::{Hmac, Mac};
use serde_json::{Value, json};
use sha1::Sha1;

use crate::domain::security;
use crate::models::user::User;

type HmacSha1 = Hmac<Sha1>;

pub struct TotpEnrollment {
    pub secret: String,
    pub recovery_codes: Vec<String>,
    pub otpauth_uri: String,
}

pub fn create_enrollment(email: &str) -> TotpEnrollment {
    let secret = security::random_base32_secret(20);
    let recovery_codes = security::generate_recovery_codes(8);

    TotpEnrollment {
        otpauth_uri: format!(
            "otpauth://totp/OpenFoundry:{email}?secret={secret}&issuer=OpenFoundry&algorithm=SHA1&digits=6&period=30"
        ),
        secret,
        recovery_codes,
    }
}

pub fn verify_totp(secret: &str, code: &str) -> bool {
    let code = code.trim().replace(' ', "");
    (-1_i64..=1_i64)
        .any(|offset| generate_totp(secret, offset).is_some_and(|candidate| candidate == code))
}

pub fn hash_recovery_codes(codes: &[String]) -> Value {
    json!(
        codes
            .iter()
            .map(|code| security::hash_token(code))
            .collect::<Vec<_>>()
    )
}

pub fn consume_recovery_code(hashes: &Value, code: &str) -> Option<Value> {
    let current_hash = security::hash_token(code);
    let existing = hashes.as_array()?;
    let mut next_codes = Vec::new();
    let mut consumed = false;

    for candidate in existing {
        if candidate.as_str() == Some(current_hash.as_str()) && !consumed {
            consumed = true;
            continue;
        }
        next_codes.push(candidate.clone());
    }

    consumed.then(|| Value::Array(next_codes))
}

pub fn issue_challenge(
    config: &JwtConfig,
    user: &User,
    auth_method: &str,
) -> Result<String, auth_middleware::JwtError> {
    let claims = Claims {
        sub: user.id,
        iat: Utc::now().timestamp(),
        exp: Utc::now().timestamp() + 300,
        iss: config.issuer().map(str::to_string),
        aud: config.audience().map(str::to_string),
        jti: uuid::Uuid::now_v7(),
        email: user.email.clone(),
        name: user.name.clone(),
        roles: vec![],
        permissions: vec![],
        org_id: user.organization_id,
        attributes: user.attributes.clone(),
        auth_methods: vec![auth_method.to_string()],
        token_use: Some("mfa_challenge".to_string()),
        api_key_id: None,
        session_kind: None,
        session_scope: None,
    };

    jwt::encode_token(config, &claims)
}

pub fn validate_challenge(
    config: &JwtConfig,
    token: &str,
) -> Result<Claims, auth_middleware::JwtError> {
    let claims = jwt::decode_token(config, token)?;
    if claims.token_use.as_deref() == Some("mfa_challenge") {
        Ok(claims)
    } else {
        Err(auth_middleware::JwtError::Invalid(
            "invalid challenge token".to_string(),
        ))
    }
}

fn generate_totp(secret: &str, offset_window: i64) -> Option<String> {
    let secret_bytes = base32::decode(base32::Alphabet::RFC4648 { padding: false }, secret)?;
    let counter = ((Utc::now().timestamp() / 30) + offset_window) as u64;

    let mut mac = HmacSha1::new_from_slice(&secret_bytes).ok()?;
    mac.update(&counter.to_be_bytes());
    let digest = mac.finalize().into_bytes();

    let offset = (digest[19] & 0x0f) as usize;
    let binary = (u32::from(digest[offset] & 0x7f) << 24)
        | (u32::from(digest[offset + 1]) << 16)
        | (u32::from(digest[offset + 2]) << 8)
        | u32::from(digest[offset + 3]);

    Some(format!("{:06}", binary % 1_000_000))
}

pub fn normalize_scopes(methods: &[String]) -> Vec<String> {
    methods
        .iter()
        .cloned()
        .collect::<HashSet<_>>()
        .into_iter()
        .collect()
}
