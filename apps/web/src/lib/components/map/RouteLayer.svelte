<script lang="ts">
	import type { RouteResponse } from '$lib/api/geospatial';

	export let routeResponse: RouteResponse | null = null;
</script>

<section class="rounded-3xl border border-stone-200 bg-white p-5 shadow-sm shadow-stone-200/60">
	<div>
		<p class="text-xs font-semibold uppercase tracking-[0.24em] text-cyan-700">Routing</p>
		<h3 class="mt-2 text-xl font-semibold text-stone-900">Shortest path and isochrone summary</h3>
	</div>

	{#if routeResponse}
		<div class="mt-5 grid gap-4 xl:grid-cols-[0.9fr_1.1fr]">
			<div class="space-y-4 rounded-2xl border border-stone-200 bg-stone-50 p-4">
				<div class="grid gap-3 sm:grid-cols-3 xl:grid-cols-1">
					<div class="rounded-2xl border border-stone-200 bg-white p-4">
						<p class="text-xs font-semibold uppercase tracking-[0.18em] text-stone-500">Mode</p>
						<p class="mt-3 text-xl font-semibold text-stone-900">{routeResponse.mode}</p>
					</div>
					<div class="rounded-2xl border border-stone-200 bg-white p-4">
						<p class="text-xs font-semibold uppercase tracking-[0.18em] text-stone-500">Distance</p>
						<p class="mt-3 text-xl font-semibold text-stone-900">{routeResponse.distance_km.toFixed(1)} km</p>
					</div>
					<div class="rounded-2xl border border-stone-200 bg-white p-4">
						<p class="text-xs font-semibold uppercase tracking-[0.18em] text-stone-500">ETA</p>
						<p class="mt-3 text-xl font-semibold text-stone-900">{routeResponse.duration_min} min</p>
					</div>
				</div>
			</div>

			<div class="rounded-2xl border border-stone-200 bg-stone-50 p-4">
				<p class="text-sm font-semibold text-stone-900">Isochrone anchors</p>
				<div class="mt-3 space-y-3">
					{#each routeResponse.isochrone as point}
						<div class="rounded-2xl border border-stone-200 bg-white p-4">
							<div class="flex items-center justify-between gap-3">
								<p class="font-medium text-stone-900">{point.label}</p>
								<p class="text-sm text-stone-500">{point.coordinate.lat.toFixed(3)}, {point.coordinate.lon.toFixed(3)}</p>
							</div>
							<p class="mt-2 text-sm text-stone-600">ETA target: {point.eta_minutes} minutes</p>
						</div>
					{/each}
				</div>
			</div>
		</div>
	{:else}
		<div class="mt-5 rounded-2xl border border-dashed border-stone-300 bg-stone-50 p-8 text-center text-sm text-stone-500">
			Compute a route to inspect polyline and isochrone outputs.
		</div>
	{/if}
</section>
