<script lang="ts">
	import {
		cloneDashboard,
		createWidget,
		type DashboardWidget,
		type DashboardWidgetType,
	} from '$lib/utils/dashboards';

	interface Props {
		open: boolean;
		initialWidget: DashboardWidget | null;
		onSave?: (widget: DashboardWidget) => void;
		onClose?: () => void;
	}

	let {
		open,
		initialWidget,
		onSave,
		onClose,
	}: Props = $props();

	let draft = $state<DashboardWidget | null>(null);
	let seriesColumnsInput = $state('');

	$effect(() => {
		draft = initialWidget ? cloneDashboard(initialWidget) : null;
		seriesColumnsInput = draft && draft.type === 'chart' ? draft.seriesColumns.join(', ') : '';
	});

	function close() {
		onClose?.();
	}

	function switchType(type: DashboardWidgetType) {
		if (!draft || draft.type === type) {
			return;
		}

		const template = createWidget(type);
		const nextDraft: DashboardWidget = {
			...template,
			id: draft.id,
			title: draft.title,
			description: draft.description,
			query: draft.query,
			layout: draft.layout,
		};
		draft = nextDraft;
		seriesColumnsInput = nextDraft.type === 'chart' ? nextDraft.seriesColumns.join(', ') : '';
	}

	function save() {
		if (!draft) {
			return;
		}

		if (draft.type === 'chart') {
			draft.seriesColumns = seriesColumnsInput
				.split(',')
				.map((value) => value.trim())
				.filter(Boolean);
		}

		onSave?.(cloneDashboard(draft));
		close();
	}
</script>

