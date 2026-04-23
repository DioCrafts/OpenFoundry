<script lang="ts">
	import ListingCard from '$components/marketplace/ListingCard.svelte';
	import type { CategoryDefinition, ListingDefinition, MarketplaceOverview } from '$lib/api/marketplace';

	export let overview: MarketplaceOverview | null = null;
	export let categories: CategoryDefinition[] = [];
	export let listings: ListingDefinition[] = [];
	export let selectedListingId = '';
	export let searchQuery = '';
	export let selectedCategory = 'all';
	export let scoreById: Record<string, number> = {};
	export let busy = false;
	export let onSearchQueryChange: (query: string) => void;
	export let onCategoryChange: (category: string) => void;
	export let onSearch: () => void;
	export let onSelectListing: (listingId: string) => void;

	function inputValue(event: Event) {
		return (event.currentTarget as HTMLInputElement).value;
	}
</script>

<section class="rounded-3xl border border-stone-200 bg-white p-5 shadow-sm shadow-stone-200/60">
	<div class="flex flex-col gap-4 lg:flex-row lg:items-start lg:justify-between">
		<div>
			<p class="text-xs font-semibold uppercase tracking-[0.24em] text-orange-700">Marketplace Browser</p>
			<h2 class="mt-2 text-2xl font-semibold text-stone-900">Discovery across connectors, widgets, templates, models, and agents</h2>
			<p class="mt-1 text-sm text-stone-600">Search and filter private listings backed by the new marketplace service.</p>
		</div>
		<div class="grid grid-cols-2 gap-3 sm:grid-cols-3">
			<div class="rounded-2xl bg-stone-950 px-4 py-3 text-stone-50">
				<p class="text-xs uppercase tracking-[0.18em] text-orange-300">Listings</p>
				<p class="mt-2 text-2xl font-semibold">{overview?.listing_count ?? 0}</p>
			</div>
			<div class="rounded-2xl bg-orange-50 px-4 py-3 text-orange-900">
				<p class="text-xs uppercase tracking-[0.18em] text-orange-600">Categories</p>
				<p class="mt-2 text-2xl font-semibold">{overview?.category_count ?? 0}</p>
			</div>
			<div class="rounded-2xl bg-emerald-50 px-4 py-3 text-emerald-900">
				<p class="text-xs uppercase tracking-[0.18em] text-emerald-600">Installs</p>
				<p class="mt-2 text-2xl font-semibold">{overview?.total_installs ?? 0}</p>
			</div>
		</div>
	</div>

	<div class="mt-5 flex flex-col gap-3 lg:flex-row">
		<input class="w-full rounded-full border border-stone-300 bg-white px-4 py-3 text-sm outline-none transition focus:border-orange-500" value={searchQuery} oninput={(event) => onSearchQueryChange(inputValue(event))} placeholder="Search widget, connector, agent..." />
		<select class="rounded-full border border-stone-300 bg-white px-4 py-3 text-sm outline-none transition focus:border-orange-500" value={selectedCategory} onchange={(event) => onCategoryChange((event.currentTarget as HTMLSelectElement).value)}>
			<option value="all">All categories</option>
			{#each categories as category}
				<option value={category.slug}>{category.name}</option>
			{/each}
		</select>
		<button class="rounded-full bg-orange-500 px-5 py-3 text-sm font-semibold text-stone-950 transition hover:bg-orange-400 disabled:cursor-not-allowed disabled:bg-orange-200" onclick={onSearch} disabled={busy}>Search</button>
	</div>

	<div class="mt-5 grid gap-4 xl:grid-cols-[0.9fr_1.1fr]">
		<div class="rounded-2xl border border-stone-200 bg-stone-50/80 p-4">
			<p class="text-xs font-semibold uppercase tracking-[0.18em] text-stone-500">Categories</p>
			<div class="mt-3 grid gap-3 sm:grid-cols-2">
				{#each categories as category}
					<button class={`rounded-2xl border px-4 py-4 text-left transition ${selectedCategory === category.slug ? 'border-orange-500 bg-orange-50' : 'border-stone-200 bg-white hover:border-orange-300 hover:bg-orange-50/60'}`} onclick={() => onCategoryChange(category.slug)}>
						<p class="font-semibold text-stone-900">{category.name}</p>
						<p class="mt-1 text-sm text-stone-500">{category.description}</p>
						<p class="mt-3 text-xs uppercase tracking-[0.18em] text-stone-400">{category.listing_count} listings</p>
					</button>
				{/each}
			</div>
		</div>

		<div class="space-y-3">
			{#each listings as listing}
				<ListingCard listing={listing} selected={selectedListingId === listing.id} score={scoreById[listing.id] ?? null} onSelect={() => onSelectListing(listing.id)} />
			{/each}
			{#if listings.length === 0}
				<div class="rounded-2xl border border-dashed border-stone-300 bg-stone-50 px-4 py-8 text-center text-sm text-stone-500">No listings match the current discovery query.</div>
			{/if}
		</div>
	</div>
</section>
