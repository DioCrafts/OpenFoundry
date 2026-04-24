use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::feature::{Bounds, Coordinate, MapFeature};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SpatialOperation {
    Within,
    Intersects,
    Nearest,
    Buffer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpatialQueryRequest {
    pub layer_id: Uuid,
    pub operation: SpatialOperation,
    #[serde(default)]
    pub bounds: Option<Bounds>,
    #[serde(default)]
    pub point: Option<Coordinate>,
    #[serde(default)]
    pub radius_km: Option<f64>,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpatialQuerySummary {
    pub matched_count: usize,
    pub query_time_ms: i32,
    pub nearest_distance_km: Option<f64>,
    pub indexed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpatialQueryResponse {
    pub operation: SpatialOperation,
    pub matched_features: Vec<MapFeature>,
    pub summary: SpatialQuerySummary,
    pub buffer_ring: Vec<Coordinate>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ClusterAlgorithm {
    Dbscan,
    KMeans,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterRequest {
    pub layer_id: Uuid,
    pub algorithm: ClusterAlgorithm,
    #[serde(default)]
    pub cluster_count: Option<usize>,
    #[serde(default)]
    pub radius_km: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterSummary {
    pub cluster_id: String,
    pub centroid: Coordinate,
    pub member_count: usize,
    pub density: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterResponse {
    pub algorithm: ClusterAlgorithm,
    pub clusters: Vec<ClusterSummary>,
    pub outliers: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TileHexBin {
    pub cell_id: String,
    pub centroid: Coordinate,
    pub count: usize,
    pub intensity: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorTileResponse {
    pub layer_id: Uuid,
    pub layer_name: String,
    pub tile_url_template: String,
    pub format: String,
    pub zoom_range: [u8; 2],
    pub h3_bins: Vec<TileHexBin>,
    pub feature_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeocodeRequest {
    pub address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReverseGeocodeRequest {
    pub coordinate: Coordinate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeocodeResponse {
    pub address: String,
    pub coordinate: Coordinate,
    pub confidence: f64,
    pub source: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RouteMode {
    Drive,
    Bike,
    Walk,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteRequest {
    pub origin: Coordinate,
    pub destination: Coordinate,
    pub mode: RouteMode,
    #[serde(default)]
    pub max_minutes: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IsochronePoint {
    pub label: String,
    pub coordinate: Coordinate,
    pub eta_minutes: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteResponse {
    pub mode: RouteMode,
    pub distance_km: f64,
    pub duration_min: u32,
    pub polyline: Vec<Coordinate>,
    pub isochrone: Vec<IsochronePoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeospatialOverview {
    pub layer_count: usize,
    pub indexed_layers: usize,
    pub total_features: usize,
    pub tile_ready_layers: usize,
    pub supported_operations: Vec<String>,
}