{#if open && draft}
	{@const widget = draft}
	<div class="fixed inset-0 z-50 flex items-center justify-center bg-slate-950/60 p-4 backdrop-blur-sm">
		<div class="max-h-[90vh] w-full max-w-3xl overflow-auto rounded-3xl border border-slate-200 bg-white p-6 shadow-2xl dark:border-slate-800 dark:bg-slate-950">
			<div class="mb-6 flex items-start justify-between gap-3">
				<div>
					<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Widget Editor</div>
					<h2 class="mt-1 text-2xl font-semibold text-slate-950 dark:text-slate-100">Configure widget</h2>
				</div>
				<button class="rounded-lg border border-slate-300 px-3 py-2 text-sm dark:border-slate-700" onclick={close}>Close</button>
			</div>

			<div class="grid gap-4 md:grid-cols-2">
				<label class="space-y-2 text-sm font-medium text-slate-700 dark:text-slate-200">
					<span>Title</span>
					<input
						type="text"
						value={widget.title}
						oninput={(event) => widget.title = (event.currentTarget as HTMLInputElement).value}
						class="w-full rounded-xl border border-slate-300 bg-slate-50 px-3 py-2 dark:border-slate-700 dark:bg-slate-900"
					/>
				</label>

				<label class="space-y-2 text-sm font-medium text-slate-700 dark:text-slate-200">
					<span>Widget Type</span>
					<select
						class="w-full rounded-xl border border-slate-300 bg-slate-50 px-3 py-2 dark:border-slate-700 dark:bg-slate-900"
						value={widget.type}
						onchange={(event) => switchType((event.currentTarget as HTMLSelectElement).value as DashboardWidgetType)}
					>
						<option value="chart">Chart</option>
						<option value="table">Table</option>
						<option value="kpi">KPI</option>
					</select>
				</label>
			</div>

			<label class="mt-4 block space-y-2 text-sm font-medium text-slate-700 dark:text-slate-200">
				<span>Description</span>
				<textarea
					rows="2"
					value={widget.description}
					oninput={(event) => widget.description = (event.currentTarget as HTMLTextAreaElement).value}
					class="w-full rounded-xl border border-slate-300 bg-slate-50 px-3 py-2 dark:border-slate-700 dark:bg-slate-900"
				></textarea>
			</label>

			<div class="mt-4 grid gap-4 md:grid-cols-4">
				<label class="space-y-2 text-sm font-medium text-slate-700 dark:text-slate-200">
					<span>Columns</span>
					<input
						type="number"
						min="1"
						max="12"
						value={widget.layout.colSpan}
						oninput={(event) => widget.layout.colSpan = Number((event.currentTarget as HTMLInputElement).value)}
						class="w-full rounded-xl border border-slate-300 bg-slate-50 px-3 py-2 dark:border-slate-700 dark:bg-slate-900"
					/>
				</label>

				<label class="space-y-2 text-sm font-medium text-slate-700 dark:text-slate-200">
					<span>Rows</span>
					<input
						type="number"
						min="1"
						max="4"
						value={widget.layout.rowSpan}
						oninput={(event) => widget.layout.rowSpan = Number((event.currentTarget as HTMLInputElement).value)}
						class="w-full rounded-xl border border-slate-300 bg-slate-50 px-3 py-2 dark:border-slate-700 dark:bg-slate-900"
					/>
				</label>

				<label class="space-y-2 text-sm font-medium text-slate-700 dark:text-slate-200 md:col-span-2">
					<span>Query Limit</span>
					<input
						type="number"
						min="1"
						max="1000"
						value={widget.query.limit}
						oninput={(event) => widget.query.limit = Number((event.currentTarget as HTMLInputElement).value)}
						class="w-full rounded-xl border border-slate-300 bg-slate-50 px-3 py-2 dark:border-slate-700 dark:bg-slate-900"
					/>
				</label>
			</div>

			<label class="mt-4 block space-y-2 text-sm font-medium text-slate-700 dark:text-slate-200">
				<span>SQL Query</span>
				<textarea
					rows="8"
					value={widget.query.sql}
					oninput={(event) => widget.query.sql = (event.currentTarget as HTMLTextAreaElement).value}
					class="w-full rounded-2xl border border-slate-300 bg-slate-950 px-4 py-3 font-mono text-sm text-slate-100 dark:border-slate-700"
				></textarea>
			</label>

			<div class="mt-3 rounded-xl border border-dashed border-slate-300 px-3 py-2 text-xs text-slate-500 dark:border-slate-700 dark:text-slate-400">
				Available placeholders:
				<span class="font-mono">{'{{search}}'}</span>,
				<span class="font-mono">{'{{date_from}}'}</span>,
				<span class="font-mono">{'{{date_to}}'}</span>
			</div>

			{#if widget.type === 'chart'}
				<div class="mt-6 grid gap-4 md:grid-cols-2">
					<label class="space-y-2 text-sm font-medium text-slate-700 dark:text-slate-200">
						<span>Chart Type</span>
						<select
							class="w-full rounded-xl border border-slate-300 bg-slate-50 px-3 py-2 dark:border-slate-700 dark:bg-slate-900"
							value={widget.chartType}
							onchange={(event) => widget.chartType = (event.currentTarget as HTMLSelectElement).value as typeof widget.chartType}
						>
							<option value="bar">Bar</option>
							<option value="line">Line</option>
							<option value="area">Area</option>
							<option value="pie">Pie</option>
							<option value="scatter">Scatter</option>
						</select>
					</label>

					<label class="space-y-2 text-sm font-medium text-slate-700 dark:text-slate-200">
						<span>Category Column</span>
						<input
							type="text"
							value={widget.categoryColumn}
							oninput={(event) => widget.categoryColumn = (event.currentTarget as HTMLInputElement).value}
							class="w-full rounded-xl border border-slate-300 bg-slate-50 px-3 py-2 dark:border-slate-700 dark:bg-slate-900"
						/>
					</label>

					<label class="space-y-2 text-sm font-medium text-slate-700 dark:text-slate-200 md:col-span-2">
						<span>Series Columns</span>
						<input
							type="text"
							value={seriesColumnsInput}
							oninput={(event) => seriesColumnsInput = (event.currentTarget as HTMLInputElement).value}
							placeholder="ingested, published"
							class="w-full rounded-xl border border-slate-300 bg-slate-50 px-3 py-2 dark:border-slate-700 dark:bg-slate-900"
						/>
					</label>

					<label class="inline-flex items-center gap-2 text-sm font-medium text-slate-700 dark:text-slate-200">
						<input type="checkbox" checked={widget.stacked} onchange={(event) => widget.stacked = (event.currentTarget as HTMLInputElement).checked} />
						<span>Stack series</span>
					</label>
				</div>
			{:else if widget.type === 'table'}
				<div class="mt-6 grid gap-4 md:grid-cols-3">
					<label class="space-y-2 text-sm font-medium text-slate-700 dark:text-slate-200">
						<span>Page Size</span>
						<input
							type="number"
							min="3"
							max="50"
							value={widget.pageSize}
							oninput={(event) => widget.pageSize = Number((event.currentTarget as HTMLInputElement).value)}
							class="w-full rounded-xl border border-slate-300 bg-slate-50 px-3 py-2 dark:border-slate-700 dark:bg-slate-900"
						/>
					</label>

					<label class="space-y-2 text-sm font-medium text-slate-700 dark:text-slate-200">
						<span>Default Sort Column</span>
						<input
							type="text"
							value={widget.defaultSortColumn}
							oninput={(event) => widget.defaultSortColumn = (event.currentTarget as HTMLInputElement).value}
							class="w-full rounded-xl border border-slate-300 bg-slate-50 px-3 py-2 dark:border-slate-700 dark:bg-slate-900"
						/>
					</label>

					<label class="space-y-2 text-sm font-medium text-slate-700 dark:text-slate-200">
						<span>Sort Direction</span>
						<select
							class="w-full rounded-xl border border-slate-300 bg-slate-50 px-3 py-2 dark:border-slate-700 dark:bg-slate-900"
							value={widget.defaultSortDirection}
							onchange={(event) => widget.defaultSortDirection = (event.currentTarget as HTMLSelectElement).value as 'asc' | 'desc'}
						>
							<option value="asc">Ascending</option>
							<option value="desc">Descending</option>
						</select>
					</label>
				</div>
			{:else}
				<div class="mt-6 grid gap-4 md:grid-cols-2">
					<label class="space-y-2 text-sm font-medium text-slate-700 dark:text-slate-200">
						<span>Value Column</span>
						<input
							type="text"
							value={widget.valueColumn}
							oninput={(event) => widget.valueColumn = (event.currentTarget as HTMLInputElement).value}
							class="w-full rounded-xl border border-slate-300 bg-slate-50 px-3 py-2 dark:border-slate-700 dark:bg-slate-900"
						/>
					</label>

					<label class="space-y-2 text-sm font-medium text-slate-700 dark:text-slate-200">
						<span>Delta Column</span>
						<input
							type="text"
							value={widget.deltaColumn}
							oninput={(event) => widget.deltaColumn = (event.currentTarget as HTMLInputElement).value}
							class="w-full rounded-xl border border-slate-300 bg-slate-50 px-3 py-2 dark:border-slate-700 dark:bg-slate-900"
						/>
					</label>

					<label class="space-y-2 text-sm font-medium text-slate-700 dark:text-slate-200">
						<span>Sparkline Column</span>
						<input
							type="text"
							value={widget.sparklineColumn}
							oninput={(event) => widget.sparklineColumn = (event.currentTarget as HTMLInputElement).value}
							class="w-full rounded-xl border border-slate-300 bg-slate-50 px-3 py-2 dark:border-slate-700 dark:bg-slate-900"
						/>
					</label>

					<label class="space-y-2 text-sm font-medium text-slate-700 dark:text-slate-200">
						<span>Value Format</span>
						<select
							class="w-full rounded-xl border border-slate-300 bg-slate-50 px-3 py-2 dark:border-slate-700 dark:bg-slate-900"
							value={widget.valueFormat}
							onchange={(event) => widget.valueFormat = (event.currentTarget as HTMLSelectElement).value as typeof widget.valueFormat}
						>
							<option value="number">Number</option>
							<option value="currency">Currency</option>
							<option value="percent">Percent</option>
						</select>
					</label>
				</div>
			{/if}

			<div class="mt-8 flex justify-end gap-3">
				<button class="rounded-xl border border-slate-300 px-4 py-2 text-sm font-medium dark:border-slate-700" onclick={close}>Cancel</button>
				<button
					class="rounded-xl bg-slate-900 px-4 py-2 text-sm font-medium text-white disabled:opacity-60 dark:bg-slate-100 dark:text-slate-950"
					onclick={save}
					disabled={!widget.title.trim() || !widget.query.sql.trim()}
				>
					Save Widget
				</button>
			</div>
		</div>
	</div>
{/if}
