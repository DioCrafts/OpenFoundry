use axum::{
    Json,
    extract::{Path, State},
};
use uuid::Uuid;

use crate::{
    AppState,
    domain::slate,
    handlers::{ServiceResult, bad_request, load_app, persist_app},
    models::app::{ImportSlatePackageRequest, SlatePackageResponse, SlateRoundTripResponse},
};

pub async fn export_slate_package(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ServiceResult<Json<SlatePackageResponse>> {
    let app = load_app(&state, id).await?;
    Ok(Json(slate::build_slate_package(&app)))
}

pub async fn import_slate_package(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(request): Json<ImportSlatePackageRequest>,
) -> ServiceResult<Json<SlateRoundTripResponse>> {
    let mut app = load_app(&state, id).await?;
    let mut response = slate::apply_slate_round_trip(&mut app, request).map_err(bad_request)?;
    let persisted = persist_app(&state, &response.app).await?;
    response.app = persisted.clone();
    response.slate_package = slate::build_slate_package(&persisted);
    Ok(Json(response))
}
