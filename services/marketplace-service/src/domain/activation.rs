use anyhow::{Context, Result, bail};
use axum::http::HeaderMap;
use serde_json::Value;

use crate::{
    AppState,
    models::{
        install::InstallActivation,
        listing::ListingDefinition,
        package::{PackageType, PackageVersion},
    },
};

pub async fn activate_install(
    state: &AppState,
    headers: &HeaderMap,
    listing: &ListingDefinition,
    version: &PackageVersion,
    workspace_name: &str,
    install_id: uuid::Uuid,
) -> Result<InstallActivation> {
    match listing.package_kind {
        PackageType::AppTemplate => {
            activate_app_template(state, headers, listing, version, workspace_name, install_id)
                .await
        }
        _ => Ok(InstallActivation {
            kind: "marketplace_record".to_string(),
            status: "recorded".to_string(),
            resource_id: None,
            resource_slug: Some(listing.slug.clone()),
            public_url: None,
            notes: Some(format!(
                "Recorded product package on channel `{}` with {} packaged resource(s). Runtime activation is only wired for app templates in this milestone.",
                version.release_channel,
                version.packaged_resources.len()
            )),
        }),
    }
}

async fn activate_app_template(
    state: &AppState,
    headers: &HeaderMap,
    listing: &ListingDefinition,
    version: &PackageVersion,
    workspace_name: &str,
    install_id: uuid::Uuid,
) -> Result<InstallActivation> {
    let template_key = version
        .manifest
        .get("template_key")
        .and_then(Value::as_str)
        .or_else(|| {
            version
                .manifest
                .get("app_template_key")
                .and_then(Value::as_str)
        })
        .context("app template install is missing manifest.template_key")?;
    let short_install_id = install_id.simple().to_string();
    let short_install_id = &short_install_id[..8];

    let slug = format!(
        "{}-{}-{}",
        slugify(workspace_name),
        slugify(&listing.slug),
        short_install_id
    );
    let create_payload = serde_json::json!({
        "name": format!("{workspace_name} - {}", listing.name),
        "slug": slug,
        "description": format!("Installed from marketplace listing {} {}", listing.slug, version.version),
        "template_key": template_key,
    });

    let create_response = authorized_request(
        state.http_client.post(format!(
            "{}/api/v1/apps/from-template",
            state.app_builder_service_url.trim_end_matches('/')
        )),
        headers,
    )
    .json(&create_payload)
    .send()
    .await
    .context("failed to create app from marketplace template")?;

    let create_status = create_response.status();
    let create_body: Value = create_response
        .json::<Value>()
        .await
        .with_context(|| "failed to decode app-builder create response")?;
    if !create_status.is_success() {
        bail!(
            "app-builder create-from-template returned {}: {}",
            create_status,
            create_body
        );
    }

    let app_id = create_body
        .pointer("/id")
        .and_then(Value::as_str)
        .context("app-builder create response did not include app id")?
        .to_string();
    let app_slug = create_body
        .pointer("/slug")
        .and_then(Value::as_str)
        .context("app-builder create response did not include app slug")?
        .to_string();

    let publish_response = authorized_request(
        state.http_client.post(format!(
            "{}/api/v1/apps/{app_id}/publish",
            state.app_builder_service_url.trim_end_matches('/')
        )),
        headers,
    )
    .json(&serde_json::json!({
        "notes": format!("Installed from marketplace listing {} {}", listing.slug, version.version)
    }))
    .send()
    .await
    .context("failed to publish installed app template")?;

    let publish_status = publish_response.status();
    let publish_body: Value = publish_response
        .json::<Value>()
        .await
        .with_context(|| "failed to decode app publish response")?;
    if !publish_status.is_success() {
        bail!(
            "app-builder publish returned {}: {}",
            publish_status,
            publish_body
        );
    }

    Ok(InstallActivation {
        kind: "app_template".to_string(),
        status: "activated".to_string(),
        resource_id: Some(
            uuid::Uuid::parse_str(&app_id).context("invalid app id returned by app-builder")?,
        ),
        resource_slug: Some(app_slug.clone()),
        public_url: Some(format!("/apps/runtime/{app_slug}")),
        notes: Some(format!(
            "Created and published from template `{template_key}` via app-builder-service on channel `{}`.",
            version.release_channel
        )),
    })
}

fn authorized_request(
    request: reqwest::RequestBuilder,
    headers: &HeaderMap,
) -> reqwest::RequestBuilder {
    if let Some(header_value) = headers.get(axum::http::header::AUTHORIZATION) {
        return request.header(axum::http::header::AUTHORIZATION, header_value);
    }

    request
}

fn slugify(value: &str) -> String {
    let mut slug = String::new();
    let mut last_dash = false;
    for character in value.chars().flat_map(char::to_lowercase) {
        if character.is_ascii_alphanumeric() {
            slug.push(character);
            last_dash = false;
        } else if !last_dash {
            slug.push('-');
            last_dash = true;
        }
    }
    let slug = slug.trim_matches('-').to_string();
    if slug.is_empty() {
        "install".to_string()
    } else {
        slug
    }
}
