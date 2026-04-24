use std::collections::HashMap;

use axum::{
    Json,
    extract::{Path, State},
};
use chrono::Utc;
use serde_json::Value;
use sqlx::{FromRow, query_as};
use uuid::Uuid;

use crate::{
    AppState,
    domain::predictions,
    models::{
        deployment::TrafficSplitEntry,
        prediction::{
            BatchPredictionJob, CreateBatchPredictionRequest, ListBatchPredictionsResponse,
            RealtimePredictionRequest, RealtimePredictionResponse,
        },
    },
};

use super::{ServiceResult, bad_request, db_error, deserialize_json, not_found, to_json};

#[derive(Debug, FromRow)]
struct DeploymentRow {
    traffic_split: Value,
}

#[derive(Debug, FromRow)]
struct ModelVersionRow {
    id: Uuid,
    version_number: i32,
}

#[derive(Debug, FromRow)]
struct BatchPredictionRow {
    id: Uuid,
    deployment_id: Uuid,
    status: String,
    record_count: i64,
    output_destination: Option<String>,
    outputs: Value,
    created_at: chrono::DateTime<Utc>,
    completed_at: Option<chrono::DateTime<Utc>>,
}

fn to_batch_job(row: BatchPredictionRow) -> BatchPredictionJob {
    BatchPredictionJob {
        id: row.id,
        deployment_id: row.deployment_id,
        status: row.status,
        record_count: row.record_count,
        output_destination: row.output_destination,
        outputs: deserialize_json(row.outputs),
        created_at: row.created_at,
        completed_at: row.completed_at,
    }
}

async fn load_deployment(
    db: &sqlx::PgPool,
    deployment_id: Uuid,
) -> Result<Option<DeploymentRow>, sqlx::Error> {
    query_as::<_, DeploymentRow>("SELECT traffic_split FROM ml_deployments WHERE id = $1")
        .bind(deployment_id)
        .fetch_optional(db)
        .await
}

async fn load_versions(
    db: &sqlx::PgPool,
    splits: &[TrafficSplitEntry],
) -> Result<HashMap<Uuid, i32>, sqlx::Error> {
    let mut versions = HashMap::new();
    for split in splits {
        if versions.contains_key(&split.model_version_id) {
            continue;
        }

        if let Some(row) = query_as::<_, ModelVersionRow>(
            "SELECT id, version_number FROM ml_model_versions WHERE id = $1",
        )
        .bind(split.model_version_id)
        .fetch_optional(db)
        .await?
        {
            versions.insert(row.id, row.version_number);
        }
    }

    Ok(versions)
}

pub async fn realtime_predict(
    State(state): State<AppState>,
    Path(deployment_id): Path<Uuid>,
    Json(body): Json<RealtimePredictionRequest>,
) -> ServiceResult<RealtimePredictionResponse> {
    if body.inputs.is_empty() {
        return Err(bad_request("prediction inputs are required"));
    }

    let Some(deployment) = load_deployment(&state.db, deployment_id)
        .await
        .map_err(|cause| db_error(&cause))?
    else {
        return Err(not_found("deployment not found"));
    };

    let splits: Vec<TrafficSplitEntry> = deserialize_json(deployment.traffic_split);
    if splits.is_empty() {
        return Err(bad_request("deployment has no traffic split configured"));
    }

    let version_map = load_versions(&state.db, &splits)
        .await
        .map_err(|cause| db_error(&cause))?;
    if version_map.is_empty() {
        return Err(not_found("deployment versions not found"));
    }

    let outputs = body
        .inputs
        .iter()
        .enumerate()
        .filter_map(|(index, input)| {
            let split = predictions::route_variant(&splits, index)?;
            let version_number = *version_map.get(&split.model_version_id)?;
            Some(predictions::predict_record(
                input,
                &split,
                version_number,
                body.explain,
                index,
            ))
        })
        .collect();

    Ok(Json(RealtimePredictionResponse {
        deployment_id,
        outputs,
        predicted_at: Utc::now(),
    }))
}

pub async fn list_batch_predictions(
    State(state): State<AppState>,
) -> ServiceResult<ListBatchPredictionsResponse> {
    let rows = query_as::<_, BatchPredictionRow>(
        r#"
		SELECT
			id,
			deployment_id,
			status,
			record_count,
			output_destination,
			outputs,
			created_at,
			completed_at
		FROM ml_batch_predictions
		ORDER BY created_at DESC
		"#,
    )
    .fetch_all(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?;

    Ok(Json(ListBatchPredictionsResponse {
        data: rows.into_iter().map(to_batch_job).collect(),
    }))
}

pub async fn create_batch_prediction(
    State(state): State<AppState>,
    Json(body): Json<CreateBatchPredictionRequest>,
) -> ServiceResult<BatchPredictionJob> {
    if body.records.is_empty() {
        return Err(bad_request("batch prediction records are required"));
    }

    let Some(deployment) = load_deployment(&state.db, body.deployment_id)
        .await
        .map_err(|cause| db_error(&cause))?
    else {
        return Err(not_found("deployment not found"));
    };

    let splits: Vec<TrafficSplitEntry> = deserialize_json(deployment.traffic_split);
    if splits.is_empty() {
        return Err(bad_request("deployment has no traffic split configured"));
    }

    let version_map = load_versions(&state.db, &splits)
        .await
        .map_err(|cause| db_error(&cause))?;
    if version_map.is_empty() {
        return Err(not_found("deployment versions not found"));
    }

    let outputs = body
        .records
        .iter()
        .enumerate()
        .filter_map(|(index, input)| {
            let split = predictions::route_variant(&splits, index)?;
            let version_number = *version_map.get(&split.model_version_id)?;
            Some(predictions::predict_record(
                input,
                &split,
                version_number,
                true,
                index,
            ))
        })
        .collect::<Vec<_>>();

    let row = query_as::<_, BatchPredictionRow>(
        r#"
		INSERT INTO ml_batch_predictions (
			id,
			deployment_id,
			status,
			record_count,
			output_destination,
			outputs,
			created_at,
			completed_at
		)
		VALUES ($1, $2, 'completed', $3, $4, $5, $6, $7)
		RETURNING
			id,
			deployment_id,
			status,
			record_count,
			output_destination,
			outputs,
			created_at,
			completed_at
		"#,
    )
    .bind(Uuid::now_v7())
    .bind(body.deployment_id)
    .bind(outputs.len() as i64)
    .bind(body.output_destination)
    .bind(to_json(&outputs))
    .bind(Utc::now())
    .bind(Some(Utc::now()))
    .fetch_one(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?;

    Ok(Json(to_batch_job(row)))
}
