use axum::{
    Json,
    extract::{Path, State},
};
use chrono::Utc;
use serde_json::{Value, json};
use sqlx::{FromRow, query_as};
use uuid::Uuid;

use crate::{
    AppState,
    domain::drift,
    models::deployment::{
        CreateDeploymentRequest, GenerateDriftReportRequest, ListDeploymentsResponse,
        ModelDeployment, TrafficSplitEntry, UpdateDeploymentRequest,
    },
};

use super::{
    ServiceResult, bad_request, db_error, deserialize_json, deserialize_optional_json, not_found,
    to_json,
};

#[derive(Debug, FromRow)]
struct DeploymentRow {
    id: Uuid,
    model_id: Uuid,
    name: String,
    status: String,
    strategy_type: String,
    endpoint_path: String,
    traffic_split: Value,
    monitoring_window: String,
    baseline_dataset_id: Option<Uuid>,
    drift_report: Option<Value>,
    created_at: chrono::DateTime<Utc>,
    updated_at: chrono::DateTime<Utc>,
}

fn to_deployment(row: DeploymentRow) -> ModelDeployment {
    ModelDeployment {
        id: row.id,
        model_id: row.model_id,
        name: row.name,
        status: row.status,
        strategy_type: row.strategy_type,
        endpoint_path: row.endpoint_path,
        traffic_split: deserialize_json(row.traffic_split),
        monitoring_window: row.monitoring_window,
        baseline_dataset_id: row.baseline_dataset_id,
        drift_report: deserialize_optional_json(row.drift_report),
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
}

async fn load_deployment_row(
    db: &sqlx::PgPool,
    deployment_id: Uuid,
) -> Result<Option<DeploymentRow>, sqlx::Error> {
    query_as::<_, DeploymentRow>(
        r#"
		SELECT
			id,
			model_id,
			name,
			status,
			strategy_type,
			endpoint_path,
			traffic_split,
			monitoring_window,
			baseline_dataset_id,
			drift_report,
			created_at,
			updated_at
		FROM ml_deployments
		WHERE id = $1
		"#,
    )
    .bind(deployment_id)
    .fetch_optional(db)
    .await
}

fn normalize_traffic_split(
    strategy_type: &str,
    mut splits: Vec<TrafficSplitEntry>,
) -> Result<Vec<TrafficSplitEntry>, String> {
    if splits.is_empty() {
        return Err("at least one traffic split entry is required".to_string());
    }

    for (index, split) in splits.iter_mut().enumerate() {
        if split.label.trim().is_empty() {
            split.label = format!("variant-{}", index + 1);
        }
    }

    if strategy_type != "ab_test" {
        let first = splits.remove(0);
        return Ok(vec![TrafficSplitEntry {
            allocation: 100,
            ..first
        }]);
    }

    let total: u32 = splits.iter().map(|entry| entry.allocation as u32).sum();
    if total == 0 {
        return Err("traffic allocation must be greater than zero".to_string());
    }

    let mut normalized = Vec::with_capacity(splits.len());
    let mut remaining = 100u32;
    let last_index = splits.len().saturating_sub(1);

    for (index, split) in splits.into_iter().enumerate() {
        let allocation = if index == last_index {
            remaining as u8
        } else {
            let scaled = ((split.allocation as f64 / total as f64) * 100.0).round() as u32;
            let bounded = scaled.min(remaining);
            remaining -= bounded;
            bounded as u8
        };

        normalized.push(TrafficSplitEntry {
            allocation,
            ..split
        });
    }

    Ok(normalized)
}

pub async fn list_deployments(
    State(state): State<AppState>,
) -> ServiceResult<ListDeploymentsResponse> {
    let rows = query_as::<_, DeploymentRow>(
        r#"
		SELECT
			id,
			model_id,
			name,
			status,
			strategy_type,
			endpoint_path,
			traffic_split,
			monitoring_window,
			baseline_dataset_id,
			drift_report,
			created_at,
			updated_at
		FROM ml_deployments
		ORDER BY updated_at DESC, created_at DESC
		"#,
    )
    .fetch_all(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?;

    Ok(Json(ListDeploymentsResponse {
        data: rows.into_iter().map(to_deployment).collect(),
    }))
}

pub async fn create_deployment(
    State(state): State<AppState>,
    Json(body): Json<CreateDeploymentRequest>,
) -> ServiceResult<ModelDeployment> {
    if body.name.trim().is_empty() || body.endpoint_path.trim().is_empty() {
        return Err(bad_request(
            "deployment name and endpoint path are required",
        ));
    }

    let traffic_split =
        normalize_traffic_split(&body.strategy_type, body.traffic_split).map_err(bad_request)?;
    let deployment_id = Uuid::now_v7();

    let row = query_as::<_, DeploymentRow>(
        r#"
		INSERT INTO ml_deployments (
			id,
			model_id,
			name,
			status,
			strategy_type,
			endpoint_path,
			traffic_split,
			monitoring_window,
			baseline_dataset_id,
			drift_report
		)
		VALUES ($1, $2, $3, 'active', $4, $5, $6, $7, $8, NULL)
		RETURNING
			id,
			model_id,
			name,
			status,
			strategy_type,
			endpoint_path,
			traffic_split,
			monitoring_window,
			baseline_dataset_id,
			drift_report,
			created_at,
			updated_at
		"#,
    )
    .bind(deployment_id)
    .bind(body.model_id)
    .bind(body.name.trim())
    .bind(body.strategy_type)
    .bind(body.endpoint_path.trim())
    .bind(to_json(&traffic_split))
    .bind(body.monitoring_window)
    .bind(body.baseline_dataset_id)
    .fetch_one(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?;

    sqlx::query("UPDATE ml_models SET active_deployment_id = $2, updated_at = NOW() WHERE id = $1")
        .bind(row.model_id)
        .bind(deployment_id)
        .execute(&state.db)
        .await
        .map_err(|cause| db_error(&cause))?;

    Ok(Json(to_deployment(row)))
}

pub async fn update_deployment(
    State(state): State<AppState>,
    Path(deployment_id): Path<Uuid>,
    Json(body): Json<UpdateDeploymentRequest>,
) -> ServiceResult<ModelDeployment> {
    let Some(current) = load_deployment_row(&state.db, deployment_id)
        .await
        .map_err(|cause| db_error(&cause))?
    else {
        return Err(not_found("deployment not found"));
    };

    let strategy_type = body
        .strategy_type
        .clone()
        .unwrap_or_else(|| current.strategy_type.clone());
    let current_split: Vec<TrafficSplitEntry> = deserialize_json(current.traffic_split.clone());
    let traffic_split =
        normalize_traffic_split(&strategy_type, body.traffic_split.unwrap_or(current_split))
            .map_err(bad_request)?;
    let status = body.status.unwrap_or_else(|| current.status.clone());

    let row = query_as::<_, DeploymentRow>(
        r#"
		UPDATE ml_deployments
		SET
			name = $2,
			status = $3,
			strategy_type = $4,
			endpoint_path = $5,
			traffic_split = $6,
			monitoring_window = $7,
			baseline_dataset_id = $8,
			updated_at = NOW()
		WHERE id = $1
		RETURNING
			id,
			model_id,
			name,
			status,
			strategy_type,
			endpoint_path,
			traffic_split,
			monitoring_window,
			baseline_dataset_id,
			drift_report,
			created_at,
			updated_at
		"#,
    )
    .bind(deployment_id)
    .bind(body.name.unwrap_or(current.name))
    .bind(status.clone())
    .bind(strategy_type)
    .bind(body.endpoint_path.unwrap_or(current.endpoint_path))
    .bind(to_json(&traffic_split))
    .bind(body.monitoring_window.unwrap_or(current.monitoring_window))
    .bind(body.baseline_dataset_id.or(current.baseline_dataset_id))
    .fetch_one(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?;

    if status == "active" {
        sqlx::query(
            "UPDATE ml_models SET active_deployment_id = $2, updated_at = NOW() WHERE id = $1",
        )
        .bind(row.model_id)
        .bind(deployment_id)
        .execute(&state.db)
        .await
        .map_err(|cause| db_error(&cause))?;
    } else {
        sqlx::query(
			"UPDATE ml_models SET active_deployment_id = NULL, updated_at = NOW() WHERE id = $1 AND active_deployment_id = $2",
		)
		.bind(row.model_id)
		.bind(deployment_id)
		.execute(&state.db)
		.await
		.map_err(|cause| db_error(&cause))?;
    }

    Ok(Json(to_deployment(row)))
}

pub async fn generate_drift_report(
    State(state): State<AppState>,
    Path(deployment_id): Path<Uuid>,
    Json(body): Json<GenerateDriftReportRequest>,
) -> ServiceResult<ModelDeployment> {
    let Some(current) = load_deployment_row(&state.db, deployment_id)
        .await
        .map_err(|cause| db_error(&cause))?
    else {
        return Err(not_found("deployment not found"));
    };

    let split: Vec<TrafficSplitEntry> = deserialize_json(current.traffic_split.clone());
    let mut report = drift::generate_report(&body, split.len());

    if report.recommend_retraining && body.auto_retrain {
        let job_id = Uuid::now_v7();
        sqlx::query(
            r#"
			INSERT INTO ml_training_jobs (
				id,
				experiment_id,
				model_id,
				name,
				status,
				dataset_ids,
				training_config,
				hyperparameter_search,
				objective_metric_name,
				trials,
				best_model_version_id,
				submitted_at,
				started_at,
				completed_at,
				created_at
			)
			VALUES ($1, NULL, $2, $3, 'queued', $4, $5, $6, $7, $8, NULL, $9, NULL, NULL, $9)
			"#,
        )
        .bind(job_id)
        .bind(current.model_id)
        .bind(format!("Auto retrain for {}", current.name))
        .bind(json!([]))
        .bind(json!({
            "trigger": "drift-monitor",
            "deployment_id": deployment_id,
            "endpoint_path": current.endpoint_path,
        }))
        .bind(json!({ "mode": "drift-triggered" }))
        .bind("drift_recovery")
        .bind(json!([]))
        .bind(Utc::now())
        .execute(&state.db)
        .await
        .map_err(|cause| db_error(&cause))?;

        report.auto_retraining_job_id = Some(job_id);
    }

    let row = query_as::<_, DeploymentRow>(
        r#"
		UPDATE ml_deployments
		SET
			drift_report = $2,
			updated_at = NOW()
		WHERE id = $1
		RETURNING
			id,
			model_id,
			name,
			status,
			strategy_type,
			endpoint_path,
			traffic_split,
			monitoring_window,
			baseline_dataset_id,
			drift_report,
			created_at,
			updated_at
		"#,
    )
    .bind(deployment_id)
    .bind(to_json(&report))
    .fetch_one(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?;

    Ok(Json(to_deployment(row)))
}
