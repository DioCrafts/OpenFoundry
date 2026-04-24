use crate::models::{layer::LayerDefinition, spatial_index::GeospatialOverview};

pub fn build_overview(layers: &[LayerDefinition]) -> GeospatialOverview {
    GeospatialOverview {
        layer_count: layers.len(),
        indexed_layers: layers.iter().filter(|layer| layer.indexed).count(),
        total_features: layers.iter().map(|layer| layer.features.len()).sum(),
        tile_ready_layers: layers
            .iter()
            .filter(|layer| layer.indexed && !layer.features.is_empty())
            .count(),
        supported_operations: vec![
            "within".to_string(),
            "intersects".to_string(),
            "nearest".to_string(),
            "buffer".to_string(),
            "dbscan".to_string(),
            "kmeans".to_string(),
            "vector_tiles".to_string(),
            "routing".to_string(),
        ],
    }
}
