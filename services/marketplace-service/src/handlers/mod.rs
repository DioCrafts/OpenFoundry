pub mod browse;
pub mod devops;
pub mod install;
pub mod publish;
pub mod reviews;

use axum::{Json, http::StatusCode};
use serde::Serialize;

use crate::models::{
    devops::{EnrollmentBranchRow, ProductFleetRow, PromotionGateRow},
    install::InstallRow,
    listing::ListingRow,
    package::PackageVersionRow,
    review::ReviewRow,
};

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

pub type ServiceResult<T> = Result<Json<T>, (StatusCode, Json<ErrorResponse>)>;

pub fn bad_request(message: impl Into<String>) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::BAD_REQUEST,
        Json(ErrorResponse {
            error: message.into(),
        }),
    )
}

pub fn not_found(message: impl Into<String>) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse {
            error: message.into(),
        }),
    )
}

pub fn internal_error(message: impl Into<String>) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: message.into(),
        }),
    )
}

pub fn db_error(cause: &sqlx::Error) -> (StatusCode, Json<ErrorResponse>) {
    tracing::error!("marketplace-service database error: {cause}");
    internal_error("database operation failed")
}

pub async fn load_listing_row(
    db: &sqlx::PgPool,
    id: uuid::Uuid,
) -> Result<Option<ListingRow>, sqlx::Error> {
    sqlx::query_as::<_, ListingRow>(
		"SELECT id, name, slug, summary, description, publisher, category_slug, package_kind, repository_slug, visibility, tags, capabilities, install_count, average_rating, created_at, updated_at
		 FROM marketplace_listings
		 WHERE id = $1",
	)
	.bind(id)
	.fetch_optional(db)
	.await
}

pub async fn load_listings(
    db: &sqlx::PgPool,
) -> Result<Vec<crate::models::listing::ListingDefinition>, sqlx::Error> {
    let rows = sqlx::query_as::<_, ListingRow>(
		"SELECT id, name, slug, summary, description, publisher, category_slug, package_kind, repository_slug, visibility, tags, capabilities, install_count, average_rating, created_at, updated_at
		 FROM marketplace_listings
		 ORDER BY install_count DESC, average_rating DESC, updated_at DESC",
	)
	.fetch_all(db)
	.await?;

    rows.into_iter()
        .map(crate::models::listing::ListingDefinition::try_from)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|cause| {
            sqlx::Error::Decode(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                cause,
            )))
        })
}

pub async fn load_versions(
    db: &sqlx::PgPool,
    listing_id: uuid::Uuid,
) -> Result<Vec<crate::models::package::PackageVersion>, sqlx::Error> {
    let rows = sqlx::query_as::<_, PackageVersionRow>(
		"SELECT id, listing_id, version, release_channel, changelog, dependency_mode, dependencies, packaged_resources, manifest, published_at
		 FROM marketplace_package_versions
		 WHERE listing_id = $1
		 ORDER BY published_at DESC",
	)
	.bind(listing_id)
	.fetch_all(db)
	.await?;

    rows.into_iter()
        .map(crate::models::package::PackageVersion::try_from)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|cause| {
            sqlx::Error::Decode(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                cause,
            )))
        })
}

pub async fn load_reviews(
    db: &sqlx::PgPool,
    listing_id: uuid::Uuid,
) -> Result<Vec<crate::models::review::ListingReview>, sqlx::Error> {
    let rows = sqlx::query_as::<_, ReviewRow>(
        "SELECT id, listing_id, author, rating, headline, body, recommended, created_at
		 FROM marketplace_reviews
		 WHERE listing_id = $1
		 ORDER BY created_at DESC",
    )
    .bind(listing_id)
    .fetch_all(db)
    .await?;

    rows.into_iter()
        .map(crate::models::review::ListingReview::try_from)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|cause| {
            sqlx::Error::Decode(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                cause,
            )))
        })
}

pub async fn load_installs(
    db: &sqlx::PgPool,
) -> Result<Vec<crate::models::install::InstallRecord>, sqlx::Error> {
    let rows = sqlx::query_as::<_, InstallRow>(
        "SELECT installs.id,
		        installs.listing_id,
		        installs.listing_name,
		        installs.version,
		        installs.release_channel,
		        installs.workspace_name,
		        installs.status,
		        installs.dependency_plan,
		        installs.activation,
		        installs.fleet_id,
		        fleets.name AS fleet_name,
		        installs.maintenance_window,
		        installs.auto_upgrade_enabled,
		        installs.enrollment_branch,
		        installs.installed_at,
		        installs.ready_at
		 FROM marketplace_installs installs
		 LEFT JOIN marketplace_product_fleets fleets ON fleets.id = installs.fleet_id
		 ORDER BY installed_at DESC",
    )
    .fetch_all(db)
    .await?;

    rows.into_iter()
        .map(crate::models::install::InstallRecord::try_from)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|cause| {
            sqlx::Error::Decode(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                cause,
            )))
        })
}

