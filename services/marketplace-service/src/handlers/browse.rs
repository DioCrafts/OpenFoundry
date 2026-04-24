use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::Deserialize;

use crate::{
    AppState,
    domain::{discovery, registry},
    handlers::{
        ServiceResult, db_error, load_listing_row, load_listings, load_reviews, load_versions,
        not_found,
    },
    models::{
        ListResponse,
        category::CategoryDefinition,
        listing::{ListingDefinition, ListingDetail, MarketplaceOverview, SearchResponse},
    },
};

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: Option<String>,
    pub category: Option<String>,
}

pub async fn get_overview(State(state): State<AppState>) -> ServiceResult<MarketplaceOverview> {
    let listings = load_listings(&state.db)
        .await
        .map_err(|cause| db_error(&cause))?;
    let categories = discovery::featured_categories(&listings);
    let featured = listings.iter().take(3).cloned().collect();

    Ok(Json(MarketplaceOverview {
        listing_count: listings.len(),
        category_count: categories.len(),
        featured,
        total_installs: listings.iter().map(|listing| listing.install_count).sum(),
    }))
}

pub async fn list_categories(
    State(state): State<AppState>,
) -> ServiceResult<ListResponse<CategoryDefinition>> {
    let listings = load_listings(&state.db)
        .await
        .map_err(|cause| db_error(&cause))?;
    Ok(Json(ListResponse {
        items: discovery::featured_categories(&listings),
    }))
}

pub async fn list_listings(
    State(state): State<AppState>,
) -> ServiceResult<ListResponse<ListingDefinition>> {
    let listings = load_listings(&state.db)
        .await
        .map_err(|cause| db_error(&cause))?;
    Ok(Json(ListResponse { items: listings }))
}

pub async fn get_listing(
    Path(id): Path<uuid::Uuid>,
    State(state): State<AppState>,
) -> ServiceResult<ListingDetail> {
    let row = load_listing_row(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?
        .ok_or_else(|| not_found("listing not found"))?;
    let listing = ListingDefinition::try_from(row)
        .map_err(|cause| crate::handlers::internal_error(cause.to_string()))?;
    let versions = load_versions(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?;
    let reviews = load_reviews(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?;
    let latest_version = registry::latest_version(&listing, &versions);
    Ok(Json(ListingDetail {
        listing,
        latest_version,
        versions,
        reviews,
    }))
}

pub async fn search_listings(
    Query(query): Query<SearchQuery>,
    State(state): State<AppState>,
) -> ServiceResult<SearchResponse> {
    let listings = load_listings(&state.db)
        .await
        .map_err(|cause| db_error(&cause))?;
    let query_text = query.q.unwrap_or_else(|| "widget".to_string());
    let results = listings
        .into_iter()
        .filter(|listing| {
            query
                .category
                .as_ref()
                .map(|category| category == &listing.category_slug)
                .unwrap_or(true)
        })
        .map(|listing| {
            let score = discovery::score_listing(&listing, &query_text);
            (listing, score)
        })
        .filter(|(_, score)| *score > 0.45)
        .collect::<Vec<_>>();

    Ok(Json(SearchResponse {
        query: query_text,
        results,
    }))
}
