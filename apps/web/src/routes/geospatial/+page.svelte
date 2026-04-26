<script lang="ts">
	import { onMount } from 'svelte';

	import HeatmapLayer from '$components/map/HeatmapLayer.svelte';
	import LayerPanel from '$components/map/LayerPanel.svelte';
	import MapView from '$components/map/MapView.svelte';
	import RouteLayer from '$components/map/RouteLayer.svelte';
	import SpatialAnalysis from '$components/map/SpatialAnalysis.svelte';
	import {
		clusterFeatures,
		createLayer,
		geocodeAddress,
		getOverview,
		getVectorTile,
		listLayers,
		reverseGeocode,
		routeFeatures,
		runSpatialQuery,
		updateLayer,
		type ClusterAlgorithm,
		type ClusterResponse,
		type Coordinate,
		type GeocodeResponse,
		type GeospatialOverview,
		type LayerDefinition,
		type LayerStyle,
		type MapFeature,
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

	type TemporalFeatureEntry = {
		feature: MapFeature;
		timestamp: number;
		label: string;
	};

	type HistogramBucket = {
		label: string;
		count: number;
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
	let styleDraft = $state<LayerStyle>(defaultLayerStyle());
	let tagText = $state('');
	let persistIndexed = $state(true);
	let selectedNumericField = $state('');
	let numericMin = $state(0);
	let numericMax = $state(0);
	let timelineCursor = $state(100);
	let timelineMode = $state<'up_to' | 'single'>('up_to');
	let templateName = $state('');
	let templateDescription = $state('');

	const busy = $derived(loading || busyAction.length > 0);
	const selectedLayer = $derived(layers.find((layer) => layer.id === selectedLayerId) ?? null);
	const searchResults = $derived([
		...(geocodeResponse ? [geocodeResponse] : []),
		...(reverseGeocodeResponse ? [reverseGeocodeResponse] : []),
	]);
	const templateLayers = $derived(
		layers.filter((layer) => layer.tags.includes('template') || layer.tags.includes('saved-map')),
	);

	onMount(() => {
		void refreshAll();
	});

	function defaultLayerStyle(): LayerStyle {
		return {
			color: '#0f766e',
			opacity: 0.82,
			radius: 8,
			line_width: 3,
			heatmap_intensity: 0.9,
			cluster_color: '#0f766e',
			show_labels: true,
		};
	}

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

	function updateStyleDraft(patch: Partial<LayerStyle>) {
		styleDraft = { ...styleDraft, ...patch };
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
			seedControlsFromLayer(layer);
			tile = await getVectorTile(layer.id);
			queryResponse = null;
			clusterResponse = null;
			if (notify) notifications.info(`Loaded ${layer.name}`);
		}
	}

	function seedControlsFromLayer(layer: LayerDefinition) {
		styleDraft = { ...layer.style };
		tagText = layer.tags.join(', ');
		persistIndexed = layer.indexed;
		templateName = `${layer.name} template`;
		templateDescription = `Saved map template derived from ${layer.name}`;

		const numericField = firstNumericField(layer.features);
		selectedNumericField = numericField;
		if (numericField) {
			const range = numericFieldRange(layer.features, numericField);
			numericMin = range.min;
			numericMax = range.max;
		} else {
			numericMin = 0;
			numericMax = 0;
		}
		timelineCursor = 100;
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

	function timestampKeys() {
		return ['timestamp', 'event_time', 'time', 'datetime', 'date', 'created_at', 'updated_at'];
	}

	function parseFeatureTimestamp(feature: MapFeature, index: number, layer: LayerDefinition): number {
		for (const key of timestampKeys()) {
			const value = feature.properties[key];
			if (typeof value === 'string' || typeof value === 'number') {
				const parsed = new Date(value).getTime();
				if (!Number.isNaN(parsed)) return parsed;
			}
		}
		const base = new Date(layer.created_at).getTime();
		return base + index * 3_600_000;
	}

	function featureLabel(feature: MapFeature, index: number) {
		return feature.label || String(feature.properties.name ?? `Feature ${index + 1}`);
	}

	function temporalEntries(layer: LayerDefinition | null): TemporalFeatureEntry[] {
		if (!layer) return [];
		return layer.features
			.map((feature, index) => ({
				feature,
				timestamp: parseFeatureTimestamp(feature, index, layer),
				label: featureLabel(feature, index),
			}))
			.sort((left, right) => left.timestamp - right.timestamp);
	}

	function firstNumericField(features: MapFeature[]) {
		const counts = new Map<string, number>();
		for (const feature of features) {
			for (const [key, value] of Object.entries(feature.properties)) {
				if (typeof value === 'number' && Number.isFinite(value)) {
					counts.set(key, (counts.get(key) ?? 0) + 1);
				}
			}
		}
		return [...counts.entries()].sort((left, right) => right[1] - left[1])[0]?.[0] ?? '';
	}

	function numericFieldRange(features: MapFeature[], field: string) {
		const values = features
			.map((feature) => feature.properties[field])
			.filter((value): value is number => typeof value === 'number' && Number.isFinite(value));
		if (values.length === 0) return { min: 0, max: 0 };
		return { min: Math.min(...values), max: Math.max(...values) };
	}

	function filteredFeatures() {
		const layer = selectedLayer;
		if (!layer) return [] as MapFeature[];
		let features = layer.features;

		const temporal = temporalEntries(layer);
		if (temporal.length > 0) {
			const index = Math.max(0, Math.round((timelineCursor / 100) * Math.max(0, temporal.length - 1)));
			const selectedTimestamp = temporal[index]?.timestamp ?? temporal[temporal.length - 1]?.timestamp ?? 0;
			const allowedIds =
				timelineMode === 'single'
					? new Set([temporal[index]?.feature.id].filter(Boolean) as string[])
					: new Set(temporal.filter((entry) => entry.timestamp <= selectedTimestamp).map((entry) => entry.feature.id));
			features = features.filter((feature) => allowedIds.has(feature.id));
		}

		if (selectedNumericField) {
			features = features.filter((feature) => {
				const value = feature.properties[selectedNumericField];
				if (typeof value !== 'number' || !Number.isFinite(value)) return false;
				return value >= numericMin && value <= numericMax;
			});
		}

		return features;
	}

	function visibleLayer() {
		if (!selectedLayer) return null;
		return {
			...selectedLayer,
			style: styleDraft,
			indexed: persistIndexed,
			features: filteredFeatures(),
		};
	}

	function timelineIndexLabel() {
		const entries = temporalEntries(selectedLayer);
		if (entries.length === 0) return 'No events';
		const index = Math.max(0, Math.round((timelineCursor / 100) * Math.max(0, entries.length - 1)));
		return new Intl.DateTimeFormat('en', {
			month: 'short',
			day: 'numeric',
			hour: 'numeric',
			minute: '2-digit',
		}).format(new Date(entries[index].timestamp));
	}

	function timelineEvents() {
		const entries = temporalEntries(selectedLayer);
		if (entries.length === 0) return [] as TemporalFeatureEntry[];
		const index = Math.max(0, Math.round((timelineCursor / 100) * Math.max(0, entries.length - 1)));
		if (timelineMode === 'single') return entries.slice(Math.max(0, index - 2), index + 3);
		return entries.filter((entry) => entry.timestamp <= entries[index].timestamp).slice(-6);
	}

	function histogramBuckets() {
		if (!selectedLayer || !selectedNumericField) return [] as HistogramBucket[];
		const values = selectedLayer.features
			.map((feature) => feature.properties[selectedNumericField])
			.filter((value): value is number => typeof value === 'number' && Number.isFinite(value));
		if (values.length === 0) return [];

		const range = numericFieldRange(selectedLayer.features, selectedNumericField);
		if (range.min === range.max) return [{ label: String(range.min), count: values.length }];

		const bucketCount = 6;
		const step = (range.max - range.min) / bucketCount;
		const buckets = Array.from({ length: bucketCount }, (_, index) => ({
			label: `${(range.min + step * index).toFixed(0)}-${(range.min + step * (index + 1)).toFixed(0)}`,
			count: 0,
		}));

		for (const value of values) {
			const bucketIndex = Math.min(bucketCount - 1, Math.floor((value - range.min) / step));
			buckets[bucketIndex].count += 1;
		}

		return buckets;
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

	async function saveLayerConfiguration() {
		if (!selectedLayer) {
			notifications.warning('Select a layer before saving settings');
			return;
		}

		busyAction = 'save-layer';
		uiError = '';
		try {
			const tags = tagText
				.split(',')
				.map((value) => value.trim())
				.filter(Boolean);
			await updateLayer(selectedLayer.id, {
				style: styleDraft,
				indexed: persistIndexed,
				tags,
			});
			notifications.success(`Saved settings for ${selectedLayer.name}`);
			await refreshAll(selectedLayer.id);
		} catch (error) {
			uiError = error instanceof Error ? error.message : 'Unable to save layer settings';
			notifications.error(uiError);
		} finally {
			busyAction = '';
		}
	}

	async function saveCurrentMapTemplate() {
		const layer = visibleLayer();
		const baseLayer = selectedLayer;
		if (!layer || !baseLayer) {
			notifications.warning('Select a layer before saving a template');
			return;
		}
		if (layer.features.length === 0) {
			notifications.warning('The current filtered view has no features to save');
			return;
		}

		busyAction = 'save-template';
		uiError = '';
		try {
			const tags = [...new Set([...baseLayer.tags, 'template', 'saved-map'])];
			const created = await createLayer({
				name: templateName.trim() || `${baseLayer.name} template`,
				description: templateDescription.trim() || `Saved map template derived from ${baseLayer.name}`,
				source_kind: baseLayer.source_kind,
				source_dataset: `${baseLayer.source_dataset}.template`,
				geometry_type: baseLayer.geometry_type,
				style: styleDraft,
				features: layer.features,
				tags,
				indexed: persistIndexed,
			});
			notifications.success(`Saved template ${created.name}`);
			await refreshAll(created.id);
		} catch (error) {
			uiError = error instanceof Error ? error.message : 'Unable to save template';
			notifications.error(uiError);
		} finally {
			busyAction = '';
		}
	}

	function querySummaryCards() {
		const response = queryResponse;
		if (!response) return [];
		return response.matched_features.slice(0, 6).map((feature) => ({
			id: feature.id,
			label: feature.label,
			detail: Object.entries(feature.properties)
				.slice(0, 3)
				.map(([key, value]) => `${key}: ${String(value)}`)
				.join(' • '),
		}));
	}
</script>

<svelte:head>
	<title>OpenFoundry Map</title>
</svelte:head>

<div class="min-h-screen bg-[radial-gradient(circle_at_top_left,_rgba(6,182,212,0.16),_transparent_32%),linear-gradient(180deg,_#f7fbfb_0%,_#eef5f6_52%,_#e6eeeb_100%)] px-6 py-8 text-stone-900 lg:px-10">
	<div class="mx-auto max-w-7xl space-y-6">
		<section class="grid gap-6 rounded-[2rem] border border-stone-200/80 bg-white/82 p-6 shadow-xl shadow-stone-200/60 backdrop-blur xl:grid-cols-[1.15fr_0.85fr]">
			<div>
				<p class="text-xs font-semibold uppercase tracking-[0.28em] text-cyan-700">Map</p>
				<h1 class="mt-3 text-4xl font-semibold tracking-tight text-stone-950">Geospatial and temporal analysis workspace</h1>
				<p class="mt-3 max-w-3xl text-base leading-7 text-stone-600">
					Explore geospatial objects, search around points and polygons, visualize contextual layers, filter by time and metrics,
					and save reusable map templates from the same operational canvas.
				</p>
				<div class="mt-6 flex flex-wrap gap-3">
					<a href="#map-interface" class="rounded-full bg-cyan-600 px-5 py-3 text-sm font-semibold text-white transition hover:bg-cyan-700">Open map interface</a>
					<a href="#time-controls" class="rounded-full border border-stone-300 px-5 py-3 text-sm font-semibold text-stone-700 transition hover:bg-stone-50">Time and events</a>
					<a href="#templates" class="rounded-full border border-stone-300 px-5 py-3 text-sm font-semibold text-stone-700 transition hover:bg-stone-50">Create and save maps</a>
				</div>
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
					<p class="text-xs font-semibold uppercase tracking-[0.2em] text-stone-500">Templates</p>
					<p class="mt-3 text-3xl font-semibold text-stone-950">{templateLayers.length}</p>
				</div>
			</div>
		</section>

		{#if uiError}
			<div class="rounded-2xl border border-rose-300 bg-rose-50 px-4 py-3 text-sm text-rose-700">{uiError}</div>
		{/if}

		<div id="map-interface" class="grid gap-6 xl:grid-cols-[0.92fr_1.08fr]">
			<LayerPanel layers={layers} selectedLayerId={selectedLayerId} onSelectLayer={selectLayer} />
			<MapView layer={visibleLayer()} {tile} {queryResponse} {clusterResponse} {routeResponse} searchResults={searchResults} />
		</div>

		<div class="grid gap-6 xl:grid-cols-[0.98fr_1.02fr]">
			<section class="rounded-3xl border border-stone-200 bg-white p-5 shadow-sm shadow-stone-200/60">
				<div class="flex items-center justify-between gap-3">
					<div>
						<p class="text-xs font-semibold uppercase tracking-[0.24em] text-cyan-700">Control panel</p>
						<h3 class="mt-2 text-xl font-semibold text-stone-900">Layer management and styling</h3>
						<p class="mt-1 text-sm text-stone-500">Adjust visual defaults, indexing posture, and saved tags for the active layer.</p>
					</div>
					<button class="rounded-full bg-stone-900 px-4 py-2.5 text-sm font-semibold text-white transition hover:bg-stone-800 disabled:cursor-not-allowed disabled:bg-stone-400" onclick={saveLayerConfiguration} disabled={busy || !selectedLayer}>
						{busyAction === 'save-layer' ? 'Saving...' : 'Save settings'}
					</button>
				</div>

				<div class="mt-5 grid gap-4 md:grid-cols-2">
					<label class="block text-sm text-stone-700">
						<span class="mb-2 block font-medium">Layer color</span>
						<input type="color" value={styleDraft.color} oninput={(event) => updateStyleDraft({ color: (event.currentTarget as HTMLInputElement).value })} class="h-12 w-full rounded-2xl border border-stone-300 bg-white px-2 py-2" />
					</label>
					<label class="block text-sm text-stone-700">
						<span class="mb-2 block font-medium">Cluster color</span>
						<input type="color" value={styleDraft.cluster_color} oninput={(event) => updateStyleDraft({ cluster_color: (event.currentTarget as HTMLInputElement).value })} class="h-12 w-full rounded-2xl border border-stone-300 bg-white px-2 py-2" />
					</label>
					<label class="block text-sm text-stone-700">
						<span class="mb-2 block font-medium">Opacity</span>
						<input type="range" min="0.1" max="1" step="0.05" value={styleDraft.opacity} oninput={(event) => updateStyleDraft({ opacity: Number((event.currentTarget as HTMLInputElement).value) })} class="w-full" />
					</label>
					<label class="block text-sm text-stone-700">
						<span class="mb-2 block font-medium">Heatmap intensity</span>
						<input type="range" min="0.1" max="2" step="0.1" value={styleDraft.heatmap_intensity} oninput={(event) => updateStyleDraft({ heatmap_intensity: Number((event.currentTarget as HTMLInputElement).value) })} class="w-full" />
					</label>
					<label class="block text-sm text-stone-700">
						<span class="mb-2 block font-medium">Point radius</span>
						<input type="number" value={styleDraft.radius} oninput={(event) => updateStyleDraft({ radius: Number((event.currentTarget as HTMLInputElement).value) })} class="w-full rounded-2xl border border-stone-300 bg-white px-4 py-3" />
					</label>
					<label class="block text-sm text-stone-700">
						<span class="mb-2 block font-medium">Line width</span>
						<input type="number" value={styleDraft.line_width} oninput={(event) => updateStyleDraft({ line_width: Number((event.currentTarget as HTMLInputElement).value) })} class="w-full rounded-2xl border border-stone-300 bg-white px-4 py-3" />
					</label>
					<label class="flex items-center justify-between rounded-2xl border border-stone-200 bg-stone-50 px-4 py-3 text-sm text-stone-700">
						<span class="font-medium">Show labels</span>
						<input type="checkbox" checked={styleDraft.show_labels} onchange={(event) => updateStyleDraft({ show_labels: (event.currentTarget as HTMLInputElement).checked })} />
					</label>
					<label class="flex items-center justify-between rounded-2xl border border-stone-200 bg-stone-50 px-4 py-3 text-sm text-stone-700">
						<span class="font-medium">Indexed layer</span>
						<input type="checkbox" bind:checked={persistIndexed} />
					</label>
				</div>

				<label class="mt-4 block text-sm text-stone-700">
					<span class="mb-2 block font-medium">Tags</span>
					<input value={tagText} oninput={(event) => tagText = (event.currentTarget as HTMLInputElement).value} class="w-full rounded-2xl border border-stone-300 bg-white px-4 py-3" placeholder="operations, airports, saved-map" />
				</label>
			</section>

			<section id="time-controls" class="rounded-3xl border border-stone-200 bg-white p-5 shadow-sm shadow-stone-200/60">
				<div>
					<p class="text-xs font-semibold uppercase tracking-[0.24em] text-cyan-700">Time</p>
					<h3 class="mt-2 text-xl font-semibold text-stone-900">Timeline, events, and temporal filtering</h3>
					<p class="mt-1 text-sm text-stone-500">Move through ordered events and limit the map to a single moment or an accumulated time window.</p>
				</div>

				<div class="mt-5 rounded-3xl border border-stone-200 bg-stone-50 p-4">
					<div class="flex items-center justify-between gap-3">
						<div>
							<p class="text-sm font-semibold text-stone-900">{timelineIndexLabel()}</p>
							<p class="text-xs uppercase tracking-[0.18em] text-stone-500">{timelineMode === 'up_to' ? 'Timeline window' : 'Single event'}</p>
						</div>
						<div class="flex gap-2">
							<button class={`rounded-full px-3 py-2 text-xs font-semibold transition ${timelineMode === 'up_to' ? 'bg-cyan-600 text-white' : 'border border-stone-300 text-stone-700 hover:bg-white'}`} onclick={() => timelineMode = 'up_to'}>Timeline</button>
							<button class={`rounded-full px-3 py-2 text-xs font-semibold transition ${timelineMode === 'single' ? 'bg-cyan-600 text-white' : 'border border-stone-300 text-stone-700 hover:bg-white'}`} onclick={() => timelineMode = 'single'}>Event</button>
						</div>
					</div>
					<input class="mt-4 w-full" type="range" min="0" max="100" step="1" bind:value={timelineCursor} />
					<div class="mt-4 space-y-3">
						{#each timelineEvents() as event}
							<div class="rounded-2xl border border-stone-200 bg-white px-4 py-3">
								<p class="text-sm font-semibold text-stone-900">{event.label}</p>
								<p class="mt-1 text-sm text-stone-600">{new Date(event.timestamp).toLocaleString()}</p>
							</div>
						{/each}
						{#if timelineEvents().length === 0}
							<p class="text-sm text-stone-500">No temporal entries are available for the current layer.</p>
						{/if}
					</div>
				</div>
			</section>
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

		<div class="grid gap-6 xl:grid-cols-[0.92fr_1.08fr]">
			<section class="rounded-3xl border border-stone-200 bg-white p-5 shadow-sm shadow-stone-200/60">
				<div>
					<p class="text-xs font-semibold uppercase tracking-[0.24em] text-cyan-700">Selection and filtering</p>
					<h3 class="mt-2 text-xl font-semibold text-stone-900">Histogram and query-driven inspection</h3>
					<p class="mt-1 text-sm text-stone-500">Filter the active layer by a numeric property and inspect query matches as map selections.</p>
				</div>

				<div class="mt-5 grid gap-4 md:grid-cols-[220px_minmax(0,1fr)]">
					<div class="space-y-4">
						<label class="block text-sm text-stone-700">
							<span class="mb-2 block font-medium">Numeric field</span>
							<select value={selectedNumericField} onchange={(event) => {
								selectedNumericField = (event.currentTarget as HTMLSelectElement).value;
								if (selectedLayer && selectedNumericField) {
									const range = numericFieldRange(selectedLayer.features, selectedNumericField);
									numericMin = range.min;
									numericMax = range.max;
								}
							}} class="w-full rounded-2xl border border-stone-300 bg-white px-4 py-3">
								<option value="">No numeric filter</option>
								{#if selectedLayer}
									{#each Object.keys(selectedLayer.features[0]?.properties ?? {}).filter((key) => typeof selectedLayer.features.find((feature) => typeof feature.properties[key] === 'number')?.properties[key] === 'number') as field}
										<option value={field}>{field}</option>
									{/each}
								{/if}
							</select>
						</label>
						{#if selectedNumericField}
							<label class="block text-sm text-stone-700">
								<span class="mb-2 block font-medium">Min value</span>
								<input type="number" bind:value={numericMin} class="w-full rounded-2xl border border-stone-300 bg-white px-4 py-3" />
							</label>
							<label class="block text-sm text-stone-700">
								<span class="mb-2 block font-medium">Max value</span>
								<input type="number" bind:value={numericMax} class="w-full rounded-2xl border border-stone-300 bg-white px-4 py-3" />
							</label>
						{/if}
					</div>

					<div class="rounded-3xl border border-stone-200 bg-stone-50 p-4">
						<div class="grid grid-cols-6 gap-3">
							{#each histogramBuckets() as bucket}
								<div class="flex flex-col items-center gap-2">
									<div class="flex h-32 w-full items-end rounded-2xl bg-white px-2 pb-2">
										<div
											class="w-full rounded-xl bg-[linear-gradient(180deg,#5eead4_0%,#0891b2_100%)]"
											style={`height:${Math.max(10, Math.round((bucket.count / Math.max(1, ...histogramBuckets().map((item) => item.count))) * 100))}%`}
										></div>
									</div>
									<p class="text-center text-[11px] text-stone-500">{bucket.label}</p>
									<p class="text-sm font-semibold text-stone-900">{bucket.count}</p>
								</div>
							{/each}
						</div>
						{#if histogramBuckets().length === 0}
							<p class="text-sm text-stone-500">No numeric histogram is available for the active layer.</p>
						{/if}
					</div>
				</div>

				<div class="mt-5 space-y-3">
					{#each querySummaryCards() as card}
						<div class="rounded-2xl border border-stone-200 bg-stone-50 px-4 py-3">
							<p class="text-sm font-semibold text-stone-900">{card.label}</p>
							<p class="mt-1 text-sm text-stone-600">{card.detail}</p>
						</div>
					{/each}
					{#if querySummaryCards().length === 0}
						<p class="text-sm text-stone-500">Run a spatial query to inspect selected map objects here.</p>
					{/if}
				</div>
			</section>

			<section id="templates" class="rounded-3xl border border-stone-200 bg-white p-5 shadow-sm shadow-stone-200/60">
				<div class="flex items-center justify-between gap-3">
					<div>
						<p class="text-xs font-semibold uppercase tracking-[0.24em] text-cyan-700">Templates and Workshop widget</p>
						<h3 class="mt-2 text-xl font-semibold text-stone-900">Create and save maps</h3>
						<p class="mt-1 text-sm text-stone-500">Persist the current filtered view as a reusable map template backed by a real geospatial layer.</p>
					</div>
					<button class="rounded-full bg-stone-900 px-4 py-2.5 text-sm font-semibold text-white transition hover:bg-stone-800 disabled:cursor-not-allowed disabled:bg-stone-400" onclick={saveCurrentMapTemplate} disabled={busy || !selectedLayer}>
						{busyAction === 'save-template' ? 'Saving...' : 'Save template'}
					</button>
				</div>

				<div class="mt-5 grid gap-4 md:grid-cols-2">
					<label class="block text-sm text-stone-700">
						<span class="mb-2 block font-medium">Template name</span>
						<input bind:value={templateName} class="w-full rounded-2xl border border-stone-300 bg-white px-4 py-3" placeholder="Regional operations view" />
					</label>
					<label class="block text-sm text-stone-700">
						<span class="mb-2 block font-medium">Description</span>
						<input bind:value={templateDescription} class="w-full rounded-2xl border border-stone-300 bg-white px-4 py-3" placeholder="Saved view for operations review" />
					</label>
				</div>

				<div class="mt-5 space-y-3">
					{#each templateLayers as layer}
						<button class="w-full rounded-2xl border border-stone-200 bg-stone-50 px-4 py-4 text-left transition hover:border-cyan-300 hover:bg-cyan-50/60" onclick={() => void selectLayer(layer.id)}>
							<div class="flex items-start justify-between gap-3">
								<div>
									<p class="font-semibold text-stone-900">{layer.name}</p>
									<p class="text-sm text-stone-500">{layer.features.length} features • {layer.geometry_type} • {layer.source_kind}</p>
								</div>
								<p class="rounded-full bg-white px-3 py-1 text-xs font-semibold uppercase tracking-[0.18em] text-stone-600">Template</p>
							</div>
							<p class="mt-3 text-sm text-stone-600">{layer.description}</p>
						</button>
					{/each}
					{#if templateLayers.length === 0}
						<p class="text-sm text-stone-500">No saved map templates yet.</p>
					{/if}
				</div>
			</section>
		</div>

		<div class="grid gap-6 xl:grid-cols-[1.05fr_0.95fr]">
			<HeatmapLayer {tile} {queryResponse} {clusterResponse} />
			<RouteLayer {routeResponse} />
		</div>
	</div>
</div>
