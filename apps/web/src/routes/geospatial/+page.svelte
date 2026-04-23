<script lang="ts">
	import { onMount } from 'svelte';

	import HeatmapLayer from '$components/map/HeatmapLayer.svelte';
	import LayerPanel from '$components/map/LayerPanel.svelte';
	import MapView from '$components/map/MapView.svelte';
	import RouteLayer from '$components/map/RouteLayer.svelte';
	import SpatialAnalysis from '$components/map/SpatialAnalysis.svelte';
	import {
		clusterFeatures,
		geocodeAddress,
		getOverview,
		getVectorTile,
		listLayers,
		reverseGeocode,
		routeFeatures,
		runSpatialQuery,
		type ClusterAlgorithm,
		type ClusterResponse,
		type Coordinate,
		type GeocodeResponse,
		type GeospatialOverview,
		type LayerDefinition,
		type RouteMode,
		type RouteResponse,
		type SpatialOperation,
		type SpatialQueryResponse,
		type VectorTileResponse,
	} from '$lib/api/geospatial';
	import { notifications } from '$lib/stores/notifications';

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

	let overview = $state<GeospatialOverview | null>(null);
	let layers = $state<LayerDefinition[]>([]);
	let selectedLayerId = $state('');
	let tile = $state<VectorTileResponse | null>(null);
	let queryResponse = $state<SpatialQueryResponse | null>(null);
	let clusterResponse = $state<ClusterResponse | null>(null);
	let routeResponse = $state<RouteResponse | null>(null);
	let geocodeResponse = $state<GeocodeResponse | null>(null);
	let reverseGeocodeResponse = $state<GeocodeResponse | null>(null);
	let loading = $state(true);
	let busyAction = $state('');
	let uiError = $state('');
	let draft = $state<AnalysisDraft>(createEmptyAnalysisDraft());

	const busy = $derived(loading || busyAction.length > 0);
	const selectedLayer = $derived(layers.find((layer) => layer.id === selectedLayerId) ?? null);
	const searchResults = $derived([
		...(geocodeResponse ? [geocodeResponse] : []),
		...(reverseGeocodeResponse ? [reverseGeocodeResponse] : []),
	]);

	onMount(() => {
		void refreshAll();
	});

	function createEmptyAnalysisDraft(): AnalysisDraft {
		return {
			operation: 'nearest',
			point_lat: '40.4168',
			point_lon: '-3.7038',
			radius_km: 18,
			limit: 5,
			cluster_algorithm: 'dbscan',
			cluster_count: 3,
			cluster_radius_km: 30,
			geocode_query: 'Madrid',
			origin_lat: '40.4168',
			origin_lon: '-3.7038',
			destination_lat: '39.4699',
			destination_lon: '-0.3763',
			route_mode: 'drive',
			route_max_minutes: 45,
		};
	}

	function updateDraft(patch: Partial<AnalysisDraft>) {
		draft = { ...draft, ...patch };
	}

	async function refreshAll(preferredLayerId?: string) {
		loading = true;
		uiError = '';
		try {
			const [overviewResponse, layersResponse] = await Promise.all([getOverview(), listLayers()]);
			overview = overviewResponse;
			layers = layersResponse.items;
			const nextLayerId = preferredLayerId ?? selectedLayerId ?? layers[0]?.id ?? '';
			if (nextLayerId) {
				await selectLayer(nextLayerId, false);
			} else {
				tile = null;
			}
		} catch (error) {
			uiError = error instanceof Error ? error.message : 'Unable to load geospatial surfaces';
			notifications.error(uiError);
		} finally {
			loading = false;
		}
	}

	async function selectLayer(layerId: string, notify = true) {
		selectedLayerId = layerId;
		const layer = layers.find((entry) => entry.id === layerId) ?? null;
		if (layer) {
			seedDraftFromLayer(layer);
			tile = await getVectorTile(layer.id);
			queryResponse = null;
			clusterResponse = null;
			if (notify) notifications.info(`Loaded ${layer.name}`);
		}
	}

	function seedDraftFromLayer(layer: LayerDefinition) {
		const coordinates = layer.features.flatMap((feature) => geometryCoordinates(feature.geometry));
		const first = coordinates[0];
		const second = coordinates[1] ?? first;
		if (!first || !second) return;
		draft = {
			...draft,
			point_lat: first.lat.toFixed(4),
			point_lon: first.lon.toFixed(4),
			origin_lat: first.lat.toFixed(4),
			origin_lon: first.lon.toFixed(4),
			destination_lat: second.lat.toFixed(4),
			destination_lon: second.lon.toFixed(4),
		};
	}

	function geometryCoordinates(geometry: LayerDefinition['features'][number]['geometry']): Coordinate[] {
		if (geometry.type === 'point') return [geometry.coordinates];
		return geometry.coordinates;
	}

	function currentPoint(): Coordinate {
		return { lat: Number(draft.point_lat), lon: Number(draft.point_lon) };
	}

	function pointToBounds(point: Coordinate, radiusKm: number) {
		const delta = radiusKm / 111.0;
		return {
			min_lat: point.lat - delta,
			min_lon: point.lon - delta,
			max_lat: point.lat + delta,
			max_lon: point.lon + delta,
		};
	}

	async function runQuery() {
		if (!selectedLayerId) {
			notifications.warning('Select a layer before running a query');
			return;
		}

		busyAction = 'spatial-query';
		uiError = '';
		try {
			const point = currentPoint();
			queryResponse = await runSpatialQuery({
				layer_id: selectedLayerId,
				operation: draft.operation,
				bounds: draft.operation === 'within' || draft.operation === 'intersects' ? pointToBounds(point, draft.radius_km) : undefined,
				point: draft.operation === 'nearest' || draft.operation === 'buffer' ? point : undefined,
				radius_km: draft.radius_km,
				limit: draft.limit,
			});
			notifications.success(`Spatial query returned ${queryResponse.summary.matched_count} matches`);
		} catch (error) {
			uiError = error instanceof Error ? error.message : 'Unable to run spatial query';
			notifications.error(uiError);
		} finally {
			busyAction = '';
		}
	}

	async function runClusters() {
		if (!selectedLayerId) {
			notifications.warning('Select a layer before clustering');
			return;
		}

		busyAction = 'cluster';
		uiError = '';
		try {
			clusterResponse = await clusterFeatures({
				layer_id: selectedLayerId,
				algorithm: draft.cluster_algorithm,
				cluster_count: draft.cluster_count,
				radius_km: draft.cluster_radius_km,
			});
			notifications.success(`Generated ${clusterResponse.clusters.length} clusters`);
		} catch (error) {
			uiError = error instanceof Error ? error.message : 'Unable to cluster layer';
			notifications.error(uiError);
		} finally {
			busyAction = '';
		}
	}

	async function runRouting() {
		busyAction = 'route';
		uiError = '';
		try {
			routeResponse = await routeFeatures({
				origin: { lat: Number(draft.origin_lat), lon: Number(draft.origin_lon) },
				destination: { lat: Number(draft.destination_lat), lon: Number(draft.destination_lon) },
				mode: draft.route_mode,
				max_minutes: draft.route_max_minutes,
			});
			notifications.success(`Route computed in ${routeResponse.duration_min} minutes`);
		} catch (error) {
			uiError = error instanceof Error ? error.message : 'Unable to compute route';
			notifications.error(uiError);
		} finally {
			busyAction = '';
		}
	}

	async function runGeocodeSearch() {
		busyAction = 'geocode';
		uiError = '';
		try {
			geocodeResponse = await geocodeAddress({ address: draft.geocode_query });
			updateDraft({
				point_lat: geocodeResponse.coordinate.lat.toFixed(4),
				point_lon: geocodeResponse.coordinate.lon.toFixed(4),
				origin_lat: geocodeResponse.coordinate.lat.toFixed(4),
				origin_lon: geocodeResponse.coordinate.lon.toFixed(4),
			});
			notifications.success(`Geocoded ${geocodeResponse.address}`);
		} catch (error) {
			uiError = error instanceof Error ? error.message : 'Unable to geocode address';
			notifications.error(uiError);
		} finally {
			busyAction = '';
		}
	}

	async function runReverseGeocodeSearch() {
		busyAction = 'reverse-geocode';
		uiError = '';
		try {
			reverseGeocodeResponse = await reverseGeocode({ coordinate: currentPoint() });
			notifications.success(`Reverse geocoded ${reverseGeocodeResponse.address}`);
		} catch (error) {
			uiError = error instanceof Error ? error.message : 'Unable to reverse geocode coordinate';
			notifications.error(uiError);
		} finally {
			busyAction = '';
		}
	}
