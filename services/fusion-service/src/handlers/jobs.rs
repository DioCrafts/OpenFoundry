use axum::{
    Json,
    extract::{Path, State},
};
use chrono::Utc;
use sqlx::{query_as, query_scalar, types::Json as SqlJson};
use uuid::Uuid;

use crate::{
    AppState,
    domain::{
        deduplication,
        engine::{blocking, graph_resolution, ml_matcher, rule_matcher},
        merge,
    },
    models::{
        FusionOverview, ListResponse,
        golden_record::GoldenRecord,
        job::{
            CreateFusionJobRequest, FusionJob, FusionJobMetrics, FusionJobRow,
            RunResolutionJobResponse,
        },
        match_rule::MatchRuleRow,
        merge_strategy::MergeStrategyRow,
    },
};

use super::{ServiceResult, bad_request, db_error, not_found};

async fn load_job_row(
    db: &sqlx::PgPool,
    job_id: Uuid,
) -> Result<Option<FusionJobRow>, sqlx::Error> {
    query_as::<_, FusionJobRow>(
        r#"
        SELECT
            id,
            name,
            description,
            status,
            entity_type,
            match_rule_id,
            merge_strategy_id,
            config,
            metrics,
            last_run_summary,
            last_run_at,
            created_at,
            updated_at
        FROM fusion_jobs
        WHERE id = $1
        "#,
    )
    .bind(job_id)
    .fetch_optional(db)
    .await
}

async fn load_rule_row(
    db: &sqlx::PgPool,
    rule_id: Uuid,
) -> Result<Option<MatchRuleRow>, sqlx::Error> {
    query_as::<_, MatchRuleRow>(
        r#"
        SELECT
            id,
            name,
            description,
            status,
            entity_type,
            blocking_strategy,
            conditions,
            review_threshold,
            auto_merge_threshold,
            created_at,
            updated_at
        FROM fusion_match_rules
        WHERE id = $1
        "#,
    )
    .bind(rule_id)
    .fetch_optional(db)
    .await
}

async fn load_merge_strategy_row(
    db: &sqlx::PgPool,
    strategy_id: Uuid,
) -> Result<Option<MergeStrategyRow>, sqlx::Error> {
    query_as::<_, MergeStrategyRow>(
        r#"
        SELECT
            id,
            name,
            description,
            status,
            entity_type,
            default_strategy,
            rules,
            created_at,
            updated_at
        FROM fusion_merge_strategies
        WHERE id = $1
        "#,
    )
    .bind(strategy_id)
    .fetch_optional(db)
    .await
}

pub async fn get_overview(State(state): State<AppState>) -> ServiceResult<FusionOverview> {
    let rule_count = query_scalar::<_, i64>("SELECT COUNT(*) FROM fusion_match_rules")
        .fetch_one(&state.db)
        .await
        .map_err(|cause| db_error(&cause))?;
    let active_job_count = query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM fusion_jobs WHERE status IN ('draft', 'running', 'awaiting_review')",
    )
    .fetch_one(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?;
    let completed_job_count = query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM fusion_jobs WHERE status IN ('completed', 'awaiting_review')",
    )
    .fetch_one(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?;
    let cluster_count = query_scalar::<_, i64>("SELECT COUNT(*) FROM fusion_clusters")
        .fetch_one(&state.db)
        .await
        .map_err(|cause| db_error(&cause))?;
    let pending_review_count =
        query_scalar::<_, i64>("SELECT COUNT(*) FROM fusion_review_queue WHERE status = 'pending'")
            .fetch_one(&state.db)
            .await
            .map_err(|cause| db_error(&cause))?;
    let golden_record_count = query_scalar::<_, i64>("SELECT COUNT(*) FROM fusion_golden_records")
        .fetch_one(&state.db)
        .await
        .map_err(|cause| db_error(&cause))?;
    let auto_merged_cluster_count = query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM fusion_clusters WHERE status = 'resolved' AND requires_review = FALSE",
    )
    .fetch_one(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?;

    Ok(Json(FusionOverview {
        rule_count,
        active_job_count,
        completed_job_count,
        cluster_count,
        pending_review_count,
        golden_record_count,
        auto_merged_cluster_count,
    }))
}

