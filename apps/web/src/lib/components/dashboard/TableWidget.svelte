<script lang="ts">
	import type { QueryResult } from '$lib/api/queries';
	import { toNumber, type DashboardTableWidget } from '$lib/utils/dashboards';

	interface Props {
		widget: DashboardTableWidget;
		result: QueryResult | null;
		globalSearch?: string;
	}

	let {
		widget,
		result,
		globalSearch = '',
	}: Props = $props();

	let localSearch = $state('');
	let currentPage = $state(1);
	let sortColumn = $state('');
	let sortDirection = $state<'asc' | 'desc'>('asc');

	const columns = $derived(result?.columns ?? []);
	const columnIndexMap = $derived(new Map(columns.map((column, index) => [column.name, index])));
	const effectiveSearch = $derived(`${globalSearch} ${localSearch}`.trim().toLowerCase());
	const filteredRows = $derived.by(() => {
		if (!result) {
			return [] as string[][];
		}

		if (!effectiveSearch) {
			return result.rows;
		}

		return result.rows.filter((row) => row.some((cell) => cell.toLowerCase().includes(effectiveSearch)));
	});

	const sortedRows = $derived.by(() => {
		const index = columnIndexMap.get(sortColumn) ?? 0;
		return [...filteredRows].sort((left, right) => {
			const leftNumeric = toNumber(left[index]);
			const rightNumeric = toNumber(right[index]);

			if (leftNumeric !== null && rightNumeric !== null) {
				return sortDirection === 'asc' ? leftNumeric - rightNumeric : rightNumeric - leftNumeric;
			}

			const comparison = left[index].localeCompare(right[index]);
			return sortDirection === 'asc' ? comparison : -comparison;
		});
	});

	const pageSize = $derived(Math.max(widget.pageSize, 1));
	const totalPages = $derived(Math.max(1, Math.ceil(sortedRows.length / pageSize)));
	const pagedRows = $derived(sortedRows.slice((currentPage - 1) * pageSize, currentPage * pageSize));

	$effect(() => {
		widget.defaultSortColumn;
		widget.defaultSortDirection;
		sortColumn = widget.defaultSortColumn;
		sortDirection = widget.defaultSortDirection;
	});

	$effect(() => {
		effectiveSearch;
		sortColumn;
		sortDirection;
		pageSize;
		result?.rows.length;
		currentPage = 1;
	});

	function toggleSort(column: string) {
		if (sortColumn === column) {
			sortDirection = sortDirection === 'asc' ? 'desc' : 'asc';
			return;
		}

		sortColumn = column;
		sortDirection = 'asc';
	}
</script>

<div class="flex h-full min-h-[280px] flex-col gap-3">
	<div class="flex flex-wrap items-center justify-between gap-2">
		<div class="text-xs text-slate-500 dark:text-slate-400">
			{sortedRows.length} rows after filters
		</div>

		<input
			type="text"
			value={localSearch}
			oninput={(event) => localSearch = (event.currentTarget as HTMLInputElement).value}
			placeholder="Filter visible rows"
			class="rounded-lg border border-slate-300 bg-slate-50 px-3 py-2 text-sm dark:border-slate-700 dark:bg-slate-950"
		/>
	</div>

	{#if result && columns.length > 0}
		<div class="min-h-0 flex-1 overflow-auto rounded-xl border border-slate-200 dark:border-slate-800">
			<table class="min-w-full text-left text-sm">
				<thead class="sticky top-0 bg-slate-50 text-xs uppercase tracking-wide text-slate-500 dark:bg-slate-900 dark:text-slate-400">
					<tr>
						{#each columns as column}
							<th class="border-b border-slate-200 px-3 py-2 font-semibold dark:border-slate-800">
								<button class="inline-flex items-center gap-1" onclick={() => toggleSort(column.name)}>
									<span>{column.name}</span>
									{#if sortColumn === column.name}
										<span>{sortDirection === 'asc' ? '↑' : '↓'}</span>
									{/if}
								</button>
							</th>
						{/each}
					</tr>
				</thead>
				<tbody>
					{#each pagedRows as row, index}
						<tr class={index % 2 === 0 ? 'bg-white dark:bg-slate-950/60' : 'bg-slate-50/80 dark:bg-slate-900/70'}>
							{#each row as cell}
								<td class="border-b border-slate-100 px-3 py-2 text-slate-700 dark:border-slate-900 dark:text-slate-200">{cell}</td>
							{/each}
						</tr>
					{/each}
				</tbody>
			</table>
		</div>

		<div class="flex items-center justify-between text-sm text-slate-500 dark:text-slate-400">
			<span>Page {currentPage} of {totalPages}</span>
			<div class="flex gap-2">
				<button
					class="rounded-lg border border-slate-300 px-3 py-1.5 disabled:opacity-50 dark:border-slate-700"
					onclick={() => currentPage = Math.max(1, currentPage - 1)}
					disabled={currentPage <= 1}
				>
					Prev
				</button>
				<button
					class="rounded-lg border border-slate-300 px-3 py-1.5 disabled:opacity-50 dark:border-slate-700"
					onclick={() => currentPage = Math.min(totalPages, currentPage + 1)}
					disabled={currentPage >= totalPages}
				>
					Next
				</button>
			</div>
		</div>
	{:else}
		<div class="flex flex-1 items-center justify-center rounded-xl border border-dashed border-slate-300 text-sm text-slate-500 dark:border-slate-700 dark:text-slate-400">
			This table widget is waiting for query results.
		</div>
	{/if}
</div>