</script>

<svelte:head>
	<title>OpenFoundry Geospatial</title>
</svelte:head>

<div class="min-h-screen bg-[radial-gradient(circle_at_top_left,_rgba(6,182,212,0.18),_transparent_30%),linear-gradient(180deg,_#f3fbfb_0%,_#eef6f5_58%,_#e3ece9_100%)] px-6 py-8 text-stone-900 lg:px-10">
	<div class="mx-auto max-w-7xl space-y-6">
		<section class="grid gap-6 rounded-[2rem] border border-stone-200/80 bg-white/80 p-6 shadow-xl shadow-stone-200/60 backdrop-blur xl:grid-cols-[1.1fr_0.9fr]">
			<div>
				<p class="text-xs font-semibold uppercase tracking-[0.28em] text-cyan-700">Milestone 4.3</p>
				<h1 class="mt-3 text-4xl font-semibold tracking-tight text-stone-950">Geospatial workspace</h1>
				<p class="mt-3 max-w-2xl text-base leading-7 text-stone-600">
					Inspect indexed layers, push spatial predicates, render heatmaps over vector-ready bins, cluster features, and trace shortest-path previews with isochrone anchors.
				</p>
			</div>
			<div class="grid gap-4 sm:grid-cols-2">
				<div class="rounded-3xl border border-stone-200 bg-stone-50 p-4">
					<p class="text-xs font-semibold uppercase tracking-[0.2em] text-stone-500">Layers</p>
					<p class="mt-3 text-3xl font-semibold text-stone-950">{overview?.layer_count ?? 0}</p>
				</div>
				<div class="rounded-3xl border border-stone-200 bg-stone-50 p-4">
					<p class="text-xs font-semibold uppercase tracking-[0.2em] text-stone-500">Indexed</p>
					<p class="mt-3 text-3xl font-semibold text-stone-950">{overview?.indexed_layers ?? 0}</p>
				</div>
				<div class="rounded-3xl border border-stone-200 bg-stone-50 p-4">
					<p class="text-xs font-semibold uppercase tracking-[0.2em] text-stone-500">Features</p>
					<p class="mt-3 text-3xl font-semibold text-stone-950">{overview?.total_features ?? 0}</p>
				</div>
				<div class="rounded-3xl border border-stone-200 bg-stone-50 p-4">
					<p class="text-xs font-semibold uppercase tracking-[0.2em] text-stone-500">Tile-ready</p>
					<p class="mt-3 text-3xl font-semibold text-stone-950">{overview?.tile_ready_layers ?? 0}</p>
				</div>
			</div>
		</section>

		{#if uiError}
			<div class="rounded-2xl border border-rose-300 bg-rose-50 px-4 py-3 text-sm text-rose-700">{uiError}</div>
		{/if}

		<div class="grid gap-6 xl:grid-cols-[0.92fr_1.08fr]">
			<LayerPanel layers={layers} selectedLayerId={selectedLayerId} onSelectLayer={selectLayer} />
			<MapView layer={selectedLayer} {tile} {queryResponse} {clusterResponse} {routeResponse} searchResults={searchResults} />
		</div>

		<SpatialAnalysis
			selectedLayer={selectedLayer}
			{draft}
			{busy}
			{queryResponse}
			{clusterResponse}
			{geocodeResponse}
			reverseGeocodeResponse={reverseGeocodeResponse}
			onDraftChange={updateDraft}
			onRunQuery={runQuery}
			onRunCluster={runClusters}
			onRunRoute={runRouting}
			onGeocode={runGeocodeSearch}
			onReverseGeocode={runReverseGeocodeSearch}
		/>

		<div class="grid gap-6 xl:grid-cols-[1.05fr_0.95fr]">
			<HeatmapLayer {tile} {queryResponse} {clusterResponse} />
			<RouteLayer {routeResponse} />
		</div>
	</div>
</div>