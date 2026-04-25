use auth_middleware::Claims;
use auth_middleware::jwt::{self, JwtConfig};
use reqwest::Client;
use serde_json::Value;
use url::Url;

use crate::models::sso::SsoProvider;

pub fn build_authorization_url(
    config: &JwtConfig,
    provider: &SsoProvider,
    redirect_uri: &str,
    redirect_to: Option<&str>,
) -> Result<String, String> {
    let Some(base_authorize_url) = provider.authorization_url.as_deref() else {
        return Err("provider is missing authorization_url".to_string());
    };

    let mut authorize_url = Url::parse(base_authorize_url).map_err(|error| error.to_string())?;
    let state = issue_state(config, provider.id, redirect_to)?;
    let scope = if provider.scopes.is_empty() {
        "openid profile email".to_string()
    } else {
        provider.scopes.join(" ")
    };

    authorize_url
        .query_pairs_mut()
        .append_pair("response_type", "code")
        .append_pair(
            "client_id",
            provider.client_id.as_deref().unwrap_or_default(),
        )
        .append_pair("redirect_uri", redirect_uri)
        .append_pair("scope", &scope)
        .append_pair("state", &state);

    Ok(authorize_url.to_string())
}

pub async fn exchange_code(
    provider: &SsoProvider,
    code: &str,
    redirect_uri: &str,
) -> Result<Value, String> {
    let Some(token_url) = provider.token_url.as_deref() else {
        return Err("provider is missing token_url".to_string());
    };

    let response = Client::new()
        .post(token_url)
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", redirect_uri),
            (
                "client_id",
                provider.client_id.as_deref().unwrap_or_default(),
            ),
            (
                "client_secret",
                provider.client_secret.as_deref().unwrap_or_default(),
            ),
        ])
        .send()
        .await
        .map_err(|error| error.to_string())?;

    if !response.status().is_success() {
        return Err(format!(
            "token exchange failed with status {}",
            response.status()
        ));
    }

    response
        .json::<Value>()
        .await
        .map_err(|error| error.to_string())
}

pub async fn fetch_userinfo(provider: &SsoProvider, access_token: &str) -> Result<Value, String> {
    let Some(userinfo_url) = provider.userinfo_url.as_deref() else {
        return Err("provider is missing userinfo_url".to_string());
    };

    let response = Client::new()
        .get(userinfo_url)
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|error| error.to_string())?;

    if !response.status().is_success() {
        return Err(format!(
            "userinfo request failed with status {}",
            response.status()
        ));
    }

    response
        .json::<Value>()
        .await
        .map_err(|error| error.to_string())
}

pub fn issue_state(
    config: &JwtConfig,
    provider_id: uuid::Uuid,
    redirect_to: Option<&str>,
) -> Result<String, String> {
    let claims = Claims {
        sub: provider_id,
        iat: chrono::Utc::now().timestamp(),
        exp: chrono::Utc::now().timestamp() + 600,
        jti: uuid::Uuid::now_v7(),
        email: String::new(),
        name: String::new(),
        roles: vec![],
        permissions: vec![],
        org_id: None,
        attributes: serde_json::json!({
            "redirect_to": redirect_to.unwrap_or("/"),
        }),
        auth_methods: vec!["sso_state".to_string()],
        token_use: Some("sso_state".to_string()),
        api_key_id: None,
        session_kind: None,
        session_scope: None,
    };

    jwt::encode_token(config, &claims).map_err(|error| error.to_string())
}

pub fn validate_state(config: &JwtConfig, state: &str) -> Result<Claims, String> {
    let claims = jwt::decode_token(config, state).map_err(|error| error.to_string())?;
    if claims.token_use.as_deref() == Some("sso_state") {
        Ok(claims)
    } else {
        Err("invalid SSO state".to_string())
    }
}

pub fn map_identity(
    provider: &SsoProvider,
    payload: &Value,
) -> Result<(String, String, String), String> {
    let subject_key = provider
        .attribute_mapping
        .get("subject")
        .and_then(Value::as_str)
        .unwrap_or("sub");
    let email_key = provider
        .attribute_mapping
        .get("email")
        .and_then(Value::as_str)
        .unwrap_or("email");
    let name_key = provider
        .attribute_mapping
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("name");

    let subject = payload
        .get(subject_key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "provider payload is missing subject".to_string())?;
    let email = payload
        .get(email_key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "provider payload is missing email".to_string())?;
    let name = payload
        .get(name_key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .unwrap_or(email);

    Ok((subject.to_string(), email.to_string(), name.to_string()))
}
