pub mod browse;
pub mod install;
pub mod publish;
pub mod reviews;

use axum::{Json, http::StatusCode};
use serde::Serialize;

use crate::models::{
    install::InstallRow, listing::ListingRow, package::PackageVersionRow, review::ReviewRow,
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
		"SELECT id, listing_id, version, changelog, dependency_mode, dependencies, manifest, published_at
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
		"SELECT id, listing_id, listing_name, version, workspace_name, status, dependency_plan, activation, installed_at, ready_at
		 FROM marketplace_installs
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
