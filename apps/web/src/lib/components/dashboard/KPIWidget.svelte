<script lang="ts">
	import { onMount } from 'svelte';
	import type { QueryResult } from '$lib/api/queries';
	import {
		formatMetricValue,
		parseSparklineSeries,
		toNumber,
		type DashboardKpiWidget,
	} from '$lib/utils/dashboards';

	interface Props {
		widget: DashboardKpiWidget;
		result: QueryResult | null;
	}

	let { widget, result }: Props = $props();

	let container = $state<HTMLDivElement | null>(null);
	let chart = $state<import('echarts').ECharts | null>(null);

	const firstRow = $derived(result?.rows[0] ?? null);
	const columns = $derived(result?.columns ?? []);

	function columnIndex(name: string) {
		return columns.findIndex((column) => column.name === name);
	}

	const value = $derived(firstRow ? firstRow[columnIndex(widget.valueColumn)] : null);
	const delta = $derived(firstRow ? toNumber(firstRow[columnIndex(widget.deltaColumn)]) : null);
	const sparkline = $derived(firstRow ? parseSparklineSeries(firstRow[columnIndex(widget.sparklineColumn)]) : []);

	function updateSparkline() {
		if (!chart) {
			return;
		}

		if (sparkline.length === 0) {
			chart.clear();
			return;
		}

		chart.setOption({
			animation: false,
			grid: { left: 0, right: 0, top: 0, bottom: 0 },
			xAxis: { type: 'category', show: false, data: sparkline.map((_, index) => index) },
			yAxis: { type: 'value', show: false },
			series: [
				{
					type: 'line',
					smooth: true,
					data: sparkline,
					showSymbol: false,
					lineStyle: { color: '#0f766e', width: 3 },
					areaStyle: { color: 'rgba(15, 118, 110, 0.18)' },
				},
			],
		}, true);
	}

	onMount(() => {
		let resizeObserver: ResizeObserver | null = null;
		let disposed = false;

		async function initialize() {
			const echartsModule = await import('echarts');

			if (disposed || !container) {
				return;
			}

			chart = echartsModule.init(container, undefined, { renderer: 'canvas' });
			updateSparkline();
			resizeObserver = new ResizeObserver(() => chart?.resize());
			resizeObserver.observe(container);
		}

		void initialize();

		return () => {
			disposed = true;
			resizeObserver?.disconnect();
			chart?.dispose();
			chart = null;
		};
	});

	$effect(() => {
		updateSparkline();
	});
</script>

<div class="flex h-full min-h-[200px] flex-col justify-between gap-6 rounded-2xl bg-[radial-gradient(circle_at_top_left,_rgba(15,118,110,0.16),_transparent_55%)] p-1">
	<div>
		<div class="text-xs font-semibold uppercase tracking-[0.24em] text-slate-500">Current Value</div>
		<div class="mt-3 text-4xl font-semibold text-slate-950 dark:text-slate-50">
			{formatMetricValue(value, widget.valueFormat)}
		</div>

		{#if delta !== null}
			<div class={`mt-3 inline-flex items-center gap-2 rounded-full px-3 py-1 text-sm font-medium ${delta >= 0 ? 'bg-emerald-100 text-emerald-700 dark:bg-emerald-950 dark:text-emerald-300' : 'bg-rose-100 text-rose-700 dark:bg-rose-950 dark:text-rose-300'}`}>
				<span>{delta >= 0 ? '▲' : '▼'}</span>
				<span>{Math.abs(delta).toFixed(1)}%</span>
			</div>
		{/if}
	</div>

	<div class="space-y-2">
		<div class="text-xs font-semibold uppercase tracking-[0.24em] text-slate-500">Sparkline</div>
		<div bind:this={container} class="h-20 w-full"></div>
	</div>
</div>
