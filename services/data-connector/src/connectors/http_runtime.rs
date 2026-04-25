use std::collections::BTreeMap;

use base64::{Engine as _, engine::general_purpose::STANDARD};
use reqwest::{
    Method, Url,
    header::{HeaderMap, HeaderName, HeaderValue},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    AppState,
    domain::egress::{EgressPolicy, validate_url},
};

#[derive(Debug, Clone)]
pub struct HttpResponseEnvelope {
    pub status: u16,
    pub headers: BTreeMap<String, String>,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Serialize)]
struct AgentProxyRequest {
    method: String,
    url: String,
    headers: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    bearer_token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AgentProxyResponse {
    status: u16,
    #[serde(default)]
    headers: BTreeMap<String, String>,
    #[serde(default)]
    body_base64: Option<String>,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    json: Option<Value>,
}

pub async fn get(
    state: &AppState,
    config: &Value,
    url: Url,
    headers: HeaderMap,
    bearer_token: Option<String>,
    agent_url: Option<&str>,
) -> Result<HttpResponseEnvelope, String> {
    let policy = EgressPolicy::from_state_and_config(state, config);
    validate_url(&url, &policy)?;
    if let Some(agent_url) = agent_url {
        return proxy_via_agent(state, agent_url, Method::GET, url, headers, bearer_token).await;
    }

    let mut request = state.http_client.get(url.clone());
    if let Some(token) = bearer_token {
        request = request.bearer_auth(token);
    }
    request = request.headers(headers);
    let response = request.send().await.map_err(|error| error.to_string())?;
    let status = response.status().as_u16();
    let headers = header_map_to_strings(response.headers());
    let bytes = response
        .bytes()
        .await
        .map_err(|error| error.to_string())?
        .to_vec();

    Ok(HttpResponseEnvelope {
        status,
        headers,
        bytes,
    })
}

pub fn json_body(response: &HttpResponseEnvelope) -> Result<Value, String> {
    serde_json::from_slice(&response.bytes).map_err(|error| error.to_string())
}

pub fn header_map(config: &Value) -> Result<HeaderMap, String> {
    let mut headers = HeaderMap::new();
    if let Some(header_map) = config.get("headers").and_then(Value::as_object) {
        for (name, value) in header_map {
            let header_name =
                HeaderName::from_bytes(name.as_bytes()).map_err(|error| error.to_string())?;
            let header_value = HeaderValue::from_str(
                value
                    .as_str()
                    .ok_or_else(|| format!("header '{name}' must be a string"))?,
            )
            .map_err(|error| error.to_string())?;
            headers.insert(header_name, header_value);
        }
    }
    Ok(headers)
}

async fn proxy_via_agent(
    state: &AppState,
    agent_url: &str,
    method: Method,
    url: Url,
    headers: HeaderMap,
    bearer_token: Option<String>,
) -> Result<HttpResponseEnvelope, String> {
    let proxy_url = Url::parse(agent_url)
        .map_err(|error| error.to_string())?
        .join("/api/v1/connector-agent/http")
        .map_err(|error| error.to_string())?;
    let request = AgentProxyRequest {
        method: method.as_str().to_string(),
        url: url.to_string(),
        headers: header_map_to_strings(&headers),
        bearer_token,
    };

    let response = state
        .http_client
        .post(proxy_url)
        .json(&request)
        .send()
        .await
        .map_err(|error| error.to_string())?;
    let status = response.status().as_u16();
    if !response.status().is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(format!("connector agent returned HTTP {status}: {body}"));
    }

    let body = response
        .json::<AgentProxyResponse>()
        .await
        .map_err(|error| error.to_string())?;
    let bytes = if let Some(body_base64) = body.body_base64 {
        STANDARD
            .decode(body_base64)
            .map_err(|error| error.to_string())?
    } else if let Some(text) = body.text {
        text.into_bytes()
    } else if let Some(json) = body.json {
        serde_json::to_vec(&json).map_err(|error| error.to_string())?
    } else {
        Vec::new()
    };

    Ok(HttpResponseEnvelope {
        status: body.status,
        headers: body.headers,
        bytes,
    })
}

fn header_map_to_strings(headers: &HeaderMap) -> BTreeMap<String, String> {
    headers
        .iter()
        .filter_map(|(key, value)| {
            value
                .to_str()
                .ok()
                .map(|value| (key.to_string(), value.to_string()))
        })
        .collect()
}
