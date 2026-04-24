use crate::{
    domain::engine::aggregation,
    models::{layer::LayerDefinition, spatial_index::VectorTileResponse},
};

pub fn vector_tile(layer: &LayerDefinition) -> VectorTileResponse {
    VectorTileResponse {
        layer_id: layer.id,
        layer_name: layer.name.clone(),
        tile_url_template: format!(
            "/api/v1/geospatial/tiles/{}?z={{z}}&x={{x}}&y={{y}}",
            layer.id
        ),
        format: "mvt".to_string(),
        zoom_range: [4, 14],
        h3_bins: aggregation::hex_aggregate(layer),
        feature_count: layer.features.len(),
    }
}
