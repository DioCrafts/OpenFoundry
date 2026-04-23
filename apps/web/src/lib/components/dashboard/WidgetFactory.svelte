<script lang="ts">
	import { executeQuery, type QueryResult } from '$lib/api/queries';
	import ChartWidget from './ChartWidget.svelte';
	import TableWidget from './TableWidget.svelte';
	import KPIWidget from './KPIWidget.svelte';
	import {
		applyDashboardQueryTemplate,
		formatDashboardTimestamp,
		type DashboardFilterState,
		type DashboardWidget,
	} from '$lib/utils/dashboards';

	interface Props {
		widget: DashboardWidget;
		filters: DashboardFilterState;
	}

	let { widget, filters }: Props = $props();

	let result = $state<QueryResult | null>(null);
	let loading = $state(true);
	let error = $state('');
	let lastLoadedAt = $state<string | null>(null);
	let currentRequest = 0;

	const renderedSql = $derived(applyDashboardQueryTemplate(widget.query.sql, filters));
	const requestKey = $derived(`${widget.id}:${widget.query.limit}:${renderedSql}`);

	async function loadData() {
		currentRequest += 1;
		const requestId = currentRequest;
		loading = true;
		error = '';

		try {
			const next = await executeQuery(renderedSql, widget.query.limit);
			if (requestId !== currentRequest) {
				return;
			}

			result = next;
			lastLoadedAt = new Date().toISOString();
		} catch (loadError: any) {
			if (requestId !== currentRequest) {
				return;
			}

			result = null;
			error = loadError?.message ?? 'Widget query failed';
		} finally {
			if (requestId === currentRequest) {
				loading = false;
			}
		}
	}

	$effect(() => {
		requestKey;
		void loadData();
	});
</script>

<article class="flex h-full min-h-[220px] flex-col rounded-2xl border border-slate-200 bg-white p-4 shadow-sm dark:border-slate-800 dark:bg-slate-950">
	<header class="mb-4 flex flex-wrap items-start justify-between gap-3">
		<div>
			<div class="flex items-center gap-2">
				<h3 class="text-lg font-semibold text-slate-950 dark:text-slate-100">{widget.title}</h3>
				<span class="rounded-full bg-slate-100 px-2 py-0.5 text-[11px] font-semibold uppercase tracking-[0.2em] text-slate-500 dark:bg-slate-800 dark:text-slate-400">
					{widget.type}
				</span>
			</div>
			<p class="mt-1 text-sm text-slate-500 dark:text-slate-400">{widget.description}</p>
		</div>

		<div class="flex items-center gap-2">
			{#if lastLoadedAt}
				<span class="text-xs text-slate-400">{formatDashboardTimestamp(lastLoadedAt)}</span>
			{/if}
			<button
				class="rounded-lg border border-slate-300 px-3 py-1.5 text-sm font-medium text-slate-700 hover:bg-slate-100 dark:border-slate-700 dark:text-slate-200 dark:hover:bg-slate-800"
				onclick={() => void loadData()}
				disabled={loading}
			>
				{loading ? 'Refreshing...' : 'Refresh'}
			</button>
		</div>
	</header>

	{#if error}
		<div class="mb-4 rounded-xl border border-rose-200 bg-rose-50 px-3 py-2 text-sm text-rose-700 dark:border-rose-900 dark:bg-rose-950/40 dark:text-rose-300">
			{error}
		</div>
	{/if}

	<div class="min-h-0 flex-1">
		{#if widget.type === 'chart'}
			<ChartWidget widget={widget} result={result} />
		{:else if widget.type === 'table'}
			<TableWidget widget={widget} result={result} globalSearch={filters.search} />
		{:else}
			<KPIWidget widget={widget} result={result} />
		{/if}
	</div>
</article>
