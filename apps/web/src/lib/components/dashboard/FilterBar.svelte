<script lang="ts">
	import DateRangeFilter from './DateRangeFilter.svelte';
	import {
		createDefaultDateRange,
		type DashboardDateRange,
		type DashboardFilterState,
	} from '$lib/utils/dashboards';

	interface Props {
		search: string;
		dateRange: DashboardDateRange;
		busy?: boolean;
		onApply?: (filters: DashboardFilterState) => void;
		onReset?: () => void;
	}

	let {
		search,
		dateRange,
		busy = false,
		onApply,
		onReset,
	}: Props = $props();

	let draftSearch = $state('');
	let draftDateRange = $state(createDefaultDateRange());

	$effect(() => {
		draftSearch = search;
		draftDateRange = { ...dateRange };
	});

	function applyFilters() {
		onApply?.({
			search: draftSearch.trim(),
			dateRange: draftDateRange,
		});
	}

	function resetFilters() {
		draftSearch = '';
		draftDateRange = createDefaultDateRange();
		onReset?.();
	}
</script>

<div class="space-y-3 rounded-2xl border border-slate-200 bg-gradient-to-br from-white to-slate-50 p-4 shadow-sm dark:border-slate-800 dark:from-slate-900 dark:to-slate-950">
	<div class="flex flex-wrap items-start justify-between gap-3">
		<div>
			<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Global Filters</div>
			<h2 class="mt-1 text-lg font-semibold text-slate-900 dark:text-slate-100">Propagate one filter context across every widget</h2>
		</div>

		<div class="flex flex-wrap gap-2">
			<button
				class="rounded-lg border border-slate-300 px-3 py-2 text-sm font-medium text-slate-700 hover:bg-slate-100 dark:border-slate-700 dark:text-slate-200 dark:hover:bg-slate-800"
				onclick={resetFilters}
				disabled={busy}
			>
				Reset
			</button>
			<button
				class="rounded-lg bg-slate-900 px-4 py-2 text-sm font-medium text-white hover:bg-slate-800 disabled:opacity-60 dark:bg-slate-100 dark:text-slate-950 dark:hover:bg-white"
				onclick={applyFilters}
				disabled={busy}
			>
				{busy ? 'Applying...' : 'Apply Filters'}
			</button>
		</div>
	</div>

	<div class="grid gap-3 xl:grid-cols-[minmax(0,1.2fr)_minmax(0,1fr)]">
		<label class="rounded-xl border border-slate-200 bg-white px-3 py-2 dark:border-slate-800 dark:bg-slate-900">
			<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Search</div>
			<input
				type="text"
				value={draftSearch}
				oninput={(event) => draftSearch = (event.currentTarget as HTMLInputElement).value}
				placeholder={'Use {{search}} in widget SQL or rely on table filtering'}
				class="mt-2 w-full bg-transparent text-sm text-slate-900 outline-none placeholder:text-slate-400 dark:text-slate-100"
			/>
		</label>

		<DateRangeFilter value={draftDateRange} onChange={(value) => draftDateRange = value} disabled={busy} />
	</div>

	<div class="rounded-xl border border-dashed border-slate-300 px-3 py-2 text-xs text-slate-500 dark:border-slate-700 dark:text-slate-400">
		Query placeholders available in every widget:
		<span class="font-mono">{'{{search}}'}</span>,
		<span class="font-mono">{'{{date_from}}'}</span>,
		<span class="font-mono">{'{{date_to}}'}</span>
	</div>
</div>
