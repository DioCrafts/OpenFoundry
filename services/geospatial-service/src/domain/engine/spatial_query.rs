use crate::models::{
    feature::{Bounds, Coordinate, MapFeature},
    layer::LayerDefinition,
    spatial_index::{
        SpatialOperation, SpatialQueryRequest, SpatialQueryResponse, SpatialQuerySummary,
    },
};

pub fn execute(layer: &LayerDefinition, request: &SpatialQueryRequest) -> SpatialQueryResponse {
    let mut features = match request.operation {
        SpatialOperation::Within => within_query(layer, request.bounds),
        SpatialOperation::Intersects => intersects_query(layer, request.bounds),
        SpatialOperation::Nearest => {
            nearest_query(layer, request.point, request.limit.unwrap_or(5))
        }
        SpatialOperation::Buffer => {
            buffer_query(layer, request.point, request.radius_km.unwrap_or(10.0))
        }
    };

    let nearest_distance_km = if matches!(request.operation, SpatialOperation::Nearest) {
        request.point.and_then(|point| {
            features
                .first()
                .map(|feature| point.distance_km(feature.geometry.centroid()))
        })
    } else {
        None
    };

    let buffer_ring = match request.operation {
        SpatialOperation::Buffer => request
            .point
            .map(|center| buffer_ring(center, request.radius_km.unwrap_or(10.0)))
            .unwrap_or_default(),
        _ => Vec::new(),
    };

    features.truncate(request.limit.unwrap_or(25));

    SpatialQueryResponse {
        operation: request.operation,
        matched_features: features.clone(),
        summary: SpatialQuerySummary {
            matched_count: features.len(),
            query_time_ms: 18 + (features.len() as i32 * 4),
            nearest_distance_km,
            indexed: layer.indexed,
        },
        buffer_ring,
    }
}

fn within_query(layer: &LayerDefinition, bounds: Option<Bounds>) -> Vec<MapFeature> {
    let Some(bounds) = bounds else {
        return layer.features.clone();
    };

    layer
        .features
        .iter()
        .filter(|feature| bounds.contains(feature.geometry.centroid()))
        .cloned()
        .collect()
}

fn intersects_query(layer: &LayerDefinition, bounds: Option<Bounds>) -> Vec<MapFeature> {
    let Some(bounds) = bounds else {
        return layer.features.clone();
    };

    layer
        .features
        .iter()
        .filter(|feature| feature.geometry.bounds().intersects(bounds))
        .cloned()
        .collect()
}

fn nearest_query(
    layer: &LayerDefinition,
    point: Option<Coordinate>,
    limit: usize,
) -> Vec<MapFeature> {
    let Some(point) = point else {
        return layer.features.iter().take(limit).cloned().collect();
    };

    let mut features = layer.features.clone();
    features.sort_by(|left, right| {
        point
            .distance_km(left.geometry.centroid())
            .partial_cmp(&point.distance_km(right.geometry.centroid()))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    features.into_iter().take(limit).collect()
}

fn buffer_query(
    layer: &LayerDefinition,
    point: Option<Coordinate>,
    radius_km: f64,
) -> Vec<MapFeature> {
    let Some(point) = point else {
        return Vec::new();
    };

    layer
        .features
        .iter()
        .filter(|feature| point.distance_km(feature.geometry.centroid()) <= radius_km)
        .cloned()
        .collect()
}

fn buffer_ring(center: Coordinate, radius_km: f64) -> Vec<Coordinate> {
    let delta = radius_km / 111.0;
    vec![
        Coordinate {
            lat: center.lat + delta,
            lon: center.lon,
        },
        Coordinate {
            lat: center.lat + delta / 2.0,
            lon: center.lon + delta,
        },
        Coordinate {
            lat: center.lat - delta / 2.0,
            lon: center.lon + delta,
        },
        Coordinate {
            lat: center.lat - delta,
            lon: center.lon,
        },
        Coordinate {
            lat: center.lat - delta / 2.0,
            lon: center.lon - delta,
        },
        Coordinate {
            lat: center.lat + delta / 2.0,
            lon: center.lon - delta,
        },
        Coordinate {
            lat: center.lat + delta,
            lon: center.lon,
        },
    ]
}
