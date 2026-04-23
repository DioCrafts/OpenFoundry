use chrono::{Duration, Utc};

use crate::{
	domain::{backpressure, connectors, engine},
	models::{
		sink::{BackpressureSnapshot, CepMatch, LiveTailEvent, StateStoreSnapshot, WindowAggregate},
		stream::StreamDefinition,
		topology::{TopologyDefinition, TopologyRunMetrics},
		window::WindowDefinition,
	},
};

pub struct SimulatedTopologyExecution {
	pub metrics: TopologyRunMetrics,
	pub live_tail: Vec<LiveTailEvent>,
	pub cep_matches: Vec<CepMatch>,
	pub aggregate_windows: Vec<WindowAggregate>,
	pub state_snapshot: StateStoreSnapshot,
	pub backpressure_snapshot: BackpressureSnapshot,
	pub started_at: chrono::DateTime<Utc>,
	pub completed_at: chrono::DateTime<Utc>,
}

pub fn simulate_topology_run(
	topology: &TopologyDefinition,
	streams: &[StreamDefinition],
	windows: &[WindowDefinition],
	previous_runs: usize,
) -> SimulatedTopologyExecution {
	let referenced_window = topology
		.nodes
		.iter()
		.find_map(|node| node.window_id)
		.and_then(|window_id| windows.iter().find(|window| window.id == window_id));

	let connector_statuses = connectors::catalog_entries(topology, streams);
	let live_tail = connectors::live_events(topology, streams);
	let cep_matches = engine::cep::simulate_cep_matches(topology.cep_definition.as_ref());
	let aggregate_windows = engine::aggregator::simulate_window_aggregates(referenced_window, topology);
	let backpressure_snapshot = backpressure::simulate_backpressure(
		&topology.backpressure_policy,
		topology.source_stream_ids.len(),
		topology.join_definition.is_some(),
	);
	let join_output_rows = engine::joiner::simulate_join_output(
		topology.join_definition.as_ref(),
		topology.source_stream_ids.len(),
	);

	let input_events = live_tail.len() as i32 * 120 + previous_runs as i32 * 7 + topology.source_stream_ids.len() as i32 * 80;
	let dropped_events = if backpressure_snapshot.status == "throttling" { 9 } else { 2 };
	let throughput_per_second = connector_statuses
		.iter()
		.map(|connector| connector.throughput_per_second)
		.sum::<f32>()
		/ connector_statuses.len().max(1) as f32;
	let state_entries = (aggregate_windows.len() as i32 * 14) + join_output_rows + cep_matches.len() as i32 * 5;
	let state_snapshot = engine::state_store::simulate_state_store(topology, state_entries);
	let output_events = input_events - dropped_events + join_output_rows + aggregate_windows.len() as i32 * 3;

	let metrics = TopologyRunMetrics {
		input_events,
		output_events,
		avg_latency_ms: 82 + topology.source_stream_ids.len() as i32 * 18,
		p95_latency_ms: 145 + topology.source_stream_ids.len() as i32 * 34,
		throughput_per_second,
		dropped_events,
		backpressure_ratio: backpressure_snapshot.queue_depth as f32
			/ backpressure_snapshot.queue_capacity as f32,
		join_output_rows,
		cep_match_count: cep_matches.len() as i32,
		state_entries,
	};

	let completed_at = Utc::now();
	let started_at = completed_at - Duration::seconds(22);

	SimulatedTopologyExecution {
		metrics,
		live_tail,
		cep_matches,
		aggregate_windows,
		state_snapshot,
		backpressure_snapshot,
		started_at,
		completed_at,
	}
}
