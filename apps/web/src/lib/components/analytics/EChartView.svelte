<script lang="ts">
  import { onMount } from 'svelte';

  interface Props {
    rows: Array<Record<string, unknown>>;
    categoryKey: string;
    valueKeys: string[];
    mode?: 'bar' | 'line' | 'area' | 'pie';
    emptyLabel?: string;
    onCategoryClick?: (value: string) => void;
  }

  let {
    rows,
    categoryKey,
    valueKeys,
    mode = 'bar',
    emptyLabel = 'No data available for this view.',
    onCategoryClick,
  }: Props = $props();

  let container = $state<HTMLDivElement | null>(null);
  let chart = $state<import('echarts').ECharts | null>(null);

  function toNumber(value: unknown) {
    if (typeof value === 'number' && Number.isFinite(value)) {
      return value;
    }

    if (typeof value === 'string') {
      const numeric = Number(value.replace(/,/g, ''));
      return Number.isFinite(numeric) ? numeric : 0;
    }

    return 0;
  }

  function options() {
    if (rows.length === 0 || !categoryKey || valueKeys.length === 0) {
      return null;
    }

    const palette = ['#0f766e', '#0369a1', '#f97316', '#be123c', '#7c3aed'];
    const categories = rows.map((row, index) => String(row[categoryKey] ?? `Row ${index + 1}`));

    if (mode === 'pie') {
      const valueKey = valueKeys[0];
      return {
        color: palette,
        tooltip: { trigger: 'item' },
        legend: { bottom: 0, textStyle: { color: '#64748b' } },
        series: [
          {
            type: 'pie',
            radius: ['36%', '72%'],
            data: rows.map((row, index) => ({
              name: String(row[categoryKey] ?? `Row ${index + 1}`),
              value: toNumber(row[valueKey]),
            })),
          },
        ],
      };
    }

    return {
      color: palette,
      tooltip: { trigger: 'axis' },
      legend: { top: 0, textStyle: { color: '#64748b' } },
      grid: { left: 12, right: 12, top: 32, bottom: 12, containLabel: true },
      xAxis: {
        type: 'category',
        boundaryGap: mode === 'bar',
        data: categories,
        axisLabel: { color: '#64748b' },
      },
      yAxis: {
        type: 'value',
        axisLabel: { color: '#64748b' },
        splitLine: { lineStyle: { color: 'rgba(148, 163, 184, 0.15)' } },
      },
      series: valueKeys.map((valueKey) => ({
        name: valueKey,
        type: mode === 'area' ? 'line' : mode,
        smooth: mode !== 'bar',
        areaStyle: mode === 'area' ? { opacity: 0.18 } : undefined,
        data: rows.map((row) => toNumber(row[valueKey])),
      })),
    };
  }

  function refresh() {
    if (!chart) {
      return;
    }

    const next = options();
    if (!next) {
      chart.clear();
      return;
    }

    chart.setOption(next, true);
  }

  onMount(() => {
    let resizeObserver: ResizeObserver | null = null;
    let disposed = false;

    async function initialize() {
      const echarts = await import('echarts');
      if (!container || disposed) {
        return;
      }

      chart = echarts.init(container, undefined, { renderer: 'canvas' });
      chart.on('click', (params) => {
        if (params?.name != null) {
          onCategoryClick?.(String(params.name));
        }
      });
      refresh();
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
    rows;
    categoryKey;
    valueKeys;
    mode;
    refresh();
  });
</script>

{#if rows.length > 0}
  <div bind:this={container} class="h-full min-h-[280px] w-full"></div>
{:else}
  <div class="flex min-h-[280px] items-center justify-center rounded-2xl border border-dashed border-slate-300 text-sm text-slate-500 dark:border-slate-700 dark:text-slate-400">
    {emptyLabel}
  </div>
{/if}
