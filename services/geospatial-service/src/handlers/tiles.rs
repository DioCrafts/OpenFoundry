use axum::{
    Json,
    extract::{Path, State},
};

use crate::{
    AppState,
    domain::tile_server,
    handlers::{ServiceResult, db_error, internal_error, load_layer_row, not_found},
    models::{layer::LayerDefinition, spatial_index::VectorTileResponse},
};

pub async fn get_vector_tile(
    Path(id): Path<uuid::Uuid>,
    State(state): State<AppState>,
) -> ServiceResult<VectorTileResponse> {
    let row = load_layer_row(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?
        .ok_or_else(|| not_found("layer not found"))?;
    let layer =
        LayerDefinition::try_from(row).map_err(|cause| internal_error(cause.to_string()))?;
    Ok(Json(tile_server::vector_tile(&layer)))
}
