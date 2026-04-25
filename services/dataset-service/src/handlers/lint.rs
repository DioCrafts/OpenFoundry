use std::cmp::Reverse;

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use chrono::{DateTime, Duration, Utc};
use storage_abstraction::backend::ObjectMeta;
use uuid::Uuid;

use crate::{
    AppState,
    models::{
        branch::DatasetBranch,
        dataset::Dataset,
        lint::{
            DatasetLintFinding, DatasetLintRecommendation, DatasetLintResponse, DatasetLintSummary,
        },
        quality::DatasetProfileRecord,
        transaction::DatasetTransaction,
        version::DatasetVersion,
        view::DatasetView,
    },
};

const LARGE_DATASET_BYTES: i64 = 100 * 1024 * 1024;
const HUGE_DATASET_BYTES: i64 = 1024 * 1024 * 1024;
const LARGE_DATASET_ROWS: i64 = 500_000;
const SMALL_FILE_THRESHOLD_BYTES: i64 = 8 * 1024 * 1024;
const VERSION_SPRAWL_THRESHOLD: usize = 12;
const VERSION_SPRAWL_HIGH_THRESHOLD: usize = 24;
const STALE_BRANCH_DAYS: i64 = 14;
const INACTIVE_DATASET_DAYS: i64 = 45;

#[derive(Debug, Clone, Copy, Default)]
struct StorageLayoutStats {
    object_count: usize,
    small_file_count: usize,
    total_object_bytes: i64,
    largest_object_bytes: i64,
    average_object_size_bytes: i64,
}

