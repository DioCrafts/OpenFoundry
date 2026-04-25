<script lang="ts">
  import { onMount } from 'svelte';
  import EChartView from '$components/analytics/EChartView.svelte';
  import {
    createDataset,
    listDatasets,
    previewDataset,
    uploadData,
    type Dataset,
  } from '$lib/api/datasets';

  let datasets = $state<Dataset[]>([]);
  let primaryDatasetId = $state('');
  let secondaryDatasetId = $state('');
  let primaryRows = $state<Array<Record<string, unknown>>>([]);
  let secondaryRows = $state<Array<Record<string, unknown>>>([]);
  let _loading = $state(true);
  let loadingPrimary = $state(false);
  let _loadingSecondary = $state(false);
  let exporting = $state(false);
  let fullscreen = $state(false);
  let error = $state('');

  let primaryJoinKey = $state('');
  let secondaryJoinKey = $state('');
  let search = $state('');
  let dateField = $state('');
  let dateFrom = $state('');
  let dateTo = $state('');
  let dimension = $state('');
  let secondaryDimension = $state('');
  let metric = $state('');
  let aggregation = $state<'sum' | 'avg' | 'count' | 'max'>('sum');
  let selectedCategory = $state('');

  async function loadDatasets() {
    _loading = true;
    try {
      const response = await listDatasets({ per_page: 100 });
      datasets = response.data;
      primaryDatasetId = primaryDatasetId || datasets[0]?.id || '';
      secondaryDatasetId = secondaryDatasetId || '';
      if (primaryDatasetId) {
        await loadPrimaryRows();
      }
    } catch (cause) {
      error = cause instanceof Error ? cause.message : 'Failed to load datasets';
    } finally {
      _loading = false;
    }
  }

  async function loadDatasetRows(datasetId: string) {
    const firstPage = await previewDataset(datasetId, { limit: 1000, offset: 0 });
    const total = firstPage.total_rows ?? firstPage.rows?.length ?? 0;
    const rows = [...(firstPage.rows ?? [])];

    for (let offset = rows.length; offset < total; offset += 1000) {
      const next = await previewDataset(datasetId, { limit: 1000, offset });
      rows.push(...(next.rows ?? []));
    }

    return rows;
  }

  async function loadPrimaryRows() {
    if (!primaryDatasetId) {
      primaryRows = [];
      return;
    }

    loadingPrimary = true;
    error = '';
    try {
      primaryRows = await loadDatasetRows(primaryDatasetId);
      hydrateFieldDefaults();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : 'Failed to load primary dataset';
      primaryRows = [];
    } finally {
      loadingPrimary = false;
    }
  }

  async function loadSecondaryRows() {
    if (!secondaryDatasetId) {
      secondaryRows = [];
      return;
    }

    _loadingSecondary = true;
    error = '';
    try {
      secondaryRows = await loadDatasetRows(secondaryDatasetId);
      hydrateFieldDefaults();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : 'Failed to load secondary dataset';
      secondaryRows = [];
    } finally {
      _loadingSecondary = false;
    }
  }

  function hydrateFieldDefaults() {
    const sample = sourceRows()[0] ?? {};
    const keys = Object.keys(sample);
    if (!dimension) dimension = keys[0] ?? '';
    if (!secondaryDimension) secondaryDimension = keys[1] ?? keys[0] ?? '';
    if (!metric) metric = keys.find((key) => typeof sample[key] === 'number') ?? keys[1] ?? '';
    if (!dateField) dateField = keys.find((key) => /date|time|day|month/i.test(key)) ?? '';
    if (!primaryJoinKey) primaryJoinKey = Object.keys(primaryRows[0] ?? {})[0] ?? '';
    if (!secondaryJoinKey) secondaryJoinKey = Object.keys(secondaryRows[0] ?? {})[0] ?? '';
  }

  function sourceRows() {
    if (!secondaryDatasetId || !primaryJoinKey || !secondaryJoinKey || secondaryRows.length === 0) {
      return primaryRows;
    }

    const secondaryIndex = Object.fromEntries(
      secondaryRows.map((row) => [String(row[secondaryJoinKey] ?? ''), row]),
    );

    return primaryRows.map((row) => {
      const joined = secondaryIndex[String(row[primaryJoinKey] ?? '')];
      if (!joined) {
        return row;
      }

      const prefixed = Object.fromEntries(
        Object.entries(joined).map(([key, value]) => [`joined_${key}`, value]),
      );
      return { ...row, ...prefixed };
    });
  }

  function searchableText(row: Record<string, unknown>) {
    return Object.values(row).map((value) => String(value ?? '')).join(' ').toLowerCase();
  }

  function numericValue(value: unknown) {
    if (typeof value === 'number' && Number.isFinite(value)) {
      return value;
    }
    if (typeof value === 'string') {
      const parsed = Number(value.replace(/,/g, ''));
      return Number.isFinite(parsed) ? parsed : 0;
    }
    return 0;
  }

  function matchesDateFilter(row: Record<string, unknown>) {
    if (!dateField || (!dateFrom && !dateTo)) {
      return true;
    }

    const raw = row[dateField];
    const date = raw ? new Date(String(raw)) : null;
    if (!date || Number.isNaN(date.getTime())) {
      return true;
    }

    if (dateFrom && date < new Date(dateFrom)) {
      return false;
    }

    if (dateTo) {
      const rowDate = date.toISOString().slice(0, 10);
      if (rowDate > dateTo) {
        return false;
      }
    }

    return true;
  }

  function filteredRows() {
    return sourceRows().filter((row) => {
      if (search.trim() && !searchableText(row).includes(search.trim().toLowerCase())) {
        return false;
      }

      if (selectedCategory && String(row[dimension] ?? '') !== selectedCategory) {
        return false;
      }

      return matchesDateFilter(row);
    });
  }

  function aggregateRows(rows: Array<Record<string, unknown>>, groupField: string) {
    const bucket: Record<string, { count: number; total: number; max: number }> = {};

    for (const row of rows) {
      const key = String(row[groupField] ?? 'Unknown');
      const nextValue = numericValue(row[metric]);
      const current = bucket[key] ?? { count: 0, total: 0, max: Number.NEGATIVE_INFINITY };
      current.count += 1;
      current.total += nextValue;
      current.max = Math.max(current.max, nextValue);
      bucket[key] = current;
    }

    return Object.entries(bucket)
      .map(([group, stats]) => {
        const value = aggregation === 'count'
          ? stats.count
          : aggregation === 'avg'
            ? (stats.count === 0 ? 0 : stats.total / stats.count)
            : aggregation === 'max'
              ? (stats.max === Number.NEGATIVE_INFINITY ? 0 : stats.max)
              : stats.total;

        return {
          group,
          value: Number(value.toFixed(2)),
          count: stats.count,
        };
      })
      .sort((left, right) => Number(right.value) - Number(left.value))
      .slice(0, 24);
  }

  const analysisRows = $derived(aggregateRows(filteredRows(), dimension || (Object.keys(sourceRows()[0] ?? {})[0] ?? '')));
  const breakdownRows = $derived(aggregateRows(filteredRows(), secondaryDimension || dimension || (Object.keys(sourceRows()[0] ?? {})[0] ?? '')));
  const analysisPath = $derived([
    `dataset:${datasets.find((dataset) => dataset.id === primaryDatasetId)?.name ?? 'none'}`,
    secondaryDatasetId ? `join:${datasets.find((dataset) => dataset.id === secondaryDatasetId)?.name ?? 'secondary'}` : null,
    search.trim() ? `search:${search.trim()}` : null,
    selectedCategory ? `drill:${selectedCategory}` : null,
  ].filter(Boolean) as string[]);

  async function exportCurrentView() {
    if (analysisRows.length === 0) {
      return;
    }

    exporting = true;
    error = '';
    try {
      const dataset = await createDataset({
        name: `Contour Export ${new Date().toISOString().slice(0, 16)}`,
        description: 'Materialized from the Contour analysis board.',
        format: 'json',
        tags: ['contour', 'analysis-export'],
      });

      const payload = JSON.stringify(analysisRows, null, 2);
      const file = new File([payload], 'contour-export.json', { type: 'application/json' });
      await uploadData(dataset.id, file);
      alert(`Exported to dataset ${dataset.name}`);
    } catch (cause) {
      error = cause instanceof Error ? cause.message : 'Failed to export analysis';
    } finally {
      exporting = false;
    }
  }

  onMount(() => {
    void loadDatasets();
  });
