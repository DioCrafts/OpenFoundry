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
        asset_lineage::{
            ExperimentAssetLineageResponse, ModelAssetEdge, ModelAssetLineageSummary,
            ModelAssetNode,
        },
        experiment::{
            CreateExperimentRequest, Experiment, ListExperimentsResponse, UpdateExperimentRequest,
        },
        model::RegisteredModel,
        model_version::ModelVersion,
        run::{
            CompareRunsRequest, CompareRunsResponse, CreateExperimentRunRequest, ExperimentRun,
            ListRunsResponse, MetricValue, UpdateExperimentRunRequest,
        },
        training_job::TrainingJob,
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
    objective_spec: Value,
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

#[derive(Debug, FromRow)]
struct ModelRow {
    id: Uuid,
    name: String,
    description: String,
    problem_type: String,
    status: String,
    tags: Value,
    owner_id: Option<Uuid>,
    current_stage: String,
    latest_version_number: Option<i32>,
    active_deployment_id: Option<Uuid>,
    created_at: chrono::DateTime<Utc>,
    updated_at: chrono::DateTime<Utc>,
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
}

fn to_experiment(row: ExperimentRow) -> Experiment {
    Experiment {
        id: row.id,
        name: row.name,
        description: row.description,
        objective: row.objective,
        objective_spec: deserialize_json(row.objective_spec),
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

fn to_model(row: ModelRow) -> RegisteredModel {
    RegisteredModel {
        id: row.id,
        name: row.name,
        description: row.description,
        problem_type: row.problem_type,
        status: row.status,
        tags: deserialize_json(row.tags),
        owner_id: row.owner_id,
        current_stage: row.current_stage,
        latest_version_number: row.latest_version_number,
        active_deployment_id: row.active_deployment_id,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
}

fn to_model_version(row: ModelVersionRow) -> ModelVersion {
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
			objective_spec,
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
			objective_spec,
			task_type,
			primary_metric,
			status,
			tags,
			run_count,
			best_metric
		)
		VALUES ($1, $2, $3, $4, $5, $6, $7, 'active', $8, 0, NULL)
		RETURNING
			id,
			name,
			description,
			objective,
			objective_spec,
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
    .bind(to_json(&body.objective_spec))
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
			objective_spec,
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
    let objective_spec = body
        .objective_spec
        .unwrap_or_else(|| deserialize_json(current.objective_spec.clone()));

    let row = query_as::<_, ExperimentRow>(
        r#"
		UPDATE ml_experiments
		SET
			name = $2,
			description = $3,
			objective = $4,
			objective_spec = $5,
			task_type = $6,
			primary_metric = $7,
			status = $8,
			tags = $9,
			updated_at = NOW()
		WHERE id = $1
		RETURNING
			id,
			name,
			description,
			objective,
			objective_spec,
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
    .bind(to_json(&objective_spec))
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

fn asset_node_id(kind: &str, id: impl std::fmt::Display) -> String {
    format!("{kind}:{id}")
}

pub async fn get_experiment_asset_lineage(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ServiceResult<ExperimentAssetLineageResponse> {
    let Some(experiment_row) = query_as::<_, ExperimentRow>(
        r#"
		SELECT
			id,
			name,
			description,
			objective,
			objective_spec,
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
    let experiment = to_experiment(experiment_row);

    let runs = query_as::<_, RunRow>(
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
    .bind(id)
    .fetch_all(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?
    .into_iter()
    .map(to_run)
    .collect::<Vec<_>>();

    let training_jobs = query_as::<_, TrainingJobRow>(
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
		WHERE experiment_id = $1
		ORDER BY submitted_at DESC, created_at DESC
		"#,
    )
    .bind(id)
    .fetch_all(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?
    .into_iter()
    .map(to_training_job)
    .collect::<Vec<_>>();

    let mut dataset_ids = BTreeSet::new();
    let mut model_ids = experiment
        .objective_spec
        .linked_model_ids
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let mut version_ids = BTreeSet::new();
    let mut frameworks = BTreeSet::new();

    for dataset_id in &experiment.objective_spec.linked_dataset_ids {
        dataset_ids.insert(*dataset_id);
    }
    for run in &runs {
        for dataset_id in &run.source_dataset_ids {
            dataset_ids.insert(*dataset_id);
        }
        if let Some(model_version_id) = run.model_version_id {
            version_ids.insert(model_version_id);
        }
    }
    for job in &training_jobs {
        for dataset_id in &job.dataset_ids {
            dataset_ids.insert(*dataset_id);
        }
        if let Some(model_id) = job.model_id {
            model_ids.insert(model_id);
        }
        if let Some(best_model_version_id) = job.best_model_version_id {
            version_ids.insert(best_model_version_id);
        }
        if let Some(engine) = job.training_config.get("engine").and_then(Value::as_str) {
            frameworks.insert(engine.to_string());
        }
    }

    let mut model_versions = Vec::<ModelVersion>::new();
    if !version_ids.is_empty() {
        model_versions = query_as::<_, ModelVersionRow>(
            r#"
			SELECT
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
			FROM ml_model_versions
			WHERE id = ANY($1)
			ORDER BY version_number DESC, created_at DESC
			"#,
        )
        .bind(version_ids.iter().copied().collect::<Vec<_>>())
        .fetch_all(&state.db)
        .await
        .map_err(|cause| db_error(&cause))?
        .into_iter()
        .map(to_model_version)
        .collect::<Vec<_>>();
    }

    for version in &model_versions {
        model_ids.insert(version.model_id);
        if let Some(engine) = version.schema.get("engine").and_then(Value::as_str) {
            frameworks.insert(engine.to_string());
        }
        if let Some(framework) = version
            .schema
            .get("model_adapter")
            .and_then(|value| value.get("framework"))
            .and_then(Value::as_str)
        {
            frameworks.insert(framework.to_string());
        }
    }

    let models = if model_ids.is_empty() {
        Vec::new()
    } else {
        query_as::<_, ModelRow>(
            r#"
			SELECT
				id,
				name,
				description,
				problem_type,
				status,
				tags,
				owner_id,
				current_stage,
				latest_version_number,
				active_deployment_id,
				created_at,
				updated_at
			FROM ml_models
			WHERE id = ANY($1)
			ORDER BY updated_at DESC, created_at DESC
			"#,
        )
        .bind(model_ids.iter().copied().collect::<Vec<_>>())
        .fetch_all(&state.db)
        .await
        .map_err(|cause| db_error(&cause))?
        .into_iter()
        .map(to_model)
        .collect::<Vec<_>>()
    };

    let deployments = if model_ids.is_empty() {
        Vec::new()
    } else {
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
				drift_report
			FROM ml_deployments
			WHERE model_id = ANY($1)
			ORDER BY updated_at DESC, created_at DESC
			"#,
        )
        .bind(model_ids.iter().copied().collect::<Vec<_>>())
        .fetch_all(&state.db)
        .await
        .map_err(|cause| db_error(&cause))?
    };

    let mut nodes = Vec::<ModelAssetNode>::new();
    let mut edges = Vec::<ModelAssetEdge>::new();

    let experiment_node_id = asset_node_id("experiment", experiment.id);
    nodes.push(ModelAssetNode {
        id: experiment_node_id.clone(),
        kind: "experiment".to_string(),
        label: experiment.name.clone(),
        status: experiment.objective_spec.status.clone(),
        metadata: json!({
            "objective": experiment.objective,
            "primary_metric": experiment.primary_metric,
            "deployment_target": experiment.objective_spec.deployment_target,
            "stakeholders": experiment.objective_spec.stakeholders,
            "success_criteria": experiment.objective_spec.success_criteria,
            "documentation_uri": experiment.objective_spec.documentation_uri,
            "collaboration_notes": experiment.objective_spec.collaboration_notes,
        }),
    });

    for dataset_id in &dataset_ids {
        let node_id = asset_node_id("dataset", dataset_id);
        nodes.push(ModelAssetNode {
            id: node_id.clone(),
            kind: "dataset".to_string(),
            label: dataset_id.to_string(),
            status: "referenced".to_string(),
            metadata: json!({}),
        });
    }

    for dataset_id in &experiment.objective_spec.linked_dataset_ids {
        edges.push(ModelAssetEdge {
            source: experiment_node_id.clone(),
            target: asset_node_id("dataset", dataset_id),
            relation: "targets_dataset".to_string(),
        });
    }

    for run in &runs {
        let node_id = asset_node_id("run", run.id);
        nodes.push(ModelAssetNode {
            id: node_id.clone(),
            kind: "run".to_string(),
            label: run.name.clone(),
            status: run.status.clone(),
            metadata: json!({
                "metrics": run.metrics,
                "params": run.params,
                "artifacts": run.artifacts,
                "notes": run.notes,
                "source_dataset_ids": run.source_dataset_ids,
            }),
        });
        edges.push(ModelAssetEdge {
            source: experiment_node_id.clone(),
            target: node_id.clone(),
            relation: "tracks_run".to_string(),
        });
        for dataset_id in &run.source_dataset_ids {
            edges.push(ModelAssetEdge {
                source: node_id.clone(),
                target: asset_node_id("dataset", dataset_id),
                relation: "consumes_dataset".to_string(),
            });
        }
        if let Some(model_version_id) = run.model_version_id {
            edges.push(ModelAssetEdge {
                source: node_id,
                target: asset_node_id("version", model_version_id),
                relation: "logged_model_version".to_string(),
            });
        }
    }

    for job in &training_jobs {
        let node_id = asset_node_id("training_job", job.id);
        nodes.push(ModelAssetNode {
            id: node_id.clone(),
            kind: "training_job".to_string(),
            label: job.name.clone(),
            status: job.status.clone(),
            metadata: json!({
                "objective_metric_name": job.objective_metric_name,
                "training_config": job.training_config,
                "hyperparameter_search": job.hyperparameter_search,
                "dataset_ids": job.dataset_ids,
                "trial_count": job.trials.len(),
            }),
        });
        edges.push(ModelAssetEdge {
            source: experiment_node_id.clone(),
            target: node_id.clone(),
            relation: "orchestrates_training".to_string(),
        });
        for dataset_id in &job.dataset_ids {
            edges.push(ModelAssetEdge {
                source: node_id.clone(),
                target: asset_node_id("dataset", dataset_id),
                relation: "trains_on".to_string(),
            });
        }
        if let Some(model_id) = job.model_id {
            edges.push(ModelAssetEdge {
                source: node_id.clone(),
                target: asset_node_id("model", model_id),
                relation: "produces_for_model".to_string(),
            });
        }
        if let Some(best_model_version_id) = job.best_model_version_id {
            edges.push(ModelAssetEdge {
                source: node_id,
                target: asset_node_id("version", best_model_version_id),
                relation: "best_candidate".to_string(),
            });
        }
    }

    for model_id in &experiment.objective_spec.linked_model_ids {
        edges.push(ModelAssetEdge {
            source: experiment_node_id.clone(),
            target: asset_node_id("model", model_id),
            relation: "targets_model".to_string(),
        });
    }

    for model in &models {
        nodes.push(ModelAssetNode {
            id: asset_node_id("model", model.id),
            kind: "model".to_string(),
            label: model.name.clone(),
            status: model.current_stage.clone(),
            metadata: json!({
                "problem_type": model.problem_type,
                "tags": model.tags,
                "latest_version_number": model.latest_version_number,
            }),
        });
    }

    for version in &model_versions {
        nodes.push(ModelAssetNode {
            id: asset_node_id("version", version.id),
            kind: "model_version".to_string(),
            label: version.version_label.clone(),
            status: version.stage.clone(),
            metadata: json!({
                "version_number": version.version_number,
                "artifact_uri": version.artifact_uri,
                "metrics": version.metrics,
                "hyperparameters": version.hyperparameters,
                "schema": version.schema,
            }),
        });
        edges.push(ModelAssetEdge {
            source: asset_node_id("version", version.id),
            target: asset_node_id("model", version.model_id),
            relation: "belongs_to_model".to_string(),
        });
    }

    for deployment in &deployments {
        nodes.push(ModelAssetNode {
            id: asset_node_id("deployment", deployment.id),
            kind: "deployment".to_string(),
            label: deployment.name.clone(),
            status: deployment.status.clone(),
            metadata: json!({
                "strategy_type": deployment.strategy_type,
                "endpoint_path": deployment.endpoint_path,
                "monitoring_window": deployment.monitoring_window,
                "traffic_split": deployment.traffic_split,
                "baseline_dataset_id": deployment.baseline_dataset_id,
                "drift_report": deployment.drift_report,
            }),
        });
        edges.push(ModelAssetEdge {
            source: asset_node_id("deployment", deployment.id),
            target: asset_node_id("model", deployment.model_id),
            relation: "serves_model".to_string(),
        });
        if let Some(baseline_dataset_id) = deployment.baseline_dataset_id {
            edges.push(ModelAssetEdge {
                source: asset_node_id("deployment", deployment.id),
                target: asset_node_id("dataset", baseline_dataset_id),
                relation: "monitors_against_dataset".to_string(),
            });
        }
    }

    Ok(Json(ExperimentAssetLineageResponse {
        experiment_id: experiment.id,
        objective_status: experiment.objective_spec.status.clone(),
        nodes,
        edges,
        summary: ModelAssetLineageSummary {
            dataset_count: dataset_ids.len(),
            run_count: runs.len(),
            training_job_count: training_jobs.len(),
            model_count: models.len(),
            version_count: model_versions.len(),
            deployment_count: deployments.len(),
            frameworks: frameworks.into_iter().collect(),
        },
    }))
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
