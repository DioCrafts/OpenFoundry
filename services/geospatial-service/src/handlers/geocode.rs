use axum::{Json, extract::State};

use crate::{
    AppState,
    domain::geocoding,
    handlers::{ServiceResult, bad_request},
    models::spatial_index::{GeocodeRequest, GeocodeResponse, ReverseGeocodeRequest},
};

pub async fn forward_geocode(
    State(_state): State<AppState>,
    Json(request): Json<GeocodeRequest>,
) -> ServiceResult<GeocodeResponse> {
    if request.address.trim().is_empty() {
        return Err(bad_request("address is required"));
    }
    Ok(Json(geocoding::forward(&request.address)))
}

pub async fn reverse_geocode(
    State(_state): State<AppState>,
    Json(request): Json<ReverseGeocodeRequest>,
) -> ServiceResult<GeocodeResponse> {
    Ok(Json(geocoding::reverse(request.coordinate)))
}