pub async fn list_jobs(State(state): State<AppState>) -> ServiceResult<ListResponse<FusionJob>> {
    let rows = query_as::<_, FusionJobRow>(
        r#"
        SELECT
            id,
            name,
            description,
            status,
            entity_type,
            match_rule_id,
            merge_strategy_id,
            config,
            metrics,
            last_run_summary,
            last_run_at,
            created_at,
            updated_at
        FROM fusion_jobs
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

pub async fn create_job(
    State(state): State<AppState>,
    Json(body): Json<CreateFusionJobRequest>,
) -> ServiceResult<FusionJob> {
    if body.name.trim().is_empty() {
        return Err(bad_request("job name is required"));
    }

    let rule_exists = load_rule_row(&state.db, body.match_rule_id)
        .await
        .map_err(|cause| db_error(&cause))?
        .is_some();
    let strategy_exists = load_merge_strategy_row(&state.db, body.merge_strategy_id)
        .await
        .map_err(|cause| db_error(&cause))?
        .is_some();
    if !rule_exists || !strategy_exists {
        return Err(bad_request(
            "job requires an existing match rule and merge strategy",
        ));
    }

    let row = query_as::<_, FusionJobRow>(
        r#"
        INSERT INTO fusion_jobs (
            id,
            name,
            description,
            status,
            entity_type,
            match_rule_id,
            merge_strategy_id,
            config,
            metrics,
            last_run_summary,
            last_run_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'Not run yet', NULL)
        RETURNING
            id,
            name,
            description,
            status,
            entity_type,
            match_rule_id,
            merge_strategy_id,
            config,
            metrics,
            last_run_summary,
            last_run_at,
            created_at,
            updated_at
        "#,
    )
    .bind(Uuid::now_v7())
    .bind(body.name.trim())
    .bind(body.description.unwrap_or_default())
    .bind(body.status.unwrap_or_else(|| "draft".to_string()))
    .bind(body.entity_type.unwrap_or_else(|| "person".to_string()))
    .bind(body.match_rule_id)
    .bind(body.merge_strategy_id)
    .bind(SqlJson(body.config.unwrap_or_default()))
    .bind(SqlJson(FusionJobMetrics::default()))
    .fetch_one(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?;

    Ok(Json(row.into()))
}

pub async fn run_job(
    State(state): State<AppState>,
    Path(job_id): Path<Uuid>,
) -> ServiceResult<RunResolutionJobResponse> {
    let Some(job_row) = load_job_row(&state.db, job_id)
        .await
        .map_err(|cause| db_error(&cause))?
    else {
        return Err(not_found("fusion job not found"));
    };
    let Some(rule_row) = load_rule_row(&state.db, job_row.match_rule_id)
        .await
        .map_err(|cause| db_error(&cause))?
    else {
        return Err(not_found("match rule not found"));
    };
    let Some(strategy_row) = load_merge_strategy_row(&state.db, job_row.merge_strategy_id)
        .await
        .map_err(|cause| db_error(&cause))?
    else {
        return Err(not_found("merge strategy not found"));
    };

    let job: FusionJob = job_row.into();
    let rule: crate::models::match_rule::MatchRule = rule_row.into();
    let strategy: crate::models::merge_strategy::MergeStrategy = strategy_row.into();

    let records = deduplication::synthesize_entity_records(&job.entity_type, &job.config);
    if records.len() < 2 {
        return Err(bad_request("resolution job requires at least two records"));
    }

    let blocking_strategy = job
        .config
        .blocking_strategy_override
        .clone()
        .unwrap_or_else(|| rule.blocking_strategy.clone());
    let candidate_pairs = blocking::build_candidate_pairs(&records, &blocking_strategy);

    let mut evidences = Vec::new();
    for pair in &candidate_pairs {
        let mut evidence = rule_matcher::evaluate_candidate(&rule, pair);
        let ml_score = ml_matcher::score_candidate(&pair.left, &pair.right, &evidence);
        evidence.ml_score = ml_score;
        evidence.final_score = ml_matcher::blend_scores(evidence.rule_score, ml_score);
        evidence.requires_review = evidence.final_score >= rule.review_threshold
            && evidence.final_score < rule.auto_merge_threshold;

        if evidence.final_score >= rule.review_threshold {
            evidences.push(evidence);
        }
    }

    let resolution = graph_resolution::resolve_clusters(
        job.id,
        &records,
        &evidences,
        rule.review_threshold,
        rule.auto_merge_threshold,
    );
    let mut clusters = resolution.clusters;
    let review_items = resolution.review_items;
    let mut golden_records = Vec::<GoldenRecord>::new();

    for cluster in &mut clusters {
        if cluster.status == "rejected" {
            continue;
        }
        let golden_record = merge::synthesize_golden_record(cluster, &strategy);
        cluster.suggested_golden_record_id = Some(golden_record.id);
        golden_records.push(golden_record);
    }

    sqlx::query("DELETE FROM fusion_clusters WHERE job_id = $1")
        .bind(job.id)
        .execute(&state.db)
        .await
        .map_err(|cause| db_error(&cause))?;

    for cluster in &clusters {
        sqlx::query(
            r#"
            INSERT INTO fusion_clusters (
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
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            "#,
        )
        .bind(cluster.id)
        .bind(cluster.job_id)
        .bind(&cluster.cluster_key)
        .bind(&cluster.status)
        .bind(SqlJson(cluster.records.clone()))
        .bind(SqlJson(cluster.evidence.clone()))
        .bind(cluster.confidence_score)
        .bind(cluster.requires_review)
        .bind(cluster.suggested_golden_record_id)
        .bind(cluster.created_at)
        .bind(cluster.updated_at)
        .execute(&state.db)
        .await
        .map_err(|cause| db_error(&cause))?;
    }

    for golden_record in &golden_records {
        sqlx::query(
            r#"
            INSERT INTO fusion_golden_records (
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
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#,
        )
        .bind(golden_record.id)
        .bind(golden_record.cluster_id)
        .bind(&golden_record.title)
        .bind(SqlJson(golden_record.canonical_values.clone()))
        .bind(SqlJson(golden_record.provenance.clone()))
        .bind(golden_record.completeness_score)
        .bind(golden_record.confidence_score)
        .bind(&golden_record.status)
        .bind(golden_record.created_at)
        .bind(golden_record.updated_at)
        .execute(&state.db)
        .await
        .map_err(|cause| db_error(&cause))?;
    }

    for review_item in &review_items {
        sqlx::query(
            r#"
            INSERT INTO fusion_review_queue (
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
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            "#,
        )
        .bind(review_item.id)
        .bind(review_item.cluster_id)
        .bind(&review_item.status)
        .bind(&review_item.severity)
        .bind(&review_item.recommended_action)
        .bind(SqlJson(review_item.rationale.clone()))
        .bind(&review_item.assigned_to)
        .bind(&review_item.reviewed_by)
        .bind(&review_item.notes)
        .bind(review_item.created_at)
        .bind(review_item.updated_at)
        .execute(&state.db)
        .await
        .map_err(|cause| db_error(&cause))?;
    }

    let average_score = if evidences.is_empty() {
        0.0
    } else {
        evidences
            .iter()
            .map(|evidence| evidence.final_score)
            .sum::<f32>()
            / evidences.len() as f32
    };
    let metrics = FusionJobMetrics {
        candidate_pairs: candidate_pairs.len() as i32,
        matched_pairs: evidences
            .iter()
            .filter(|evidence| evidence.final_score >= rule.auto_merge_threshold)
            .count() as i32,
        review_pairs: evidences
            .iter()
            .filter(|evidence| evidence.requires_review)
            .count() as i32,
        cluster_count: clusters.len() as i32,
        golden_record_count: golden_records.len() as i32,
        precision_estimate: average_score.clamp(0.0, 1.0),
        recall_estimate: ((clusters
            .iter()
            .filter(|cluster| cluster.records.len() > 1)
            .count() as f32)
            / (records.len().max(1) as f32 / 2.0))
            .clamp(0.0, 1.0),
    };

    let updated_row = query_as::<_, FusionJobRow>(
        r#"
        UPDATE fusion_jobs
        SET status = $2,
            metrics = $3,
            last_run_summary = $4,
            last_run_at = NOW(),
            updated_at = NOW()
        WHERE id = $1
        RETURNING
            id,
            name,
            description,
            status,
            entity_type,
            match_rule_id,
            merge_strategy_id,
            config,
            metrics,
            last_run_summary,
            last_run_at,
            created_at,
            updated_at
        "#,
    )
    .bind(job.id)
    .bind(if review_items.is_empty() {
        "completed"
    } else {
        "awaiting_review"
    })
    .bind(SqlJson(metrics))
    .bind(format!(
        "Generated {} clusters, {} golden records, {} review items.",
        clusters.len(),
        golden_records.len(),
        review_items.len(),
    ))
    .fetch_one(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?;

    Ok(Json(RunResolutionJobResponse {
        job: updated_row.into(),
        cluster_ids: clusters.iter().map(|cluster| cluster.id).collect(),
        golden_record_ids: golden_records.iter().map(|record| record.id).collect(),
        review_queue_item_ids: review_items.iter().map(|item| item.id).collect(),
        executed_at: Utc::now(),
    }))
}
