<script lang="ts">
	import ChartWidget from '$lib/components/dashboard/ChartWidget.svelte';
	import TableWidget from '$lib/components/dashboard/TableWidget.svelte';
	import { getDataset, type Dataset } from '$lib/api/datasets';
	import type { ObjectInstance } from '$lib/api/ontology';
	import { listObjects } from '$lib/api/ontology';
	import { executeQuery, type QueryResult } from '$lib/api/queries';
	import type { AppWidget, WidgetEvent } from '$lib/api/apps';
	import AppWidgetRenderer from './AppWidgetRenderer.svelte';

	interface Props {
		widget: AppWidget;
		globalFilter?: string;
		onAction?: (event: WidgetEvent, payload?: Record<string, unknown>) => Promise<void> | void;
	}

	type FormField = {
		name: string;
		label: string;
		type: string;
		options?: string[];
	};

	let { widget, globalFilter = '', onAction }: Props = $props();

	let result = $state<QueryResult | null>(null);
	let dataset = $state<Dataset | null>(null);
	let loading = $state(false);
	let error = $state('');
	let formState = $state<Record<string, string>>({});

	const bindingKey = $derived(JSON.stringify(widget.binding ?? null));
	const content = $derived(stringProp('content', ''));
	const imageUrl = $derived(stringProp('url', ''));
	const imageAlt = $derived(stringProp('alt', widget.title));
	const formFields = $derived(parseFormFields(widget.props.fields));
	const mapPoints = $derived(buildMapPoints(result, widget));
	const chartWidget = $derived({
		id: widget.id,
		type: 'chart' as const,
		title: widget.title,
		description: widget.description,
		layout: { colSpan: 1, rowSpan: 1 },
		query: { sql: '', limit: numberProp('limit', 50) },
		chartType: normalizeChartType(stringProp('chart_type', 'line')),
		categoryColumn: stringProp('x_field', ''),
		seriesColumns: arrayProp('series_fields').map(String).filter(Boolean).length > 0
			? arrayProp('series_fields').map(String).filter(Boolean)
			: [stringProp('y_field', '')].filter(Boolean),
		stacked: booleanProp('stacked', false),
	});
	const tableWidget = $derived({
		id: widget.id,
		type: 'table' as const,
		title: widget.title,
		description: widget.description,
		layout: { colSpan: 1, rowSpan: 1 },
		query: { sql: '', limit: numberProp('limit', 50) },
		pageSize: Math.max(1, numberProp('page_size', 10)),
		defaultSortColumn: stringProp('default_sort_column', result?.columns[0]?.name ?? ''),
		defaultSortDirection: stringProp('default_sort_direction', 'asc') === 'desc' ? 'desc' as const : 'asc' as const,
	});

	$effect(() => {
		widget.id;
		formFields;
		const firstRow = result?.rows[0] ?? [];
		const nextState = Object.fromEntries(
			formFields.map((field, index) => [field.name, firstRow[index] ?? '']),
		);
		formState = nextState;
	});

	$effect(() => {
		bindingKey;
		void loadBinding();
	});

	async function loadBinding() {
		loading = true;
		error = '';
		result = null;
		dataset = null;

		try {
			if (!widget.binding) {
				loading = false;
				return;
			}

			if (widget.binding.source_type === 'query') {
				if (!widget.binding.query_text) {
					throw new Error('Query binding requires SQL');
				}
				result = await executeQuery(widget.binding.query_text, widget.binding.limit ?? 50);
				return;
			}

			if (widget.binding.source_type === 'ontology') {
				if (!widget.binding.source_id) {
					throw new Error('Ontology binding requires an object type');
				}
				const response = await listObjects(widget.binding.source_id, { per_page: widget.binding.limit ?? 25 });
				result = objectsToQueryResult(response.data);
				return;
			}

			if (widget.binding.source_type === 'dataset') {
				if (!widget.binding.source_id) {
					throw new Error('Dataset binding requires a dataset');
				}
				dataset = await getDataset(widget.binding.source_id);
				result = datasetToQueryResult(dataset);
			}
		} catch (cause) {
			error = cause instanceof Error ? cause.message : 'Binding load failed';
		} finally {
			loading = false;
		}
	}

	function stringProp(key: string, fallback: string) {
		const value = widget.props?.[key];
		return typeof value === 'string' ? value : fallback;
	}

	function numberProp(key: string, fallback: number) {
		const value = widget.props?.[key];
		if (typeof value === 'number' && Number.isFinite(value)) return value;
		if (typeof value === 'string') {
			const numeric = Number(value);
			return Number.isFinite(numeric) ? numeric : fallback;
		}
		return fallback;
	}

	function booleanProp(key: string, fallback: boolean) {
		const value = widget.props?.[key];
		return typeof value === 'boolean' ? value : fallback;
	}

	function arrayProp(key: string) {
		const value = widget.props?.[key];
		return Array.isArray(value) ? value : [];
	}

	function parseFormFields(value: unknown): FormField[] {
		if (!Array.isArray(value)) return [];
		return value
			.filter((entry): entry is Record<string, unknown> => Boolean(entry && typeof entry === 'object'))
			.map((entry) => ({
				name: typeof entry.name === 'string' ? entry.name : 'field',
				label: typeof entry.label === 'string' ? entry.label : (typeof entry.name === 'string' ? entry.name : 'Field'),
				type: typeof entry.type === 'string' ? entry.type : 'text',
				options: Array.isArray(entry.options) ? entry.options.map(String) : undefined,
			}));
	}

	function normalizeChartType(value: string) {
		return ['bar', 'line', 'area', 'pie', 'scatter'].includes(value) ? value as 'bar' | 'line' | 'area' | 'pie' | 'scatter' : 'line';
	}

	function objectsToQueryResult(objects: ObjectInstance[]): QueryResult {
		const fieldNames = Array.from(
			new Set(objects.flatMap((entry) => Object.keys(entry.properties ?? {}))),
		);
		const columns = ['id', ...fieldNames].map((name) => ({ name, data_type: 'text' }));
		const rows = objects.map((entry) => [
			entry.id,
			...fieldNames.map((field) => stringifyValue(entry.properties?.[field])),
		]);

		return {
			columns,
			rows,
			total_rows: objects.length,
			execution_time_ms: 0,
		};
	}

	function datasetToQueryResult(datasetValue: Dataset): QueryResult {
		return {
			columns: [
				{ name: 'attribute', data_type: 'text' },
				{ name: 'value', data_type: 'text' },
			],
			rows: [
				['name', datasetValue.name],
				['format', datasetValue.format],
				['rows', String(datasetValue.row_count)],
				['version', String(datasetValue.current_version)],
				['branch', datasetValue.active_branch],
				['tags', datasetValue.tags.join(', ') || 'none'],
			],
			total_rows: 6,
			execution_time_ms: 0,
		};
	}

	function stringifyValue(value: unknown) {
		if (value === null || value === undefined) return '';
		if (typeof value === 'string') return value;
		if (typeof value === 'number' || typeof value === 'boolean') return String(value);
		return JSON.stringify(value);
	}

	function buildMapPoints(value: QueryResult | null, currentWidget: AppWidget) {
		if (!value) return [] as Array<{ x: number; y: number; label: string }>;

		const latitudeField = typeof currentWidget.props.latitude_field === 'string' ? currentWidget.props.latitude_field : 'lat';
		const longitudeField = typeof currentWidget.props.longitude_field === 'string' ? currentWidget.props.longitude_field : 'lon';
		const labelField = typeof currentWidget.props.label_field === 'string' ? currentWidget.props.label_field : value.columns[0]?.name;

		const latitudeIndex = value.columns.findIndex((column) => column.name === latitudeField);
		const longitudeIndex = value.columns.findIndex((column) => column.name === longitudeField);
		const labelIndex = value.columns.findIndex((column) => column.name === labelField);

		if (latitudeIndex < 0 || longitudeIndex < 0) return [];

		return value.rows
			.map((row) => {
				const lat = Number(row[latitudeIndex]);
				const lon = Number(row[longitudeIndex]);
				if (!Number.isFinite(lat) || !Number.isFinite(lon)) return null;
				return {
					x: ((lon + 180) / 360) * 100,
					y: ((90 - lat) / 180) * 100,
					label: labelIndex >= 0 ? row[labelIndex] : `${lat.toFixed(2)}, ${lon.toFixed(2)}`,
				};
			})
			.filter((point): point is { x: number; y: number; label: string } => Boolean(point));
	}

	async function triggerEvents(trigger: string, payload?: Record<string, unknown>) {
		for (const event of widget.events.filter((entry) => entry.trigger === trigger)) {
			await onAction?.(event, payload);
		}
	}

	async function handleFormSubmit(event: SubmitEvent) {
		event.preventDefault();
		await triggerEvents('submit', formState);
	}

	function contentLines() {
		return content.split('\n');
	}
