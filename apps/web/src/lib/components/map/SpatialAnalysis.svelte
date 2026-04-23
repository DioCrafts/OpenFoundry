<script lang="ts">
	import type {
		ClusterAlgorithm,
		ClusterResponse,
		GeocodeResponse,
		LayerDefinition,
		RouteMode,
		SpatialOperation,
		SpatialQueryResponse,
	} from '$lib/api/geospatial';

	type AnalysisDraft = {
		operation: SpatialOperation;
		point_lat: string;
		point_lon: string;
		radius_km: number;
		limit: number;
		cluster_algorithm: ClusterAlgorithm;
		cluster_count: number;
		cluster_radius_km: number;
		geocode_query: string;
		origin_lat: string;
		origin_lon: string;
		destination_lat: string;
		destination_lon: string;
		route_mode: RouteMode;
		route_max_minutes: number;
	};

	export let selectedLayer: LayerDefinition | null = null;
	export let draft: AnalysisDraft;
	export let busy = false;
	export let queryResponse: SpatialQueryResponse | null = null;
	export let clusterResponse: ClusterResponse | null = null;
	export let geocodeResponse: GeocodeResponse | null = null;
	export let reverseGeocodeResponse: GeocodeResponse | null = null;
	export let onDraftChange: (patch: Partial<AnalysisDraft>) => void;
	export let onRunQuery: () => void;
	export let onRunCluster: () => void;
	export let onRunRoute: () => void;
	export let onGeocode: () => void;
	export let onReverseGeocode: () => void;

	const operations: SpatialOperation[] = ['within', 'intersects', 'nearest', 'buffer'];
	const algorithms: ClusterAlgorithm[] = ['dbscan', 'kmeans'];
	const routeModes: RouteMode[] = ['drive', 'bike', 'walk'];

	function inputValue(event: Event) {
		return (event.currentTarget as HTMLInputElement).value;
	}

	function numberValue(event: Event) {
		return Number((event.currentTarget as HTMLInputElement).value);
	}
</script>

