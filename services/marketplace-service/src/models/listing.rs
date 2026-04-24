use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::models::{decode_json, package::PackageVersion, review::ListingReview};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListingDefinition {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub summary: String,
    pub description: String,
    pub publisher: String,
    pub category_slug: String,
    pub package_kind: crate::models::package::PackageType,
    pub repository_slug: String,
    pub visibility: String,
    pub tags: Vec<String>,
    pub capabilities: Vec<String>,
    pub install_count: i64,
    pub average_rating: f64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateListingRequest {
    pub name: String,
    pub slug: String,
    pub summary: String,
    #[serde(default)]
    pub description: String,
    pub publisher: String,
    pub category_slug: String,
    pub package_kind: crate::models::package::PackageType,
    pub repository_slug: String,
    #[serde(default = "default_visibility")]
    pub visibility: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateListingRequest {
    pub name: Option<String>,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub category_slug: Option<String>,
    pub repository_slug: Option<String>,
    pub visibility: Option<String>,
    pub tags: Option<Vec<String>>,
    pub capabilities: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListingDetail {
    pub listing: ListingDefinition,
    pub latest_version: Option<PackageVersion>,
    pub versions: Vec<PackageVersion>,
    pub reviews: Vec<ListingReview>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceOverview {
    pub listing_count: usize,
    pub category_count: usize,
    pub featured: Vec<ListingDefinition>,
    pub total_installs: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    pub query: String,
    pub results: Vec<(ListingDefinition, f64)>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ListingRow {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub summary: String,
    pub description: String,
    pub publisher: String,
    pub category_slug: String,
    pub package_kind: String,
    pub repository_slug: String,
    pub visibility: String,
    pub tags: Value,
    pub capabilities: Value,
    pub install_count: i64,
    pub average_rating: f64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TryFrom<ListingRow> for ListingDefinition {
    type Error = String;

    fn try_from(row: ListingRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            name: row.name,
            slug: row.slug,
            summary: row.summary,
            description: row.description,
            publisher: row.publisher,
            category_slug: row.category_slug,
            package_kind: crate::models::package::PackageType::from_str(&row.package_kind)?,
            repository_slug: row.repository_slug,
            visibility: row.visibility,
            tags: decode_json(row.tags, "tags")?,
            capabilities: decode_json(row.capabilities, "capabilities")?,
            install_count: row.install_count,
            average_rating: row.average_rating,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

fn default_visibility() -> String {
    "private".to_string()
}