pub async fn load_fleet_rows(db: &sqlx::PgPool) -> Result<Vec<ProductFleetRow>, sqlx::Error> {
    sqlx::query_as::<_, ProductFleetRow>(
        "SELECT fleets.id,
                fleets.listing_id,
                listings.name AS listing_name,
                fleets.name,
                fleets.environment,
                fleets.workspace_targets,
                fleets.release_channel,
                fleets.auto_upgrade_enabled,
                fleets.maintenance_window,
                fleets.branch_strategy,
                fleets.rollout_strategy,
                fleets.deployment_cells,
                fleets.residency_policy,
                fleets.status,
                fleets.last_synced_at,
                fleets.created_at,
                fleets.updated_at
         FROM marketplace_product_fleets fleets
         INNER JOIN marketplace_listings listings ON listings.id = fleets.listing_id
         ORDER BY fleets.updated_at DESC, fleets.created_at DESC",
    )
    .fetch_all(db)
    .await
}

pub async fn load_fleet_row(
    db: &sqlx::PgPool,
    id: uuid::Uuid,
) -> Result<Option<ProductFleetRow>, sqlx::Error> {
    sqlx::query_as::<_, ProductFleetRow>(
        "SELECT fleets.id,
                fleets.listing_id,
                listings.name AS listing_name,
                fleets.name,
                fleets.environment,
                fleets.workspace_targets,
                fleets.release_channel,
                fleets.auto_upgrade_enabled,
                fleets.maintenance_window,
                fleets.branch_strategy,
                fleets.rollout_strategy,
                fleets.deployment_cells,
                fleets.residency_policy,
                fleets.status,
                fleets.last_synced_at,
                fleets.created_at,
                fleets.updated_at
         FROM marketplace_product_fleets fleets
         INNER JOIN marketplace_listings listings ON listings.id = fleets.listing_id
         WHERE fleets.id = $1",
    )
    .bind(id)
    .fetch_optional(db)
    .await
}

pub async fn load_enrollment_branch_rows(
    db: &sqlx::PgPool,
) -> Result<Vec<EnrollmentBranchRow>, sqlx::Error> {
    sqlx::query_as::<_, EnrollmentBranchRow>(
        "SELECT branches.id,
                branches.fleet_id,
                fleets.name AS fleet_name,
                branches.listing_id,
                listings.name AS listing_name,
                branches.name,
                branches.repository_branch,
                branches.source_release_channel,
                branches.source_version,
                branches.workspace_targets,
                branches.status,
                branches.notes,
                branches.created_at,
                branches.updated_at
         FROM marketplace_enrollment_branches branches
         INNER JOIN marketplace_product_fleets fleets ON fleets.id = branches.fleet_id
         INNER JOIN marketplace_listings listings ON listings.id = branches.listing_id
         ORDER BY branches.updated_at DESC, branches.created_at DESC",
    )
    .fetch_all(db)
    .await
}

pub async fn load_promotion_gate_rows(
    db: &sqlx::PgPool,
    fleet_id: Option<uuid::Uuid>,
) -> Result<Vec<PromotionGateRow>, sqlx::Error> {
    if let Some(fleet_id) = fleet_id {
        return sqlx::query_as::<_, PromotionGateRow>(
            "SELECT gates.id,
                    gates.fleet_id,
                    fleets.name AS fleet_name,
                    gates.name,
                    gates.gate_kind,
                    gates.required,
                    gates.status,
                    gates.evidence,
                    gates.notes,
                    gates.last_evaluated_at,
                    gates.created_at,
                    gates.updated_at
             FROM marketplace_fleet_promotion_gates gates
             INNER JOIN marketplace_product_fleets fleets ON fleets.id = gates.fleet_id
             WHERE gates.fleet_id = $1
             ORDER BY gates.updated_at DESC, gates.created_at DESC",
        )
        .bind(fleet_id)
        .fetch_all(db)
        .await;
    }

    sqlx::query_as::<_, PromotionGateRow>(
        "SELECT gates.id,
                gates.fleet_id,
                fleets.name AS fleet_name,
                gates.name,
                gates.gate_kind,
                gates.required,
                gates.status,
                gates.evidence,
                gates.notes,
                gates.last_evaluated_at,
                gates.created_at,
                gates.updated_at
         FROM marketplace_fleet_promotion_gates gates
         INNER JOIN marketplace_product_fleets fleets ON fleets.id = gates.fleet_id
         ORDER BY gates.updated_at DESC, gates.created_at DESC",
    )
    .fetch_all(db)
    .await
}

pub async fn load_promotion_gate_row(
    db: &sqlx::PgPool,
    id: uuid::Uuid,
) -> Result<Option<PromotionGateRow>, sqlx::Error> {
    sqlx::query_as::<_, PromotionGateRow>(
        "SELECT gates.id,
                gates.fleet_id,
                fleets.name AS fleet_name,
                gates.name,
                gates.gate_kind,
                gates.required,
                gates.status,
                gates.evidence,
                gates.notes,
                gates.last_evaluated_at,
                gates.created_at,
                gates.updated_at
         FROM marketplace_fleet_promotion_gates gates
         INNER JOIN marketplace_product_fleets fleets ON fleets.id = gates.fleet_id
         WHERE gates.id = $1",
    )
    .bind(id)
    .fetch_optional(db)
    .await
}
