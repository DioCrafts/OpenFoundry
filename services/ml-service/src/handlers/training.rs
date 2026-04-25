use axum::{Json, extract::State};
use chrono::Utc;
use serde_json::{Value, json};
use sqlx::{FromRow, query_as, query_scalar};
use uuid::Uuid;

use crate::{
    AppState,
    domain::training,
    models::{
        model_version::ModelVersion,
        training_job::{CreateTrainingJobRequest, ListTrainingJobsResponse, TrainingJob},
    },
};

use super::{ServiceResult, bad_request, db_error, deserialize_json, to_json};

#[derive(Debug, FromRow)]
struct TrainingJobRow {
    id: Uuid,
    experiment_id: Option<Uuid>,
    model_id: Option<Uuid>,
    name: String,
    status: String,
    dataset_ids: Value,
    training_config: Value,
    hyperparameter_search: Value,
    objective_metric_name: String,
    trials: Value,
    best_model_version_id: Option<Uuid>,
    submitted_at: chrono::DateTime<Utc>,
    started_at: Option<chrono::DateTime<Utc>>,
    completed_at: Option<chrono::DateTime<Utc>>,
    created_at: chrono::DateTime<Utc>,
}

#[derive(Debug, FromRow)]
struct ModelVersionRow {
    id: Uuid,
    model_id: Uuid,
    version_number: i32,
    version_label: String,
    stage: String,
    source_run_id: Option<Uuid>,
    training_job_id: Option<Uuid>,
    hyperparameters: Value,
    metrics: Value,
    artifact_uri: Option<String>,
    schema: Value,
    created_at: chrono::DateTime<Utc>,
    promoted_at: Option<chrono::DateTime<Utc>>,
}

fn to_training_job(row: TrainingJobRow) -> TrainingJob {
    TrainingJob {
        id: row.id,
        experiment_id: row.experiment_id,
        model_id: row.model_id,
        name: row.name,
        status: row.status,
        dataset_ids: deserialize_json(row.dataset_ids),
        training_config: row.training_config,
        hyperparameter_search: row.hyperparameter_search,
        objective_metric_name: row.objective_metric_name,
        trials: deserialize_json(row.trials),
        best_model_version_id: row.best_model_version_id,
        submitted_at: row.submitted_at,
        started_at: row.started_at,
        completed_at: row.completed_at,
        created_at: row.created_at,
    }
}

#[allow(dead_code)]
fn _to_model_version(row: ModelVersionRow) -> ModelVersion {
    ModelVersion {
        id: row.id,
        model_id: row.model_id,
        version_number: row.version_number,
        version_label: row.version_label,
        stage: row.stage,
        source_run_id: row.source_run_id,
        training_job_id: row.training_job_id,
        hyperparameters: row.hyperparameters,
        metrics: deserialize_json(row.metrics),
        artifact_uri: row.artifact_uri,
        schema: row.schema,
        created_at: row.created_at,
        promoted_at: row.promoted_at,
    }
}

pub async fn list_training_jobs(
    State(state): State<AppState>,
) -> ServiceResult<ListTrainingJobsResponse> {
    let rows = query_as::<_, TrainingJobRow>(
        r#"
        SELECT
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
        FROM ml_training_jobs
        ORDER BY submitted_at DESC, created_at DESC
        "#,
    )
    .fetch_all(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?;

    Ok(Json(ListTrainingJobsResponse {
        data: rows.into_iter().map(to_training_job).collect(),
    }))
}

pub async fn create_training_job(
    State(state): State<AppState>,
    Json(body): Json<CreateTrainingJobRequest>,
) -> ServiceResult<TrainingJob> {
    if body.name.trim().is_empty() {
        return Err(bad_request("training job name is required"));
    }

    let objective_metric_name = body
        .objective_metric_name
        .unwrap_or_else(|| "accuracy".to_string());
    let search = body.hyperparameter_search.unwrap_or_else(|| json!({}));
    let resolved_training_config = if body.training_config.is_null() {
        json!({ "engine": "tabular-logistic" })
    } else {
        body.training_config
    };
    let execution = training::execute_training(
        &resolved_training_config,
        Some(&search),
        &objective_metric_name,
    )
    .map_err(bad_request)?;
    let now = Utc::now();
    let job_id = Uuid::now_v7();

    let mut best_model_version_id = None;

    if body.auto_register_model_version {
        if let (Some(model_id), Some(best_hyperparameters)) =
            (body.model_id, execution.best_hyperparameters.clone())
        {
            let next_version_number = query_scalar::<_, i32>(
                "SELECT COALESCE(MAX(version_number), 0) + 1 FROM ml_model_versions WHERE model_id = $1",
            )
            .bind(model_id)
            .fetch_one(&state.db)
            .await
            .map_err(|cause| db_error(&cause))?;

            let version_row = query_as::<_, ModelVersionRow>(
                r#"
                INSERT INTO ml_model_versions (
                    id,
                    model_id,
                    version_number,
                    version_label,
                    stage,
                    source_run_id,
                    training_job_id,
                    hyperparameters,
                    metrics,
                    artifact_uri,
                    schema,
                    promoted_at
                )
                VALUES ($1, $2, $3, $4, 'candidate', NULL, $5, $6, $7, $8, $9, NULL)
                RETURNING
                    id,
                    model_id,
                    version_number,
                    version_label,
                    stage,
                    source_run_id,
                    training_job_id,
                    hyperparameters,
                    metrics,
                    artifact_uri,
                    schema,
                    created_at,
                    promoted_at
                "#,
            )
            .bind(Uuid::now_v7())
            .bind(model_id)
            .bind(next_version_number)
            .bind(format!("autotune-v{next_version_number}"))
            .bind(job_id)
            .bind(best_hyperparameters)
            .bind(to_json(&execution.best_metrics))
            .bind(Some(format!(
                "ml://models/{model_id}/versions/{next_version_number}"
            )))
            .bind(execution.best_schema.clone().unwrap_or_else(|| {
                json!({
                    "signature": "tabular",
                    "objective_metric": objective_metric_name,
                    "generated_by": "training-orchestrator"
                })
            }))
            .fetch_one(&state.db)
            .await
            .map_err(|cause| db_error(&cause))?;

            best_model_version_id = Some(version_row.id);

            sqlx::query(
                "UPDATE ml_models SET latest_version_number = $2, current_stage = 'candidate', updated_at = NOW() WHERE id = $1",
            )
            .bind(model_id)
            .bind(next_version_number)
            .execute(&state.db)
            .await
            .map_err(|cause| db_error(&cause))?;
        }
    }

    let row = query_as::<_, TrainingJobRow>(
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
        VALUES ($1, $2, $3, $4, 'completed', $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
        RETURNING
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
        "#,
    )
    .bind(job_id)
    .bind(body.experiment_id)
    .bind(body.model_id)
    .bind(body.name.trim())
    .bind(to_json(&body.dataset_ids))
    .bind(resolved_training_config)
    .bind(search)
    .bind(objective_metric_name)
    .bind(to_json(&execution.trials))
    .bind(best_model_version_id)
    .bind(now)
    .bind(Some(now))
    .bind(Some(now))
    .bind(now)
    .fetch_one(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?;

    Ok(Json(to_training_job(row)))
}
