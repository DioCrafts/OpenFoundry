use std::{env, fs};

use jsonwebtoken::{
    Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, decode_header, encode,
};
use serde_json::Value;
use thiserror::Error;
use uuid::Uuid;

use crate::claims::{Claims, SessionScope};

const JWT_ISSUER_ENV_KEYS: &[&str] = &["OPENFOUNDRY_JWT_ISSUER", "JWT_ISSUER"];
const JWT_AUDIENCE_ENV_KEYS: &[&str] = &["OPENFOUNDRY_JWT_AUDIENCE", "JWT_AUDIENCE"];
const JWT_KEY_ID_ENV_KEYS: &[&str] = &["OPENFOUNDRY_JWT_KID", "JWT_KID"];
const JWT_PRIVATE_KEY_ENV_KEYS: &[&str] =
    &["OPENFOUNDRY_JWT_PRIVATE_KEY_PEM", "JWT_PRIVATE_KEY_PEM"];
const JWT_PRIVATE_KEY_PATH_ENV_KEYS: &[&str] =
    &["OPENFOUNDRY_JWT_PRIVATE_KEY_PATH", "JWT_PRIVATE_KEY_PATH"];
const JWT_PUBLIC_KEY_ENV_KEYS: &[&str] = &["OPENFOUNDRY_JWT_PUBLIC_KEY_PEM", "JWT_PUBLIC_KEY_PEM"];
const JWT_PUBLIC_KEY_PATH_ENV_KEYS: &[&str] =
    &["OPENFOUNDRY_JWT_PUBLIC_KEY_PATH", "JWT_PUBLIC_KEY_PATH"];

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
    /// Optional token issuer applied to issued tokens and required during validation.
    issuer: Option<String>,
    /// Optional token audience applied to issued tokens and required during validation.
    audience: Option<String>,
    /// Optional JOSE key id embedded in JWT headers.
    key_id: Option<String>,
    /// Optional PEM-encoded RSA private key used for RS256 signing.
    rsa_private_key_pem: Option<String>,
    /// Optional PEM-encoded RSA public key used for RS256 verification.
    rsa_public_key_pem: Option<String>,
    /// Access token TTL in seconds (default: 3600).
    pub access_ttl_secs: i64,
    /// Refresh token TTL in seconds (default: 604800 = 7 days).
    pub refresh_ttl_secs: i64,
}

