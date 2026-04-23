<script lang="ts">
	import type { ListingDefinition } from '$lib/api/marketplace';

	export let listing: ListingDefinition;
	export let selected = false;
	export let score: number | null = null;
	export let onSelect: () => void;
</script>

<button class={`w-full rounded-3xl border px-4 py-4 text-left transition ${selected ? 'border-amber-500 bg-amber-50' : 'border-stone-200 bg-white hover:border-amber-300 hover:bg-amber-50/60'}`} onclick={onSelect}>
	<div class="flex items-start justify-between gap-3">
		<div>
			<p class="font-semibold text-stone-900">{listing.name}</p>
			<p class="text-sm text-stone-500">{listing.publisher} • {listing.package_kind}</p>
		</div>
		{#if score !== null}
			<span class="rounded-full bg-stone-950 px-3 py-1 text-xs font-semibold uppercase tracking-[0.18em] text-stone-50">{score.toFixed(2)}</span>
		{/if}
	</div>
	<p class="mt-3 text-sm text-stone-600">{listing.summary}</p>
	<div class="mt-3 flex flex-wrap gap-2">
		{#each listing.tags as tag}
			<span class="rounded-full bg-stone-100 px-2 py-1 text-xs text-stone-600">{tag}</span>
		{/each}
	</div>
	<div class="mt-4 flex items-center justify-between text-xs text-stone-500">
		<span>{listing.install_count} installs</span>
		<span>{listing.average_rating.toFixed(1)} rating</span>
	</div>
</button>
