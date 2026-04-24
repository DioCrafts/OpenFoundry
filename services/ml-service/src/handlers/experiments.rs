use std::collections::BTreeSet;

use axum::{
    Json,
    extract::{Path, State},
};
use chrono::Utc;
use serde_json::{Value, json};
use sqlx::{FromRow, PgPool, query_as, query_scalar};
use uuid::Uuid;

use crate::{
    AppState,
    models::{
        experiment::{
            CreateExperimentRequest, Experiment, ListExperimentsResponse, UpdateExperimentRequest,
        },
        run::{
            CompareRunsRequest, CompareRunsResponse, CreateExperimentRunRequest, ExperimentRun,
            ListRunsResponse, MetricValue, UpdateExperimentRunRequest,
        },
    },
};

use super::{
    ServiceResult, bad_request, db_error, deserialize_json, deserialize_optional_json, not_found,
    to_json,
};

#[derive(Debug, FromRow)]
struct ExperimentRow {
    id: Uuid,
    name: String,
    description: String,
    objective: String,
    task_type: String,
    primary_metric: String,
    status: String,
    tags: Value,
    owner_id: Option<Uuid>,
    run_count: i64,
    best_metric: Option<Value>,
    created_at: chrono::DateTime<Utc>,
    updated_at: chrono::DateTime<Utc>,
}

#[derive(Debug, FromRow)]
struct RunRow {
    id: Uuid,
    experiment_id: Uuid,
    name: String,
    status: String,
    params: Value,
    metrics: Value,
    artifacts: Value,
    notes: String,
    source_dataset_ids: Value,
    model_version_id: Option<Uuid>,
    started_at: Option<chrono::DateTime<Utc>>,
    finished_at: Option<chrono::DateTime<Utc>>,
    created_at: chrono::DateTime<Utc>,
    updated_at: chrono::DateTime<Utc>,
}

#[derive(Debug, FromRow)]
struct RunMetricsRow {
    metrics: Value,
}

