use std::collections::HashMap;

use crate::models::{feature::Coordinate, layer::LayerDefinition, spatial_index::TileHexBin};

pub fn hex_aggregate(layer: &LayerDefinition) -> Vec<TileHexBin> {
    let mut bins = HashMap::<String, (usize, f64, f64)>::new();
    for feature in &layer.features {
        let centroid = feature.geometry.centroid();
        let key = format!(
            "{}:{}",
            (centroid.lat * 10.0).round() / 10.0,
            (centroid.lon * 10.0).round() / 10.0
        );
        let entry = bins.entry(key).or_insert((0, 0.0, 0.0));
        entry.0 += 1;
        entry.1 += centroid.lat;
        entry.2 += centroid.lon;
    }

    let max_count = bins.values().map(|(count, _, _)| *count).max().unwrap_or(1) as f64;
    let mut result = bins
        .into_iter()
        .map(|(cell_id, (count, lat_sum, lon_sum))| TileHexBin {
            cell_id,
            centroid: Coordinate {
                lat: lat_sum / count as f64,
                lon: lon_sum / count as f64,
            },
            count,
            intensity: count as f64 / max_count,
        })
        .collect::<Vec<_>>();

    result.sort_by(|left, right| right.count.cmp(&left.count));
    result
}
