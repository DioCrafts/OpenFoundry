use std::collections::HashMap;

use crate::models::{
    feature::Coordinate,
    layer::LayerDefinition,
    spatial_index::{ClusterAlgorithm, ClusterRequest, ClusterResponse, ClusterSummary},
};

pub fn cluster(layer: &LayerDefinition, request: &ClusterRequest) -> ClusterResponse {
    match request.algorithm {
        ClusterAlgorithm::Dbscan => dbscan(layer, request.radius_km.unwrap_or(8.0)),
        ClusterAlgorithm::KMeans => kmeans(layer, request.cluster_count.unwrap_or(3).max(1)),
    }
}

fn dbscan(layer: &LayerDefinition, radius_km: f64) -> ClusterResponse {
    let grid_step = (radius_km / 111.0).max(0.05);
    let mut buckets = HashMap::<(i64, i64), Vec<Coordinate>>::new();
    for feature in &layer.features {
        let centroid = feature.geometry.centroid();
        let key = (
            (centroid.lat / grid_step).floor() as i64,
            (centroid.lon / grid_step).floor() as i64,
        );
        buckets.entry(key).or_default().push(centroid);
    }

    let mut outliers = 0usize;
    let mut clusters = Vec::new();
    for (cluster_id, (_bucket, members)) in buckets.into_iter().enumerate() {
        if members.len() < 2 {
            outliers += members.len();
            continue;
        }

        let centroid = average_coordinate(&members);
        clusters.push(ClusterSummary {
            cluster_id: format!("dbscan-{}", cluster_id + 1),
            centroid,
            member_count: members.len(),
            density: members.len() as f64 / radius_km.max(1.0),
        });
    }

    clusters.sort_by(|left, right| right.member_count.cmp(&left.member_count));
    ClusterResponse {
        algorithm: ClusterAlgorithm::Dbscan,
        clusters,
        outliers,
    }
}

fn kmeans(layer: &LayerDefinition, cluster_count: usize) -> ClusterResponse {
    let mut groups = vec![Vec::new(); cluster_count];
    for (index, feature) in layer.features.iter().enumerate() {
        groups[index % cluster_count].push(feature.geometry.centroid());
    }

    let clusters = groups
        .into_iter()
        .enumerate()
        .filter(|(_, members)| !members.is_empty())
        .map(|(index, members)| ClusterSummary {
            cluster_id: format!("kmeans-{}", index + 1),
            centroid: average_coordinate(&members),
            member_count: members.len(),
            density: members.len() as f64 / cluster_count as f64,
        })
        .collect();

    ClusterResponse {
        algorithm: ClusterAlgorithm::KMeans,
        clusters,
        outliers: 0,
    }
}

fn average_coordinate(points: &[Coordinate]) -> Coordinate {
    let (lat_sum, lon_sum) = points.iter().fold((0.0, 0.0), |acc, point| {
        (acc.0 + point.lat, acc.1 + point.lon)
    });
    Coordinate {
        lat: lat_sum / points.len() as f64,
        lon: lon_sum / points.len() as f64,
    }
}
