use axum::{
    Json,
    extract::{Path, State},
};
use chrono::Utc;

use crate::{
    AppState,
    domain::indexer,
    handlers::{
        ServiceResult, bad_request, db_error, internal_error, load_all_layers, load_layer_row,
        not_found,
    },
    models::{
        ListResponse,
        layer::{CreateLayerRequest, LayerDefinition, UpdateLayerRequest},
        spatial_index::GeospatialOverview,
    },
};

pub async fn get_overview(State(state): State<AppState>) -> ServiceResult<GeospatialOverview> {
    let layers = load_all_layers(&state.db)
        .await
        .map_err(|cause| db_error(&cause))?;
    Ok(Json(indexer::build_overview(&layers)))
}

pub async fn list_layers(
    State(state): State<AppState>,
) -> ServiceResult<ListResponse<LayerDefinition>> {
    let layers = load_all_layers(&state.db)
        .await
        .map_err(|cause| db_error(&cause))?;
    Ok(Json(ListResponse { items: layers }))
}

pub async fn create_layer(
    State(state): State<AppState>,
    Json(request): Json<CreateLayerRequest>,
) -> ServiceResult<LayerDefinition> {
    if request.name.trim().is_empty() {
        return Err(bad_request("layer name is required"));
    }
    if request.features.is_empty() {
        return Err(bad_request("layer requires at least one feature"));
    }

    let id = uuid::Uuid::now_v7();
    let now = Utc::now();
    let style =
        serde_json::to_value(&request.style).map_err(|cause| internal_error(cause.to_string()))?;
    let features = serde_json::to_value(&request.features)
        .map_err(|cause| internal_error(cause.to_string()))?;
    let tags =
        serde_json::to_value(&request.tags).map_err(|cause| internal_error(cause.to_string()))?;

    sqlx::query(
		"INSERT INTO geospatial_layers (id, name, description, source_kind, source_dataset, geometry_type, style, features, tags, indexed, created_at, updated_at)
		 VALUES ($1, $2, $3, $4, $5, $6, $7::jsonb, $8::jsonb, $9::jsonb, $10, $11, $12)",
	)
	.bind(id)
	.bind(request.name)
	.bind(request.description)
	.bind(request.source_kind.as_str())
	.bind(request.source_dataset)
	.bind(request.geometry_type.as_str())
	.bind(style)
	.bind(features)
	.bind(tags)
	.bind(request.indexed)
	.bind(now)
	.bind(now)
	.execute(&state.db)
	.await
	.map_err(|cause| db_error(&cause))?;

    let row = load_layer_row(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?
        .ok_or_else(|| internal_error("created layer could not be reloaded"))?;
    let layer =
        LayerDefinition::try_from(row).map_err(|cause| internal_error(cause.to_string()))?;
    Ok(Json(layer))
}

pub async fn update_layer(
    Path(id): Path<uuid::Uuid>,
    State(state): State<AppState>,
    Json(request): Json<UpdateLayerRequest>,
) -> ServiceResult<LayerDefinition> {
    let existing = load_layer_row(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?
        .ok_or_else(|| not_found("layer not found"))?;
    let mut layer =
        LayerDefinition::try_from(existing).map_err(|cause| internal_error(cause.to_string()))?;

    if let Some(name) = request.name {
        if name.trim().is_empty() {
            return Err(bad_request("layer name cannot be empty"));
        }
        layer.name = name;
    }
    if let Some(description) = request.description {
        layer.description = description;
    }
    if let Some(source_kind) = request.source_kind {
        layer.source_kind = source_kind;
    }
    if let Some(source_dataset) = request.source_dataset {
        layer.source_dataset = source_dataset;
    }
    if let Some(geometry_type) = request.geometry_type {
        layer.geometry_type = geometry_type;
    }
    if let Some(style) = request.style {
        layer.style = style;
    }
    if let Some(features) = request.features {
        if features.is_empty() {
            return Err(bad_request("layer requires at least one feature"));
        }
        layer.features = features;
    }
    if let Some(tags) = request.tags {
        layer.tags = tags;
    }
    if let Some(indexed) = request.indexed {
        layer.indexed = indexed;
    }

    let now = Utc::now();
    let style =
        serde_json::to_value(&layer.style).map_err(|cause| internal_error(cause.to_string()))?;
    let features =
        serde_json::to_value(&layer.features).map_err(|cause| internal_error(cause.to_string()))?;
    let tags =
        serde_json::to_value(&layer.tags).map_err(|cause| internal_error(cause.to_string()))?;

    sqlx::query(
        "UPDATE geospatial_layers
		 SET name = $2,
		     description = $3,
		     source_kind = $4,
		     source_dataset = $5,
		     geometry_type = $6,
		     style = $7::jsonb,
		     features = $8::jsonb,
		     tags = $9::jsonb,
		     indexed = $10,
		     updated_at = $11
		 WHERE id = $1",
    )
    .bind(id)
    .bind(&layer.name)
    .bind(&layer.description)
    .bind(layer.source_kind.as_str())
    .bind(&layer.source_dataset)
    .bind(layer.geometry_type.as_str())
    .bind(style)
    .bind(features)
    .bind(tags)
    .bind(layer.indexed)
    .bind(now)
    .execute(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?;

    let row = load_layer_row(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?
        .ok_or_else(|| internal_error("updated layer could not be reloaded"))?;
    let layer =
        LayerDefinition::try_from(row).map_err(|cause| internal_error(cause.to_string()))?;
    Ok(Json(layer))
}
