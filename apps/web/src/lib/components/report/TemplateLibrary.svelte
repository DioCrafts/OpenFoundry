<script lang="ts">
	import type { ReportCatalog } from '$lib/api/reports';

	export let catalog: ReportCatalog | null = null;
</script>

<section class="rounded-3xl border border-stone-200 bg-white p-5 shadow-sm shadow-stone-200/60">
	<div class="flex items-center justify-between gap-4">
		<div>
			<p class="text-xs font-semibold uppercase tracking-[0.24em] text-amber-700">Generator Catalog</p>
			<h3 class="mt-2 text-xl font-semibold text-stone-900">Templates, engines, and delivery channels</h3>
		</div>
		<p class="max-w-lg text-right text-sm text-stone-500">
			PDF and PPTX decks are simulated through the control plane, while HTML and CSV remain ideal for quick preview loops.
		</p>
	</div>

	{#if catalog}
		<div class="mt-5 grid gap-4 xl:grid-cols-2">
			<div class="space-y-3 rounded-2xl border border-stone-200 bg-stone-50 p-4">
				<p class="text-sm font-semibold text-stone-800">Generators</p>
				{#each catalog.generators as generator}
					<div class="rounded-2xl border border-stone-200 bg-white p-4">
						<div class="flex items-start justify-between gap-3">
							<div>
								<p class="text-base font-semibold text-stone-900">{generator.display_name}</p>
								<p class="text-sm text-stone-500">Engine: {generator.engine}</p>
							</div>
							<p class="rounded-full bg-amber-100 px-3 py-1 text-xs font-semibold uppercase tracking-[0.18em] text-amber-700">{generator.kind}</p>
						</div>
						<div class="mt-3 flex flex-wrap gap-2">
							{#each generator.extensions as extension}
								<span class="rounded-full bg-stone-100 px-2 py-1 text-xs text-stone-600">.{extension}</span>
							{/each}
						</div>
						<ul class="mt-3 space-y-1 text-sm text-stone-600">
							{#each generator.capabilities as capability}
								<li>{capability}</li>
							{/each}
						</ul>
					</div>
				{/each}
			</div>

			<div class="space-y-3 rounded-2xl border border-stone-200 bg-stone-50 p-4">
				<p class="text-sm font-semibold text-stone-800">Distribution channels</p>
				{#each catalog.delivery_channels as channel}
					<div class="rounded-2xl border border-stone-200 bg-white p-4">
						<div class="flex items-start justify-between gap-3">
							<div>
								<p class="text-base font-semibold text-stone-900">{channel.display_name}</p>
								<p class="text-sm text-stone-500">{channel.description}</p>
							</div>
							<p class="rounded-full bg-stone-100 px-3 py-1 text-xs font-semibold uppercase tracking-[0.18em] text-stone-600">{channel.channel}</p>
						</div>
						<div class="mt-3 flex flex-wrap gap-2">
							{#each channel.configuration_fields as field}
								<span class="rounded-full bg-emerald-50 px-2 py-1 text-xs font-medium text-emerald-700">{field}</span>
							{/each}
						</div>
					</div>
				{/each}
			</div>
		</div>
	{:else}
		<p class="mt-5 text-sm text-stone-500">Catalog data will appear once the report service responds.</p>
	{/if}
</section>
