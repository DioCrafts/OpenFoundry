use axum::extract::State;
use sqlx::query_scalar;

use crate::{AppState, models::MlStudioOverview};

use super::{ServiceResult, db_error};

pub async fn get_overview(State(state): State<AppState>) -> ServiceResult<MlStudioOverview> {
    let experiment_count = query_scalar::<_, i64>("SELECT COUNT(*) FROM ml_experiments")
        .fetch_one(&state.db)
        .await
        .map_err(|cause| db_error(&cause))?;

    let active_run_count = query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM ml_runs WHERE status IN ('queued', 'running', 'completed')",
    )
    .fetch_one(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?;

    let model_count = query_scalar::<_, i64>("SELECT COUNT(*) FROM ml_models")
        .fetch_one(&state.db)
        .await
        .map_err(|cause| db_error(&cause))?;

    let production_model_count =
        query_scalar::<_, i64>("SELECT COUNT(*) FROM ml_model_versions WHERE stage = 'production'")
            .fetch_one(&state.db)
            .await
            .map_err(|cause| db_error(&cause))?;

    let feature_count = query_scalar::<_, i64>("SELECT COUNT(*) FROM ml_features")
        .fetch_one(&state.db)
        .await
        .map_err(|cause| db_error(&cause))?;

    let online_feature_count =
        query_scalar::<_, i64>("SELECT COUNT(*) FROM ml_features WHERE online_enabled = TRUE")
            .fetch_one(&state.db)
            .await
            .map_err(|cause| db_error(&cause))?;

    let deployment_count = query_scalar::<_, i64>("SELECT COUNT(*) FROM ml_deployments")
        .fetch_one(&state.db)
        .await
        .map_err(|cause| db_error(&cause))?;

    let ab_test_count = query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM ml_deployments WHERE strategy_type = 'ab_test'",
    )
    .fetch_one(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?;

    let drift_alert_count = query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM ml_deployments WHERE COALESCE((drift_report->>'recommend_retraining')::boolean, FALSE)",
    )
    .fetch_one(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?;

    let queued_training_jobs = query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM ml_training_jobs WHERE status IN ('queued', 'running')",
    )
    .fetch_one(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?;

    Ok(axum::Json(MlStudioOverview {
        experiment_count,
        active_run_count,
        model_count,
        production_model_count,
        feature_count,
        online_feature_count,
        deployment_count,
        ab_test_count,
        drift_alert_count,
        queued_training_jobs,
    }))
}
