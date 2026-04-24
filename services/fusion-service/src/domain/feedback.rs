use chrono::Utc;

use crate::models::cluster::{ResolvedCluster, ReviewQueueItem, SubmitReviewRequest};

pub fn apply_review(
    cluster: &ResolvedCluster,
    review_item: Option<&ReviewQueueItem>,
    request: &SubmitReviewRequest,
) -> (ResolvedCluster, Option<ReviewQueueItem>) {
    let now = Utc::now();
    let mut updated_cluster = cluster.clone();
    let mut updated_review = review_item.cloned();

    match request.decision.as_str() {
        "confirm_match" => {
            updated_cluster.status = "resolved".to_string();
            updated_cluster.requires_review = false;
        }
        "split_cluster" => {
            updated_cluster.status = "split_requested".to_string();
            updated_cluster.requires_review = false;
        }
        "reject_match" => {
            updated_cluster.status = "rejected".to_string();
            updated_cluster.requires_review = false;
        }
        _ => {
            updated_cluster.status = "manually_resolved".to_string();
            updated_cluster.requires_review = false;
        }
    }
    updated_cluster.updated_at = now;

    if let Some(review_item) = &mut updated_review {
        review_item.status = match request.decision.as_str() {
            "split_cluster" => "split_requested",
            "reject_match" => "rejected",
            _ => "resolved",
        }
        .to_string();
        review_item.reviewed_by = request.reviewed_by.clone();
        review_item.notes = request.notes.clone().unwrap_or_default();
        review_item.updated_at = now;
    }

    (updated_cluster, updated_review)
}
