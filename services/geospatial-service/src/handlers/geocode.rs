use axum::{extract::State, Json};

use crate::{
	domain::geocoding,
	handlers::{bad_request, ServiceResult},
	models::spatial_index::{GeocodeRequest, GeocodeResponse, ReverseGeocodeRequest},
	AppState,
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
