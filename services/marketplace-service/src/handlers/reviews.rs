use axum::{
    Json,
    extract::{Path, State},
};
use chrono::Utc;

use crate::{
    AppState,
    handlers::{
        ServiceResult, bad_request, db_error, internal_error, load_listing_row, load_reviews,
        not_found,
    },
    models::{
        ListResponse,
        review::{CreateReviewRequest, ListingReview},
    },
};

pub async fn list_reviews(
    Path(id): Path<uuid::Uuid>,
    State(state): State<AppState>,
) -> ServiceResult<ListResponse<ListingReview>> {
    load_listing_row(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?
        .ok_or_else(|| not_found("listing not found"))?;
    let reviews = load_reviews(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?;
    Ok(Json(ListResponse { items: reviews }))
}

pub async fn create_review(
    Path(id): Path<uuid::Uuid>,
    State(state): State<AppState>,
    Json(request): Json<CreateReviewRequest>,
) -> ServiceResult<ListingReview> {
    if !(1..=5).contains(&request.rating) {
        return Err(bad_request("rating must be between 1 and 5"));
    }
    load_listing_row(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?
        .ok_or_else(|| not_found("listing not found"))?;
    let review_id = uuid::Uuid::now_v7();
    let now = Utc::now();

    sqlx::query(
		"INSERT INTO marketplace_reviews (id, listing_id, author, rating, headline, body, recommended, created_at)
		 VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
	)
	.bind(review_id)
	.bind(id)
	.bind(&request.author)
	.bind(request.rating)
	.bind(&request.headline)
	.bind(&request.body)
	.bind(request.recommended)
	.bind(now)
	.execute(&state.db)
	.await
	.map_err(|cause| db_error(&cause))?;

    let reviews = load_reviews(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?;
    let review = reviews
        .iter()
        .find(|entry| entry.id == review_id)
        .cloned()
        .ok_or_else(|| internal_error("created review could not be reloaded"))?;

    let average = if reviews.is_empty() {
        0.0
    } else {
        reviews.iter().map(|entry| entry.rating as f64).sum::<f64>() / reviews.len() as f64
    };

    sqlx::query(
        "UPDATE marketplace_listings SET average_rating = $2, updated_at = $3 WHERE id = $1",
    )
    .bind(id)
    .bind(average)
    .bind(now)
    .execute(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?;

    Ok(Json(review))
}
