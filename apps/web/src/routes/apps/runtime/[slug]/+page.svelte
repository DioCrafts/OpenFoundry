<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/stores';

	import AppRenderer from '$lib/components/apps/AppRenderer.svelte';
	import { getPublishedApp, type AppDefinition } from '$lib/api/apps';

	let app = $state<AppDefinition | null>(null);
	let loading = $state(true);
	let error = $state('');
	let publishedAt = $state('');
	let versionNumber = $state<number | null>(null);

	async function load() {
		loading = true;
		error = '';
		try {
			const slug = $page.params.slug;
			if (!slug) {
				throw new Error('Missing app slug');
			}

			const response = await getPublishedApp(slug);
			app = response.app;
			publishedAt = response.published_at;
			versionNumber = response.published_version_number;
		} catch (cause) {
			error = cause instanceof Error ? cause.message : 'Failed to load published app';
		} finally {
			loading = false;
		}
	}

	onMount(() => {
		void load();
	});
</script>

<svelte:head>
	<title>OpenFoundry — App Runtime</title>
</svelte:head>

<div class="space-y-6">
	<div class="flex flex-wrap items-center justify-between gap-4">
		<div>
			<h1 class="text-3xl font-semibold tracking-tight text-slate-950 dark:text-slate-50">Published App Runtime</h1>
			<p class="mt-2 text-sm text-slate-500 dark:text-slate-400">End-user rendering for published Workshop apps, suitable for direct navigation or iframe embedding.</p>
		</div>

		{#if versionNumber !== null}
			<div class="flex flex-wrap gap-2 text-xs text-slate-500 dark:text-slate-400">
				<span class="rounded-full border border-slate-200 px-3 py-1 dark:border-slate-700">Version {versionNumber}</span>
				<span class="rounded-full border border-slate-200 px-3 py-1 dark:border-slate-700">Published {publishedAt}</span>
			</div>
		{/if}
	</div>

	{#if loading}
		<div class="rounded-[1.75rem] border border-dashed border-slate-300 px-6 py-24 text-center text-sm text-slate-500 dark:border-slate-700">Loading published app...</div>
	{:else if error}
		<div class="rounded-[1.75rem] border border-rose-200 bg-rose-50 px-6 py-12 text-sm text-rose-700 dark:border-rose-900/40 dark:bg-rose-950/20 dark:text-rose-300">{error}</div>
	{:else if app}
		<AppRenderer {app} mode="published" />
	{/if}
</div>