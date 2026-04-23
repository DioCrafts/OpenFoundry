<script lang="ts">
	import type { ClusterDetail, ResolvedCluster } from '$lib/api/fusion';

	interface Props {
		clusters: ResolvedCluster[];
		selectedClusterId: string;
		clusterDetail: ClusterDetail | null;
		busy?: boolean;
		onSelectCluster?: (clusterId: string) => void;
	}

	let { clusters, selectedClusterId, clusterDetail, busy = false, onSelectCluster }: Props = $props();
</script>

<section class="rounded-[28px] border border-slate-200 bg-white p-5 shadow-sm dark:border-slate-800 dark:bg-slate-950">
	<div>
		<div class="text-[11px] font-semibold uppercase tracking-[0.28em] text-slate-500">Cluster Viewer</div>
		<h2 class="mt-2 text-xl font-semibold text-slate-900 dark:text-slate-100">Transitive clusters, pair evidence, and confidence</h2>
	</div>

	<div class="mt-5 grid gap-5 xl:grid-cols-[minmax(0,0.75fr)_minmax(0,1.25fr)]">
		<div class="space-y-3">
			{#if clusters.length === 0}
				<div class="rounded-2xl border border-dashed border-slate-300 px-4 py-5 text-sm text-slate-500 dark:border-slate-700 dark:text-slate-400">Run a resolution job to generate clusters.</div>
			{:else}
				{#each clusters as cluster}
					<button
						class={`w-full rounded-2xl border px-4 py-3 text-left transition ${selectedClusterId === cluster.id ? 'border-cyan-400 bg-cyan-50 dark:border-cyan-700 dark:bg-cyan-950/30' : 'border-slate-200 bg-slate-50 hover:border-slate-300 dark:border-slate-800 dark:bg-slate-900 dark:hover:border-slate-700'}`}
						onclick={() => onSelectCluster?.(cluster.id)}
						type="button"
						disabled={busy}
					>
						<div class="flex items-center justify-between gap-3">
							<div>
								<div class="text-sm font-semibold text-slate-900 dark:text-slate-100">{cluster.records.length} records</div>
								<div class="mt-1 text-xs text-slate-500">{cluster.status} • confidence {cluster.confidence_score.toFixed(2)}</div>
							</div>
							{#if cluster.requires_review}
								<span class="rounded-full bg-amber-100 px-2 py-1 text-[11px] font-semibold uppercase tracking-[0.2em] text-amber-700 dark:bg-amber-950/40 dark:text-amber-300">Review</span>
							{/if}
						</div>
					</button>
				{/each}
			{/if}
		</div>

		<div class="space-y-4">
			{#if clusterDetail}
				<div class="rounded-[24px] border border-slate-200 bg-slate-50 p-4 dark:border-slate-800 dark:bg-slate-900">
					<div class="flex flex-wrap items-center justify-between gap-3">
						<div>
							<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Selected Cluster</div>
							<h3 class="mt-2 text-lg font-semibold text-slate-900 dark:text-slate-100">{clusterDetail.cluster.cluster_key}</h3>
						</div>
						<div class="rounded-full bg-white px-3 py-1 text-sm text-slate-600 dark:bg-slate-950 dark:text-slate-300">{clusterDetail.cluster.status}</div>
					</div>

					<div class="mt-4 grid gap-4 xl:grid-cols-2">
						<div>
							<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Records</div>
							<div class="mt-3 space-y-3">
								{#each clusterDetail.cluster.records as record}
									<div class="rounded-2xl border border-slate-200 bg-white px-4 py-3 dark:border-slate-800 dark:bg-slate-950">
										<div class="text-sm font-semibold text-slate-900 dark:text-slate-100">{record.display_name}</div>
										<div class="mt-1 text-xs text-slate-500">{record.source} • {record.external_id} • confidence {record.confidence.toFixed(2)}</div>
									</div>
								{/each}
							</div>
						</div>

						<div>
							<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Pair Evidence</div>
							<div class="mt-3 space-y-3">
								{#if clusterDetail.cluster.evidence.length === 0}
									<p class="text-sm text-slate-500">No pair evidence captured for this cluster.</p>
								{:else}
									{#each clusterDetail.cluster.evidence as evidence}
										<div class="rounded-2xl border border-slate-200 bg-white px-4 py-3 dark:border-slate-800 dark:bg-slate-950">
											<div class="flex items-center justify-between gap-3 text-sm text-slate-700 dark:text-slate-200">
												<span>{evidence.left_record_id} ↔ {evidence.right_record_id}</span>
												<span class="font-semibold">{evidence.final_score.toFixed(2)}</span>
											</div>
											<p class="mt-2 text-sm text-slate-600 dark:text-slate-300">{evidence.explanation}</p>
										</div>
									{/each}
								{/if}
							</div>
						</div>
					</div>
				</div>
			{:else}
				<div class="rounded-2xl border border-dashed border-slate-300 px-4 py-5 text-sm text-slate-500 dark:border-slate-700 dark:text-slate-400">Select a cluster to inspect its records and pairwise evidence.</div>
			{/if}
		</div>
	</div>
</section>
