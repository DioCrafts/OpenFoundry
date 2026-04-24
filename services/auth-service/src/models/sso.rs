use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct SsoProvider {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub provider_type: String,
    pub enabled: bool,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub issuer_url: Option<String>,
    pub authorization_url: Option<String>,
    pub token_url: Option<String>,
    pub userinfo_url: Option<String>,
    pub scopes: Vec<String>,
    pub saml_metadata_url: Option<String>,
    pub saml_entity_id: Option<String>,
    pub saml_sso_url: Option<String>,
    pub saml_certificate: Option<String>,
    pub attribute_mapping: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SsoProviderResponse {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub provider_type: String,
    pub enabled: bool,
    pub client_id: Option<String>,
    pub client_secret_configured: bool,
    pub issuer_url: Option<String>,
    pub authorization_url: Option<String>,
    pub token_url: Option<String>,
    pub userinfo_url: Option<String>,
    pub scopes: Vec<String>,
    pub saml_metadata_url: Option<String>,
    pub saml_entity_id: Option<String>,
    pub saml_sso_url: Option<String>,
    pub attribute_mapping: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl SsoProvider {
    pub fn into_response(self) -> SsoProviderResponse {
        SsoProviderResponse {
            id: self.id,
            slug: self.slug,
            name: self.name,
            provider_type: self.provider_type,
            enabled: self.enabled,
            client_id: self.client_id,
            client_secret_configured: self.client_secret.is_some(),
            issuer_url: self.issuer_url,
            authorization_url: self.authorization_url,
            token_url: self.token_url,
            userinfo_url: self.userinfo_url,
            scopes: self.scopes,
            saml_metadata_url: self.saml_metadata_url,
            saml_entity_id: self.saml_entity_id,
            saml_sso_url: self.saml_sso_url,
            attribute_mapping: self.attribute_mapping,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}