</script>

<svelte:head>
  <title>OpenFoundry — Contour</title>
</svelte:head>

<div class={fullscreen ? 'fixed inset-0 z-50 overflow-auto bg-slate-100 p-6 dark:bg-slate-950' : 'mx-auto max-w-[1600px] space-y-6'}>
  <section class="rounded-[2rem] border border-slate-200 bg-white p-6 shadow-sm dark:border-slate-800 dark:bg-slate-950">
    <div class="flex flex-wrap items-start justify-between gap-4">
      <div class="max-w-3xl">
        <div class="text-[11px] font-semibold uppercase tracking-[0.28em] text-teal-600">Contour</div>
        <h1 class="mt-2 text-4xl font-semibold tracking-tight text-slate-950 dark:text-slate-50">Top-down analysis with transform boards and materialized exports</h1>
        <p class="mt-3 text-base leading-7 text-slate-600 dark:text-slate-300">
          Join datasets, drill through paths, filter chart-to-chart, and persist the resulting analysis as a new dataset.
        </p>
      </div>

      <div class="flex flex-wrap gap-2">
        <button class="rounded-xl border border-slate-300 px-4 py-2 text-sm font-medium dark:border-slate-700" onclick={() => fullscreen = !fullscreen}>
          {fullscreen ? 'Exit fullscreen' : 'Fullscreen'}
        </button>
        <button class="rounded-xl bg-slate-900 px-4 py-2 text-sm font-medium text-white disabled:opacity-60 dark:bg-slate-100 dark:text-slate-950" onclick={exportCurrentView} disabled={exporting || analysisRows.length === 0}>
          {exporting ? 'Exporting...' : 'Export to dataset'}
        </button>
      </div>
    </div>

    {#if error}
      <div class="mt-4 rounded-2xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-700 dark:border-rose-900 dark:bg-rose-950/40 dark:text-rose-300">
        {error}
      </div>
    {/if}
  </section>

  <section class="grid gap-6 xl:grid-cols-[minmax(0,0.95fr)_minmax(0,1.05fr)]">
    <div class="space-y-6">
      <section class="rounded-[2rem] border border-slate-200 bg-white p-6 shadow-sm dark:border-slate-800 dark:bg-slate-950">
        <div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Transform board</div>
        <h2 class="mt-1 text-xl font-semibold text-slate-950 dark:text-slate-50">Shape the analysis</h2>

        <div class="mt-4 grid gap-3 md:grid-cols-2">
          <label class="rounded-xl border border-slate-200 bg-slate-50 px-3 py-2 dark:border-slate-800 dark:bg-slate-900">
            <div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Primary dataset</div>
            <select bind:value={primaryDatasetId} class="mt-2 w-full bg-transparent text-sm outline-none" onchange={() => void loadPrimaryRows()}>
              {#each datasets as dataset}
                <option value={dataset.id}>{dataset.name}</option>
              {/each}
            </select>
          </label>

          <label class="rounded-xl border border-slate-200 bg-slate-50 px-3 py-2 dark:border-slate-800 dark:bg-slate-900">
            <div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Join dataset</div>
            <select bind:value={secondaryDatasetId} class="mt-2 w-full bg-transparent text-sm outline-none" onchange={() => void loadSecondaryRows()}>
              <option value="">No join</option>
              {#each datasets.filter((dataset) => dataset.id !== primaryDatasetId) as dataset}
                <option value={dataset.id}>{dataset.name}</option>
              {/each}
            </select>
          </label>

          {#if secondaryDatasetId}
            <label class="rounded-xl border border-slate-200 bg-slate-50 px-3 py-2 dark:border-slate-800 dark:bg-slate-900">
              <div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Primary key</div>
              <select bind:value={primaryJoinKey} class="mt-2 w-full bg-transparent text-sm outline-none">
                {#each Object.keys(primaryRows[0] ?? {}) as key}
                  <option value={key}>{key}</option>
                {/each}
              </select>
            </label>

            <label class="rounded-xl border border-slate-200 bg-slate-50 px-3 py-2 dark:border-slate-800 dark:bg-slate-900">
              <div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Join key</div>
              <select bind:value={secondaryJoinKey} class="mt-2 w-full bg-transparent text-sm outline-none">
                {#each Object.keys(secondaryRows[0] ?? {}) as key}
                  <option value={key}>{key}</option>
                {/each}
              </select>
            </label>
          {/if}

          <label class="rounded-xl border border-slate-200 bg-slate-50 px-3 py-2 dark:border-slate-800 dark:bg-slate-900">
            <div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Dimension</div>
            <select bind:value={dimension} class="mt-2 w-full bg-transparent text-sm outline-none">
              {#each Object.keys(sourceRows()[0] ?? {}) as key}
                <option value={key}>{key}</option>
              {/each}
            </select>
          </label>

          <label class="rounded-xl border border-slate-200 bg-slate-50 px-3 py-2 dark:border-slate-800 dark:bg-slate-900">
            <div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Secondary dimension</div>
            <select bind:value={secondaryDimension} class="mt-2 w-full bg-transparent text-sm outline-none">
              {#each Object.keys(sourceRows()[0] ?? {}) as key}
                <option value={key}>{key}</option>
              {/each}
            </select>
          </label>

          <label class="rounded-xl border border-slate-200 bg-slate-50 px-3 py-2 dark:border-slate-800 dark:bg-slate-900">
            <div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Metric</div>
            <select bind:value={metric} class="mt-2 w-full bg-transparent text-sm outline-none">
              {#each Object.keys(sourceRows()[0] ?? {}) as key}
                <option value={key}>{key}</option>
              {/each}
            </select>
          </label>

          <label class="rounded-xl border border-slate-200 bg-slate-50 px-3 py-2 dark:border-slate-800 dark:bg-slate-900">
            <div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Aggregation</div>
            <select bind:value={aggregation} class="mt-2 w-full bg-transparent text-sm outline-none">
              <option value="sum">sum</option>
              <option value="avg">avg</option>
              <option value="count">count</option>
              <option value="max">max</option>
            </select>
          </label>

          <label class="rounded-xl border border-slate-200 bg-slate-50 px-3 py-2 dark:border-slate-800 dark:bg-slate-900 md:col-span-2">
            <div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Search parameter</div>
            <input bind:value={search} class="mt-2 w-full bg-transparent text-sm outline-none" placeholder="Search across the joined rows" />
          </label>

          <label class="rounded-xl border border-slate-200 bg-slate-50 px-3 py-2 dark:border-slate-800 dark:bg-slate-900">
            <div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Date field</div>
            <select bind:value={dateField} class="mt-2 w-full bg-transparent text-sm outline-none">
              <option value="">No date filter</option>
              {#each Object.keys(sourceRows()[0] ?? {}) as key}
                <option value={key}>{key}</option>
              {/each}
            </select>
          </label>

          <label class="rounded-xl border border-slate-200 bg-slate-50 px-3 py-2 dark:border-slate-800 dark:bg-slate-900">
            <div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">From</div>
            <input bind:value={dateFrom} type="date" class="mt-2 w-full bg-transparent text-sm outline-none" />
          </label>

          <label class="rounded-xl border border-slate-200 bg-slate-50 px-3 py-2 dark:border-slate-800 dark:bg-slate-900">
            <div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">To</div>
            <input bind:value={dateTo} type="date" class="mt-2 w-full bg-transparent text-sm outline-none" />
          </label>
        </div>
      </section>

      <section class="rounded-[2rem] border border-slate-200 bg-white p-6 shadow-sm dark:border-slate-800 dark:bg-slate-950">
        <div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Analysis path</div>
        <h2 class="mt-1 text-xl font-semibold text-slate-950 dark:text-slate-50">Sequence and drill breadcrumbs</h2>

        <div class="mt-4 flex flex-wrap gap-2">
          {#each analysisPath as step}
            <span class="rounded-full border border-slate-300 px-3 py-1 text-xs font-medium text-slate-600 dark:border-slate-700 dark:text-slate-300">{step}</span>
          {/each}
        </div>

        {#if selectedCategory}
          <button class="mt-4 rounded-lg border border-slate-300 px-3 py-1.5 text-sm font-medium dark:border-slate-700" onclick={() => selectedCategory = ''}>
            Clear drill into {selectedCategory}
          </button>
        {/if}
      </section>
    </div>

    <div class="space-y-6">
      <section class="rounded-[2rem] border border-slate-200 bg-white p-6 shadow-sm dark:border-slate-800 dark:bg-slate-950">
        <div class="flex items-start justify-between gap-3">
          <div>
            <div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Display board</div>
            <h2 class="mt-1 text-xl font-semibold text-slate-950 dark:text-slate-50">Primary analysis chart</h2>
          </div>
          <div class="text-xs text-slate-500 dark:text-slate-400">{analysisRows.length} grouped rows</div>
        </div>

        <div class="mt-4 h-[320px]">
          <EChartView
            rows={analysisRows.map((row) => ({ category: row.group, value: row.value }))}
            categoryKey="category"
            valueKeys={['value']}
            mode="bar"
            emptyLabel={loadingPrimary ? 'Loading dataset...' : 'Pick a dataset and metric to start the board.'}
            onCategoryClick={(value) => selectedCategory = value}
          />
        </div>
      </section>

      <section class="rounded-[2rem] border border-slate-200 bg-white p-6 shadow-sm dark:border-slate-800 dark:bg-slate-950">
        <div class="flex items-start justify-between gap-3">
          <div>
            <div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Linked board</div>
            <h2 class="mt-1 text-xl font-semibold text-slate-950 dark:text-slate-50">Chart-to-chart filtering</h2>
          </div>
          <div class="text-xs text-slate-500 dark:text-slate-400">{selectedCategory ? `Scoped to ${selectedCategory}` : 'Global view'}</div>
        </div>

        <div class="mt-4 h-[320px]">
          <EChartView
            rows={breakdownRows.map((row) => ({ category: row.group, value: row.value }))}
            categoryKey="category"
            valueKeys={['value']}
            mode="pie"
            emptyLabel="Choose a secondary dimension to break the analysis down."
          />
        </div>
      </section>

      <section class="rounded-[2rem] border border-slate-200 bg-white p-6 shadow-sm dark:border-slate-800 dark:bg-slate-950">
        <div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Result table</div>
        <h2 class="mt-1 text-xl font-semibold text-slate-950 dark:text-slate-50">Materializable rows</h2>

        <div class="mt-4 overflow-x-auto">
          <table class="min-w-full text-sm">
            <thead>
              <tr class="border-b border-slate-200 text-left dark:border-slate-800">
                <th class="px-3 py-2 font-semibold text-slate-600 dark:text-slate-300">{dimension || 'group'}</th>
                <th class="px-3 py-2 font-semibold text-slate-600 dark:text-slate-300">{aggregation}({metric || 'value'})</th>
                <th class="px-3 py-2 font-semibold text-slate-600 dark:text-slate-300">count</th>
              </tr>
            </thead>
            <tbody>
              {#each analysisRows as row}
                <tr class="border-b border-slate-100 dark:border-slate-900">
                  <td class="px-3 py-2 text-slate-700 dark:text-slate-200">{row.group}</td>
                  <td class="px-3 py-2 text-slate-700 dark:text-slate-200">{row.value}</td>
                  <td class="px-3 py-2 text-slate-500 dark:text-slate-400">{row.count}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      </section>
    </div>
  </section>
</div>
