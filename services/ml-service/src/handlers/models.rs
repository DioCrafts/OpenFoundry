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
        model::{CreateModelRequest, ListModelsResponse, RegisteredModel, UpdateModelRequest},
        model_version::{
            CreateModelVersionRequest, ListModelVersionsResponse, ModelVersion,
            TransitionModelVersionRequest,
        },
    },
};

use super::{ServiceResult, bad_request, db_error, deserialize_json, not_found, to_json};

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

fn to_version(row: ModelVersionRow) -> ModelVersion {
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

async fn refresh_model_rollup(db: &PgPool, model_id: Uuid) -> Result<(), sqlx::Error> {
    let latest_version_number = query_scalar::<_, Option<i32>>(
        "SELECT MAX(version_number) FROM ml_model_versions WHERE model_id = $1",
    )
    .bind(model_id)
    .fetch_one(db)
    .await?;

    let production_versions = query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM ml_model_versions WHERE model_id = $1 AND stage = 'production'",
    )
    .bind(model_id)
    .fetch_one(db)
    .await?;
    let staging_versions = query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM ml_model_versions WHERE model_id = $1 AND stage = 'staging'",
    )
    .bind(model_id)
    .fetch_one(db)
    .await?;
    let candidate_versions = query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM ml_model_versions WHERE model_id = $1 AND stage = 'candidate'",
    )
    .bind(model_id)
    .fetch_one(db)
    .await?;

    let current_stage = if production_versions > 0 {
        "production"
    } else if staging_versions > 0 {
        "staging"
    } else if candidate_versions > 0 {
        "candidate"
    } else {
        "none"
    };

    sqlx::query(
		"UPDATE ml_models SET latest_version_number = $2, current_stage = $3, updated_at = NOW() WHERE id = $1",
	)
	.bind(model_id)
	.bind(latest_version_number)
	.bind(current_stage)
	.execute(db)
	.await?;

    Ok(())
}

async fn load_model(db: &PgPool, model_id: Uuid) -> Result<Option<ModelRow>, sqlx::Error> {
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
		WHERE id = $1
		"#,
    )
    .bind(model_id)
    .fetch_optional(db)
    .await
}

