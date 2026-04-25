use auth_middleware::jwt::JwtConfig;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use chrono::Utc;
use regex::Regex;
use serde_json::{Map, Value, json};
use url::Url;
use uuid::Uuid;

use crate::{
    domain::oauth,
    models::sso::SsoProvider,
};

#[derive(Debug, Clone)]
pub struct SamlMetadataDefaults {
    pub entity_id: Option<String>,
    pub sso_url: Option<String>,
    pub certificate: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SamlIdentity {
    pub subject: String,
    pub email: String,
    pub name: String,
    pub raw_claims: Value,
}

pub async fn resolve_metadata_defaults(metadata_url: &str) -> Result<SamlMetadataDefaults, String> {
    let response = reqwest::get(metadata_url)
        .await
        .map_err(|error| error.to_string())?;
    if !response.status().is_success() {
        return Err(format!(
            "metadata fetch failed with status {}",
            response.status()
        ));
    }
    let xml = response.text().await.map_err(|error| error.to_string())?;
    Ok(parse_metadata_defaults(&xml))
}

pub fn parse_metadata_defaults(xml: &str) -> SamlMetadataDefaults {
    SamlMetadataDefaults {
        entity_id: capture_first(
            xml,
            r#"entityID\s*=\s*"([^"]+)""#,
        ),
        sso_url: capture_first(
            xml,
            r#"<[^>]*SingleSignOnService[^>]*Location\s*=\s*"([^"]+)""#,
        ),
        certificate: capture_first(xml, r#"<[^>]*X509Certificate[^>]*>([^<]+)</"#)
            .map(|value| value.replace(char::is_whitespace, "")),
    }
}

pub fn build_authorization_url(
    config: &JwtConfig,
    provider: &SsoProvider,
    assertion_consumer_service_url: &str,
    redirect_to: Option<&str>,
) -> Result<String, String> {
    let destination = provider
        .saml_sso_url
        .as_deref()
        .ok_or_else(|| "provider is missing saml_sso_url".to_string())?;
    let issuer = provider
        .saml_entity_id
        .as_deref()
        .unwrap_or("openfoundry-sp");
    let relay_state = oauth::issue_state(config, provider.id, redirect_to)?;
    let request_id = format!("_{}", Uuid::now_v7().simple());
    let issue_instant = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let xml = format!(
        r#"<samlp:AuthnRequest xmlns:samlp="urn:oasis:names:tc:SAML:2.0:protocol" xmlns:saml="urn:oasis:names:tc:SAML:2.0:assertion" ID="{request_id}" Version="2.0" IssueInstant="{issue_instant}" ProtocolBinding="urn:oasis:names:tc:SAML:2.0:bindings:HTTP-Redirect" AssertionConsumerServiceURL="{assertion_consumer_service_url}" Destination="{destination}"><saml:Issuer>{issuer}</saml:Issuer></samlp:AuthnRequest>"#
    );

    let mut url = Url::parse(destination).map_err(|error| error.to_string())?;
    url.query_pairs_mut()
        .append_pair("SAMLRequest", &STANDARD.encode(xml.as_bytes()))
        .append_pair("RelayState", &relay_state);
    Ok(url.to_string())
}

pub fn parse_saml_response(
    provider: &SsoProvider,
    saml_response: &str,
) -> Result<SamlIdentity, String> {
    let decoded = STANDARD
        .decode(saml_response)
        .map_err(|error| error.to_string())?;
    let xml = String::from_utf8(decoded).map_err(|error| error.to_string())?;

    if xml.contains("StatusCode")
        && xml.contains("Responder")
        || xml.contains("Requester")
    {
        return Err("saml response returned a non-success status".to_string());
    }

    let subject = capture_first(
        &xml,
        r#"<[^>]*NameID[^>]*>([^<]+)</[^>]*NameID>"#,
    )
    .ok_or_else(|| "saml response is missing NameID".to_string())?;

    let mut attributes = extract_attributes(&xml);
    attributes.insert("NameID".to_string(), Value::String(subject.clone()));

    let subject_key = provider
        .attribute_mapping
        .get("subject")
        .and_then(Value::as_str)
        .unwrap_or("NameID");
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

    let email = attributes
        .get(email_key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "saml response is missing email attribute".to_string())?
        .to_string();
    let mapped_subject = attributes
        .get(subject_key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .unwrap_or(subject.as_str())
        .to_string();
    let name = attributes
        .get(name_key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .unwrap_or(email.as_str())
        .to_string();

    Ok(SamlIdentity {
        subject: mapped_subject,
        email,
        name,
        raw_claims: Value::Object(attributes),
    })
}

fn extract_attributes(xml: &str) -> Map<String, Value> {
    let mut attributes = Map::new();
    let attribute_regex = Regex::new(
        r#"<[^>]*Attribute[^>]*Name\s*=\s*"([^"]+)"[^>]*>.*?<[^>]*AttributeValue[^>]*>(.*?)</[^>]*AttributeValue>.*?</[^>]*Attribute>"#,
    )
    .expect("saml attribute regex should compile");

    for capture in attribute_regex.captures_iter(xml) {
        let Some(name) = capture.get(1).map(|value| value.as_str()) else {
            continue;
        };
        let Some(raw_value) = capture.get(2).map(|value| html_unescape(value.as_str())) else {
            continue;
        };
        attributes.insert(name.to_string(), json!(raw_value));
    }

    attributes
}

fn capture_first(input: &str, pattern: &str) -> Option<String> {
    Regex::new(pattern)
        .ok()
        .and_then(|regex| regex.captures(input))
        .and_then(|captures| captures.get(1).map(|value| value.as_str().trim().to_string()))
        .filter(|value| !value.is_empty())
}

fn html_unescape(value: &str) -> String {
    value
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn metadata_parser_extracts_defaults() {
        let metadata = parse_metadata_defaults(
            r#"<EntityDescriptor entityID="https://idp.example.com/metadata"><IDPSSODescriptor><SingleSignOnService Location="https://idp.example.com/sso"/><KeyDescriptor><KeyInfo><X509Data><X509Certificate>ABC123</X509Certificate></X509Data></KeyInfo></KeyDescriptor></IDPSSODescriptor></EntityDescriptor>"#,
        );

        assert_eq!(
            metadata.entity_id.as_deref(),
            Some("https://idp.example.com/metadata")
        );
        assert_eq!(metadata.sso_url.as_deref(), Some("https://idp.example.com/sso"));
        assert_eq!(metadata.certificate.as_deref(), Some("ABC123"));
    }

    #[test]
    fn saml_response_parser_extracts_identity() {
        let provider = SsoProvider {
            id: Uuid::now_v7(),
            slug: "saml".to_string(),
            name: "SAML".to_string(),
            provider_type: "saml".to_string(),
            enabled: true,
            client_id: None,
            client_secret: None,
            issuer_url: None,
            authorization_url: None,
            token_url: None,
            userinfo_url: None,
            scopes: vec![],
            saml_metadata_url: None,
            saml_entity_id: Some("https://sp.example.com".to_string()),
            saml_sso_url: Some("https://idp.example.com/sso".to_string()),
            saml_certificate: None,
            attribute_mapping: json!({
                "subject": "uid",
                "email": "mail",
                "name": "displayName"
            }),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let xml = r#"<Response><Assertion><Subject><NameID>user-123</NameID></Subject><AttributeStatement><Attribute Name="uid"><AttributeValue>user-123</AttributeValue></Attribute><Attribute Name="mail"><AttributeValue>user@example.com</AttributeValue></Attribute><Attribute Name="displayName"><AttributeValue>Example User</AttributeValue></Attribute></AttributeStatement></Assertion></Response>"#;
        let identity = parse_saml_response(&provider, &STANDARD.encode(xml.as_bytes()))
            .expect("saml response should parse");
        assert_eq!(identity.subject, "user-123");
        assert_eq!(identity.email, "user@example.com");
        assert_eq!(identity.name, "Example User");
    }
}
