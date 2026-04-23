<script lang="ts">
	import { onDestroy, onMount } from 'svelte';
	import type { GeoJSONSource, Map as MapLibreMap } from 'maplibre-gl';
	import 'maplibre-gl/dist/maplibre-gl.css';

	import type {
		ClusterResponse,
		Coordinate,
		GeocodeResponse,
		Geometry,
		LayerDefinition,
		MapFeature,
		RouteResponse,
		SpatialQueryResponse,
		VectorTileResponse,
	} from '$lib/api/geospatial';

	export let layer: LayerDefinition | null = null;
	export let tile: VectorTileResponse | null = null;
	export let queryResponse: SpatialQueryResponse | null = null;
	export let clusterResponse: ClusterResponse | null = null;
	export let routeResponse: RouteResponse | null = null;
	export let searchResults: GeocodeResponse[] = [];

	let container: HTMLDivElement;
	let map: MapLibreMap | null = null;
	let lastLayerId = '';
	let fittedToLayer = false;

	type GeoJsonFeature = {
		type: 'Feature';
		properties: Record<string, unknown>;
		geometry:
			| { type: 'Point'; coordinates: [number, number] }
			| { type: 'LineString'; coordinates: [number, number][] }
			| { type: 'Polygon'; coordinates: [number, number][][] };
	};

	type GeoJsonCollection = {
		type: 'FeatureCollection';
		features: GeoJsonFeature[];
	};

	type GeoJsonGeometry = GeoJsonFeature['geometry'];

	const emptyCollection: GeoJsonCollection = { type: 'FeatureCollection', features: [] };

	onMount(async () => {
		const maplibre = await import('maplibre-gl');
		map = new maplibre.Map({
			container,
			style: {
				version: 8,
				sources: {},
				layers: [
					{
						id: 'background',
						type: 'background',
						paint: {
							'background-color': '#f5efe4',
						},
					},
				],
			},
			center: [2.1734, 41.3851],
			zoom: 3.3,
			attributionControl: false,
		});

		map.addControl(new maplibre.NavigationControl({ showCompass: false }), 'top-right');
		map.on('load', () => {
			initializeLayers();
			syncMapData();
		});
	});

	onDestroy(() => {
		map?.remove();
	});

	$: if (map && map.isStyleLoaded()) {
		layer;
		tile;
		queryResponse;
		clusterResponse;
		routeResponse;
		searchResults;
		syncMapData();
	}

	function initializeLayers() {
		if (!map) return;

		addGeoJsonSource('base-source');
		addGeoJsonSource('query-source');
		addGeoJsonSource('cluster-source');
		addGeoJsonSource('route-source');
		addGeoJsonSource('search-source');

		map.addLayer({
			id: 'heatmap-layer',
			type: 'heatmap',
			source: 'base-source',
			filter: ['==', ['geometry-type'], 'Point'],
			paint: {
				'heatmap-intensity': 0.9,
				'heatmap-radius': 28,
				'heatmap-opacity': 0.72,
				'heatmap-color': [
					'interpolate',
					['linear'],
					['heatmap-density'],
					0,
					'rgba(34, 211, 238, 0)',
					0.3,
					'rgba(14, 165, 233, 0.35)',
					0.6,
					'rgba(8, 145, 178, 0.58)',
					1,
					'rgba(6, 95, 70, 0.82)',
				],
			},
		});

		map.addLayer({
			id: 'polygon-layer',
			type: 'fill',
			source: 'base-source',
			filter: ['==', ['geometry-type'], 'Polygon'],
			paint: {
				'fill-color': ['coalesce', ['get', 'color'], '#1d4ed8'],
				'fill-opacity': 0.26,
			},
		});

		map.addLayer({
			id: 'line-layer',
			type: 'line',
			source: 'base-source',
			filter: ['==', ['geometry-type'], 'LineString'],
			paint: {
				'line-color': ['coalesce', ['get', 'color'], '#0f766e'],
				'line-width': ['coalesce', ['get', 'lineWidth'], 3],
				'line-opacity': 0.86,
			},
		});

		map.addLayer({
			id: 'point-layer',
			type: 'circle',
			source: 'base-source',
			filter: ['==', ['geometry-type'], 'Point'],
			paint: {
				'circle-color': ['coalesce', ['get', 'color'], '#d97706'],
				'circle-radius': ['coalesce', ['get', 'radius'], 8],
				'circle-stroke-width': 2,
				'circle-stroke-color': '#f8fafc',
				'circle-opacity': 0.9,
			},
		});

		map.addLayer({
			id: 'query-layer',
			type: 'circle',
			source: 'query-source',
			paint: {
				'circle-color': '#06b6d4',
				'circle-radius': 9,
				'circle-stroke-width': 2,
				'circle-stroke-color': '#083344',
			},
		});

		map.addLayer({
			id: 'cluster-layer',
			type: 'circle',
			source: 'cluster-source',
			paint: {
				'circle-color': '#0f766e',
				'circle-radius': ['+', 10, ['coalesce', ['get', 'member_count'], 1]],
				'circle-opacity': 0.82,
			},
		});

		map.addLayer({
			id: 'route-layer',
			type: 'line',
			source: 'route-source',
			paint: {
				'line-color': '#1e293b',
				'line-width': 4,
				'line-dasharray': [2, 1],
			},
		});

		map.addLayer({
			id: 'search-layer',
			type: 'circle',
			source: 'search-source',
			paint: {
				'circle-color': '#ec4899',
				'circle-radius': 7,
				'circle-stroke-color': '#831843',
				'circle-stroke-width': 2,
			},
		});
	}

	function addGeoJsonSource(id: string) {
		if (!map || map.getSource(id)) return;
		map.addSource(id, {
			type: 'geojson',
			data: emptyCollection,
		});
	}

	function syncMapData() {
		if (!map) return;
		if (layer?.id !== lastLayerId) {
			lastLayerId = layer?.id ?? '';
			fittedToLayer = false;
		}

		setSourceData('base-source', toFeatureCollection(layer?.features ?? [], layer?.style));
		setSourceData('query-source', toFeatureCollection(queryResponse?.matched_features ?? []));
		setSourceData('cluster-source', toClusterCollection(clusterResponse));
		setSourceData('route-source', toRouteCollection(routeResponse));
		setSourceData('search-source', toSearchCollection(searchResults));
		fitToLayer();
	}

	function setSourceData(id: string, data: GeoJsonCollection) {
		const source = map?.getSource(id) as GeoJSONSource | undefined;
		source?.setData(data);
	}

	function toFeatureCollection(features: MapFeature[], style?: LayerDefinition['style']): GeoJsonCollection {
		return {
			type: 'FeatureCollection',
			features: features.map((feature) => ({
				type: 'Feature',
				properties: {
					label: feature.label,
					...(feature.properties ?? {}),
					color: style?.color,
					radius: style?.radius,
					lineWidth: style?.line_width,
				},
				geometry: toGeoJsonGeometry(feature.geometry),
			})),
		};
	}

	function toClusterCollection(response: ClusterResponse | null): GeoJsonCollection {
		return {
			type: 'FeatureCollection',
			features: response?.clusters.map((cluster) => ({
				type: 'Feature',
				properties: {
					cluster_id: cluster.cluster_id,
					member_count: cluster.member_count,
				},
				geometry: {
					type: 'Point',
					coordinates: toLngLat(cluster.centroid),
				},
			})) ?? [],
		};
	}

	function toRouteCollection(response: RouteResponse | null): GeoJsonCollection {
		if (!response) return emptyCollection;
		return {
			type: 'FeatureCollection',
			features: [
				{
					type: 'Feature',
					properties: {
						mode: response.mode,
					},
					geometry: {
						type: 'LineString',
						coordinates: response.polyline.map(toLngLat),
					},
				},
			],
		};
	}

	function toSearchCollection(results: GeocodeResponse[]): GeoJsonCollection {
		return {
			type: 'FeatureCollection',
			features: results.map((result) => ({
				type: 'Feature',
				properties: {
					label: result.address,
					source: result.source,
				},
				geometry: {
					type: 'Point',
					coordinates: toLngLat(result.coordinate),
				},
			})),
		};
	}

	function toGeoJsonGeometry(geometry: Geometry): GeoJsonGeometry {
		if (geometry.type === 'point') {
			return {
				type: 'Point',
				coordinates: toLngLat(geometry.coordinates),
			};
		}
		if (geometry.type === 'line_string') {
			return {
				type: 'LineString',
				coordinates: geometry.coordinates.map(toLngLat),
			};
		}
		return {
			type: 'Polygon',
			coordinates: [geometry.coordinates.map(toLngLat)],
		};
	}

	function toLngLat(point: Coordinate): [number, number] {
		return [point.lon, point.lat];
	}

	function fitToLayer() {
		if (!map || fittedToLayer || !layer || layer.features.length === 0) return;
		const points = collectCoordinates(layer.features);
		if (points.length === 0) return;

		if (points.length === 1) {
			map.flyTo({ center: [points[0].lon, points[0].lat], zoom: 6 });
			fittedToLayer = true;
			return;
		}

		const lats = points.map((point) => point.lat);
		const lons = points.map((point) => point.lon);
		map.fitBounds(
			[
				[Math.min(...lons), Math.min(...lats)],
				[Math.max(...lons), Math.max(...lats)],
			],
			{ padding: 40, duration: 0 },
		);
		fittedToLayer = true;
	}

	function collectCoordinates(features: MapFeature[]): Coordinate[] {
		return features.flatMap((feature) => {
			if (feature.geometry.type === 'point') return [feature.geometry.coordinates];
			return feature.geometry.coordinates;
		});
	}
</script>

<section class="rounded-[2rem] border border-stone-200 bg-white p-4 shadow-sm shadow-stone-200/60">
	<div class="mb-3 flex items-center justify-between gap-3 px-1">
		<div>
			<p class="text-xs font-semibold uppercase tracking-[0.24em] text-cyan-700">MapLibre GL</p>
			<h3 class="mt-2 text-xl font-semibold text-stone-900">Live layer canvas</h3>
		</div>
		{#if tile}
			<p class="rounded-full bg-stone-100 px-3 py-1 text-xs font-semibold uppercase tracking-[0.18em] text-stone-600">{tile.format} • {tile.feature_count} features</p>
		{/if}
	</div>
	<div bind:this={container} class="h-[28rem] overflow-hidden rounded-[1.5rem] border border-stone-200"></div>
</section>