</script>

<article class="flex h-full min-h-[160px] flex-col rounded-[24px] border border-slate-200 bg-white p-4 shadow-sm">
	<header class="mb-3 flex items-start justify-between gap-3">
		<div>
			<div class="flex items-center gap-2">
				<h3 class="text-base font-semibold text-slate-950">{widget.title}</h3>
				<span class="rounded-full bg-slate-100 px-2 py-0.5 text-[11px] uppercase tracking-[0.2em] text-slate-500">{widget.widget_type}</span>
			</div>
			{#if widget.description}
				<p class="mt-1 text-sm text-slate-500">{widget.description}</p>
			{/if}
		</div>

		{#if widget.binding}
			<span class="rounded-full border border-slate-200 px-2 py-1 text-[11px] uppercase tracking-[0.2em] text-slate-400">{widget.binding.source_type}</span>
		{/if}
	</header>

	{#if error}
		<div class="mb-3 rounded-xl border border-rose-200 bg-rose-50 px-3 py-2 text-sm text-rose-700">{error}</div>
	{/if}

	{#if loading}
		<div class="flex flex-1 items-center justify-center rounded-xl border border-dashed border-slate-300 text-sm text-slate-500">
			Loading binding data...
		</div>
	{:else if widget.widget_type === 'text'}
		<div class="flex-1 space-y-2 text-slate-700">
			{#each contentLines() as line}
				{#if line.startsWith('### ')}
					<h4 class="text-lg font-semibold">{line.slice(4)}</h4>
				{:else if line.startsWith('## ')}
					<h3 class="text-2xl font-semibold">{line.slice(3)}</h3>
				{:else if line.startsWith('# ')}
					<h2 class="text-3xl font-semibold">{line.slice(2)}</h2>
				{:else}
					<p class="whitespace-pre-wrap text-sm leading-6 text-slate-600">{line}</p>
				{/if}
			{/each}
		</div>
	{:else if widget.widget_type === 'image'}
		<div class="flex flex-1 items-center justify-center overflow-hidden rounded-2xl bg-slate-100">
			{#if imageUrl}
				<img src={imageUrl} alt={imageAlt} class="h-full w-full object-cover" />
			{:else}
				<div class="text-sm text-slate-500">Add an image URL</div>
			{/if}
		</div>
	{:else if widget.widget_type === 'button'}
		<div class="flex flex-1 items-center justify-center">
			<button
				type="button"
				class="rounded-2xl bg-slate-900 px-5 py-3 text-sm font-medium text-white"
				onclick={() => void triggerEvents('click')}
			>
				{stringProp('label', widget.title || 'Run action')}
			</button>
		</div>
	{:else if widget.widget_type === 'form'}
		<form class="grid flex-1 gap-3" onsubmit={handleFormSubmit}>
			{#each formFields as field}
				<label class="space-y-1 text-sm">
					<span class="font-medium text-slate-700">{field.label}</span>
					{#if field.type === 'textarea'}
						<textarea
							rows="3"
							value={formState[field.name] ?? ''}
							oninput={(event) => formState = { ...formState, [field.name]: (event.currentTarget as HTMLTextAreaElement).value }}
							class="w-full rounded-xl border border-slate-200 px-3 py-2"
						></textarea>
					{:else if field.type === 'select'}
						<select
							value={formState[field.name] ?? ''}
							oninput={(event) => formState = { ...formState, [field.name]: (event.currentTarget as HTMLSelectElement).value }}
							class="w-full rounded-xl border border-slate-200 px-3 py-2"
						>
							<option value="">Select...</option>
							{#each field.options ?? [] as option}
								<option value={option}>{option}</option>
							{/each}
						</select>
					{:else}
						<input
							type={field.type}
							value={formState[field.name] ?? ''}
							oninput={(event) => formState = { ...formState, [field.name]: (event.currentTarget as HTMLInputElement).value }}
							class="w-full rounded-xl border border-slate-200 px-3 py-2"
						/>
					{/if}
				</label>
			{/each}

			<div class="pt-2">
				<button type="submit" class="rounded-xl bg-[var(--app-primary,#0f766e)] px-4 py-2 text-sm font-medium text-white">
					{stringProp('submit_label', 'Submit')}
				</button>
			</div>
		</form>
	{:else if widget.widget_type === 'table'}
		<div class="min-h-0 flex-1">
			<TableWidget widget={tableWidget} result={result} globalSearch={globalFilter} />
		</div>
	{:else if widget.widget_type === 'chart'}
		<div class="min-h-0 flex-1">
			<ChartWidget widget={chartWidget} result={result} />
		</div>
	{:else if widget.widget_type === 'map'}
		<div class="relative flex-1 overflow-hidden rounded-2xl border border-slate-200 bg-[radial-gradient(circle_at_top_left,_rgba(15,118,110,0.18),_transparent_40%),linear-gradient(135deg,_#e0f2fe,_#f8fafc)]">
			<div class="absolute inset-0 bg-[linear-gradient(rgba(15,23,42,0.05)_1px,transparent_1px),linear-gradient(90deg,rgba(15,23,42,0.05)_1px,transparent_1px)] bg-[size:48px_48px]"></div>
			{#each mapPoints as point}
				<div class="absolute -translate-x-1/2 -translate-y-1/2" style={`left:${point.x}%;top:${point.y}%;`}>
					<div class="flex flex-col items-center gap-1">
						<span class="h-3 w-3 rounded-full border-2 border-white bg-[var(--app-primary,#0f766e)] shadow"></span>
						<span class="rounded-full bg-white/90 px-2 py-1 text-[11px] font-medium text-slate-700 shadow">{point.label}</span>
					</div>
				</div>
			{/each}
			{#if mapPoints.length === 0}
				<div class="flex h-full items-center justify-center text-sm text-slate-500">Map bindings need `lat` and `lon` columns.</div>
			{/if}
		</div>
	{:else if widget.widget_type === 'container'}
		<div class="flex flex-1 flex-col gap-3 rounded-2xl border border-dashed border-slate-300 bg-slate-50 p-3">
			<div class="text-sm font-medium text-slate-600">{stringProp('title', widget.title)}</div>
			{#if widget.children.length === 0}
				<div class="flex flex-1 items-center justify-center text-sm text-slate-400">Drop related widgets inside this section from a template or nested configuration.</div>
			{:else}
				<div class="grid flex-1 gap-3 md:grid-cols-2">
					{#each widget.children as child (child.id)}
						<AppWidgetRenderer widget={child} globalFilter={globalFilter} onAction={onAction} />
					{/each}
				</div>
			{/if}
		</div>
	{:else}
		<div class="flex flex-1 items-center justify-center rounded-xl border border-dashed border-slate-300 text-sm text-slate-500">
			Unsupported widget type.
		</div>
	{/if}

	{#if dataset && widget.binding?.source_type === 'dataset'}
		<div class="mt-3 rounded-xl bg-slate-50 px-3 py-2 text-xs text-slate-500">
			Dataset binding currently exposes metadata while row preview is still limited in the dataset service.
		</div>
	{/if}
</article>