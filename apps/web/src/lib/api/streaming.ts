import api from './client';

export interface ListResponse<T> {
	data: T[];
}

export interface StreamingOverview {
	stream_count: number;
	active_topology_count: number;
	window_count: number;
	connector_count: number;
	running_topology_count: number;
	backpressured_topology_count: number;
	live_event_count: number;
}

export interface StreamField {
	name: string;
	data_type: string;
	nullable: boolean;
	semantic_role: string;
}

export interface StreamSchema {
	fields: StreamField[];
	primary_key: string | null;
	watermark_field: string | null;
}

export interface ConnectorBinding {
	connector_type: string;
	endpoint: string;
	format: string;
	config: Record<string, unknown>;
}

export interface StreamDefinition {
	id: string;
	name: string;
	description: string;
	status: string;
	schema: StreamSchema;
	source_binding: ConnectorBinding;
	retention_hours: number;
	created_at: string;
	updated_at: string;
}

export interface WindowDefinition {
	id: string;
	name: string;
	description: string;
	status: string;
	window_type: string;
	duration_seconds: number;
	slide_seconds: number;
	session_gap_seconds: number;
	allowed_lateness_seconds: number;
	aggregation_keys: string[];
	measure_fields: string[];
	created_at: string;
	updated_at: string;
}

export interface TopologyNode {
	id: string;
	label: string;
	node_type: string;
	stream_id: string | null;
	window_id: string | null;
	config: Record<string, unknown>;
}

export interface TopologyEdge {
	source_node_id: string;
	target_node_id: string;
	label: string;
}

export interface JoinDefinition {
	join_type: string;
	left_stream_id: string;
	right_stream_id: string;
	table_name: string;
	key_fields: string[];
	window_seconds: number;
}

export interface CepDefinition {
	pattern_name: string;
	sequence: string[];
	within_seconds: number;
	output_stream: string;
}

export interface BackpressurePolicy {
	max_in_flight: number;
	queue_capacity: number;
	throttle_strategy: string;
}

export interface TopologyDefinition {
	id: string;
	name: string;
	description: string;
	status: string;
	nodes: TopologyNode[];
	edges: TopologyEdge[];
	join_definition: JoinDefinition | null;
	cep_definition: CepDefinition | null;
	backpressure_policy: BackpressurePolicy;
	source_stream_ids: string[];
	sink_bindings: ConnectorBinding[];
	state_backend: string;
	created_at: string;
	updated_at: string;
}

export interface ConnectorCatalogEntry {
	connector_type: string;
	direction: string;
	endpoint: string;
	status: string;
	backlog: number;
	throughput_per_second: number;
	details: Record<string, unknown>;
}

export interface BackpressureSnapshot {
	queue_depth: number;
	queue_capacity: number;
	lag_ms: number;
	throttle_factor: number;
	status: string;
}

export interface StateStoreSnapshot {
	backend: string;
	namespace: string;
	key_count: number;
	disk_usage_mb: number;
	checkpoint_count: number;
	last_checkpoint_at: string;
}

export interface WindowAggregate {
	window_name: string;
	window_type: string;
	bucket_start: string;
	bucket_end: string;
	group_key: string;
	measure_name: string;
	value: number;
}

export interface LiveTailEvent {
	id: string;
	topology_id: string;
	stream_name: string;
	connector_type: string;
	payload: Record<string, unknown>;
	event_time: string;
	processing_time: string;
	tags: string[];
}

export interface CepMatch {
	pattern_name: string;
	matched_sequence: string[];
	confidence: number;
	detected_at: string;
}

export interface TopologyRunMetrics {
	input_events: number;
	output_events: number;
	avg_latency_ms: number;
	p95_latency_ms: number;
	throughput_per_second: number;
	dropped_events: number;
	backpressure_ratio: number;
	join_output_rows: number;
	cep_match_count: number;
	state_entries: number;
}

