use auth_middleware::layer::AuthUser;
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::AppState;
use crate::domain::{idp_mapping, jwt, mfa, oauth, rbac, saml};
use crate::models::control_panel::{IdentityProviderMapping, ResourceManagementPolicy};
use crate::models::mfa::TotpConfiguration;
use crate::models::sso::{SsoProvider, SsoProviderResponse};
use crate::models::user::User;

use super::common::{json_error, require_permission};
use super::login::LoginResponse;

#[derive(Debug, Deserialize)]
pub struct UpsertProviderRequest {
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
    #[serde(default)]
    pub scopes: Vec<String>,
    pub saml_metadata_url: Option<String>,
    pub saml_entity_id: Option<String>,
    pub saml_sso_url: Option<String>,
    pub saml_certificate: Option<String>,
    #[serde(default)]
    pub attribute_mapping: Value,
}

#[derive(Debug, Deserialize)]
pub struct CompleteSsoLoginRequest {
    pub code: Option<String>,
    pub state: Option<String>,
    #[serde(alias = "SAMLResponse")]
    pub saml_response: Option<String>,
    #[serde(alias = "RelayState")]
    pub relay_state: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PublicProviderResponse {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub provider_type: String,
}

#[derive(Debug, Clone)]
struct SsoProvisioningProfile {
    organization_id: Option<Uuid>,
    attributes: Value,
    role_names: Vec<String>,
}

pub async fn list_public_providers(State(state): State<AppState>) -> impl IntoResponse {
    match list_enabled_public_providers(&state.db).await {
        Ok(providers) => Json(
            providers
                .into_iter()
                .map(|provider| PublicProviderResponse {
                    id: provider.id,
                    slug: provider.slug,
                    name: provider.name,
                    provider_type: provider.provider_type,
                })
                .collect::<Vec<_>>(),
        )
        .into_response(),
        Err(e) => {
            tracing::error!("failed to list public providers: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn start_login(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> impl IntoResponse {
    let provider = match load_provider_by_slug(&state.db, &slug).await {
        Ok(Some(provider)) => provider,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("failed to load SSO provider: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let redirect_uri = format!(
        "{}/auth/callback",
        state.public_web_origin.trim_end_matches('/')
    );
    let authorization_url = match provider.provider_type.as_str() {
        "oidc" => {
            oauth::build_authorization_url(&state.jwt_config, &provider, &redirect_uri, Some("/"))
        }
        "saml" => {
            saml::build_authorization_url(&state.jwt_config, &provider, &redirect_uri, Some("/"))
        }
        _ => Err("unsupported provider type".to_string()),
    };

    match authorization_url {
        Ok(authorization_url) => {
            Json(json!({ "authorization_url": authorization_url })).into_response()
        }
        Err(error) => json_error(StatusCode::BAD_REQUEST, error),
    }
}

pub async fn complete_login(
    State(state): State<AppState>,
    Json(body): Json<CompleteSsoLoginRequest>,
) -> impl IntoResponse {
    let state_token = body
        .state
        .as_deref()
        .or(body.relay_state.as_deref())
        .unwrap_or_default();
    let state_claims = match oauth::validate_state(&state.jwt_config, state_token) {
        Ok(claims) => claims,
        Err(error) => return json_error(StatusCode::UNAUTHORIZED, error),
    };

    let provider = match load_provider_by_id(&state.db, state_claims.sub).await {
        Ok(Some(provider)) => provider,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("failed to load SSO provider by state: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let redirect_uri = format!(
        "{}/auth/callback",
        state.public_web_origin.trim_end_matches('/')
    );
    let (subject, email, name, raw_claims) = if provider.provider_type == "saml" {
        let Some(saml_response) = body.saml_response.as_deref() else {
            return json_error(StatusCode::BAD_REQUEST, "missing saml_response");
        };
        match saml::parse_saml_response(&provider, saml_response) {
            Ok(identity) => (
                identity.subject,
                identity.email,
                identity.name,
                identity.raw_claims,
            ),
            Err(error) => return json_error(StatusCode::BAD_GATEWAY, error),
        }
    } else {
        let Some(code) = body.code.as_deref() else {
            return json_error(StatusCode::BAD_REQUEST, "missing authorization code");
        };
        let token_payload = match oauth::exchange_code(&provider, code, &redirect_uri).await {
            Ok(payload) => payload,
            Err(error) => return json_error(StatusCode::BAD_GATEWAY, error),
        };

        let access_token = token_payload
            .get("access_token")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string);
        let Some(access_token) = access_token else {
            return json_error(
                StatusCode::BAD_GATEWAY,
                "provider token response is missing access_token",
            );
        };

        let userinfo = match oauth::fetch_userinfo(&provider, &access_token).await {
            Ok(payload) => payload,
            Err(error) => return json_error(StatusCode::BAD_GATEWAY, error),
        };

        let (subject, email, name) = match oauth::map_identity(&provider, &userinfo) {
            Ok(identity) => identity,
            Err(error) => return json_error(StatusCode::BAD_GATEWAY, error),
        };
        (subject, email, name, userinfo)
    };

    let provisioning =
        match build_sso_provisioning_profile(&state.db, &provider, &email, &raw_claims).await {
            Ok(profile) => profile,
            Err(error) => return json_error(StatusCode::FORBIDDEN, error),
        };
    let user = match find_or_create_sso_user(
        &state.db,
        &provider,
        &subject,
        &email,
        &name,
        &raw_claims,
        &provisioning,
    )
    .await
    {
        Ok(user) => user,
        Err(e) => {
            tracing::error!("failed to materialize SSO user: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let mfa_configuration = match load_mfa_configuration(&state.db, user.id).await {
        Ok(configuration) => configuration,
        Err(e) => {
            tracing::error!("failed to load MFA after SSO: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    if let Some(configuration) = mfa_configuration {
        if configuration.enabled {
            return match mfa::issue_challenge(&state.jwt_config, &user, "sso") {
                Ok(challenge_token) => Json(LoginResponse::MfaRequired {
                    challenge_token,
                    methods: vec!["totp".to_string()],
                    expires_in: 300,
                })
                .into_response(),
                Err(e) => {
                    tracing::error!("failed to issue MFA challenge after SSO: {e}");
                    StatusCode::INTERNAL_SERVER_ERROR.into_response()
                }
            };
        }
    } else if user.mfa_enforced {
        return json_error(StatusCode::FORBIDDEN, "mfa setup required by administrator");
    }

    let provider_auth_method = if provider.provider_type == "saml" {
        "saml".to_string()
    } else {
        "sso".to_string()
    };
    match jwt::issue_tokens(
        &state.db,
        &state.jwt_config,
        &user,
        vec![provider_auth_method],
    )
    .await
    {
        Ok((platform_access_token, refresh_token)) => Json(LoginResponse::Authenticated {
            access_token: platform_access_token,
            refresh_token,
            token_type: "Bearer".to_string(),
            expires_in: state.jwt_config.access_ttl_secs,
        })
        .into_response(),
        Err(e) => {
            tracing::error!("failed to issue SSO tokens: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn list_providers(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> impl IntoResponse {
    if let Err(response) = require_permission(&claims, "sso", "read") {
        return response;
    }

    match sqlx::query_as::<_, SsoProvider>(
        "SELECT id, slug, name, provider_type, enabled, client_id, client_secret, issuer_url, authorization_url, token_url, userinfo_url, scopes, saml_metadata_url, saml_entity_id, saml_sso_url, saml_certificate, attribute_mapping, created_at, updated_at FROM sso_providers ORDER BY name",
    )
    .fetch_all(&state.db)
    .await
    {
        Ok(providers) => Json(providers.into_iter().map(SsoProvider::into_response).collect::<Vec<SsoProviderResponse>>()).into_response(),
        Err(e) => {
            tracing::error!("failed to list SSO providers: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn create_provider(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(body): Json<UpsertProviderRequest>,
) -> impl IntoResponse {
    if let Err(response) = require_permission(&claims, "sso", "write") {
        return response;
    }

    let metadata_defaults = if body.provider_type == "saml" {
        match body.saml_metadata_url.as_deref() {
            Some(metadata_url) => match saml::resolve_metadata_defaults(metadata_url).await {
                Ok(defaults) => Some(defaults),
                Err(error) => return json_error(StatusCode::BAD_REQUEST, error),
            },
            None => None,
        }
    } else {
        None
    };

    match sqlx::query_as::<_, SsoProvider>(
        r#"INSERT INTO sso_providers (id, slug, name, provider_type, enabled, client_id, client_secret, issuer_url, authorization_url, token_url, userinfo_url, scopes, saml_metadata_url, saml_entity_id, saml_sso_url, saml_certificate, attribute_mapping)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)
           RETURNING id, slug, name, provider_type, enabled, client_id, client_secret, issuer_url, authorization_url, token_url, userinfo_url, scopes, saml_metadata_url, saml_entity_id, saml_sso_url, saml_certificate, attribute_mapping, created_at, updated_at"#,
    )
    .bind(Uuid::now_v7())
    .bind(body.slug)
    .bind(body.name)
    .bind(body.provider_type)
    .bind(body.enabled)
    .bind(body.client_id)
    .bind(body.client_secret)
    .bind(body.issuer_url)
    .bind(body.authorization_url)
    .bind(body.token_url)
    .bind(body.userinfo_url)
    .bind(body.scopes)
    .bind(body.saml_metadata_url)
    .bind(body.saml_entity_id.or_else(|| metadata_defaults.as_ref().and_then(|defaults| defaults.entity_id.clone())))
    .bind(body.saml_sso_url.or_else(|| metadata_defaults.as_ref().and_then(|defaults| defaults.sso_url.clone())))
    .bind(body.saml_certificate.or_else(|| metadata_defaults.as_ref().and_then(|defaults| defaults.certificate.clone())))
    .bind(body.attribute_mapping)
    .fetch_one(&state.db)
    .await
    {
        Ok(provider) => (StatusCode::CREATED, Json(provider.into_response())).into_response(),
        Err(e) => {
            tracing::error!("failed to create SSO provider: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn update_provider(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(provider_id): Path<Uuid>,
    Json(body): Json<UpsertProviderRequest>,
) -> impl IntoResponse {
    if let Err(response) = require_permission(&claims, "sso", "write") {
        return response;
    }

    let existing = match load_provider_by_id(&state.db, provider_id).await {
        Ok(Some(provider)) => provider,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("failed to load existing provider: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let metadata_defaults = if body.provider_type == "saml" {
        match body
            .saml_metadata_url
            .as_deref()
            .or(existing.saml_metadata_url.as_deref())
        {
            Some(metadata_url) => match saml::resolve_metadata_defaults(metadata_url).await {
                Ok(defaults) => Some(defaults),
                Err(error) => return json_error(StatusCode::BAD_REQUEST, error),
            },
            None => None,
        }
    } else {
        None
    };

    match sqlx::query_as::<_, SsoProvider>(
        r#"UPDATE sso_providers
           SET slug = $2,
               name = $3,
               provider_type = $4,
               enabled = $5,
               client_id = $6,
               client_secret = $7,
               issuer_url = $8,
               authorization_url = $9,
               token_url = $10,
               userinfo_url = $11,
               scopes = $12,
               saml_metadata_url = $13,
               saml_entity_id = $14,
               saml_sso_url = $15,
               saml_certificate = $16,
               attribute_mapping = $17,
               updated_at = NOW()
           WHERE id = $1
           RETURNING id, slug, name, provider_type, enabled, client_id, client_secret, issuer_url, authorization_url, token_url, userinfo_url, scopes, saml_metadata_url, saml_entity_id, saml_sso_url, saml_certificate, attribute_mapping, created_at, updated_at"#,
    )
    .bind(provider_id)
    .bind(body.slug)
    .bind(body.name)
    .bind(body.provider_type)
    .bind(body.enabled)
    .bind(body.client_id)
    .bind(body.client_secret.or(existing.client_secret))
    .bind(body.issuer_url)
    .bind(body.authorization_url)
    .bind(body.token_url)
    .bind(body.userinfo_url)
    .bind(body.scopes)
    .bind(body.saml_metadata_url)
    .bind(body.saml_entity_id.or(existing.saml_entity_id).or_else(|| metadata_defaults.as_ref().and_then(|defaults| defaults.entity_id.clone())))
    .bind(body.saml_sso_url.or(existing.saml_sso_url).or_else(|| metadata_defaults.as_ref().and_then(|defaults| defaults.sso_url.clone())))
    .bind(body.saml_certificate.or(existing.saml_certificate).or_else(|| metadata_defaults.as_ref().and_then(|defaults| defaults.certificate.clone())))
    .bind(body.attribute_mapping)
    .fetch_optional(&state.db)
    .await
    {
        Ok(Some(provider)) => Json(provider.into_response()).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("failed to update SSO provider: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn delete_provider(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(provider_id): Path<Uuid>,
) -> impl IntoResponse {
    if let Err(response) = require_permission(&claims, "sso", "write") {
        return response;
    }

    match sqlx::query("DELETE FROM sso_providers WHERE id = $1")
        .bind(provider_id)
        .execute(&state.db)
        .await
    {
        Ok(record) if record.rows_affected() > 0 => StatusCode::NO_CONTENT.into_response(),
        Ok(_) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("failed to delete SSO provider: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

async fn list_enabled_public_providers(
    pool: &sqlx::PgPool,
) -> Result<Vec<SsoProvider>, sqlx::Error> {
    sqlx::query_as::<_, SsoProvider>(
        "SELECT id, slug, name, provider_type, enabled, client_id, client_secret, issuer_url, authorization_url, token_url, userinfo_url, scopes, saml_metadata_url, saml_entity_id, saml_sso_url, saml_certificate, attribute_mapping, created_at, updated_at FROM sso_providers WHERE enabled = true ORDER BY name",
    )
    .fetch_all(pool)
    .await
}

async fn load_provider_by_slug(
    pool: &sqlx::PgPool,
    slug: &str,
) -> Result<Option<SsoProvider>, sqlx::Error> {
    sqlx::query_as::<_, SsoProvider>(
        "SELECT id, slug, name, provider_type, enabled, client_id, client_secret, issuer_url, authorization_url, token_url, userinfo_url, scopes, saml_metadata_url, saml_entity_id, saml_sso_url, saml_certificate, attribute_mapping, created_at, updated_at FROM sso_providers WHERE slug = $1 AND enabled = true",
    )
    .bind(slug)
    .fetch_optional(pool)
    .await
}

async fn load_provider_by_id(
    pool: &sqlx::PgPool,
    provider_id: Uuid,
) -> Result<Option<SsoProvider>, sqlx::Error> {
    sqlx::query_as::<_, SsoProvider>(
        "SELECT id, slug, name, provider_type, enabled, client_id, client_secret, issuer_url, authorization_url, token_url, userinfo_url, scopes, saml_metadata_url, saml_entity_id, saml_sso_url, saml_certificate, attribute_mapping, created_at, updated_at FROM sso_providers WHERE id = $1",
    )
    .bind(provider_id)
    .fetch_optional(pool)
    .await
}

async fn load_mfa_configuration(
    pool: &sqlx::PgPool,
    user_id: Uuid,
) -> Result<Option<TotpConfiguration>, sqlx::Error> {
    sqlx::query_as::<_, TotpConfiguration>(
        "SELECT user_id, secret, recovery_code_hashes, enabled, verified_at, created_at, updated_at FROM user_mfa_totp WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
}

async fn build_sso_provisioning_profile(
    pool: &sqlx::PgPool,
    provider: &SsoProvider,
    email: &str,
    raw_claims: &Value,
) -> Result<SsoProvisioningProfile, String> {
    let mappings = load_identity_provider_mappings(pool).await?;
    let policies = load_resource_management_policies(pool).await?;
    let mapping = mappings
        .iter()
        .find(|entry| entry.provider_slug == provider.slug);
    let assignment = idp_mapping::resolve_identity_provider_assignment(
        provider, mapping, email, raw_claims, &policies,
    )?;
    let attributes = assignment.to_attributes(raw_claims)?;

    Ok(SsoProvisioningProfile {
        organization_id: assignment.organization_id,
        attributes,
        role_names: assignment.role_names,
    })
}

async fn find_or_create_sso_user(
    pool: &sqlx::PgPool,
    provider: &SsoProvider,
    subject: &str,
    email: &str,
    name: &str,
    raw_claims: &Value,
    profile: &SsoProvisioningProfile,
) -> Result<User, sqlx::Error> {
    if let Some(user) = sqlx::query_as::<_, User>(
        r#"SELECT u.id, u.email, u.name, u.password_hash, u.is_active, u.organization_id, u.attributes, u.mfa_enforced, u.auth_source, u.created_at, u.updated_at
           FROM users u
           INNER JOIN external_identities ei ON ei.user_id = u.id
           WHERE ei.provider_id = $1 AND ei.subject = $2"#,
    )
    .bind(provider.id)
    .bind(subject)
    .fetch_optional(pool)
    .await?
    {
        if profile.organization_id.is_some() || profile.attributes != user.attributes {
            let next_org = profile.organization_id.or(user.organization_id);
            let _ = sqlx::query(
                "UPDATE users SET organization_id = $2, attributes = $3, updated_at = NOW() WHERE id = $1",
            )
            .bind(user.id)
            .bind(next_org)
            .bind(profile.attributes.clone())
            .execute(pool)
            .await;
        }
        assign_roles_by_name(pool, user.id, &profile.role_names).await?;
        return load_user(pool, user.id).await;
    }

    let user = if let Some(existing_user) = sqlx::query_as::<_, User>(
        "SELECT id, email, name, password_hash, is_active, organization_id, attributes, mfa_enforced, auth_source, created_at, updated_at FROM users WHERE email = $1",
    )
    .bind(email)
    .fetch_optional(pool)
    .await?
    {
        let next_org = profile.organization_id.or(existing_user.organization_id);
        sqlx::query(
            "UPDATE users SET organization_id = $2, attributes = $3, updated_at = NOW() WHERE id = $1",
        )
        .bind(existing_user.id)
        .bind(next_org)
        .bind(profile.attributes.clone())
        .execute(pool)
        .await?;
        existing_user
    } else {
        let user_id = Uuid::now_v7();
        sqlx::query(
            r#"INSERT INTO users (id, email, name, password_hash, is_active, organization_id, attributes, auth_source)
               VALUES ($1, $2, $3, '!sso', true, $4, $5, 'sso')"#,
        )
        .bind(user_id)
        .bind(email)
        .bind(name)
        .bind(profile.organization_id)
        .bind(profile.attributes.clone())
        .execute(pool)
        .await?;

        if let Some(viewer_role) = rbac::get_role_by_name(pool, "viewer").await? {
            let _ = rbac::assign_role(pool, user_id, viewer_role.id).await;
        }

        sqlx::query_as::<_, User>(
            "SELECT id, email, name, password_hash, is_active, organization_id, attributes, mfa_enforced, auth_source, created_at, updated_at FROM users WHERE id = $1",
        )
        .bind(user_id)
        .fetch_one(pool)
        .await?
    };

    sqlx::query(
        r#"INSERT INTO external_identities (provider_id, subject, user_id, email, raw_claims)
           VALUES ($1, $2, $3, $4, $5)
           ON CONFLICT (provider_id, subject) DO UPDATE
           SET user_id = EXCLUDED.user_id,
               email = EXCLUDED.email,
               raw_claims = EXCLUDED.raw_claims"#,
    )
    .bind(provider.id)
    .bind(subject)
    .bind(user.id)
    .bind(email)
    .bind(raw_claims)
    .execute(pool)
    .await?;

    assign_roles_by_name(pool, user.id, &profile.role_names).await?;

    load_user(pool, user.id).await
}

async fn load_user(pool: &sqlx::PgPool, user_id: Uuid) -> Result<User, sqlx::Error> {
    sqlx::query_as::<_, User>(
        "SELECT id, email, name, password_hash, is_active, organization_id, attributes, mfa_enforced, auth_source, created_at, updated_at FROM users WHERE id = $1",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
}

pub(crate) async fn load_identity_provider_mappings(
    pool: &sqlx::PgPool,
) -> Result<Vec<IdentityProviderMapping>, String> {
    let value = sqlx::query_scalar::<_, Value>(
        "SELECT identity_provider_mappings FROM control_panel_settings WHERE singleton_id = TRUE",
    )
    .fetch_optional(pool)
    .await
    .map_err(|cause| cause.to_string())?
    .unwrap_or_else(|| json!([]));

    serde_json::from_value(value)
        .map_err(|cause| format!("invalid control-panel identity_provider_mappings: {cause}"))
}

pub(crate) async fn load_resource_management_policies(
    pool: &sqlx::PgPool,
) -> Result<Vec<ResourceManagementPolicy>, String> {
    let value = sqlx::query_scalar::<_, Value>(
        "SELECT resource_management_policies FROM control_panel_settings WHERE singleton_id = TRUE",
    )
    .fetch_optional(pool)
    .await
    .map_err(|cause| cause.to_string())?
    .unwrap_or_else(|| json!([]));

    serde_json::from_value(value)
        .map_err(|cause| format!("invalid control-panel resource_management_policies: {cause}"))
}

async fn assign_roles_by_name(
    pool: &sqlx::PgPool,
    user_id: Uuid,
    role_names: &[String],
) -> Result<(), sqlx::Error> {
    for role_name in role_names {
        if let Some(role) = rbac::get_role_by_name(pool, role_name).await? {
            rbac::assign_role(pool, user_id, role.id).await?;
        }
    }
    Ok(())
}