/// GET /api/v1/datasets/:id/lint
pub async fn get_dataset_lint(
    State(state): State<AppState>,
    Path(dataset_id): Path<Uuid>,
) -> impl IntoResponse {
    let dataset = match load_dataset(&state, dataset_id).await {
        Ok(Some(dataset)) => dataset,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(error) => {
            tracing::error!("dataset lint lookup failed: {error}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    match build_lint_response(&state, &dataset).await {
        Ok(response) => Json(response).into_response(),
        Err(error) => {
            tracing::error!("dataset lint failed: {error}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

async fn build_lint_response(
    state: &AppState,
    dataset: &Dataset,
) -> Result<DatasetLintResponse, sqlx::Error> {
    let versions = sqlx::query_as::<_, DatasetVersion>(
        "SELECT * FROM dataset_versions WHERE dataset_id = $1 ORDER BY version DESC",
    )
    .bind(dataset.id)
    .fetch_all(&state.db)
    .await?;

    let branches = sqlx::query_as::<_, DatasetBranch>(
        "SELECT * FROM dataset_branches WHERE dataset_id = $1 ORDER BY is_default DESC, updated_at DESC",
    )
    .bind(dataset.id)
    .fetch_all(&state.db)
    .await?;

    let views = sqlx::query_as::<_, DatasetView>(
        "SELECT * FROM dataset_views WHERE dataset_id = $1 ORDER BY created_at DESC",
    )
    .bind(dataset.id)
    .fetch_all(&state.db)
    .await?;

    let transactions = sqlx::query_as::<_, DatasetTransaction>(
        "SELECT * FROM dataset_transactions WHERE dataset_id = $1 ORDER BY created_at DESC LIMIT 200",
    )
    .bind(dataset.id)
    .fetch_all(&state.db)
    .await?;

    let profile = sqlx::query_as::<_, DatasetProfileRecord>(
        "SELECT profile, score, profiled_at FROM dataset_profiles WHERE dataset_id = $1",
    )
    .bind(dataset.id)
    .fetch_optional(&state.db)
    .await?;

    let enabled_rule_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM dataset_quality_rules WHERE dataset_id = $1 AND enabled = TRUE",
    )
    .bind(dataset.id)
    .fetch_one(&state.db)
    .await? as usize;

    let active_alert_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM dataset_quality_alerts WHERE dataset_id = $1 AND status = 'active'",
    )
    .bind(dataset.id)
    .fetch_one(&state.db)
    .await? as usize;

    let storage_stats = match state.storage.list(&dataset.storage_path).await {
        Ok(objects) => analyze_storage_layout(&objects),
        Err(error) => {
            tracing::warn!(
                "dataset lint storage listing failed for {}: {error}",
                dataset.id
            );
            StorageLayoutStats::default()
        }
    };

    Ok(analyze_dataset(
        dataset,
        &versions,
        &branches,
        &views,
        &transactions,
        profile.as_ref(),
        enabled_rule_count,
        active_alert_count,
        storage_stats,
        Utc::now(),
    ))
}

fn analyze_dataset(
    dataset: &Dataset,
    versions: &[DatasetVersion],
    branches: &[DatasetBranch],
    views: &[DatasetView],
    transactions: &[DatasetTransaction],
    profile: Option<&DatasetProfileRecord>,
    enabled_rule_count: usize,
    active_alert_count: usize,
    storage_stats: StorageLayoutStats,
    now: DateTime<Utc>,
) -> DatasetLintResponse {
    let tracked_versions = versions.len().max(dataset.current_version.max(1) as usize);
    let materialized_view_count = views.iter().filter(|view| view.materialized).count();
    let auto_refresh_view_count = views
        .iter()
        .filter(|view| view.materialized && view.refresh_on_source_update)
        .count();
    let stale_branches = branches
        .iter()
        .filter(|branch| {
            !branch.is_default
                && branch.name != dataset.active_branch
                && now.signed_duration_since(branch.updated_at) > Duration::days(STALE_BRANCH_DAYS)
        })
        .collect::<Vec<_>>();
    let failed_transaction_count = transactions
        .iter()
        .filter(|transaction| is_failed_status(&transaction.status))
        .count();
    let pending_transaction_count = transactions
        .iter()
        .filter(|transaction| is_pending_status(&transaction.status))
        .count();
    let quality_score = profile.map(|record| record.score);
    let has_quality_profile = profile.is_some();
    let has_data =
        dataset.size_bytes > 0 || dataset.row_count > 0 || storage_stats.object_count > 0;
    let is_large_dataset =
        dataset.size_bytes >= LARGE_DATASET_BYTES || dataset.row_count >= LARGE_DATASET_ROWS;
    let is_sensitive = is_sensitive_dataset(&dataset.tags);

    let mut findings = Vec::new();

    if has_data && uses_row_oriented_format(&dataset.format) && is_large_dataset {
        let severity = if dataset.size_bytes >= HUGE_DATASET_BYTES {
            "high"
        } else {
            "medium"
        };
        findings.push(finding(
            "large-row-format",
            "Large dataset stored in a row-oriented format",
            severity,
            "storage",
            format!(
                "{} datasets at this size are expensive to scan and rewrite as the primary filesystem artifact.",
                dataset.format.to_uppercase()
            ),
            vec![
                format!("format={}", dataset.format),
                format!("size={} MB", dataset.size_bytes / (1024 * 1024)),
                format!("rows={}", dataset.row_count),
            ],
            "Read amplification and rewrite costs will grow with every enrollment or downstream refresh.",
            "Store the canonical dataset in Parquet-like columnar files and keep JSON/CSV exports as derived outputs.",
        ));
    }

    if tracked_versions >= VERSION_SPRAWL_THRESHOLD {
        let severity = if tracked_versions >= VERSION_SPRAWL_HIGH_THRESHOLD {
            "high"
        } else {
            "medium"
        };
        findings.push(finding(
            "version-sprawl",
            "Version sprawl is increasing storage overhead",
            severity,
            "lifecycle",
            "The dataset is retaining many tracked versions without an obvious pruning or archive policy.".to_string(),
            vec![
                format!("tracked_versions={tracked_versions}"),
                format!("current_version={}", dataset.current_version),
            ],
            "Storage footprint and branch maintenance keep growing even when older versions are no longer hot.",
            "Prune cold versions, archive historical snapshots, or promote a retention policy per dataset tier.",
        ));
    }

    if stale_branches.len() >= 2 || branches.len() >= 6 {
        let severity = if stale_branches.len() >= 4 || branches.len() >= 8 {
            "high"
        } else {
            "medium"
        };
        findings.push(finding(
            "stale-branches",
            "Branch fan-out is creating stale storage and merge drift",
            severity,
            "lifecycle",
            "Several dataset branches look stale relative to the active branch and the default branch.".to_string(),
            stale_branches
                .iter()
                .take(4)
                .map(|branch| {
                    format!(
                        "{} last updated {} days ago at v{}",
                        branch.name,
                        now.signed_duration_since(branch.updated_at).num_days(),
                        branch.version
                    )
                })
                .chain(std::iter::once(format!("branch_count={}", branches.len())))
                .collect(),
            "Old branches dilute ownership and make storage, promotion, and merge flows more expensive to reason about.",
            "Promote or merge branches that are still useful, then retire stale ones and reset long-lived feature branches.",
        ));
    }

    if materialized_view_count >= 4 || auto_refresh_view_count >= 2 {
        let severity = if materialized_view_count >= 6 || auto_refresh_view_count >= 3 {
            "high"
        } else {
            "medium"
        };
        findings.push(finding(
            "materialization-sprawl",
            "Materialized views are amplifying compute and storage",
            severity,
            "compute",
            "This dataset is backing several materialized views, and some of them refresh automatically on source updates.".to_string(),
            vec![
                format!("materialized_views={materialized_view_count}"),
                format!("auto_refresh_views={auto_refresh_view_count}"),
            ],
            "Every upstream write now fans out into extra refresh work and more files to maintain.",
            "Demote low-value views to logical views, or disable auto-refresh where freshness is not required.",
        ));
    }

    if has_data && !has_quality_profile && (is_large_dataset || is_sensitive) {
        let severity = if is_sensitive { "high" } else { "medium" };
        findings.push(finding(
            "missing-quality-profile",
            "No quality profile exists for a meaningful dataset",
            severity,
            "quality",
            "The dataset already has data, but profiling has not been generated yet.".to_string(),
            vec![
                format!("size={} MB", dataset.size_bytes / (1024 * 1024)),
                format!("rows={}", dataset.row_count),
            ],
            "Without a profile you are flying blind on completeness, uniqueness, duplicates, and downstream trust.",
            "Generate a quality profile so enrollment anti-patterns become measurable and trendable over time.",
        ));
    }

    if has_data && enabled_rule_count == 0 && (is_large_dataset || is_sensitive) {
        let severity = if is_sensitive { "high" } else { "medium" };
        findings.push(finding(
            "missing-quality-rules",
            "Quality controls are missing for an important dataset",
            severity,
            "quality",
            "The dataset has enough scale or sensitivity to justify guardrails, but no enabled quality rules are configured.".to_string(),
            vec![
                format!("enabled_rules={enabled_rule_count}"),
                format!("sensitive={is_sensitive}"),
            ],
            "Bad enrollments can land in the canonical filesystem without an automated tripwire.",
            "Add null, range, regex, or custom SQL rules for the highest-risk columns before the next refresh cycle.",
        ));
    }

    if quality_score.is_some_and(|score| score < 80.0) || active_alert_count > 0 {
        let severity = if quality_score.is_some_and(|score| score < 60.0) || active_alert_count >= 3
        {
            "high"
        } else {
            "medium"
        };
        findings.push(finding(
            "quality-regression",
            "Current quality signals indicate drift or instability",
            severity,
            "quality",
            "The latest profiling state already shows enough signal that the dataset should be reviewed before more consumers depend on it.".to_string(),
            vec![
                format!(
                    "quality_score={}",
                    quality_score
                        .map(|value| format!("{value:.1}"))
                        .unwrap_or_else(|| "n/a".to_string())
                ),
                format!("active_alerts={active_alert_count}"),
            ],
            "Low score or unresolved alerts tend to multiply rework across downstream branches and views.",
            "Resolve active alerts and tighten the failing rule set before adding more derived artifacts.",
        ));
    }

    if failed_transaction_count > 0
        || pending_transaction_count >= 3
        || (transactions.len() >= 25 && materialized_view_count > 0)
    {
        let severity = if failed_transaction_count > 0 || pending_transaction_count >= 6 {
            "high"
        } else {
            "medium"
        };
        findings.push(finding(
            "transaction-churn",
            "Transaction churn suggests enrollment instability",
            severity,
            "compute",
            "Recent dataset transactions show retries, backlog, or enough write activity to justify optimization.".to_string(),
            vec![
                format!("recent_transactions={}", transactions.len()),
                format!("failed_transactions={failed_transaction_count}"),
                format!("pending_transactions={pending_transaction_count}"),
            ],
            "Extra churn increases the chance of partial refreshes, view fan-out, and wasted compute.",
            "Inspect the noisy transaction paths, batch writes where possible, and reduce auto-refresh pressure on dependent views.",
        ));
    }

    if storage_stats.object_count >= 25
        && storage_stats.total_object_bytes >= LARGE_DATASET_BYTES
        && storage_stats.small_file_count * 100 / storage_stats.object_count >= 60
    {
        let severity = if storage_stats.small_file_count >= 80 {
            "high"
        } else {
            "medium"
        };
        findings.push(finding(
            "small-files",
            "Filesystem layout is fragmented into many small files",
            severity,
            "storage",
            "The dataset storage tree contains a high percentage of small objects, which is a classic lakehouse anti-pattern.".to_string(),
            vec![
                format!("object_count={}", storage_stats.object_count),
                format!("small_files={}", storage_stats.small_file_count),
                format!(
                    "avg_object_size_mb={:.1}",
                    storage_stats.average_object_size_bytes as f64 / (1024.0 * 1024.0)
                ),
            ],
            "Readers pay the cost of object enumeration and metadata overhead before meaningful scan work even starts.",
            "Compact small files into fewer larger objects and align partitioning with your dominant access pattern.",
        ));
    }

    if storage_stats.object_count == 1 && storage_stats.largest_object_bytes >= 256 * 1024 * 1024 {
        findings.push(finding(
            "single-large-object",
            "Dataset is concentrated in a single large object",
            "medium",
            "storage",
            "A single large file limits parallel reads and makes partial rewrites expensive.".to_string(),
            vec![format!(
                "largest_object_mb={}",
                storage_stats.largest_object_bytes / (1024 * 1024)
            )],
            "Enrollment and downstream preview jobs will struggle to scale horizontally.",
            "Split the dataset into partition-aligned objects so reads and rewrites can operate incrementally.",
        ));
    }

    if now.signed_duration_since(dataset.updated_at) > Duration::days(INACTIVE_DATASET_DAYS)
        && (tracked_versions >= 8 || materialized_view_count >= 2)
    {
        findings.push(finding(
            "cold-footprint",
            "Cold dataset footprint is still expensive",
            "low",
            "lifecycle",
            "The dataset has been quiet for a while, but it still carries versions or materialized artifacts that consume space.".to_string(),
            vec![
                format!(
                    "days_since_update={}",
                    now.signed_duration_since(dataset.updated_at).num_days()
                ),
                format!("tracked_versions={tracked_versions}"),
                format!("materialized_views={materialized_view_count}"),
            ],
            "Inactive storage silently accumulates cost and cognitive overhead for future owners.",
            "Archive dormant versions and keep only the views that still have active consumers.",
        ));
    }

    findings.sort_by_key(|item| Reverse(severity_rank(&item.severity)));

    let recommendations = findings.iter().map(to_recommendation).collect::<Vec<_>>();

    let high_severity = findings
        .iter()
        .filter(|finding| finding.severity == "high")
        .count();
    let medium_severity = findings
        .iter()
        .filter(|finding| finding.severity == "medium")
        .count();
    let low_severity = findings
        .iter()
        .filter(|finding| finding.severity == "low")
        .count();

    DatasetLintResponse {
        dataset_id: dataset.id,
        dataset_name: dataset.name.clone(),
        analyzed_at: now,
        summary: DatasetLintSummary {
            resource_posture: resource_posture(&findings).to_string(),
            total_findings: findings.len(),
            high_severity,
            medium_severity,
            low_severity,
            tracked_versions,
            branch_count: branches.len(),
            stale_branch_count: stale_branches.len(),
            materialized_view_count,
            auto_refresh_view_count,
            transaction_count: transactions.len(),
            failed_transaction_count,
            pending_transaction_count,
            enabled_rule_count,
            active_alert_count,
            object_count: storage_stats.object_count,
            small_file_count: storage_stats.small_file_count,
            largest_object_bytes: storage_stats.largest_object_bytes,
            average_object_size_bytes: storage_stats.average_object_size_bytes,
            quality_score,
        },
        findings,
        recommendations,
    }
}

fn analyze_storage_layout(objects: &[ObjectMeta]) -> StorageLayoutStats {
    if objects.is_empty() {
        return StorageLayoutStats::default();
    }

    let total_object_bytes = objects.iter().map(|object| object.size as i64).sum::<i64>();
    let largest_object_bytes = objects
        .iter()
        .map(|object| object.size as i64)
        .max()
        .unwrap_or_default();

    StorageLayoutStats {
        object_count: objects.len(),
        small_file_count: objects
            .iter()
            .filter(|object| (object.size as i64) < SMALL_FILE_THRESHOLD_BYTES)
            .count(),
        total_object_bytes,
        largest_object_bytes,
        average_object_size_bytes: total_object_bytes / objects.len() as i64,
    }
}

fn finding(
    code: &str,
    title: &str,
    severity: &str,
    category: &str,
    description: String,
    evidence: Vec<String>,
    impact: &str,
    recommendation: &str,
) -> DatasetLintFinding {
    DatasetLintFinding {
        code: code.to_string(),
        title: title.to_string(),
        severity: severity.to_string(),
        category: category.to_string(),
        description,
        evidence,
        impact: impact.to_string(),
        recommendation: recommendation.to_string(),
    }
}

fn to_recommendation(finding: &DatasetLintFinding) -> DatasetLintRecommendation {
    DatasetLintRecommendation {
        code: finding.code.clone(),
        priority: finding.severity.clone(),
        title: finding.title.clone(),
        rationale: finding.impact.clone(),
        actions: match finding.code.as_str() {
            "large-row-format" => vec![
                "Promote a columnar canonical copy for the dataset.".to_string(),
                "Keep JSON or CSV only as edge export formats.".to_string(),
            ],
            "version-sprawl" => vec![
                "Define a retention window for hot versions.".to_string(),
                "Archive or prune cold snapshots after promotion.".to_string(),
            ],
            "stale-branches" => vec![
                "Merge or promote branches that still matter.".to_string(),
                "Delete long-lived branches without active owners.".to_string(),
            ],
            "materialization-sprawl" => vec![
                "Convert low-value materialized views into logical views.".to_string(),
                "Disable auto-refresh on consumers that do not need it.".to_string(),
            ],
            "missing-quality-profile" => vec![
                "Run profiling on the current branch.".to_string(),
                "Use the profile as the baseline for future enrollments.".to_string(),
            ],
            "missing-quality-rules" => vec![
                "Add null, range, regex, or SQL checks for critical columns.".to_string(),
                "Gate refreshes on those rules before widening access.".to_string(),
            ],
            "quality-regression" => vec![
                "Resolve active quality alerts.".to_string(),
                "Tighten or fix the failing data path before adding more views.".to_string(),
            ],
            "transaction-churn" => vec![
                "Batch noisy writes or reduce retry loops.".to_string(),
                "Inspect whether auto-refreshed views are amplifying the workload.".to_string(),
            ],
            "small-files" => vec![
                "Compact fragmented objects into larger files.".to_string(),
                "Revisit partitioning so new enrollments do not recreate the same pattern."
                    .to_string(),
            ],
            "single-large-object" => vec![
                "Split the object into partition-sized chunks.".to_string(),
                "Align partitions with the dominant read path.".to_string(),
            ],
            "cold-footprint" => vec![
                "Archive dormant versions to colder storage.".to_string(),
                "Retain only the derived artifacts that still have consumers.".to_string(),
            ],
            _ => vec![finding.recommendation.clone()],
        },
    }
}

fn resource_posture(findings: &[DatasetLintFinding]) -> &'static str {
    let high = findings
        .iter()
        .filter(|finding| finding.severity == "high")
        .count();
    let medium = findings
        .iter()
        .filter(|finding| finding.severity == "medium")
        .count();

    if high >= 2 {
        "critical"
    } else if high == 1 || medium >= 3 {
        "optimize"
    } else if medium >= 1 || !findings.is_empty() {
        "watch"
    } else {
        "healthy"
    }
}

fn severity_rank(severity: &str) -> u8 {
    match severity {
        "high" => 3,
        "medium" => 2,
        "low" => 1,
        _ => 0,
    }
}

fn uses_row_oriented_format(format: &str) -> bool {
    matches!(format.to_ascii_lowercase().as_str(), "json" | "csv")
}

fn is_sensitive_dataset(tags: &[String]) -> bool {
    tags.iter().any(|tag| {
        matches!(
            tag.to_ascii_lowercase().as_str(),
            "pii" | "phi" | "sensitive" | "confidential" | "regulated" | "restricted"
        )
    })
}

fn is_failed_status(status: &str) -> bool {
    matches!(
        status.to_ascii_lowercase().as_str(),
        "failed" | "error" | "cancelled"
    )
}

fn is_pending_status(status: &str) -> bool {
    matches!(
        status.to_ascii_lowercase().as_str(),
        "pending" | "running" | "queued" | "processing"
    )
}

async fn load_dataset(state: &AppState, dataset_id: Uuid) -> Result<Option<Dataset>, sqlx::Error> {
    sqlx::query_as::<_, Dataset>("SELECT * FROM datasets WHERE id = $1")
        .bind(dataset_id)
        .fetch_optional(&state.db)
        .await
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, Utc};
    use serde_json::json;
    use uuid::Uuid;

    use crate::models::{
        branch::DatasetBranch, dataset::Dataset, quality::DatasetProfileRecord,
        transaction::DatasetTransaction, version::DatasetVersion, view::DatasetView,
    };

    use super::{StorageLayoutStats, analyze_dataset};

    fn sample_dataset() -> Dataset {
        Dataset {
            id: Uuid::nil(),
            name: "sales".to_string(),
            description: String::new(),
            format: "json".to_string(),
            storage_path: "datasets/sales".to_string(),
            size_bytes: 600 * 1024 * 1024,
            row_count: 2_400_000,
            owner_id: Uuid::nil(),
            tags: vec!["finance".to_string(), "pii".to_string()],
            current_version: 18,
            active_branch: "main".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn sample_version(dataset_id: Uuid, version: i32) -> DatasetVersion {
        DatasetVersion {
            id: Uuid::now_v7(),
            dataset_id,
            version,
            message: format!("v{version}"),
            size_bytes: 10,
            row_count: 10,
            storage_path: format!("datasets/sales/v{version}"),
            transaction_id: None,
            created_at: Utc::now(),
        }
    }

    fn sample_branch(
        dataset_id: Uuid,
        name: &str,
        version: i32,
        updated_days_ago: i64,
        is_default: bool,
    ) -> DatasetBranch {
        DatasetBranch {
            id: Uuid::now_v7(),
            dataset_id,
            name: name.to_string(),
            version,
            base_version: version - 1,
            description: String::new(),
            is_default,
            created_at: Utc::now(),
            updated_at: Utc::now() - Duration::days(updated_days_ago),
        }
    }

    fn sample_view(dataset_id: Uuid, index: usize, refresh_on_source_update: bool) -> DatasetView {
        DatasetView {
            id: Uuid::now_v7(),
            dataset_id,
            name: format!("view_{index}"),
            description: String::new(),
            sql_text: "SELECT * FROM dataset".to_string(),
            source_branch: Some("main".to_string()),
            source_version: Some(1),
            materialized: true,
            refresh_on_source_update,
            format: "json".to_string(),
            current_version: 1,
            storage_path: Some(format!("datasets/sales/views/{index}/v1.json")),
            row_count: 100,
            schema_fields: json!([]),
            last_refreshed_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn sample_transaction(dataset_id: Uuid, status: &str) -> DatasetTransaction {
        DatasetTransaction {
            id: Uuid::now_v7(),
            dataset_id,
            view_id: None,
            operation: "refresh".to_string(),
            branch_name: Some("main".to_string()),
            status: status.to_string(),
            summary: String::new(),
            metadata: json!({}),
            created_at: Utc::now(),
            committed_at: None,
        }
    }

    #[test]
    fn flags_storage_lifecycle_and_quality_gaps() {
        let dataset = sample_dataset();
        let versions = (1..=18)
            .map(|version| sample_version(dataset.id, version))
            .collect::<Vec<_>>();
        let branches = vec![
            sample_branch(dataset.id, "main", 18, 0, true),
            sample_branch(dataset.id, "feature-a", 12, 21, false),
            sample_branch(dataset.id, "feature-b", 10, 30, false),
        ];
        let views = vec![
            sample_view(dataset.id, 1, true),
            sample_view(dataset.id, 2, true),
            sample_view(dataset.id, 3, false),
            sample_view(dataset.id, 4, false),
        ];
        let transactions = vec![
            sample_transaction(dataset.id, "failed"),
            sample_transaction(dataset.id, "pending"),
            sample_transaction(dataset.id, "running"),
        ];

        let response = analyze_dataset(
            &dataset,
            &versions,
            &branches,
            &views,
            &transactions,
            None,
            0,
            2,
            StorageLayoutStats {
                object_count: 40,
                small_file_count: 30,
                total_object_bytes: 500 * 1024 * 1024,
                largest_object_bytes: 32 * 1024 * 1024,
                average_object_size_bytes: 12 * 1024 * 1024,
            },
            Utc::now(),
        );

        assert!(
            response
                .findings
                .iter()
                .any(|finding| finding.code == "large-row-format")
        );
        assert!(
            response
                .findings
                .iter()
                .any(|finding| finding.code == "version-sprawl")
        );
        assert!(
            response
                .findings
                .iter()
                .any(|finding| finding.code == "stale-branches")
        );
        assert!(
            response
                .findings
                .iter()
                .any(|finding| finding.code == "materialization-sprawl")
        );
        assert!(
            response
                .findings
                .iter()
                .any(|finding| finding.code == "missing-quality-profile")
        );
        assert!(
            response
                .findings
                .iter()
                .any(|finding| finding.code == "missing-quality-rules")
        );
        assert!(
            response
                .findings
                .iter()
                .any(|finding| finding.code == "quality-regression")
        );
        assert!(
            response
                .findings
                .iter()
                .any(|finding| finding.code == "transaction-churn")
        );
        assert!(
            response
                .findings
                .iter()
                .any(|finding| finding.code == "small-files")
        );
        assert_eq!(response.summary.resource_posture, "critical");
    }

    #[test]
    fn flags_large_single_object_layout() {
        let mut dataset = sample_dataset();
        dataset.format = "parquet".to_string();
        dataset.size_bytes = 400 * 1024 * 1024;
        dataset.row_count = 1_000_000;
        dataset.tags = vec!["finance".to_string()];

        let response = analyze_dataset(
            &dataset,
            &[sample_version(dataset.id, 1)],
            &[sample_branch(dataset.id, "main", 1, 0, true)],
            &[],
            &[],
            Some(&DatasetProfileRecord {
                profile: json!({}),
                score: 94.0,
                profiled_at: Utc::now(),
            }),
            2,
            0,
            StorageLayoutStats {
                object_count: 1,
                small_file_count: 0,
                total_object_bytes: 400 * 1024 * 1024,
                largest_object_bytes: 400 * 1024 * 1024,
                average_object_size_bytes: 400 * 1024 * 1024,
            },
            Utc::now(),
        );

        assert!(
            response
                .findings
                .iter()
                .any(|finding| finding.code == "single-large-object")
        );
    }

    #[test]
    fn healthy_dataset_produces_no_findings() {
        let mut dataset = sample_dataset();
        dataset.format = "parquet".to_string();
        dataset.size_bytes = 16 * 1024 * 1024;
        dataset.row_count = 20_000;
        dataset.tags = vec!["finance".to_string()];
        dataset.current_version = 2;

        let response = analyze_dataset(
            &dataset,
            &[sample_version(dataset.id, 1), sample_version(dataset.id, 2)],
            &[sample_branch(dataset.id, "main", 2, 0, true)],
            &[],
            &[],
            Some(&DatasetProfileRecord {
                profile: json!({}),
                score: 97.5,
                profiled_at: Utc::now(),
            }),
            2,
            0,
            StorageLayoutStats {
                object_count: 2,
                small_file_count: 0,
                total_object_bytes: 16 * 1024 * 1024,
                largest_object_bytes: 10 * 1024 * 1024,
                average_object_size_bytes: 8 * 1024 * 1024,
            },
            Utc::now(),
        );

        assert!(response.findings.is_empty());
        assert_eq!(response.summary.resource_posture, "healthy");
    }
}
