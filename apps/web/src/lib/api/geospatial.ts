import api from './client';

export interface ListResponse<T> {
	items: T[];
}

export interface Coordinate {
	lat: number;
	lon: number;
}

export interface Bounds {
	min_lat: number;
	min_lon: number;
	max_lat: number;
	max_lon: number;
}

export type GeometryType = 'point' | 'line_string' | 'polygon';
export type LayerSourceKind = 'dataset' | 'vector_tile' | 'reference';
export type SpatialOperation = 'within' | 'intersects' | 'nearest' | 'buffer';
export type ClusterAlgorithm = 'dbscan' | 'kmeans';
export type RouteMode = 'drive' | 'bike' | 'walk';

export type Geometry =
	| { type: 'point'; coordinates: Coordinate }
	| { type: 'line_string'; coordinates: Coordinate[] }
	| { type: 'polygon'; coordinates: Coordinate[] };

export interface MapFeature {
	id: string;
	label: string;
	geometry: Geometry;
	properties: Record<string, unknown>;
}

export interface LayerStyle {
	color: string;
	opacity: number;
	radius: number;
	line_width: number;
	heatmap_intensity: number;
	cluster_color: string;
	show_labels: boolean;
}

export interface LayerDefinition {
	id: string;
	name: string;
	description: string;
	source_kind: LayerSourceKind;
	source_dataset: string;
	geometry_type: GeometryType;
	style: LayerStyle;
	features: MapFeature[];
	tags: string[];
	indexed: boolean;
	created_at: string;
	updated_at: string;
}

export interface SpatialQueryRequest {
	layer_id: string;
	operation: SpatialOperation;
	bounds?: Bounds;
	point?: Coordinate;
	radius_km?: number;
	limit?: number;
}

export interface SpatialQuerySummary {
	matched_count: number;
	query_time_ms: number;
	nearest_distance_km: number | null;
	indexed: boolean;
}

export interface SpatialQueryResponse {
	operation: SpatialOperation;
	matched_features: MapFeature[];
	summary: SpatialQuerySummary;
	buffer_ring: Coordinate[];
}

export interface ClusterRequest {
	layer_id: string;
	algorithm: ClusterAlgorithm;
	cluster_count?: number;
	radius_km?: number;
}

export interface ClusterSummary {
	cluster_id: string;
	centroid: Coordinate;
	member_count: number;
	density: number;
}

export interface ClusterResponse {
	algorithm: ClusterAlgorithm;
	clusters: ClusterSummary[];
	outliers: number;
}

export interface TileHexBin {
	cell_id: string;
	centroid: Coordinate;
	count: number;
	intensity: number;
}

export interface VectorTileResponse {
	layer_id: string;
	layer_name: string;
	tile_url_template: string;
	format: string;
	zoom_range: [number, number];
	h3_bins: TileHexBin[];
	feature_count: number;
}

export interface GeocodeRequest {
	address: string;
}

export interface ReverseGeocodeRequest {
	coordinate: Coordinate;
}

export interface GeocodeResponse {
	address: string;
	coordinate: Coordinate;
	confidence: number;
	source: string;
}

export interface RouteRequest {
	origin: Coordinate;
	destination: Coordinate;
	mode: RouteMode;
	max_minutes?: number;
}

export interface IsochronePoint {
	label: string;
	coordinate: Coordinate;
	eta_minutes: number;
}

export interface RouteResponse {
	mode: RouteMode;
	distance_km: number;
	duration_min: number;
	polyline: Coordinate[];
	isochrone: IsochronePoint[];
}

export interface GeospatialOverview {
	layer_count: number;
	indexed_layers: number;
	total_features: number;
	tile_ready_layers: number;
	supported_operations: string[];
}

export function getOverview() {
	return api.get<GeospatialOverview>('/geospatial/overview');
}

export function listLayers() {
	return api.get<ListResponse<LayerDefinition>>('/geospatial/layers');
}

export function createLayer(body: {
	name: string;
	description?: string;
	source_kind: LayerSourceKind;
	source_dataset: string;
	geometry_type: GeometryType;
	style?: LayerStyle;
	features: MapFeature[];
	tags?: string[];
	indexed?: boolean;
}) {
	return api.post<LayerDefinition>('/geospatial/layers', body);
}

export function updateLayer(
	id: string,
	body: Partial<{
		name: string;
		description: string;
		source_kind: LayerSourceKind;
		source_dataset: string;
		geometry_type: GeometryType;
		style: LayerStyle;
		features: MapFeature[];
		tags: string[];
		indexed: boolean;
	}>,
) {
	return api.patch<LayerDefinition>(`/geospatial/layers/${id}`, body);
}

export function runSpatialQuery(body: SpatialQueryRequest) {
	return api.post<SpatialQueryResponse>('/geospatial/query', body);
}

export function clusterFeatures(body: ClusterRequest) {
	return api.post<ClusterResponse>('/geospatial/clusters', body);
}

export function routeFeatures(body: RouteRequest) {
	return api.post<RouteResponse>('/geospatial/routes', body);
}

export function geocodeAddress(body: GeocodeRequest) {
	return api.post<GeocodeResponse>('/geospatial/geocode', body);
}

export function reverseGeocode(body: ReverseGeocodeRequest) {
	return api.post<GeocodeResponse>('/geospatial/reverse-geocode', body);
}

export function getVectorTile(id: string) {
	return api.get<VectorTileResponse>(`/geospatial/tiles/${id}`);
}
