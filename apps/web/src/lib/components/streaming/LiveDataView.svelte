<script lang="ts">
	import type { ConnectorCatalogEntry, LiveTailResponse, TopologyRuntimeSnapshot } from '$lib/api/streaming';

	interface Props {
		connectors: ConnectorCatalogEntry[];
		liveTail: LiveTailResponse | null;
		runtime: TopologyRuntimeSnapshot | null;
	}

	let { connectors, liveTail, runtime }: Props = $props();
</script>

<section class="rounded-[28px] border border-slate-200 bg-white p-5 shadow-sm dark:border-slate-800 dark:bg-slate-950">
	<div>
		<div class="text-[11px] font-semibold uppercase tracking-[0.28em] text-slate-500">Live Tail</div>
		<h2 class="mt-2 text-xl font-semibold text-slate-900 dark:text-slate-100">Connector health, live events, and CEP pattern matches</h2>
	</div>

	<div class="mt-5 grid gap-5 xl:grid-cols-[minmax(0,0.88fr)_minmax(0,1.12fr)]">
		<div class="space-y-4">
			<div>
				<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Connectors</div>
				<div class="mt-3 space-y-3">
					{#each connectors as connector}
						<div class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 text-sm text-slate-700 dark:border-slate-800 dark:bg-slate-900 dark:text-slate-200">
							<div class="font-semibold">{connector.connector_type} • {connector.direction}</div>
							<div class="mt-1 text-xs text-slate-500">{connector.endpoint} • backlog {connector.backlog} • {connector.throughput_per_second.toFixed(0)}/s</div>
						</div>
					{/each}
				</div>
			</div>

			{#if runtime?.latest_run || runtime?.preview}
				<div class="rounded-[24px] border border-dashed border-cyan-300 bg-cyan-50/60 p-4 dark:border-cyan-900 dark:bg-cyan-950/20">
					<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-cyan-700 dark:text-cyan-300">Latest Aggregates</div>
					<div class="mt-3 space-y-2">
						{#each (runtime.latest_run?.aggregate_windows ?? runtime.preview?.aggregate_windows ?? []) as aggregate}
							<div class="rounded-2xl border border-cyan-200 bg-white px-4 py-3 text-sm text-slate-700 dark:border-cyan-900 dark:bg-slate-950 dark:text-slate-200">
								{aggregate.window_name} • {aggregate.group_key} • {aggregate.measure_name} = {aggregate.value}
							</div>
						{/each}
					</div>
				</div>
			{/if}
		</div>

		<div class="grid gap-4 xl:grid-cols-2">
			<div class="rounded-[24px] border border-slate-200 bg-slate-50 p-4 dark:border-slate-800 dark:bg-slate-900">
				<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Events</div>
				<div class="mt-3 space-y-3">
					{#if !liveTail || liveTail.events.length === 0}
						<div class="rounded-2xl border border-dashed border-slate-300 px-4 py-5 text-sm text-slate-500 dark:border-slate-700 dark:text-slate-400">No live events captured yet.</div>
					{:else}
						{#each liveTail.events as event}
							<div class="rounded-2xl border border-slate-200 bg-white px-4 py-3 dark:border-slate-800 dark:bg-slate-950">
								<div class="flex items-center justify-between gap-3 text-sm text-slate-700 dark:text-slate-200">
									<span>{event.stream_name}</span>
									<span class="text-xs text-slate-500">{new Date(event.processing_time).toLocaleTimeString()}</span>
								</div>
								<pre class="mt-2 overflow-x-auto whitespace-pre-wrap text-xs text-slate-600 dark:text-slate-300">{JSON.stringify(event.payload, null, 2)}</pre>
							</div>
						{/each}
					{/if}
				</div>
			</div>

			<div class="rounded-[24px] border border-slate-200 bg-slate-50 p-4 dark:border-slate-800 dark:bg-slate-900">
				<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">CEP Matches</div>
				<div class="mt-3 space-y-3">
					{#if !liveTail || liveTail.matches.length === 0}
						<div class="rounded-2xl border border-dashed border-slate-300 px-4 py-5 text-sm text-slate-500 dark:border-slate-700 dark:text-slate-400">No pattern matches yet.</div>
					{:else}
						{#each liveTail.matches as match}
							<div class="rounded-2xl border border-slate-200 bg-white px-4 py-3 text-sm text-slate-700 dark:border-slate-800 dark:bg-slate-950 dark:text-slate-200">
								<div class="font-semibold">{match.pattern_name}</div>
								<div class="mt-1 text-xs text-slate-500">{match.matched_sequence.join(' → ')} • confidence {match.confidence.toFixed(2)}</div>
							</div>
						{/each}
					{/if}
				</div>
			</div>
		</div>
	</div>
</section>
