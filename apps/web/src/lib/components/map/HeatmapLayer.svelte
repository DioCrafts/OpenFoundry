<script lang="ts">
	import type { ClusterResponse, SpatialQueryResponse, VectorTileResponse } from '$lib/api/geospatial';

	export let tile: VectorTileResponse | null = null;
	export let queryResponse: SpatialQueryResponse | null = null;
	export let clusterResponse: ClusterResponse | null = null;
</script>

<section class="rounded-3xl border border-stone-200 bg-white p-5 shadow-sm shadow-stone-200/60">
	<div>
		<p class="text-xs font-semibold uppercase tracking-[0.24em] text-cyan-700">Heatmap + Tiles</p>
		<h3 class="mt-2 text-xl font-semibold text-stone-900">Vector tile summary and H3-like aggregation</h3>
	</div>

	{#if tile}
		<div class="mt-5 grid gap-4 xl:grid-cols-[0.85fr_1.15fr]">
			<div class="space-y-4 rounded-2xl border border-stone-200 bg-stone-50 p-4">
				<div>
					<p class="text-sm font-semibold text-stone-900">{tile.layer_name}</p>
					<p class="text-sm text-stone-500">{tile.format.toUpperCase()} • zoom {tile.zoom_range[0]}-{tile.zoom_range[1]} • {tile.feature_count} features</p>
				</div>
				<div class="rounded-2xl border border-stone-200 bg-white p-4 text-sm text-stone-600">
					<p class="font-semibold text-stone-900">Tile template</p>
					<p class="mt-2 break-all text-xs text-stone-500">{tile.tile_url_template}</p>
				</div>
				{#if queryResponse}
					<div class="rounded-2xl border border-stone-200 bg-white p-4 text-sm text-stone-600">
						<p class="font-semibold text-stone-900">Latest query</p>
						<p class="mt-2">{queryResponse.summary.matched_count} matches in {queryResponse.summary.query_time_ms} ms.</p>
					</div>
				{/if}
				{#if clusterResponse}
					<div class="rounded-2xl border border-stone-200 bg-white p-4 text-sm text-stone-600">
						<p class="font-semibold text-stone-900">Latest clusters</p>
						<p class="mt-2">{clusterResponse.clusters.length} clusters • {clusterResponse.outliers} outliers</p>
					</div>
				{/if}
			</div>

			<div class="rounded-2xl border border-stone-200 bg-stone-50 p-4">
				<p class="text-sm font-semibold text-stone-900">Top hex bins</p>
				<div class="mt-3 space-y-3">
					{#each tile.h3_bins.slice(0, 8) as bin}
						<div class="rounded-2xl border border-stone-200 bg-white p-4">
							<div class="flex items-center justify-between gap-3">
								<div>
									<p class="font-medium text-stone-900">{bin.cell_id}</p>
									<p class="text-sm text-stone-500">{bin.centroid.lat.toFixed(3)}, {bin.centroid.lon.toFixed(3)}</p>
								</div>
								<p class="rounded-full bg-cyan-100 px-3 py-1 text-xs font-semibold uppercase tracking-[0.18em] text-cyan-700">{bin.count} features</p>
							</div>
							<div class="mt-3 h-2 overflow-hidden rounded-full bg-stone-200">
								<div class="h-full rounded-full bg-cyan-500" style={`width: ${Math.min(bin.intensity * 20, 100)}%`}></div>
							</div>
						</div>
					{/each}
				</div>
			</div>
		</div>
	{:else}
		<div class="mt-5 rounded-2xl border border-dashed border-stone-300 bg-stone-50 p-8 text-center text-sm text-stone-500">
			Select a layer to inspect vector tile metadata and heatmap bins.
		</div>
	{/if}
</section>
