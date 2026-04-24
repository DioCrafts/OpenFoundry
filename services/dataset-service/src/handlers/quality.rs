use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use uuid::Uuid;

use crate::{
    AppState,
    domain::quality::profiler,
    models::{
        dataset::Dataset,
        quality::{CreateQualityRuleRequest, DatasetQualityRule, UpdateQualityRuleRequest},
    },
};

/// GET /api/v1/datasets/:id/quality
pub async fn get_dataset_quality(
    State(state): State<AppState>,
    Path(dataset_id): Path<Uuid>,
) -> impl IntoResponse {
    let dataset = match load_dataset(&state, dataset_id).await {
        Ok(Some(dataset)) => dataset,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(error) => {
            tracing::error!("get dataset quality lookup failed: {error}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    match profiler::fetch_dataset_quality(&state, dataset.id).await {
        Ok(response) => Json(response).into_response(),
        Err(error) => {
            tracing::error!("get dataset quality failed: {error}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

/// POST /api/v1/datasets/:id/quality/profile
pub async fn refresh_dataset_quality(
    State(state): State<AppState>,
    Path(dataset_id): Path<Uuid>,
) -> impl IntoResponse {
    let dataset = match load_dataset(&state, dataset_id).await {
        Ok(Some(dataset)) => dataset,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(error) => {
            tracing::error!("refresh dataset quality lookup failed: {error}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    if !profiler::dataset_has_uploaded_data(&state, &dataset).await {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "upload data before generating a quality profile" })),
        )
            .into_response();
    }

    match profiler::refresh_dataset_quality(&state, &dataset, None).await {
        Ok(response) => Json(response).into_response(),
        Err(error) => {
            tracing::error!("refresh dataset quality failed: {error}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": error })),
            )
                .into_response()
        }
    }
}

/// POST /api/v1/datasets/:id/quality/rules
pub async fn create_quality_rule(
    State(state): State<AppState>,
    Path(dataset_id): Path<Uuid>,
    Json(body): Json<CreateQualityRuleRequest>,
) -> impl IntoResponse {
    let dataset = match load_dataset(&state, dataset_id).await {
        Ok(Some(dataset)) => dataset,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(error) => {
            tracing::error!("create quality rule lookup failed: {error}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let result = sqlx::query_as::<_, DatasetQualityRule>(
		r#"INSERT INTO dataset_quality_rules (id, dataset_id, name, rule_type, severity, config, enabled)
		   VALUES ($1, $2, $3, $4, $5, $6, $7)
		   RETURNING *"#,
	)
	.bind(Uuid::now_v7())
	.bind(dataset_id)
	.bind(&body.name)
	.bind(&body.rule_type)
	.bind(body.severity.as_deref().unwrap_or("medium"))
	.bind(&body.config)
	.bind(body.enabled.unwrap_or(true))
	.fetch_one(&state.db)
	.await;

    match result {
        Ok(_) => match refresh_if_possible(&state, &dataset).await {
            Ok(response) => (StatusCode::CREATED, Json(response)).into_response(),
            Err(error) => {
                tracing::error!("create quality rule refresh failed: {error}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        },
        Err(error) => {
            tracing::error!("create quality rule failed: {error}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

/// PATCH /api/v1/datasets/:id/quality/rules/:rule_id
pub async fn update_quality_rule(
    State(state): State<AppState>,
    Path((dataset_id, rule_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<UpdateQualityRuleRequest>,
) -> impl IntoResponse {
    let dataset = match load_dataset(&state, dataset_id).await {
        Ok(Some(dataset)) => dataset,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(error) => {
            tracing::error!("update quality rule lookup failed: {error}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let result = sqlx::query(
        r#"UPDATE dataset_quality_rules
		   SET name = COALESCE($3, name),
		       severity = COALESCE($4, severity),
		       enabled = COALESCE($5, enabled),
		       config = COALESCE($6, config),
		       updated_at = NOW()
		   WHERE dataset_id = $1 AND id = $2"#,
    )
    .bind(dataset_id)
    .bind(rule_id)
    .bind(&body.name)
    .bind(&body.severity)
    .bind(body.enabled)
    .bind(&body.config)
    .execute(&state.db)
    .await;

    match result {
        Ok(result) if result.rows_affected() > 0 => {
            match refresh_if_possible(&state, &dataset).await {
                Ok(response) => Json(response).into_response(),
                Err(error) => {
                    tracing::error!("update quality rule refresh failed: {error}");
                    StatusCode::INTERNAL_SERVER_ERROR.into_response()
                }
            }
        }
        Ok(_) => StatusCode::NOT_FOUND.into_response(),
        Err(error) => {
            tracing::error!("update quality rule failed: {error}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

/// DELETE /api/v1/datasets/:id/quality/rules/:rule_id
pub async fn delete_quality_rule(
    State(state): State<AppState>,
    Path((dataset_id, rule_id)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
    let dataset = match load_dataset(&state, dataset_id).await {
        Ok(Some(dataset)) => dataset,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(error) => {
            tracing::error!("delete quality rule lookup failed: {error}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let result = sqlx::query("DELETE FROM dataset_quality_rules WHERE dataset_id = $1 AND id = $2")
        .bind(dataset_id)
        .bind(rule_id)
        .execute(&state.db)
        .await;

    match result {
        Ok(result) if result.rows_affected() > 0 => {
            match refresh_if_possible(&state, &dataset).await {
                Ok(response) => Json(response).into_response(),
                Err(error) => {
                    tracing::error!("delete quality rule refresh failed: {error}");
                    StatusCode::INTERNAL_SERVER_ERROR.into_response()
                }
            }
        }
        Ok(_) => StatusCode::NOT_FOUND.into_response(),
        Err(error) => {
            tracing::error!("delete quality rule failed: {error}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

async fn refresh_if_possible(
    state: &AppState,
    dataset: &Dataset,
) -> Result<crate::models::quality::DatasetQualityResponse, String> {
    if profiler::dataset_has_uploaded_data(state, dataset).await {
        profiler::refresh_dataset_quality(state, dataset, None).await
    } else {
        profiler::fetch_dataset_quality(state, dataset.id).await
    }
}

async fn load_dataset(state: &AppState, dataset_id: Uuid) -> Result<Option<Dataset>, sqlx::Error> {
    sqlx::query_as::<_, Dataset>("SELECT * FROM datasets WHERE id = $1")
        .bind(dataset_id)
        .fetch_optional(&state.db)
        .await
}