impl JwtConfig {
    pub fn new(secret: &str) -> Self {
        Self {
            secret: secret.as_bytes().to_vec(),
            issuer: None,
            audience: None,
            key_id: None,
            rsa_private_key_pem: None,
            rsa_public_key_pem: None,
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

    pub fn with_issuer(mut self, issuer: impl Into<String>) -> Self {
        self.issuer = Some(issuer.into());
        self
    }

    pub fn with_audience(mut self, audience: impl Into<String>) -> Self {
        self.audience = Some(audience.into());
        self
    }

    pub fn with_key_id(mut self, key_id: impl Into<String>) -> Self {
        self.key_id = Some(key_id.into());
        self
    }

    pub fn with_rsa_keys(
        mut self,
        private_key_pem: impl Into<String>,
        public_key_pem: impl Into<String>,
    ) -> Self {
        self.rsa_private_key_pem = Some(private_key_pem.into());
        self.rsa_public_key_pem = Some(public_key_pem.into());
        self
    }

    pub fn with_env_defaults(mut self) -> Self {
        if let Some(issuer) = read_first_env(JWT_ISSUER_ENV_KEYS) {
            self = self.with_issuer(issuer);
        }
        if let Some(audience) = read_first_env(JWT_AUDIENCE_ENV_KEYS) {
            self = self.with_audience(audience);
        }
        if let Some(key_id) = read_first_env(JWT_KEY_ID_ENV_KEYS) {
            self = self.with_key_id(key_id);
        }

        let private_key =
            read_pem_from_env(JWT_PRIVATE_KEY_ENV_KEYS, JWT_PRIVATE_KEY_PATH_ENV_KEYS);
        let public_key = read_pem_from_env(JWT_PUBLIC_KEY_ENV_KEYS, JWT_PUBLIC_KEY_PATH_ENV_KEYS);

        match (private_key, public_key) {
            (Some(private_key), Some(public_key)) => self.with_rsa_keys(private_key, public_key),
            (Some(_), None) | (None, Some(_)) => {
                tracing::warn!(
                    "partial JWT RSA configuration detected; falling back to shared-secret HS256"
                );
                self
            }
            (None, None) => self,
        }
    }

    pub fn issuer(&self) -> Option<&str> {
        self.issuer.as_deref()
    }

    pub fn audience(&self) -> Option<&str> {
        self.audience.as_deref()
    }

    fn algorithm(&self) -> Algorithm {
        if self.rsa_private_key_pem.is_some() && self.rsa_public_key_pem.is_some() {
            Algorithm::RS256
        } else {
            Algorithm::HS256
        }
    }

    fn encoding_key(&self) -> Result<EncodingKey, JwtError> {
        match self.algorithm() {
            Algorithm::HS256 => Ok(EncodingKey::from_secret(&self.secret)),
            Algorithm::RS256 => self
                .rsa_private_key_pem
                .as_ref()
                .ok_or_else(|| JwtError::Encoding("missing RSA private key".to_string()))
                .and_then(|pem| {
                    EncodingKey::from_rsa_pem(pem.as_bytes())
                        .map_err(|error| JwtError::Encoding(error.to_string()))
                }),
            other => Err(JwtError::Encoding(format!(
                "unsupported signing algorithm {other:?}"
            ))),
        }
    }

    fn decoding_key(&self) -> Result<DecodingKey, JwtError> {
        match self.algorithm() {
            Algorithm::HS256 => Ok(DecodingKey::from_secret(&self.secret)),
            Algorithm::RS256 => self
                .rsa_public_key_pem
                .as_ref()
                .ok_or_else(|| JwtError::Invalid("missing RSA public key".to_string()))
                .and_then(|pem| {
                    DecodingKey::from_rsa_pem(pem.as_bytes())
                        .map_err(|error| JwtError::Invalid(error.to_string()))
                }),
            other => Err(JwtError::Invalid(format!(
                "unsupported signing algorithm {other:?}"
            ))),
        }
    }
}

/// Encode claims into a signed JWT string.
pub fn encode_token(config: &JwtConfig, claims: &Claims) -> Result<String, JwtError> {
    let mut header = Header::new(config.algorithm());
    header.kid = config.key_id.clone();

    encode(&header, claims, &config.encoding_key()?).map_err(|e| JwtError::Encoding(e.to_string()))
}

/// Decode and validate a JWT string into Claims.
pub fn decode_token(config: &JwtConfig, token: &str) -> Result<Claims, JwtError> {
    let header = decode_header(token).map_err(|error| JwtError::Invalid(error.to_string()))?;
    let algorithm = config.algorithm();
    if header.alg != algorithm {
        return Err(JwtError::Invalid(format!(
            "unexpected signing algorithm {:?}",
            header.alg
        )));
    }

    let mut validation = Validation::new(algorithm);
    validation.validate_exp = true;
    validation.leeway = 30; // 30 second leeway for clock skew

    if let Some(issuer) = config.issuer() {
        validation.set_issuer(&[issuer]);
    }
    if let Some(audience) = config.audience() {
        validation.set_audience(&[audience]);
    }

    let data =
        decode::<Claims>(token, &config.decoding_key()?, &validation).map_err(|e| {
            match e.kind() {
                jsonwebtoken::errors::ErrorKind::ExpiredSignature => JwtError::Expired,
                _ => JwtError::Invalid(e.to_string()),
            }
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
    build_access_claims_with_scope(
        config,
        user_id,
        email,
        name,
        roles,
        permissions,
        org_id,
        attributes,
        auth_methods,
        None,
        Some("access".to_string()),
    )
}

pub fn build_access_claims_with_scope(
    config: &JwtConfig,
    user_id: Uuid,
    email: &str,
    name: &str,
    roles: Vec<String>,
    permissions: Vec<String>,
    org_id: Option<Uuid>,
    attributes: Value,
    auth_methods: Vec<String>,
    session_scope: Option<SessionScope>,
    session_kind: Option<String>,
) -> Claims {
    let now = chrono::Utc::now().timestamp();
    base_claims(
        config,
        user_id,
        now,
        now + config.access_ttl_secs,
        email.to_string(),
        name.to_string(),
        roles,
        permissions,
        org_id,
        attributes,
        auth_methods,
        Some("access".to_string()),
        None,
        session_kind,
        session_scope,
    )
}

/// Build a minimal Claims set for a refresh token.
pub fn build_refresh_claims(config: &JwtConfig, user_id: Uuid) -> Claims {
    let now = chrono::Utc::now().timestamp();
    base_claims(
        config,
        user_id,
        now,
        now + config.refresh_ttl_secs,
        String::new(),
        String::new(),
        vec![],
        vec![],
        None,
        Value::Object(Default::default()),
        vec![],
        Some("refresh".to_string()),
        None,
        None,
        None,
    )
}

/// Build claims for a long-lived API key.
pub fn build_api_key_claims(
    config: &JwtConfig,
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
    build_api_key_claims_with_scope(
        config,
        user_id,
        email,
        name,
        roles,
        permissions,
        org_id,
        attributes,
        api_key_id,
        expires_in_secs,
        None,
        None,
    )
}

pub fn build_api_key_claims_with_scope(
    config: &JwtConfig,
    user_id: Uuid,
    email: &str,
    name: &str,
    roles: Vec<String>,
    permissions: Vec<String>,
    org_id: Option<Uuid>,
    attributes: Value,
    api_key_id: Uuid,
    expires_in_secs: i64,
    session_scope: Option<SessionScope>,
    session_kind: Option<String>,
) -> Claims {
    let now = chrono::Utc::now().timestamp();
    base_claims(
        config,
        user_id,
        now,
        now + expires_in_secs,
        email.to_string(),
        name.to_string(),
        roles,
        permissions,
        org_id,
        attributes,
        vec!["api_key".to_string()],
        Some("api_key".to_string()),
        Some(api_key_id),
        session_kind,
        session_scope,
    )
}

fn base_claims(
    config: &JwtConfig,
    sub: Uuid,
    iat: i64,
    exp: i64,
    email: String,
    name: String,
    roles: Vec<String>,
    permissions: Vec<String>,
    org_id: Option<Uuid>,
    attributes: Value,
    auth_methods: Vec<String>,
    token_use: Option<String>,
    api_key_id: Option<Uuid>,
    session_kind: Option<String>,
    session_scope: Option<SessionScope>,
) -> Claims {
    Claims {
        sub,
        iat,
        exp,
        iss: config.issuer.clone(),
        aud: config.audience.clone(),
        jti: api_key_id.unwrap_or_else(Uuid::now_v7),
        email,
        name,
        roles,
        permissions,
        org_id,
        attributes,
        auth_methods,
        token_use,
        api_key_id,
        session_kind,
        session_scope,
    }
}

fn read_first_env(keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| read_env(key))
}

fn read_env(key: &str) -> Option<String> {
    env::var(key)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn read_pem_from_env(value_keys: &[&str], path_keys: &[&str]) -> Option<String> {
    read_first_env(value_keys)
        .map(|value| value.replace("\\n", "\n"))
        .or_else(|| {
            read_first_env(path_keys).and_then(|path| {
                fs::read_to_string(path)
                    .ok()
                    .map(|value| value.trim().to_string())
                    .filter(|value| !value.is_empty())
            })
        })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    const RSA_PRIVATE_KEY: &str = r#"-----BEGIN PRIVATE KEY-----
MIIEvgIBADANBgkqhkiG9w0BAQEFAASCBKgwggSkAgEAAoIBAQDgoWcspyon7gi7
/8v8jQSOFsf4DuA/fnl+bVSfLI9mGNlNqr0YZg4ugjq/NMS06tHAXaT7UlBGCiOq
DDo+PmWBBSf3rZB0DZNH+sF63SJsB2N7YXBG4xg2TWG9CRQomaVlptIqoaRAT/Ot
uPJ2GPxtdHXRfHBqnkA81mxxTxoV3UUtJIeo5WatAA3r+uHcPMjCZuIQDbXc+5Sg
//l6OQivKI4QguOv4BVgbBvIQA9WVEVcO8Na7QVp8fAccTffB32xyVH21WSsZe8P
ourdegntMsJio5SKv1l7yVxJoOD+K2owdPj6bSxe2EaxfrulmirIGs2lOeX6j4qR
Y0zqvHQZAgMBAAECggEARFIzDEzHsJ9gfrW9eFH3ybO6HIOBxy4Ti9V7AHLQJrB2
H35Hx0z7EUBA1/kXvyMQqt6QmHQfwD3DPSw85sOZodVMo7NhlTqvyhvFjzYFCzBw
HI21VYoqyhFdId7KB9M7kCBeGeNSDtGCfxsae7r7w9rBHvcnRfZd+WMKVqhFedJh
8YVHL+IV5iw01/9i7PkR4ChD4Oc1W0Lw4pXAy8v0xhOqrF7M1mHUZP5tmDdfaVzC
0YIuL6axqX/ZBJMUfiyAe3/7PUE9f4uwqsddYcoMDAV2l0TjXUsFCqAhz1ubW48l
WtiGZy39eo9ybM5LLWnEzIYpiGvaC+qwozJWEvsvFQKBgQD8PIDRsXlNoil1wmNn
qYrHLPt+zwyst0csOZXLkHpvMuHcEli1DhoCcNqwEGZoJCXngNFpaLNGKdBrG9xa
sq98rnwUFZjXGJnHkLUuxgc7fLZnePG0xLyzDJ4/dgqMvez8GqFEtWIRuL0T2Qta
G59Lxz9D3691Gpi1ZoOa1zggmwKBgQDj+3NScZ7A5bh25XtxJuSFDgbJsA56opLY
3RMI104c7vIj2IQKyOKQCr8Ez8Orrn/XJ+8m8t6IKg9g6G9zjIZ2KryYFhELRla6
T63C9ewzZcSkAYZ7dExQPIawzCvjyE8qY8khlJnR9o5PknW3+e7so42yJIKP6BWY
8q+YUmfnWwKBgCRusMSY98Zo18g0jZsZd/wQ2TqVuWTxDAytPJ+sfKK3HLxmwf1U
zhjwKAYqOEBuiDMJ/jVVdB98RqhR2+AV0xcVNMLJ48uduAiFNEZPQBgtiUMkyvSr
Pf42olzUNe3iOOqpBgYglMuufVDylpsrRjTx0IeDNZqaftgkuHmTAH5lAoGBAM/Q
+oKAh9IWlVvsO+YdKdoPuyhGkCxB3dJJU3yPpujA94CtcU/TZpMe+JkOOrNY0bfy
8xFx+l/s1y/jMRUHV9qHgnqwQsEgURZsY1yAh9siPWmy6j/G93l8ctregnOUuHVP
mJw/tSertHXcb+pQrfaP8C4fEdTUHjvZnS8gjw5ZAoGBAK2zkSk40SawCkYep/oy
Z3X6pg60JV8Sa/vyXifzzY4uBi5ByaTc9OTcxQcfxRzz8rCoy7nF101Pipotn37F
wK1X7yzmEwEi2GctHWyyPKFTpFpmjTH4gG7uTfF3cHztqufg6rRPWGh6qRMMRFm6
6dQUlev76ajL1zziuySGpdmm
-----END PRIVATE KEY-----"#;
    const RSA_PUBLIC_KEY: &str = r#"-----BEGIN PUBLIC KEY-----
MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEA4KFnLKcqJ+4Iu//L/I0E
jhbH+A7gP355fm1UnyyPZhjZTaq9GGYOLoI6vzTEtOrRwF2k+1JQRgojqgw6Pj5l
gQUn962QdA2TR/rBet0ibAdje2FwRuMYNk1hvQkUKJmlZabSKqGkQE/zrbjydhj8
bXR10Xxwap5APNZscU8aFd1FLSSHqOVmrQAN6/rh3DzIwmbiEA213PuUoP/5ejkI
ryiOEILjr+AVYGwbyEAPVlRFXDvDWu0FafHwHHE33wd9sclR9tVkrGXvD6Lq3XoJ
7TLCYqOUir9Ze8lcSaDg/itqMHT4+m0sXthGsX67pZoqyBrNpTnl+o+KkWNM6rx0
GQIDAQAB
-----END PUBLIC KEY-----"#;

    #[test]
    fn rs256_tokens_round_trip_with_standard_claims() {
        let config = JwtConfig::new("unused")
            .with_rsa_keys(RSA_PRIVATE_KEY, RSA_PUBLIC_KEY)
            .with_issuer("https://auth.openfoundry.test")
            .with_audience("openfoundry")
            .with_key_id("test-key");

        let claims = build_access_claims(
            &config,
            Uuid::now_v7(),
            "demo@example.com",
            "Demo User",
            vec!["member".to_string()],
            vec!["datasets:read".to_string()],
            Some(Uuid::now_v7()),
            json!({ "region": "eu" }),
            vec!["password".to_string()],
        );

        let token = encode_token(&config, &claims).expect("token should encode");
        let header = decode_header(&token).expect("header should decode");
        let decoded = decode_token(&config, &token).expect("token should decode");

        assert_eq!(header.alg, Algorithm::RS256);
        assert_eq!(header.kid.as_deref(), Some("test-key"));
        assert_eq!(
            decoded.iss.as_deref(),
            Some("https://auth.openfoundry.test")
        );
        assert_eq!(decoded.aud.as_deref(), Some("openfoundry"));
        assert_eq!(decoded.email, "demo@example.com");
    }

    #[test]
    fn rejects_tokens_with_wrong_audience() {
        let issuer_config = JwtConfig::new("unused")
            .with_rsa_keys(RSA_PRIVATE_KEY, RSA_PUBLIC_KEY)
            .with_issuer("https://auth.openfoundry.test")
            .with_audience("openfoundry");
        let verifier_config = issuer_config.clone().with_audience("someone-else");

        let claims = build_refresh_claims(&issuer_config, Uuid::now_v7());
        let token = encode_token(&issuer_config, &claims).expect("token should encode");
        let error = decode_token(&verifier_config, &token).expect_err("token should be rejected");

        assert!(matches!(error, JwtError::Invalid(_)));
    }
}