<section class="rounded-3xl border border-stone-200 bg-white p-5 shadow-sm shadow-stone-200/60">
	<div class="flex flex-col gap-4 lg:flex-row lg:items-start lg:justify-between">
		<div>
			<p class="text-xs font-semibold uppercase tracking-[0.24em] text-cyan-700">Spatial Analysis</p>
			<h3 class="mt-2 text-xl font-semibold text-stone-900">Queries, clustering, geocoding, and routing</h3>
			<p class="mt-1 text-sm text-stone-500">{selectedLayer ? `${selectedLayer.name} is the active layer for spatial analysis.` : 'Select a layer to enable spatial analysis controls.'}</p>
		</div>
		{#if geocodeResponse}
			<div class="rounded-2xl border border-cyan-200 bg-cyan-50 px-4 py-3 text-sm text-cyan-800">
				<p class="font-semibold">{geocodeResponse.address}</p>
				<p>{geocodeResponse.coordinate.lat.toFixed(4)}, {geocodeResponse.coordinate.lon.toFixed(4)} • {Math.round(geocodeResponse.confidence * 100)}%</p>
			</div>
		{/if}
	</div>

	<div class="mt-5 grid gap-4 xl:grid-cols-3">
		<div class="space-y-4 rounded-2xl border border-stone-200 bg-stone-50 p-4">
			<p class="text-sm font-semibold text-stone-800">Spatial query</p>
			<label class="block text-sm text-stone-700">
				<span class="mb-2 block font-medium">Operation</span>
				<select class="w-full rounded-2xl border border-stone-300 bg-white px-4 py-3 outline-none transition focus:border-cyan-500" value={draft.operation} onchange={(event) => onDraftChange({ operation: (event.currentTarget as HTMLSelectElement).value as SpatialOperation })}>
					{#each operations as operation}
						<option value={operation}>{operation}</option>
					{/each}
				</select>
			</label>
			<div class="grid gap-3 sm:grid-cols-2">
				<label class="block text-sm text-stone-700">
					<span class="mb-2 block font-medium">Latitude</span>
					<input class="w-full rounded-2xl border border-stone-300 bg-white px-4 py-3 outline-none transition focus:border-cyan-500" value={draft.point_lat} oninput={(event) => onDraftChange({ point_lat: inputValue(event) })} />
				</label>
				<label class="block text-sm text-stone-700">
					<span class="mb-2 block font-medium">Longitude</span>
					<input class="w-full rounded-2xl border border-stone-300 bg-white px-4 py-3 outline-none transition focus:border-cyan-500" value={draft.point_lon} oninput={(event) => onDraftChange({ point_lon: inputValue(event) })} />
				</label>
				<label class="block text-sm text-stone-700">
					<span class="mb-2 block font-medium">Radius km</span>
					<input type="number" class="w-full rounded-2xl border border-stone-300 bg-white px-4 py-3 outline-none transition focus:border-cyan-500" value={draft.radius_km} oninput={(event) => onDraftChange({ radius_km: numberValue(event) })} />
				</label>
				<label class="block text-sm text-stone-700">
					<span class="mb-2 block font-medium">Limit</span>
					<input type="number" class="w-full rounded-2xl border border-stone-300 bg-white px-4 py-3 outline-none transition focus:border-cyan-500" value={draft.limit} oninput={(event) => onDraftChange({ limit: numberValue(event) })} />
				</label>
			</div>
			<button class="w-full rounded-full bg-cyan-600 px-4 py-3 text-sm font-semibold text-white transition hover:bg-cyan-700 disabled:cursor-not-allowed disabled:bg-cyan-300" onclick={onRunQuery} disabled={busy || !selectedLayer}>
				Run spatial query
			</button>
			{#if queryResponse}
				<p class="text-sm text-stone-600">{queryResponse.summary.matched_count} matches in {queryResponse.summary.query_time_ms} ms.</p>
			{/if}
		</div>

		<div class="space-y-4 rounded-2xl border border-stone-200 bg-stone-50 p-4">
			<p class="text-sm font-semibold text-stone-800">Clustering + geocoding</p>
			<label class="block text-sm text-stone-700">
				<span class="mb-2 block font-medium">Algorithm</span>
				<select class="w-full rounded-2xl border border-stone-300 bg-white px-4 py-3 outline-none transition focus:border-cyan-500" value={draft.cluster_algorithm} onchange={(event) => onDraftChange({ cluster_algorithm: (event.currentTarget as HTMLSelectElement).value as ClusterAlgorithm })}>
					{#each algorithms as algorithm}
						<option value={algorithm}>{algorithm}</option>
					{/each}
				</select>
			</label>
			<div class="grid gap-3 sm:grid-cols-2">
				<label class="block text-sm text-stone-700">
					<span class="mb-2 block font-medium">Cluster count</span>
					<input type="number" class="w-full rounded-2xl border border-stone-300 bg-white px-4 py-3 outline-none transition focus:border-cyan-500" value={draft.cluster_count} oninput={(event) => onDraftChange({ cluster_count: numberValue(event) })} />
				</label>
				<label class="block text-sm text-stone-700">
					<span class="mb-2 block font-medium">Radius km</span>
					<input type="number" class="w-full rounded-2xl border border-stone-300 bg-white px-4 py-3 outline-none transition focus:border-cyan-500" value={draft.cluster_radius_km} oninput={(event) => onDraftChange({ cluster_radius_km: numberValue(event) })} />
				</label>
			</div>
			<button class="w-full rounded-full border border-cyan-300 px-4 py-3 text-sm font-semibold text-cyan-700 transition hover:bg-cyan-50 disabled:cursor-not-allowed disabled:border-stone-300 disabled:text-stone-400" onclick={onRunCluster} disabled={busy || !selectedLayer}>
				Run clustering
			</button>
			<label class="block text-sm text-stone-700">
				<span class="mb-2 block font-medium">Address search</span>
				<input class="w-full rounded-2xl border border-stone-300 bg-white px-4 py-3 outline-none transition focus:border-cyan-500" value={draft.geocode_query} oninput={(event) => onDraftChange({ geocode_query: inputValue(event) })} placeholder="Madrid, Barcelona, Berlin" />
			</label>
			<div class="grid gap-3 sm:grid-cols-2">
				<button class="rounded-full border border-stone-300 px-4 py-3 text-sm font-medium text-stone-700 transition hover:bg-stone-100 disabled:cursor-not-allowed disabled:text-stone-400" onclick={onGeocode} disabled={busy}>
					Geocode
				</button>
				<button class="rounded-full border border-stone-300 px-4 py-3 text-sm font-medium text-stone-700 transition hover:bg-stone-100 disabled:cursor-not-allowed disabled:text-stone-400" onclick={onReverseGeocode} disabled={busy}>
					Reverse geocode
				</button>
			</div>
			{#if clusterResponse}
				<p class="text-sm text-stone-600">{clusterResponse.clusters.length} clusters, {clusterResponse.outliers} outliers.</p>
			{/if}
			{#if reverseGeocodeResponse}
				<p class="text-sm text-stone-600">Nearest address: {reverseGeocodeResponse.address}</p>
			{/if}
		</div>

		<div class="space-y-4 rounded-2xl border border-stone-200 bg-stone-50 p-4">
			<p class="text-sm font-semibold text-stone-800">Routing</p>
			<div class="grid gap-3 sm:grid-cols-2">
				<label class="block text-sm text-stone-700">
					<span class="mb-2 block font-medium">Origin lat</span>
					<input class="w-full rounded-2xl border border-stone-300 bg-white px-4 py-3 outline-none transition focus:border-cyan-500" value={draft.origin_lat} oninput={(event) => onDraftChange({ origin_lat: inputValue(event) })} />
				</label>
				<label class="block text-sm text-stone-700">
					<span class="mb-2 block font-medium">Origin lon</span>
					<input class="w-full rounded-2xl border border-stone-300 bg-white px-4 py-3 outline-none transition focus:border-cyan-500" value={draft.origin_lon} oninput={(event) => onDraftChange({ origin_lon: inputValue(event) })} />
				</label>
				<label class="block text-sm text-stone-700">
					<span class="mb-2 block font-medium">Destination lat</span>
					<input class="w-full rounded-2xl border border-stone-300 bg-white px-4 py-3 outline-none transition focus:border-cyan-500" value={draft.destination_lat} oninput={(event) => onDraftChange({ destination_lat: inputValue(event) })} />
				</label>
				<label class="block text-sm text-stone-700">
					<span class="mb-2 block font-medium">Destination lon</span>
					<input class="w-full rounded-2xl border border-stone-300 bg-white px-4 py-3 outline-none transition focus:border-cyan-500" value={draft.destination_lon} oninput={(event) => onDraftChange({ destination_lon: inputValue(event) })} />
				</label>
			</div>
			<div class="grid gap-3 sm:grid-cols-2">
				<label class="block text-sm text-stone-700">
					<span class="mb-2 block font-medium">Mode</span>
					<select class="w-full rounded-2xl border border-stone-300 bg-white px-4 py-3 outline-none transition focus:border-cyan-500" value={draft.route_mode} onchange={(event) => onDraftChange({ route_mode: (event.currentTarget as HTMLSelectElement).value as RouteMode })}>
						{#each routeModes as mode}
							<option value={mode}>{mode}</option>
						{/each}
					</select>
				</label>
				<label class="block text-sm text-stone-700">
					<span class="mb-2 block font-medium">Max minutes</span>
					<input type="number" class="w-full rounded-2xl border border-stone-300 bg-white px-4 py-3 outline-none transition focus:border-cyan-500" value={draft.route_max_minutes} oninput={(event) => onDraftChange({ route_max_minutes: numberValue(event) })} />
				</label>
			</div>
			<button class="w-full rounded-full bg-stone-900 px-4 py-3 text-sm font-semibold text-white transition hover:bg-stone-800 disabled:cursor-not-allowed disabled:bg-stone-400" onclick={onRunRoute} disabled={busy}>
				Compute route
			</button>
		</div>
	</div>
</section>
