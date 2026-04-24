use axum::{
    Json,
    extract::{Path, State},
};
use sqlx::{query_as, types::Json as SqlJson};
use uuid::Uuid;

use crate::{
    AppState,
    domain::feedback,
    models::{
        ListResponse,
        cluster::{
            ClusterDetail, ClusterRow, ResolvedCluster, ReviewQueueItem, ReviewQueueRow,
            SubmitReviewRequest,
        },
        golden_record::{GoldenRecord, GoldenRecordRow},
    },
};

use super::{ServiceResult, db_error, not_found};

async fn load_cluster_row(
    db: &sqlx::PgPool,
    cluster_id: Uuid,
) -> Result<Option<ClusterRow>, sqlx::Error> {
    query_as::<_, ClusterRow>(
        r#"
        SELECT
            id,
            job_id,
            cluster_key,
            status,
            records,
            evidence,
            confidence_score,
            requires_review,
            suggested_golden_record_id,
            created_at,
            updated_at
        FROM fusion_clusters
        WHERE id = $1
        "#,
    )
    .bind(cluster_id)
    .fetch_optional(db)
    .await
}

async fn load_review_row(
    db: &sqlx::PgPool,
    cluster_id: Uuid,
) -> Result<Option<ReviewQueueRow>, sqlx::Error> {
    query_as::<_, ReviewQueueRow>(
        r#"
        SELECT
            id,
            cluster_id,
            status,
            severity,
            recommended_action,
            rationale,
            assigned_to,
            reviewed_by,
            notes,
            created_at,
            updated_at
        FROM fusion_review_queue
        WHERE cluster_id = $1
        ORDER BY updated_at DESC
        LIMIT 1
        "#,
    )
    .bind(cluster_id)
    .fetch_optional(db)
    .await
}

async fn load_golden_record_row(
    db: &sqlx::PgPool,
    cluster_id: Uuid,
) -> Result<Option<GoldenRecordRow>, sqlx::Error> {
    query_as::<_, GoldenRecordRow>(
        r#"
        SELECT
            id,
            cluster_id,
            title,
            canonical_values,
            provenance,
            completeness_score,
            confidence_score,
            status,
            created_at,
            updated_at
        FROM fusion_golden_records
        WHERE cluster_id = $1
        ORDER BY updated_at DESC
        LIMIT 1
        "#,
    )
    .bind(cluster_id)
    .fetch_optional(db)
    .await
}

pub async fn list_clusters(
    State(state): State<AppState>,
) -> ServiceResult<ListResponse<ResolvedCluster>> {
    let rows = query_as::<_, ClusterRow>(
        r#"
        SELECT
            id,
            job_id,
            cluster_key,
            status,
            records,
            evidence,
            confidence_score,
            requires_review,
            suggested_golden_record_id,
            created_at,
            updated_at
        FROM fusion_clusters
        ORDER BY updated_at DESC, created_at DESC
        "#,
    )
    .fetch_all(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?;

    Ok(Json(ListResponse {
        data: rows.into_iter().map(Into::into).collect(),
    }))
}

pub async fn get_cluster(
    State(state): State<AppState>,
    Path(cluster_id): Path<Uuid>,
) -> ServiceResult<ClusterDetail> {
    let Some(cluster_row) = load_cluster_row(&state.db, cluster_id)
        .await
        .map_err(|cause| db_error(&cause))?
    else {
        return Err(not_found("cluster not found"));
    };

    let review_item = load_review_row(&state.db, cluster_id)
        .await
        .map_err(|cause| db_error(&cause))?
        .map(Into::into);
    let golden_record = load_golden_record_row(&state.db, cluster_id)
        .await
        .map_err(|cause| db_error(&cause))?
        .map(Into::into);

    Ok(Json(ClusterDetail {
        cluster: cluster_row.into(),
        review_item,
        golden_record,
    }))
}

pub async fn list_review_queue(
    State(state): State<AppState>,
) -> ServiceResult<ListResponse<ReviewQueueItem>> {
    let rows = query_as::<_, ReviewQueueRow>(
        r#"
        SELECT
            id,
            cluster_id,
            status,
            severity,
            recommended_action,
            rationale,
            assigned_to,
            reviewed_by,
            notes,
            created_at,
            updated_at
        FROM fusion_review_queue
        ORDER BY updated_at DESC, created_at DESC
        "#,
    )
    .fetch_all(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?;

    Ok(Json(ListResponse {
        data: rows.into_iter().map(Into::into).collect(),
    }))
}

pub async fn list_golden_records(
    State(state): State<AppState>,
) -> ServiceResult<ListResponse<GoldenRecord>> {
    let rows = query_as::<_, GoldenRecordRow>(
        r#"
        SELECT
            id,
            cluster_id,
            title,
            canonical_values,
            provenance,
            completeness_score,
            confidence_score,
            status,
            created_at,
            updated_at
        FROM fusion_golden_records
        ORDER BY updated_at DESC, created_at DESC
        "#,
    )
    .fetch_all(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?;

    Ok(Json(ListResponse {
        data: rows.into_iter().map(Into::into).collect(),
    }))
}

pub async fn submit_review(
    State(state): State<AppState>,
    Path(cluster_id): Path<Uuid>,
    Json(body): Json<SubmitReviewRequest>,
) -> ServiceResult<ClusterDetail> {
    let Some(cluster_row) = load_cluster_row(&state.db, cluster_id)
        .await
        .map_err(|cause| db_error(&cause))?
    else {
        return Err(not_found("cluster not found"));
    };

    let review_row = load_review_row(&state.db, cluster_id)
        .await
        .map_err(|cause| db_error(&cause))?;
    let cluster: ResolvedCluster = cluster_row.into();
    let review_item = review_row.map(Into::into);
    let (updated_cluster, updated_review) =
        feedback::apply_review(&cluster, review_item.as_ref(), &body);

    sqlx::query(
        "UPDATE fusion_clusters SET status = $2, requires_review = $3, suggested_golden_record_id = $4, updated_at = NOW() WHERE id = $1",
    )
    .bind(cluster_id)
    .bind(&updated_cluster.status)
    .bind(updated_cluster.requires_review)
    .bind(updated_cluster.suggested_golden_record_id)
    .execute(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?;

    if let Some(review_item) = &updated_review {
        sqlx::query(
            "UPDATE fusion_review_queue SET status = $2, reviewed_by = $3, notes = $4, rationale = $5, updated_at = NOW() WHERE id = $1",
        )
        .bind(review_item.id)
        .bind(&review_item.status)
        .bind(&review_item.reviewed_by)
        .bind(&review_item.notes)
        .bind(SqlJson(review_item.rationale.clone()))
        .execute(&state.db)
        .await
        .map_err(|cause| db_error(&cause))?;
    }

    sqlx::query(
        "UPDATE fusion_golden_records SET status = $2, updated_at = NOW() WHERE cluster_id = $1",
    )
    .bind(cluster_id)
    .bind(if updated_cluster.status == "rejected" {
        "rejected"
    } else if updated_cluster.status == "split_requested" {
        "superseded"
    } else {
        "active"
    })
    .execute(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?;

    let golden_record = load_golden_record_row(&state.db, cluster_id)
        .await
        .map_err(|cause| db_error(&cause))?
        .map(Into::into);

    Ok(Json(ClusterDetail {
        cluster: updated_cluster,
        review_item: updated_review,
        golden_record,
    }))
}