fn to_experiment(row: ExperimentRow) -> Experiment {
    Experiment {
        id: row.id,
        name: row.name,
        description: row.description,
        objective: row.objective,
        task_type: row.task_type,
        primary_metric: row.primary_metric,
        status: row.status,
        tags: deserialize_json(row.tags),
        run_count: row.run_count,
        best_metric: deserialize_optional_json(row.best_metric),
        owner_id: row.owner_id,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
}

fn to_run(row: RunRow) -> ExperimentRun {
    ExperimentRun {
        id: row.id,
        experiment_id: row.experiment_id,
        name: row.name,
        status: row.status,
        params: row.params,
        metrics: deserialize_json(row.metrics),
        artifacts: deserialize_json(row.artifacts),
        notes: row.notes,
        source_dataset_ids: deserialize_json(row.source_dataset_ids),
        model_version_id: row.model_version_id,
        started_at: row.started_at,
        finished_at: row.finished_at,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
}

async fn refresh_experiment_rollup(db: &PgPool, experiment_id: Uuid) -> Result<(), sqlx::Error> {
    let primary_metric =
        query_scalar::<_, String>("SELECT primary_metric FROM ml_experiments WHERE id = $1")
            .bind(experiment_id)
            .fetch_optional(db)
            .await?;

    let Some(primary_metric) = primary_metric else {
        return Ok(());
    };

    let run_metrics = query_as::<_, RunMetricsRow>(
        "SELECT metrics FROM ml_runs WHERE experiment_id = $1 ORDER BY created_at DESC",
    )
    .bind(experiment_id)
    .fetch_all(db)
    .await?;

    let mut best_metric: Option<MetricValue> = None;
    for row in &run_metrics {
        let metrics: Vec<MetricValue> = deserialize_json(row.metrics.clone());
        let candidate = metrics
            .iter()
            .find(|metric| metric.name == primary_metric)
            .cloned()
            .or_else(|| metrics.first().cloned());

        if let Some(metric) = candidate {
            if best_metric
                .as_ref()
                .map(|existing| metric.value > existing.value)
                .unwrap_or(true)
            {
                best_metric = Some(metric);
            }
        }
    }

    sqlx::query(
		"UPDATE ml_experiments SET run_count = $2, best_metric = $3, updated_at = NOW() WHERE id = $1",
	)
	.bind(experiment_id)
	.bind(run_metrics.len() as i64)
	.bind(best_metric.as_ref().map(to_json))
	.execute(db)
	.await?;

    Ok(())
}

async fn load_run_row(db: &PgPool, run_id: Uuid) -> Result<Option<RunRow>, sqlx::Error> {
    query_as::<_, RunRow>(
        r#"
		SELECT
			id,
			experiment_id,
			name,
			status,
			params,
			metrics,
			artifacts,
			notes,
			source_dataset_ids,
			model_version_id,
			started_at,
			finished_at,
			created_at,
			updated_at
		FROM ml_runs
		WHERE id = $1
		"#,
    )
    .bind(run_id)
    .fetch_optional(db)
    .await
}

pub async fn list_experiments(
    State(state): State<AppState>,
) -> ServiceResult<ListExperimentsResponse> {
    let rows = query_as::<_, ExperimentRow>(
        r#"
		SELECT
			id,
			name,
			description,
			objective,
			task_type,
			primary_metric,
			status,
			tags,
			owner_id,
			run_count,
			best_metric,
			created_at,
			updated_at
		FROM ml_experiments
		ORDER BY updated_at DESC, created_at DESC
		"#,
    )
    .fetch_all(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?;

    Ok(Json(ListExperimentsResponse {
        data: rows.into_iter().map(to_experiment).collect(),
    }))
}

pub async fn create_experiment(
    State(state): State<AppState>,
    Json(body): Json<CreateExperimentRequest>,
) -> ServiceResult<Experiment> {
    if body.name.trim().is_empty() {
        return Err(bad_request("experiment name is required"));
    }

    let row = query_as::<_, ExperimentRow>(
        r#"
		INSERT INTO ml_experiments (
			id,
			name,
			description,
			objective,
			task_type,
			primary_metric,
			status,
			tags,
			run_count,
			best_metric
		)
		VALUES ($1, $2, $3, $4, $5, $6, 'active', $7, 0, NULL)
		RETURNING
			id,
			name,
			description,
			objective,
			task_type,
			primary_metric,
			status,
			tags,
			owner_id,
			run_count,
			best_metric,
			created_at,
			updated_at
		"#,
    )
    .bind(Uuid::now_v7())
    .bind(body.name.trim())
    .bind(body.description)
    .bind(body.objective)
    .bind(body.task_type)
    .bind(body.primary_metric)
    .bind(to_json(&body.tags))
    .fetch_one(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?;

    Ok(Json(to_experiment(row)))
}

pub async fn update_experiment(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateExperimentRequest>,
) -> ServiceResult<Experiment> {
    let Some(current) = query_as::<_, ExperimentRow>(
        r#"
		SELECT
			id,
			name,
			description,
			objective,
			task_type,
			primary_metric,
			status,
			tags,
			owner_id,
			run_count,
			best_metric,
			created_at,
			updated_at
		FROM ml_experiments
		WHERE id = $1
		"#,
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?
    else {
        return Err(not_found("experiment not found"));
    };

    let tags = body
        .tags
        .unwrap_or_else(|| deserialize_json(current.tags.clone()));

    let row = query_as::<_, ExperimentRow>(
        r#"
		UPDATE ml_experiments
		SET
			name = $2,
			description = $3,
			objective = $4,
			task_type = $5,
			primary_metric = $6,
			status = $7,
			tags = $8,
			updated_at = NOW()
		WHERE id = $1
		RETURNING
			id,
			name,
			description,
			objective,
			task_type,
			primary_metric,
			status,
			tags,
			owner_id,
			run_count,
			best_metric,
			created_at,
			updated_at
		"#,
    )
    .bind(id)
    .bind(body.name.unwrap_or(current.name))
    .bind(body.description.unwrap_or(current.description))
    .bind(body.objective.unwrap_or(current.objective))
    .bind(body.task_type.unwrap_or(current.task_type))
    .bind(body.primary_metric.unwrap_or(current.primary_metric))
    .bind(body.status.unwrap_or(current.status))
    .bind(to_json(&tags))
    .fetch_one(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?;

    refresh_experiment_rollup(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?;

    Ok(Json(to_experiment(row)))
}

pub async fn list_runs(
    State(state): State<AppState>,
    Path(experiment_id): Path<Uuid>,
) -> ServiceResult<ListRunsResponse> {
    let exists =
        query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM ml_experiments WHERE id = $1)")
            .bind(experiment_id)
            .fetch_one(&state.db)
            .await
            .map_err(|cause| db_error(&cause))?;

    if !exists {
        return Err(not_found("experiment not found"));
    }

    let rows = query_as::<_, RunRow>(
        r#"
		SELECT
			id,
			experiment_id,
			name,
			status,
			params,
			metrics,
			artifacts,
			notes,
			source_dataset_ids,
			model_version_id,
			started_at,
			finished_at,
			created_at,
			updated_at
		FROM ml_runs
		WHERE experiment_id = $1
		ORDER BY created_at DESC
		"#,
    )
    .bind(experiment_id)
    .fetch_all(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?;

    Ok(Json(ListRunsResponse {
        data: rows.into_iter().map(to_run).collect(),
    }))
}

pub async fn create_run(
    State(state): State<AppState>,
    Path(experiment_id): Path<Uuid>,
    Json(body): Json<CreateExperimentRunRequest>,
) -> ServiceResult<ExperimentRun> {
    if body.name.trim().is_empty() {
        return Err(bad_request("run name is required"));
    }

    let exists =
        query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM ml_experiments WHERE id = $1)")
            .bind(experiment_id)
            .fetch_one(&state.db)
            .await
            .map_err(|cause| db_error(&cause))?;

    if !exists {
        return Err(not_found("experiment not found"));
    }

    let status = body.status.unwrap_or_else(|| "completed".to_string());
    let started_at = body.started_at.or_else(|| Some(Utc::now()));
    let finished_at = body.finished_at.or_else(|| {
        if status == "completed" {
            Some(Utc::now())
        } else {
            None
        }
    });

    let row = query_as::<_, RunRow>(
        r#"
		INSERT INTO ml_runs (
			id,
			experiment_id,
			name,
			status,
			params,
			metrics,
			artifacts,
			notes,
			source_dataset_ids,
			started_at,
			finished_at
		)
		VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
		RETURNING
			id,
			experiment_id,
			name,
			status,
			params,
			metrics,
			artifacts,
			notes,
			source_dataset_ids,
			model_version_id,
			started_at,
			finished_at,
			created_at,
			updated_at
		"#,
    )
    .bind(Uuid::now_v7())
    .bind(experiment_id)
    .bind(body.name.trim())
    .bind(status)
    .bind(if body.params.is_null() {
        json!({})
    } else {
        body.params
    })
    .bind(to_json(&body.metrics))
    .bind(to_json(&body.artifacts))
    .bind(body.notes.unwrap_or_default())
    .bind(to_json(&body.source_dataset_ids))
    .bind(started_at)
    .bind(finished_at)
    .fetch_one(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?;

    refresh_experiment_rollup(&state.db, experiment_id)
        .await
        .map_err(|cause| db_error(&cause))?;

    Ok(Json(to_run(row)))
}

pub async fn update_run(
    State(state): State<AppState>,
    Path(run_id): Path<Uuid>,
    Json(body): Json<UpdateExperimentRunRequest>,
) -> ServiceResult<ExperimentRun> {
    let Some(current) = load_run_row(&state.db, run_id)
        .await
        .map_err(|cause| db_error(&cause))?
    else {
        return Err(not_found("run not found"));
    };

    let status = body.status.unwrap_or(current.status);
    let params = body.params.unwrap_or(current.params);
    let metrics = body
        .metrics
        .map(|metrics| to_json(&metrics))
        .unwrap_or(current.metrics);
    let artifacts = body
        .artifacts
        .map(|artifacts| to_json(&artifacts))
        .unwrap_or(current.artifacts);

    let row = query_as::<_, RunRow>(
        r#"
		UPDATE ml_runs
		SET
			status = $2,
			params = $3,
			metrics = $4,
			artifacts = $5,
			notes = $6,
			model_version_id = $7,
			finished_at = $8,
			updated_at = NOW()
		WHERE id = $1
		RETURNING
			id,
			experiment_id,
			name,
			status,
			params,
			metrics,
			artifacts,
			notes,
			source_dataset_ids,
			model_version_id,
			started_at,
			finished_at,
			created_at,
			updated_at
		"#,
    )
    .bind(run_id)
    .bind(status)
    .bind(params)
    .bind(metrics)
    .bind(artifacts)
    .bind(body.notes.unwrap_or(current.notes))
    .bind(body.model_version_id.or(current.model_version_id))
    .bind(body.finished_at.or(current.finished_at))
    .fetch_one(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?;

    refresh_experiment_rollup(&state.db, row.experiment_id)
        .await
        .map_err(|cause| db_error(&cause))?;

    Ok(Json(to_run(row)))
}

pub async fn compare_runs(
    State(state): State<AppState>,
    Json(body): Json<CompareRunsRequest>,
) -> ServiceResult<CompareRunsResponse> {
    if body.run_ids.is_empty() {
        return Err(bad_request("at least one run is required"));
    }

    let mut rows = Vec::new();
    for run_id in body.run_ids {
        let Some(row) = load_run_row(&state.db, run_id)
            .await
            .map_err(|cause| db_error(&cause))?
        else {
            return Err(not_found(format!("run {run_id} not found")));
        };
        rows.push(to_run(row));
    }

    let mut metric_names = BTreeSet::new();
    for run in &rows {
        for metric in &run.metrics {
            metric_names.insert(metric.name.clone());
        }
    }

    Ok(Json(CompareRunsResponse {
        data: rows,
        metric_names: metric_names.into_iter().collect(),
    }))
}
