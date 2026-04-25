<script lang="ts">
	import type { StreamingOverview, TopologyDefinition, TopologyRuntimeSnapshot } from '$lib/api/streaming';

	interface Props {
		overview: StreamingOverview | null;
		topologies: TopologyDefinition[];
		selectedTopologyId: string;
		runtime: TopologyRuntimeSnapshot | null;
		busy?: boolean;
		onSelectTopology?: (topologyId: string) => void;
		onRun?: () => void;
	}

	let { overview, topologies, selectedTopologyId, runtime, busy = false, onSelectTopology, onRun }: Props = $props();
</script>

<section class="rounded-[28px] border border-slate-200 bg-white p-5 shadow-sm dark:border-slate-800 dark:bg-slate-950">
	<div class="flex flex-wrap items-start justify-between gap-3">
		<div>
			<div class="text-[11px] font-semibold uppercase tracking-[0.28em] text-slate-500">Stream Monitor</div>
			<h2 class="mt-2 text-xl font-semibold text-slate-900 dark:text-slate-100">Topology runtime, throughput, latency, and state health</h2>
		</div>
		<button class="rounded-full bg-slate-950 px-3 py-1.5 text-sm font-medium text-white hover:bg-slate-800 disabled:opacity-60 dark:bg-slate-100 dark:text-slate-950" onclick={() => onRun?.()} disabled={busy || !selectedTopologyId}>Run selected</button>
	</div>

	<div class="mt-5 grid gap-4 md:grid-cols-2 xl:grid-cols-7">
		<div class="rounded-2xl bg-slate-950 px-4 py-4 text-white">
			<div class="text-[11px] uppercase tracking-[0.24em] text-slate-300">Streams</div>
			<div class="mt-2 text-3xl font-semibold">{overview?.stream_count ?? 0}</div>
		</div>
		<div class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-4 dark:border-slate-800 dark:bg-slate-900"><div class="text-[11px] uppercase tracking-[0.24em] text-slate-500">Topologies</div><div class="mt-2 text-3xl font-semibold text-slate-900 dark:text-slate-100">{overview?.active_topology_count ?? 0}</div></div>
		<div class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-4 dark:border-slate-800 dark:bg-slate-900"><div class="text-[11px] uppercase tracking-[0.24em] text-slate-500">Windows</div><div class="mt-2 text-3xl font-semibold text-slate-900 dark:text-slate-100">{overview?.window_count ?? 0}</div></div>
		<div class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-4 dark:border-slate-800 dark:bg-slate-900"><div class="text-[11px] uppercase tracking-[0.24em] text-slate-500">Connectors</div><div class="mt-2 text-3xl font-semibold text-slate-900 dark:text-slate-100">{overview?.connector_count ?? 0}</div></div>
		<div class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-4 dark:border-slate-800 dark:bg-slate-900"><div class="text-[11px] uppercase tracking-[0.24em] text-slate-500">Backpressured</div><div class="mt-2 text-3xl font-semibold text-slate-900 dark:text-slate-100">{overview?.backpressured_topology_count ?? 0}</div></div>
		<div class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-4 dark:border-slate-800 dark:bg-slate-900"><div class="text-[11px] uppercase tracking-[0.24em] text-slate-500">Live Events</div><div class="mt-2 text-3xl font-semibold text-slate-900 dark:text-slate-100">{overview?.live_event_count ?? 0}</div></div>
		<div class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-4 dark:border-slate-800 dark:bg-slate-900"><div class="text-[11px] uppercase tracking-[0.24em] text-slate-500">Running</div><div class="mt-2 text-3xl font-semibold text-slate-900 dark:text-slate-100">{overview?.running_topology_count ?? 0}</div></div>
	</div>

	<div class="mt-5 grid gap-5 xl:grid-cols-[minmax(0,0.72fr)_minmax(0,1.28fr)]">
		<div class="space-y-3">
			{#if topologies.length === 0}
				<div class="rounded-2xl border border-dashed border-slate-300 px-4 py-5 text-sm text-slate-500 dark:border-slate-700 dark:text-slate-400">No topologies to monitor yet.</div>
			{:else}
				{#each topologies as topology}
					<button class={`w-full rounded-2xl border px-4 py-3 text-left transition ${selectedTopologyId === topology.id ? 'border-amber-400 bg-amber-50 dark:border-amber-700 dark:bg-amber-950/30' : 'border-slate-200 bg-slate-50 hover:border-slate-300 dark:border-slate-800 dark:bg-slate-900 dark:hover:border-slate-700'}`} onclick={() => onSelectTopology?.(topology.id)} type="button">
						<div class="text-sm font-semibold text-slate-900 dark:text-slate-100">{topology.name}</div>
						<div class="mt-1 text-xs text-slate-500">{topology.status} • {topology.source_stream_ids.length} streams • {topology.sink_bindings.length} sinks</div>
					</button>
				{/each}
			{/if}
		</div>

		<div class="space-y-4">
			{#if runtime}
				<div class="rounded-[24px] border border-slate-200 bg-slate-50 p-4 dark:border-slate-800 dark:bg-slate-900">
					<div class="flex flex-wrap items-center justify-between gap-3">
						<div>
							<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Selected Topology</div>
							<h3 class="mt-2 text-lg font-semibold text-slate-900 dark:text-slate-100">{runtime.topology.name}</h3>
						</div>
						<div class="rounded-full bg-white px-3 py-1 text-sm text-slate-600 dark:bg-slate-950 dark:text-slate-300">{runtime.topology.state_backend}</div>
					</div>

					{#if runtime.latest_run || runtime.preview}
						<div class="mt-3 text-xs text-slate-500">
							{runtime.latest_run ? 'Latest persisted run' : `Live preview from ${runtime.preview?.backlog_events ?? 0} backlog event(s)`}
						</div>
						<div class="mt-4 grid gap-3 md:grid-cols-2 xl:grid-cols-5 text-sm text-slate-700 dark:text-slate-200">
							<div class="rounded-2xl border border-slate-200 bg-white px-3 py-3 dark:border-slate-800 dark:bg-slate-950">In {runtime.latest_run?.metrics.input_events ?? runtime.preview?.metrics.input_events ?? 0}</div>
							<div class="rounded-2xl border border-slate-200 bg-white px-3 py-3 dark:border-slate-800 dark:bg-slate-950">Out {runtime.latest_run?.metrics.output_events ?? runtime.preview?.metrics.output_events ?? 0}</div>
							<div class="rounded-2xl border border-slate-200 bg-white px-3 py-3 dark:border-slate-800 dark:bg-slate-950">P95 {runtime.latest_run?.metrics.p95_latency_ms ?? runtime.preview?.metrics.p95_latency_ms ?? 0}ms</div>
							<div class="rounded-2xl border border-slate-200 bg-white px-3 py-3 dark:border-slate-800 dark:bg-slate-950">Join {runtime.latest_run?.metrics.join_output_rows ?? runtime.preview?.metrics.join_output_rows ?? 0}</div>
							<div class="rounded-2xl border border-slate-200 bg-white px-3 py-3 dark:border-slate-800 dark:bg-slate-950">CEP {runtime.latest_run?.metrics.cep_match_count ?? runtime.preview?.metrics.cep_match_count ?? 0}</div>
						</div>
						<div class="mt-4 grid gap-4 xl:grid-cols-2">
							<div class="rounded-2xl border border-amber-200 bg-amber-50/70 px-4 py-3 dark:border-amber-900 dark:bg-amber-950/20">
								<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-amber-700 dark:text-amber-300">Backpressure</div>
								<p class="mt-2 text-sm text-slate-700 dark:text-slate-200">{runtime.latest_run?.backpressure_snapshot.status ?? runtime.preview?.backpressure_snapshot.status ?? 'healthy'} • queue {runtime.latest_run?.backpressure_snapshot.queue_depth ?? runtime.preview?.backpressure_snapshot.queue_depth ?? 0}/{runtime.latest_run?.backpressure_snapshot.queue_capacity ?? runtime.preview?.backpressure_snapshot.queue_capacity ?? 0} • lag {runtime.latest_run?.backpressure_snapshot.lag_ms ?? runtime.preview?.backpressure_snapshot.lag_ms ?? 0}ms</p>
							</div>
							<div class="rounded-2xl border border-cyan-200 bg-cyan-50/70 px-4 py-3 dark:border-cyan-900 dark:bg-cyan-950/20">
								<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-cyan-700 dark:text-cyan-300">State Store</div>
								<p class="mt-2 text-sm text-slate-700 dark:text-slate-200">{runtime.latest_run?.state_snapshot.backend ?? runtime.preview?.state_snapshot.backend ?? runtime.topology.state_backend} • {runtime.latest_run?.state_snapshot.key_count ?? runtime.preview?.state_snapshot.key_count ?? 0} keys • {runtime.latest_run?.state_snapshot.disk_usage_mb ?? runtime.preview?.state_snapshot.disk_usage_mb ?? 0}MB</p>
							</div>
						</div>
					{/if}

					<div class="mt-4 space-y-3">
						<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Connector Status</div>
						{#each runtime.connector_statuses as connector}
							<div class="rounded-2xl border border-slate-200 bg-white px-4 py-3 text-sm text-slate-700 dark:border-slate-800 dark:bg-slate-950 dark:text-slate-200">
								{connector.connector_type} • {connector.direction} • {connector.endpoint} • {connector.status} • {connector.throughput_per_second.toFixed(0)}/s
							</div>
						{/each}
					</div>
				</div>
			{:else}
				<div class="rounded-2xl border border-dashed border-slate-300 px-4 py-5 text-sm text-slate-500 dark:border-slate-700 dark:text-slate-400">Select a topology to inspect its runtime snapshot.</div>
			{/if}
		</div>
	</div>
</section>
