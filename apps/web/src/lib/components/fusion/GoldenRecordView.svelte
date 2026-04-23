<script lang="ts">
	import type { ClusterDetail, GoldenRecord } from '$lib/api/fusion';

	interface Props {
		goldenRecords: GoldenRecord[];
		clusterDetail: ClusterDetail | null;
	}

	let { goldenRecords, clusterDetail }: Props = $props();
	const activeGoldenRecord = $derived(clusterDetail?.golden_record ?? goldenRecords[0] ?? null);
</script>

<section class="rounded-[28px] border border-slate-200 bg-white p-5 shadow-sm dark:border-slate-800 dark:bg-slate-950">
	<div>
		<div class="text-[11px] font-semibold uppercase tracking-[0.28em] text-slate-500">Golden Records</div>
		<h2 class="mt-2 text-xl font-semibold text-slate-900 dark:text-slate-100">Canonical identities and provenance trails</h2>
	</div>

	<div class="mt-5 grid gap-5 xl:grid-cols-[minmax(0,0.8fr)_minmax(0,1.2fr)]">
		<div class="space-y-3">
			{#if goldenRecords.length === 0}
				<div class="rounded-2xl border border-dashed border-slate-300 px-4 py-5 text-sm text-slate-500 dark:border-slate-700 dark:text-slate-400">Golden records appear here after a resolution run.</div>
			{:else}
				{#each goldenRecords as goldenRecord}
					<div class={`rounded-2xl border px-4 py-3 ${activeGoldenRecord?.id === goldenRecord.id ? 'border-cyan-400 bg-cyan-50 dark:border-cyan-700 dark:bg-cyan-950/30' : 'border-slate-200 bg-slate-50 dark:border-slate-800 dark:bg-slate-900'}`}>
						<div class="text-sm font-semibold text-slate-900 dark:text-slate-100">{goldenRecord.title}</div>
						<div class="mt-1 text-xs text-slate-500">completeness {goldenRecord.completeness_score.toFixed(2)} • confidence {goldenRecord.confidence_score.toFixed(2)}</div>
					</div>
				{/each}
			{/if}
		</div>

		<div>
			{#if activeGoldenRecord}
				<div class="rounded-[24px] border border-slate-200 bg-slate-50 p-4 dark:border-slate-800 dark:bg-slate-900">
					<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Selected Golden Record</div>
					<h3 class="mt-2 text-lg font-semibold text-slate-900 dark:text-slate-100">{activeGoldenRecord.title}</h3>
					<pre class="mt-4 overflow-x-auto rounded-2xl bg-slate-950 px-4 py-3 text-xs text-cyan-100">{JSON.stringify(activeGoldenRecord.canonical_values, null, 2)}</pre>
					<div class="mt-4 space-y-2">
						{#each activeGoldenRecord.provenance as item}
							<div class="rounded-2xl border border-slate-200 bg-white px-4 py-3 text-sm text-slate-700 dark:border-slate-800 dark:bg-slate-950 dark:text-slate-200">
								{item.field}: {item.source} • {item.external_id} • {item.strategy}
							</div>
						{/each}
					</div>
				</div>
			{:else}
				<div class="rounded-2xl border border-dashed border-slate-300 px-4 py-5 text-sm text-slate-500 dark:border-slate-700 dark:text-slate-400">Select or generate a golden record to inspect canonical values.</div>
			{/if}
		</div>
	</div>
</section>