pub async fn list_models(State(state): State<AppState>) -> ServiceResult<ListModelsResponse> {
    let rows = query_as::<_, ModelRow>(
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
		ORDER BY updated_at DESC, created_at DESC
		"#,
    )
    .fetch_all(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?;

    Ok(Json(ListModelsResponse {
        data: rows.into_iter().map(to_model).collect(),
    }))
}

pub async fn create_model(
    State(state): State<AppState>,
    Json(body): Json<CreateModelRequest>,
) -> ServiceResult<RegisteredModel> {
    if body.name.trim().is_empty() {
        return Err(bad_request("model name is required"));
    }

    let row = query_as::<_, ModelRow>(
        r#"
		INSERT INTO ml_models (
			id,
			name,
			description,
			problem_type,
			status,
			tags,
			current_stage,
			latest_version_number
		)
		VALUES ($1, $2, $3, $4, $5, $6, 'none', NULL)
		RETURNING
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
		"#,
    )
    .bind(Uuid::now_v7())
    .bind(body.name.trim())
    .bind(body.description)
    .bind(body.problem_type)
    .bind(body.status.unwrap_or_else(|| "active".to_string()))
    .bind(to_json(&body.tags))
    .fetch_one(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?;

    Ok(Json(to_model(row)))
}

pub async fn update_model(
    State(state): State<AppState>,
    Path(model_id): Path<Uuid>,
    Json(body): Json<UpdateModelRequest>,
) -> ServiceResult<RegisteredModel> {
    let Some(current) = load_model(&state.db, model_id)
        .await
        .map_err(|cause| db_error(&cause))?
    else {
        return Err(not_found("model not found"));
    };

    let tags = body
        .tags
        .unwrap_or_else(|| deserialize_json(current.tags.clone()));

    let row = query_as::<_, ModelRow>(
        r#"
		UPDATE ml_models
		SET
			name = $2,
			description = $3,
			problem_type = $4,
			status = $5,
			tags = $6,
			updated_at = NOW()
		WHERE id = $1
		RETURNING
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
		"#,
    )
    .bind(model_id)
    .bind(body.name.unwrap_or(current.name))
    .bind(body.description.unwrap_or(current.description))
    .bind(body.problem_type.unwrap_or(current.problem_type))
    .bind(body.status.unwrap_or(current.status))
    .bind(to_json(&tags))
    .fetch_one(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?;

    Ok(Json(to_model(row)))
}

pub async fn list_model_versions(
    State(state): State<AppState>,
    Path(model_id): Path<Uuid>,
) -> ServiceResult<ListModelVersionsResponse> {
    let exists = query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM ml_models WHERE id = $1)")
        .bind(model_id)
        .fetch_one(&state.db)
        .await
        .map_err(|cause| db_error(&cause))?;

    if !exists {
        return Err(not_found("model not found"));
    }

    let rows = query_as::<_, ModelVersionRow>(
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
		WHERE model_id = $1
		ORDER BY version_number DESC, created_at DESC
		"#,
    )
    .bind(model_id)
    .fetch_all(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?;

    Ok(Json(ListModelVersionsResponse {
        data: rows.into_iter().map(to_version).collect(),
    }))
}

pub async fn create_model_version(
    State(state): State<AppState>,
    Path(model_id): Path<Uuid>,
    Json(body): Json<CreateModelVersionRequest>,
) -> ServiceResult<ModelVersion> {
    let exists = query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM ml_models WHERE id = $1)")
        .bind(model_id)
        .fetch_one(&state.db)
        .await
        .map_err(|cause| db_error(&cause))?;

    if !exists {
        return Err(not_found("model not found"));
    }

    let next_version_number = query_scalar::<_, i32>(
        "SELECT COALESCE(MAX(version_number), 0) + 1 FROM ml_model_versions WHERE model_id = $1",
    )
    .bind(model_id)
    .fetch_one(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?;

    let stage = body.stage.unwrap_or_else(|| "candidate".to_string());
    let version_label = body
        .version_label
        .unwrap_or_else(|| format!("v{next_version_number}"));

    let row = query_as::<_, ModelVersionRow>(
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
		VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
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
    .bind(version_label)
    .bind(stage.clone())
    .bind(body.source_run_id)
    .bind(body.training_job_id)
    .bind(body.hyperparameters.unwrap_or_else(|| json!({})))
    .bind(to_json(&body.metrics.unwrap_or_default()))
    .bind(body.artifact_uri)
    .bind(body.schema.unwrap_or_else(|| json!({})))
    .bind(if stage == "production" || stage == "staging" {
        Some(Utc::now())
    } else {
        None
    })
    .fetch_one(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?;

    refresh_model_rollup(&state.db, model_id)
        .await
        .map_err(|cause| db_error(&cause))?;

    Ok(Json(to_version(row)))
}

pub async fn transition_model_version(
    State(state): State<AppState>,
    Path(version_id): Path<Uuid>,
    Json(body): Json<TransitionModelVersionRequest>,
) -> ServiceResult<ModelVersion> {
    if body.stage.trim().is_empty() {
        return Err(bad_request("target stage is required"));
    }

    let Some(current) = query_as::<_, ModelVersionRow>(
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
		WHERE id = $1
		"#,
    )
    .bind(version_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?
    else {
        return Err(not_found("model version not found"));
    };

    if body.stage == "production" {
        sqlx::query(
			"UPDATE ml_model_versions SET stage = 'staging' WHERE model_id = $1 AND stage = 'production' AND id <> $2",
		)
		.bind(current.model_id)
		.bind(version_id)
		.execute(&state.db)
		.await
		.map_err(|cause| db_error(&cause))?;
    }

    let row = query_as::<_, ModelVersionRow>(
        r#"
		UPDATE ml_model_versions
		SET
			stage = $2,
			promoted_at = $3
		WHERE id = $1
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
    .bind(version_id)
    .bind(body.stage.as_str())
    .bind(Some(Utc::now()))
    .fetch_one(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?;

    refresh_model_rollup(&state.db, current.model_id)
        .await
        .map_err(|cause| db_error(&cause))?;

    Ok(Json(to_version(row)))
}
