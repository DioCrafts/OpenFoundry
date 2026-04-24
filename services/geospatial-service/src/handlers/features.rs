use axum::{Json, extract::State};

use crate::{
    AppState,
    domain::engine::{clustering, routing, spatial_query},
    handlers::{ServiceResult, bad_request, db_error, load_all_layers, not_found},
    models::spatial_index::{
        ClusterRequest, ClusterResponse, RouteRequest, RouteResponse, SpatialOperation,
        SpatialQueryRequest, SpatialQueryResponse,
    },
};

pub async fn query_features(
    State(state): State<AppState>,
    Json(request): Json<SpatialQueryRequest>,
) -> ServiceResult<SpatialQueryResponse> {
    if matches!(
        request.operation,
        SpatialOperation::Within | SpatialOperation::Intersects
    ) && request.bounds.is_none()
    {
        return Err(bad_request(
            "bounds are required for within/intersects queries",
        ));
    }
    if matches!(
        request.operation,
        SpatialOperation::Nearest | SpatialOperation::Buffer
    ) && request.point.is_none()
    {
        return Err(bad_request("point is required for nearest/buffer queries"));
    }

    let layers = load_all_layers(&state.db)
        .await
        .map_err(|cause| db_error(&cause))?;
    let layer = layers
        .into_iter()
        .find(|layer| layer.id == request.layer_id)
        .ok_or_else(|| not_found("layer not found"))?;
    Ok(Json(spatial_query::execute(&layer, &request)))
}

pub async fn cluster_features(
    State(state): State<AppState>,
    Json(request): Json<ClusterRequest>,
) -> ServiceResult<ClusterResponse> {
    let layers = load_all_layers(&state.db)
        .await
        .map_err(|cause| db_error(&cause))?;
    let layer = layers
        .into_iter()
        .find(|layer| layer.id == request.layer_id)
        .ok_or_else(|| not_found("layer not found"))?;
    Ok(Json(clustering::cluster(&layer, &request)))
}

pub async fn route_features(
    State(_state): State<AppState>,
    Json(request): Json<RouteRequest>,
) -> ServiceResult<RouteResponse> {
    if request.origin == request.destination {
        return Err(bad_request("origin and destination must differ"));
    }
    Ok(Json(routing::route(&request)))
}