export interface TopologyRun {
	id: string;
	topology_id: string;
	status: string;
	metrics: TopologyRunMetrics;
	aggregate_windows: WindowAggregate[];
	live_tail: LiveTailEvent[];
	cep_matches: CepMatch[];
	state_snapshot: StateStoreSnapshot;
	backpressure_snapshot: BackpressureSnapshot;
	started_at: string;
	completed_at: string | null;
	created_at: string;
	updated_at: string;
}

export interface TopologyRuntimeSnapshot {
	topology: TopologyDefinition;
	latest_run: TopologyRun | null;
	connector_statuses: ConnectorCatalogEntry[];
	latest_events: LiveTailEvent[];
	latest_matches: CepMatch[];
}

export interface LiveTailResponse {
	events: LiveTailEvent[];
	matches: CepMatch[];
}

export function getOverview() {
	return api.get<StreamingOverview>('/streaming/overview');
}

export function listStreams() {
	return api.get<ListResponse<StreamDefinition>>('/streaming/streams');
}

export function createStream(body: {
	name: string;
	description?: string;
	status?: string;
	schema?: StreamSchema;
	source_binding?: ConnectorBinding;
	retention_hours?: number;
}) {
	return api.post<StreamDefinition>('/streaming/streams', body);
}

export function updateStream(id: string, body: {
	name?: string;
	description?: string;
	status?: string;
	schema?: StreamSchema;
	source_binding?: ConnectorBinding;
	retention_hours?: number;
}) {
	return api.patch<StreamDefinition>(`/streaming/streams/${id}`, body);
}

export function listWindows() {
	return api.get<ListResponse<WindowDefinition>>('/streaming/windows');
}

export function createWindow(body: {
	name: string;
	description?: string;
	status?: string;
	window_type?: string;
	duration_seconds?: number;
	slide_seconds?: number;
	session_gap_seconds?: number;
	allowed_lateness_seconds?: number;
	aggregation_keys: string[];
	measure_fields: string[];
}) {
	return api.post<WindowDefinition>('/streaming/windows', body);
}

export function updateWindow(id: string, body: {
	name?: string;
	description?: string;
	status?: string;
	window_type?: string;
	duration_seconds?: number;
	slide_seconds?: number;
	session_gap_seconds?: number;
	allowed_lateness_seconds?: number;
	aggregation_keys?: string[];
	measure_fields?: string[];
}) {
	return api.patch<WindowDefinition>(`/streaming/windows/${id}`, body);
}

export function listTopologies() {
	return api.get<ListResponse<TopologyDefinition>>('/streaming/topologies');
}

export function createTopology(body: {
	name: string;
	description?: string;
	status?: string;
	nodes: TopologyNode[];
	edges: TopologyEdge[];
	join_definition?: JoinDefinition | null;
	cep_definition?: CepDefinition | null;
	backpressure_policy?: BackpressurePolicy;
	source_stream_ids: string[];
	sink_bindings: ConnectorBinding[];
	state_backend?: string;
}) {
	return api.post<TopologyDefinition>('/streaming/topologies', body);
}

export function updateTopology(id: string, body: {
	name?: string;
	description?: string;
	status?: string;
	nodes?: TopologyNode[];
	edges?: TopologyEdge[];
	join_definition?: JoinDefinition | null;
	cep_definition?: CepDefinition | null;
	backpressure_policy?: BackpressurePolicy;
	source_stream_ids?: string[];
	sink_bindings?: ConnectorBinding[];
	state_backend?: string;
}) {
	return api.patch<TopologyDefinition>(`/streaming/topologies/${id}`, body);
}

export function runTopology(id: string) {
	return api.post<TopologyRun>(`/streaming/topologies/${id}/run`, {});
}

export function getRuntime(id: string) {
	return api.get<TopologyRuntimeSnapshot>(`/streaming/topologies/${id}/runtime`);
}

export function listConnectors() {
	return api.get<ListResponse<ConnectorCatalogEntry>>('/streaming/connectors');
}

export function getLiveTail() {
	return api.get<LiveTailResponse>('/streaming/live-tail');
}
