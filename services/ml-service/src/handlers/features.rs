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
    models::feature::{
        CreateFeatureRequest, FeatureDefinition, ListFeaturesResponse, MaterializeFeatureRequest,
        OnlineFeatureSnapshot, UpdateFeatureRequest,
    },
};

use super::{ServiceResult, bad_request, db_error, deserialize_json, not_found, to_json};

#[derive(Debug, FromRow)]
struct FeatureRow {
    id: Uuid,
    name: String,
    entity_name: String,
    data_type: String,
    description: String,
    status: String,
    offline_source: String,
    transformation: String,
    online_enabled: bool,
    online_namespace: String,
    batch_schedule: String,
    freshness_sla_minutes: i32,
    tags: Value,
    samples: Value,
    last_materialized_at: Option<chrono::DateTime<Utc>>,
    last_online_sync_at: Option<chrono::DateTime<Utc>>,
    created_at: chrono::DateTime<Utc>,
    updated_at: chrono::DateTime<Utc>,
}

fn to_feature(row: FeatureRow) -> FeatureDefinition {
    FeatureDefinition {
        id: row.id,
        name: row.name,
        entity_name: row.entity_name,
        data_type: row.data_type,
        description: row.description,
        status: row.status,
        offline_source: row.offline_source,
        transformation: row.transformation,
        online_enabled: row.online_enabled,
        online_namespace: row.online_namespace,
        batch_schedule: row.batch_schedule,
        freshness_sla_minutes: row.freshness_sla_minutes,
        tags: deserialize_json(row.tags),
        samples: deserialize_json(row.samples),
        last_materialized_at: row.last_materialized_at,
        last_online_sync_at: row.last_online_sync_at,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
}

async fn load_feature_row(
    db: &sqlx::PgPool,
    feature_id: Uuid,
) -> Result<Option<FeatureRow>, sqlx::Error> {
    query_as::<_, FeatureRow>(
        r#"
		SELECT
			id,
			name,
			entity_name,
			data_type,
			description,
			status,
			offline_source,
			transformation,
			online_enabled,
			online_namespace,
			batch_schedule,
			freshness_sla_minutes,
			tags,
			samples,
			last_materialized_at,
			last_online_sync_at,
			created_at,
			updated_at
		FROM ml_features
		WHERE id = $1
		"#,
    )
    .bind(feature_id)
    .fetch_optional(db)
    .await
}

pub async fn list_features(State(state): State<AppState>) -> ServiceResult<ListFeaturesResponse> {
    let rows = query_as::<_, FeatureRow>(
        r#"
		SELECT
			id,
			name,
			entity_name,
			data_type,
			description,
			status,
			offline_source,
			transformation,
			online_enabled,
			online_namespace,
			batch_schedule,
			freshness_sla_minutes,
			tags,
			samples,
			last_materialized_at,
			last_online_sync_at,
			created_at,
			updated_at
		FROM ml_features
		ORDER BY updated_at DESC, created_at DESC
		"#,
    )
    .fetch_all(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?;

    Ok(Json(ListFeaturesResponse {
        data: rows.into_iter().map(to_feature).collect(),
    }))
}

pub async fn create_feature(
    State(state): State<AppState>,
    Json(body): Json<CreateFeatureRequest>,
) -> ServiceResult<FeatureDefinition> {
    if body.name.trim().is_empty() || body.entity_name.trim().is_empty() {
        return Err(bad_request("feature name and entity name are required"));
    }

    let row = query_as::<_, FeatureRow>(
        r#"
		INSERT INTO ml_features (
			id,
			name,
			entity_name,
			data_type,
			description,
			status,
			offline_source,
			transformation,
			online_enabled,
			online_namespace,
			batch_schedule,
			freshness_sla_minutes,
			tags,
			samples,
			last_materialized_at,
			last_online_sync_at
		)
		VALUES ($1, $2, $3, $4, $5, 'active', $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
		RETURNING
			id,
			name,
			entity_name,
			data_type,
			description,
			status,
			offline_source,
			transformation,
			online_enabled,
			online_namespace,
			batch_schedule,
			freshness_sla_minutes,
			tags,
			samples,
			last_materialized_at,
			last_online_sync_at,
			created_at,
			updated_at
		"#,
    )
    .bind(Uuid::now_v7())
    .bind(body.name.trim())
    .bind(body.entity_name.trim())
    .bind(body.data_type)
    .bind(body.description)
    .bind(body.offline_source)
    .bind(body.transformation)
    .bind(body.online_enabled)
    .bind(body.online_namespace)
    .bind(body.batch_schedule)
    .bind(body.freshness_sla_minutes)
    .bind(to_json(&body.tags))
    .bind(to_json(&body.samples))
    .bind(if body.samples.is_empty() {
        None
    } else {
        Some(Utc::now())
    })
    .bind(if body.online_enabled && !body.samples.is_empty() {
        Some(Utc::now())
    } else {
        None
    })
    .fetch_one(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?;

    Ok(Json(to_feature(row)))
}

pub async fn update_feature(
    State(state): State<AppState>,
    Path(feature_id): Path<Uuid>,
    Json(body): Json<UpdateFeatureRequest>,
) -> ServiceResult<FeatureDefinition> {
    let Some(current) = load_feature_row(&state.db, feature_id)
        .await
        .map_err(|cause| db_error(&cause))?
    else {
        return Err(not_found("feature not found"));
    };

    let tags = body
        .tags
        .unwrap_or_else(|| deserialize_json(current.tags.clone()));

    let row = query_as::<_, FeatureRow>(
        r#"
		UPDATE ml_features
		SET
			name = $2,
			entity_name = $3,
			data_type = $4,
			description = $5,
			status = $6,
			offline_source = $7,
			transformation = $8,
			online_enabled = $9,
			online_namespace = $10,
			batch_schedule = $11,
			freshness_sla_minutes = $12,
			tags = $13,
			updated_at = NOW()
		WHERE id = $1
		RETURNING
			id,
			name,
			entity_name,
			data_type,
			description,
			status,
			offline_source,
			transformation,
			online_enabled,
			online_namespace,
			batch_schedule,
			freshness_sla_minutes,
			tags,
			samples,
			last_materialized_at,
			last_online_sync_at,
			created_at,
			updated_at
		"#,
    )
    .bind(feature_id)
    .bind(body.name.unwrap_or(current.name))
    .bind(body.entity_name.unwrap_or(current.entity_name))
    .bind(body.data_type.unwrap_or(current.data_type))
    .bind(body.description.unwrap_or(current.description))
    .bind(body.status.unwrap_or(current.status))
    .bind(body.offline_source.unwrap_or(current.offline_source))
    .bind(body.transformation.unwrap_or(current.transformation))
    .bind(body.online_enabled.unwrap_or(current.online_enabled))
    .bind(body.online_namespace.unwrap_or(current.online_namespace))
    .bind(body.batch_schedule.unwrap_or(current.batch_schedule))
    .bind(
        body.freshness_sla_minutes
            .unwrap_or(current.freshness_sla_minutes),
    )
    .bind(to_json(&tags))
    .fetch_one(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?;

    Ok(Json(to_feature(row)))
}

pub async fn materialize_feature(
    State(state): State<AppState>,
    Path(feature_id): Path<Uuid>,
    Json(body): Json<MaterializeFeatureRequest>,
) -> ServiceResult<FeatureDefinition> {
    let Some(current) = load_feature_row(&state.db, feature_id)
        .await
        .map_err(|cause| db_error(&cause))?
    else {
        return Err(not_found("feature not found"));
    };

    let samples = if body.samples.is_empty() {
        deserialize_json(current.samples.clone())
    } else {
        body.samples
    };
    let now = Utc::now();

    let row = query_as::<_, FeatureRow>(
        r#"
		UPDATE ml_features
		SET
			samples = $2,
			last_materialized_at = $3,
			last_online_sync_at = $4,
			updated_at = NOW()
		WHERE id = $1
		RETURNING
			id,
			name,
			entity_name,
			data_type,
			description,
			status,
			offline_source,
			transformation,
			online_enabled,
			online_namespace,
			batch_schedule,
			freshness_sla_minutes,
			tags,
			samples,
			last_materialized_at,
			last_online_sync_at,
			created_at,
			updated_at
		"#,
    )
    .bind(feature_id)
    .bind(to_json(&samples))
    .bind(now)
    .bind(if current.online_enabled {
        Some(now)
    } else {
        current.last_online_sync_at
    })
    .fetch_one(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?;

    Ok(Json(to_feature(row)))
}

pub async fn get_online_feature_snapshot(
    State(state): State<AppState>,
    Path(feature_id): Path<Uuid>,
) -> ServiceResult<OnlineFeatureSnapshot> {
    let Some(feature) = load_feature_row(&state.db, feature_id)
        .await
        .map_err(|cause| db_error(&cause))?
    else {
        return Err(not_found("feature not found"));
    };

    Ok(Json(OnlineFeatureSnapshot {
        feature_id: feature.id,
        namespace: feature.online_namespace,
        source: if feature.online_enabled {
            "online-cache".to_string()
        } else {
            "offline-materialization".to_string()
        },
        values: deserialize_json(feature.samples),
        fetched_at: Utc::now(),
    }))
}
