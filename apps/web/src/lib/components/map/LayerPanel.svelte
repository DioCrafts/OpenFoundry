<script lang="ts">
	import type { LayerDefinition } from '$lib/api/geospatial';

	export let layers: LayerDefinition[] = [];
	export let selectedLayerId = '';
	export let onSelectLayer: (layerId: string) => void;
</script>

<section class="rounded-3xl border border-stone-200 bg-white p-5 shadow-sm shadow-stone-200/60">
	<div>
		<p class="text-xs font-semibold uppercase tracking-[0.24em] text-cyan-700">Layer Panel</p>
		<h3 class="mt-2 text-xl font-semibold text-stone-900">Indexed layers and vector-ready sources</h3>
		<p class="mt-1 text-sm text-stone-500">Switch between point, polygon, and line layers to drive tiles, heatmaps, clustering, and routing.</p>
	</div>

	<div class="mt-5 space-y-3">
		{#each layers as layer}
			<button class={`w-full rounded-2xl border px-4 py-4 text-left transition ${selectedLayerId === layer.id ? 'border-cyan-500 bg-cyan-50' : 'border-stone-200 bg-stone-50 hover:border-cyan-300 hover:bg-cyan-50/60'}`} onclick={() => onSelectLayer(layer.id)}>
				<div class="flex items-start justify-between gap-3">
					<div>
						<p class="font-semibold text-stone-900">{layer.name}</p>
						<p class="text-sm text-stone-500">{layer.source_kind} • {layer.geometry_type} • {layer.features.length} features</p>
					</div>
					<p class={`rounded-full px-3 py-1 text-xs font-semibold uppercase tracking-[0.18em] ${layer.indexed ? 'bg-emerald-100 text-emerald-700' : 'bg-stone-200 text-stone-600'}`}>
						{layer.indexed ? 'Indexed' : 'Draft'}
					</p>
				</div>
				<p class="mt-3 text-sm text-stone-600">{layer.description}</p>
				<div class="mt-3 flex flex-wrap gap-2">
					{#each layer.tags as tag}
						<span class="rounded-full bg-white px-2 py-1 text-xs font-medium text-stone-600">{tag}</span>
					{/each}
				</div>
			</button>
		{/each}
	</div>
</section>
