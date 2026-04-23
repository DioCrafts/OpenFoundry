<script lang="ts">
	import { onMount } from 'svelte';
	import type { QueryResult } from '$lib/api/queries';
	import { toNumber, type DashboardChartWidget } from '$lib/utils/dashboards';

	interface Props {
		widget: DashboardChartWidget;
		result: QueryResult | null;
	}

	let { widget, result }: Props = $props();

	let container = $state<HTMLDivElement | null>(null);
	let echartsModule = $state<typeof import('echarts') | null>(null);
	let chart = $state<import('echarts').ECharts | null>(null);

	function columnIndex(name: string) {
		return result?.columns.findIndex((column) => column.name === name) ?? -1;
	}

	function buildOptions() {
		if (!result || result.rows.length === 0) {
			return null;
		}

		const numericColumns = result.columns
			.filter((column, index) => result.rows.some((row) => toNumber(row[index]) !== null))
			.map((column) => column.name);

		const categoryColumn = widget.categoryColumn || result.columns.find((column) => !numericColumns.includes(column.name))?.name || result.columns[0]?.name;
		const categoryIndex = columnIndex(categoryColumn);
		const seriesColumns = widget.seriesColumns.length > 0
			? widget.seriesColumns
			: numericColumns.filter((column) => column !== categoryColumn);

		const palette = ['#0f766e', '#0369a1', '#c2410c', '#7c3aed', '#be123c'];

		if (widget.chartType === 'pie') {
			const valueColumn = seriesColumns[0] ?? numericColumns[0];
			const valueIndex = columnIndex(valueColumn);

			return {
				color: palette,
				tooltip: { trigger: 'item' },
				legend: { bottom: 0, textStyle: { color: '#64748b' } },
				series: [
					{
						type: 'pie',
						radius: ['36%', '70%'],
						avoidLabelOverlap: true,
						data: result.rows.map((row) => ({
							name: categoryIndex >= 0 ? row[categoryIndex] : valueColumn,
							value: toNumber(row[valueIndex]) ?? 0,
						})),
					},
				],
			};
		}

		if (widget.chartType === 'scatter') {
			const [xColumn, yColumn] = seriesColumns.slice(0, 2);
			const xIndex = columnIndex(xColumn ?? numericColumns[0]);
			const yIndex = columnIndex(yColumn ?? numericColumns[1] ?? numericColumns[0]);

			return {
				color: palette,
				tooltip: { trigger: 'item' },
				xAxis: { type: 'value', axisLabel: { color: '#64748b' } },
				yAxis: { type: 'value', axisLabel: { color: '#64748b' } },
				series: [
					{
						type: 'scatter',
						symbolSize: 14,
						data: result.rows.map((row) => [toNumber(row[xIndex]) ?? 0, toNumber(row[yIndex]) ?? 0]),
					},
				],
			};
		}

		const categories = categoryIndex >= 0
			? result.rows.map((row) => row[categoryIndex])
			: result.rows.map((_, index) => `${index + 1}`);

		return {
			color: palette,
			tooltip: { trigger: 'axis' },
			legend: { top: 0, textStyle: { color: '#64748b' } },
			grid: { left: 12, right: 12, top: 32, bottom: 12, containLabel: true },
			xAxis: {
				type: 'category',
				boundaryGap: widget.chartType === 'bar',
				data: categories,
				axisLabel: { color: '#64748b' },
			},
			yAxis: {
				type: 'value',
				axisLabel: { color: '#64748b' },
				splitLine: { lineStyle: { color: 'rgba(148, 163, 184, 0.15)' } },
			},
			series: seriesColumns.map((seriesColumn) => {
				const index = columnIndex(seriesColumn);
				const type = widget.chartType === 'area' ? 'line' : widget.chartType;

				return {
					name: seriesColumn,
					type,
					stack: widget.stacked ? 'total' : undefined,
					smooth: type === 'line',
					areaStyle: widget.chartType === 'area' ? { opacity: 0.18 } : undefined,
					emphasis: { focus: 'series' },
					data: result.rows.map((row) => toNumber(row[index]) ?? 0),
				};
			}),
		};
	}

	function updateChart() {
		if (!chart) {
			return;
		}

		const options = buildOptions();
		if (!options) {
			chart.clear();
			return;
		}

		chart.setOption(options, true);
	}

	onMount(() => {
		let resizeObserver: ResizeObserver | null = null;
		let disposed = false;

		async function initialize() {
			echartsModule = await import('echarts');

			if (disposed || !container) {
				return;
			}

			chart = echartsModule.init(container, undefined, { renderer: 'canvas' });
			updateChart();

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
		updateChart();
	});
</script>

{#if result && result.rows.length > 0}
	<div bind:this={container} class="h-full min-h-[240px] w-full"></div>
{:else}
	<div class="flex h-full min-h-[240px] items-center justify-center rounded-xl border border-dashed border-slate-300 text-sm text-slate-500 dark:border-slate-700 dark:text-slate-400">
		Run the widget query to render chart data.
	</div>
{/if}
