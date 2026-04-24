use crate::models::{
    feature::Coordinate,
    spatial_index::{IsochronePoint, RouteMode, RouteRequest, RouteResponse},
};

pub fn route(request: &RouteRequest) -> RouteResponse {
    let distance_km = request.origin.distance_km(request.destination);
    let speed_kmh = match request.mode {
        RouteMode::Drive => 58.0,
        RouteMode::Bike => 18.0,
        RouteMode::Walk => 5.0,
    };
    let duration_min = ((distance_km / speed_kmh) * 60.0).ceil().max(1.0) as u32;
    let midpoint = Coordinate {
        lat: (request.origin.lat + request.destination.lat) / 2.0 + 0.04,
        lon: (request.origin.lon + request.destination.lon) / 2.0 - 0.03,
    };
    let max_minutes = request.max_minutes.unwrap_or(duration_min.max(20));
    let step = (max_minutes / 3).max(5);

    RouteResponse {
        mode: request.mode,
        distance_km,
        duration_min,
        polyline: vec![request.origin, midpoint, request.destination],
        isochrone: vec![
            IsochronePoint {
                label: "10 min".to_string(),
                coordinate: offset(request.origin, 0.03, 0.01),
                eta_minutes: step,
            },
            IsochronePoint {
                label: "20 min".to_string(),
                coordinate: offset(request.origin, -0.02, 0.04),
                eta_minutes: step * 2,
            },
            IsochronePoint {
                label: format!("{} min", step * 3),
                coordinate: offset(request.origin, 0.01, -0.05),
                eta_minutes: step * 3,
            },
        ],
    }
}

fn offset(origin: Coordinate, lat_delta: f64, lon_delta: f64) -> Coordinate {
    Coordinate {
        lat: origin.lat + lat_delta,
        lon: origin.lon + lon_delta,
    }
}
