use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListingReview {
    pub id: Uuid,
    pub listing_id: Uuid,
    pub author: String,
    pub rating: i32,
    pub headline: String,
    pub body: String,
    pub recommended: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateReviewRequest {
    pub author: String,
    pub rating: i32,
    pub headline: String,
    pub body: String,
    #[serde(default)]
    pub recommended: bool,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ReviewRow {
    pub id: Uuid,
    pub listing_id: Uuid,
    pub author: String,
    pub rating: i32,
    pub headline: String,
    pub body: String,
    pub recommended: bool,
    pub created_at: DateTime<Utc>,
}

impl TryFrom<ReviewRow> for ListingReview {
    type Error = String;

    fn try_from(row: ReviewRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            listing_id: row.listing_id,
            author: row.author,
            rating: row.rating,
            headline: row.headline,
            body: row.body,
            recommended: row.recommended,
            created_at: row.created_at,
        })
    }
}
