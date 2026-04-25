use axum::{
    Json,
    extract::{Path, State},
};
use chrono::Utc;

use crate::{
    AppState,
    domain::{registry, validator},
    handlers::{
        ServiceResult, bad_request, db_error, internal_error, load_listing_row, load_versions,
        not_found,
    },
    models::{
        ListResponse,
        listing::{CreateListingRequest, ListingDefinition, UpdateListingRequest},
        package::{PackageVersion, PublishVersionRequest},
    },
};

pub async fn publish_listing(
    State(state): State<AppState>,
    Json(request): Json<CreateListingRequest>,
) -> ServiceResult<ListingDefinition> {
    validator::validate_listing(&request).map_err(bad_request)?;
    let id = uuid::Uuid::now_v7();
    let now = Utc::now();
    let tags =
        serde_json::to_value(&request.tags).map_err(|cause| internal_error(cause.to_string()))?;
    let capabilities = serde_json::to_value(&request.capabilities)
        .map_err(|cause| internal_error(cause.to_string()))?;

    sqlx::query(
		"INSERT INTO marketplace_listings (id, name, slug, summary, description, publisher, category_slug, package_kind, repository_slug, visibility, tags, capabilities, install_count, average_rating, created_at, updated_at)
		 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11::jsonb, $12::jsonb, 0, 0, $13, $14)",
	)
	.bind(id)
	.bind(&request.name)
	.bind(&request.slug)
	.bind(&request.summary)
	.bind(&request.description)
	.bind(&request.publisher)
	.bind(&request.category_slug)
	.bind(request.package_kind.as_str())
	.bind(&request.repository_slug)
	.bind(&request.visibility)
	.bind(tags)
	.bind(capabilities)
	.bind(now)
	.bind(now)
	.execute(&state.db)
	.await
	.map_err(|cause| db_error(&cause))?;

    let row = load_listing_row(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?
        .ok_or_else(|| internal_error("created listing could not be reloaded"))?;
    let listing =
        ListingDefinition::try_from(row).map_err(|cause| internal_error(cause.to_string()))?;
    Ok(Json(listing))
}

pub async fn update_listing(
    Path(id): Path<uuid::Uuid>,
    State(state): State<AppState>,
    Json(request): Json<UpdateListingRequest>,
) -> ServiceResult<ListingDefinition> {
    let row = load_listing_row(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?
        .ok_or_else(|| not_found("listing not found"))?;
    let mut listing =
        ListingDefinition::try_from(row).map_err(|cause| internal_error(cause.to_string()))?;

    if let Some(name) = request.name {
        listing.name = name;
    }
    if let Some(summary) = request.summary {
        listing.summary = summary;
    }
    if let Some(description) = request.description {
        listing.description = description;
    }
    if let Some(category_slug) = request.category_slug {
        listing.category_slug = category_slug;
    }
    if let Some(repository_slug) = request.repository_slug {
        listing.repository_slug = repository_slug;
    }
    if let Some(visibility) = request.visibility {
        listing.visibility = visibility;
    }
    if let Some(tags) = request.tags {
        listing.tags = tags;
    }
    if let Some(capabilities) = request.capabilities {
        listing.capabilities = capabilities;
    }

    let now = Utc::now();
    let tags =
        serde_json::to_value(&listing.tags).map_err(|cause| internal_error(cause.to_string()))?;
    let capabilities = serde_json::to_value(&listing.capabilities)
        .map_err(|cause| internal_error(cause.to_string()))?;

    sqlx::query(
		"UPDATE marketplace_listings
		 SET name = $2, summary = $3, description = $4, category_slug = $5, repository_slug = $6, visibility = $7, tags = $8::jsonb, capabilities = $9::jsonb, updated_at = $10
		 WHERE id = $1",
	)
	.bind(id)
	.bind(&listing.name)
	.bind(&listing.summary)
	.bind(&listing.description)
	.bind(&listing.category_slug)
	.bind(&listing.repository_slug)
	.bind(&listing.visibility)
	.bind(tags)
	.bind(capabilities)
	.bind(now)
	.execute(&state.db)
	.await
	.map_err(|cause| db_error(&cause))?;

    let row = load_listing_row(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?
        .ok_or_else(|| internal_error("updated listing could not be reloaded"))?;
    let listing =
        ListingDefinition::try_from(row).map_err(|cause| internal_error(cause.to_string()))?;
    Ok(Json(listing))
}

pub async fn list_versions(
    Path(id): Path<uuid::Uuid>,
    State(state): State<AppState>,
) -> ServiceResult<ListResponse<PackageVersion>> {
    load_listing_row(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?
        .ok_or_else(|| not_found("listing not found"))?;
    let versions = load_versions(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?;
    Ok(Json(ListResponse { items: versions }))
}

pub async fn publish_version(
    Path(id): Path<uuid::Uuid>,
    State(state): State<AppState>,
    Json(request): Json<PublishVersionRequest>,
) -> ServiceResult<PackageVersion> {
    validator::validate_version(&request).map_err(bad_request)?;
    load_listing_row(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?
        .ok_or_else(|| not_found("listing not found"))?;
    let version_id = uuid::Uuid::now_v7();
    let published_at = Utc::now();
    let dependencies =
        serde_json::to_value(registry::normalize_dependencies(&request.dependencies))
            .map_err(|cause| internal_error(cause.to_string()))?;
    let packaged_resources = serde_json::to_value(
        crate::domain::devops::normalize_packaged_resources(&request.packaged_resources),
    )
    .map_err(|cause| internal_error(cause.to_string()))?;

    sqlx::query(
		"INSERT INTO marketplace_package_versions (id, listing_id, version, release_channel, changelog, dependency_mode, dependencies, packaged_resources, manifest, published_at)
		 VALUES ($1, $2, $3, $4, $5, $6, $7::jsonb, $8::jsonb, $9::jsonb, $10)",
	)
	.bind(version_id)
	.bind(id)
	.bind(&request.version)
	.bind(crate::domain::devops::normalize_release_channel(&request.release_channel))
	.bind(&request.changelog)
	.bind(&request.dependency_mode)
	.bind(dependencies)
	.bind(packaged_resources)
	.bind(request.manifest)
	.bind(published_at)
	.execute(&state.db)
	.await
	.map_err(|cause| db_error(&cause))?;

    let versions = load_versions(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?;
    let version = versions
        .into_iter()
        .find(|entry| entry.id == version_id)
        .ok_or_else(|| internal_error("created version could not be reloaded"))?;
    Ok(Json(version))
}
