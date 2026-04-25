<script lang="ts">
  import { browser } from '$app/environment';
  import { page } from '$app/stores';
  import { onMount } from 'svelte';
  import EChartView from '$components/analytics/EChartView.svelte';
  import {
    getOntologyGraph,
    listObjectTypes,
    listObjects,
    type GraphResponse,
    type ObjectInstance,
    type ObjectType,
  } from '$lib/api/ontology';

  const STORAGE_KEY = 'of_quiver_visual_functions';

  type VisualFunction = {
    id: string;
    name: string;
    primaryTypeId: string;
    secondaryTypeId: string;
    joinField: string;
    dateField: string;
    metricField: string;
    groupField: string;
  };

  let types = $state<ObjectType[]>([]);
  let primaryTypeId = $state('');
  let secondaryTypeId = $state('');
  let primaryObjects = $state<ObjectInstance[]>([]);
  let secondaryObjects = $state<ObjectInstance[]>([]);
  let graph = $state<GraphResponse | null>(null);
  let loading = $state(true);
  let error = $state('');
  let embedded = $state(false);

  let dateField = $state('');
  let metricField = $state('');
  let groupField = $state('');
  let joinField = $state('');
  let secondaryJoinField = $state('');
  let selectedGroup = $state('');
  let visualFunctions = $state<VisualFunction[]>([]);
  let visualFunctionName = $state('');

  function applySearchParams() {
    const params = $page.url.searchParams;
    embedded = params.get('embedded') === '1';
    primaryTypeId = params.get('primary_type_id') ?? primaryTypeId;
    secondaryTypeId = params.get('secondary_type_id') ?? secondaryTypeId;
    joinField = params.get('join_field') ?? joinField;
    secondaryJoinField = params.get('secondary_join_field') ?? secondaryJoinField;
    dateField = params.get('date_field') ?? dateField;
    metricField = params.get('metric_field') ?? metricField;
    groupField = params.get('group_field') ?? groupField;
    selectedGroup = params.get('selected_group') ?? selectedGroup;
  }

  function restoreVisualFunctions() {
    if (!browser) {
      return;
    }

    try {
      const raw = localStorage.getItem(STORAGE_KEY);
      visualFunctions = raw ? JSON.parse(raw) as VisualFunction[] : [];
    } catch {
      visualFunctions = [];
    }
  }

  function persistVisualFunctions() {
    if (!browser) {
      return;
    }

    localStorage.setItem(STORAGE_KEY, JSON.stringify(visualFunctions));
  }

  async function loadTypes() {
    loading = true;
    error = '';
    try {
      const response = await listObjectTypes({ per_page: 100 });
      types = response.data;
      primaryTypeId = primaryTypeId || types[0]?.id || '';
      secondaryTypeId = secondaryTypeId || '';
      if (primaryTypeId) {
        await loadPrimaryObjects();
      }
      if (secondaryTypeId) {
        await loadSecondaryObjects();
      }
    } catch (cause) {
      error = cause instanceof Error ? cause.message : 'Failed to load ontology types';
    } finally {
      loading = false;
    }
  }

  async function loadObjects(typeId: string) {
    const rows: ObjectInstance[] = [];
    let page = 1;
    let total = 0;

    do {
      const response = await listObjects(typeId, { page, per_page: 100 });
      rows.push(...response.data);
      total = response.total;
      page += 1;
    } while (rows.length < total);

    return rows;
  }

  async function loadPrimaryObjects() {
    if (!primaryTypeId) {
      primaryObjects = [];
      return;
    }

    error = '';
    try {
      primaryObjects = await loadObjects(primaryTypeId);
      hydrateDefaults();
      graph = await getOntologyGraph({ root_type_id: primaryTypeId, depth: 2, limit: 120 });
    } catch (cause) {
      error = cause instanceof Error ? cause.message : 'Failed to load primary object set';
      primaryObjects = [];
    }
  }

  async function loadSecondaryObjects() {
    if (!secondaryTypeId) {
      secondaryObjects = [];
      return;
    }

    error = '';
    try {
      secondaryObjects = await loadObjects(secondaryTypeId);
      hydrateDefaults();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : 'Failed to load secondary object set';
      secondaryObjects = [];
    }
  }

  function hydrateDefaults() {
    const sample = primaryObjects[0]?.properties ?? {};
    const keys = Object.keys(sample);
    if (!dateField) dateField = keys.find((key) => /date|time|day|month/i.test(key)) ?? '';
    if (!metricField) metricField = keys.find((key) => typeof sample[key] === 'number') ?? keys[0] ?? '';
    if (!groupField) groupField = keys[0] ?? '';
    if (!joinField) joinField = keys[0] ?? '';
    if (!secondaryJoinField) secondaryJoinField = Object.keys(secondaryObjects[0]?.properties ?? {})[0] ?? joinField;
  }

  function joinedObjects() {
    if (!secondaryTypeId || !joinField || !secondaryJoinField || secondaryObjects.length === 0) {
      return primaryObjects.map((object) => object.properties);
    }

    const secondaryIndex = Object.fromEntries(
      secondaryObjects.map((object) => [String(object.properties[secondaryJoinField] ?? ''), object]),
    );

    return primaryObjects.map((object) => {
      const secondary = secondaryIndex[String(object.properties[joinField] ?? '')];
      const prefixed = secondary
        ? Object.fromEntries(
            Object.entries(secondary.properties).map(([key, value]) => [`linked_${key}`, value]),
          )
        : {};
      return { ...object.properties, ...prefixed };
    });
  }

  function numericValue(value: unknown) {
    if (typeof value === 'number' && Number.isFinite(value)) return value;
    if (typeof value === 'string') {
      const parsed = Number(value.replace(/,/g, ''));
      return Number.isFinite(parsed) ? parsed : 0;
    }
    return 0;
  }

  function timeSeriesRows() {
    const buckets: Record<string, { total: number; count: number }> = {};

    for (const row of joinedObjects()) {
      const group = String(row[dateField] ?? '').slice(0, 10);
      if (!group) continue;
      const current = buckets[group] ?? { total: 0, count: 0 };
      current.total += numericValue(row[metricField]);
      current.count += 1;
      buckets[group] = current;
    }

    return Object.entries(buckets)
      .map(([date, stats]) => ({ date, value: Number(stats.total.toFixed(2)), count: stats.count }))
      .sort((left, right) => left.date.localeCompare(right.date));
  }

  function groupedRows() {
    const buckets: Record<string, { total: number; count: number }> = {};

    for (const row of joinedObjects()) {
      const group = String(row[groupField] ?? 'Unknown');
      if (selectedGroup && group !== selectedGroup) {
        continue;
      }
      const current = buckets[group] ?? { total: 0, count: 0 };
      current.total += numericValue(row[metricField]);
      current.count += 1;
      buckets[group] = current;
    }

    return Object.entries(buckets)
      .map(([group, stats]) => ({ group, value: Number(stats.total.toFixed(2)), count: stats.count }))
      .sort((left, right) => right.value - left.value)
      .slice(0, 20);
  }

  function saveVisualFunction() {
    if (!visualFunctionName.trim()) {
      return;
    }

    visualFunctions = [
      {
        id: crypto.randomUUID(),
        name: visualFunctionName.trim(),
        primaryTypeId,
        secondaryTypeId,
        joinField,
        dateField,
        metricField,
        groupField,
      },
      ...visualFunctions,
    ];
    visualFunctionName = '';
    persistVisualFunctions();
  }

  async function applyVisualFunction(visualFunction: VisualFunction) {
    primaryTypeId = visualFunction.primaryTypeId;
    secondaryTypeId = visualFunction.secondaryTypeId;
    joinField = visualFunction.joinField;
    dateField = visualFunction.dateField;
    metricField = visualFunction.metricField;
    groupField = visualFunction.groupField;
    await loadPrimaryObjects();
    if (visualFunction.secondaryTypeId) {
      await loadSecondaryObjects();
    }
  }

  onMount(() => {
    applySearchParams();
    restoreVisualFunctions();
    void loadTypes();
  });
</script>

<svelte:head>
  <title>OpenFoundry — Quiver</title>
</svelte:head>

<div class="mx-auto max-w-[1600px] space-y-6">
  <section class={`rounded-[2rem] border border-slate-200 bg-white shadow-sm dark:border-slate-800 dark:bg-slate-950 ${embedded ? 'p-4' : 'p-6'}`}>
    <div class={embedded ? 'max-w-none' : 'max-w-3xl'}>
      <div class="text-[11px] font-semibold uppercase tracking-[0.28em] text-sky-600">{embedded ? 'Quiver Embed' : 'Quiver'}</div>
      <h1 class={`${embedded ? 'mt-2 text-2xl' : 'mt-2 text-4xl'} font-semibold tracking-tight text-slate-950 dark:text-slate-50`}>Time-series and ontology analytics with reusable visual functions</h1>
      <p class={`${embedded ? 'mt-2 text-sm leading-6' : 'mt-3 text-base leading-7'} text-slate-600 dark:text-slate-300`}>
        Explore object sets without code, join related domains, navigate the ontology graph, and save reusable visual recipes for the rest of the workspace.
      </p>
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
        <div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Object sets</div>
        <h2 class="mt-1 text-xl font-semibold text-slate-950 dark:text-slate-50">Point-and-click joins</h2>

        <div class="mt-4 grid gap-3 md:grid-cols-2">
          <label class="rounded-xl border border-slate-200 bg-slate-50 px-3 py-2 dark:border-slate-800 dark:bg-slate-900">
            <div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Primary type</div>
            <select bind:value={primaryTypeId} class="mt-2 w-full bg-transparent text-sm outline-none" onchange={() => void loadPrimaryObjects()}>
              {#each types as type}
                <option value={type.id}>{type.display_name}</option>
              {/each}
            </select>
          </label>

          <label class="rounded-xl border border-slate-200 bg-slate-50 px-3 py-2 dark:border-slate-800 dark:bg-slate-900">
            <div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Secondary type</div>
            <select bind:value={secondaryTypeId} class="mt-2 w-full bg-transparent text-sm outline-none" onchange={() => void loadSecondaryObjects()}>
              <option value="">No join</option>
              {#each types.filter((type) => type.id !== primaryTypeId) as type}
                <option value={type.id}>{type.display_name}</option>
              {/each}
            </select>
          </label>

          <label class="rounded-xl border border-slate-200 bg-slate-50 px-3 py-2 dark:border-slate-800 dark:bg-slate-900">
            <div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Join field</div>
            <select bind:value={joinField} class="mt-2 w-full bg-transparent text-sm outline-none">
              {#each Object.keys(primaryObjects[0]?.properties ?? {}) as key}
                <option value={key}>{key}</option>
              {/each}
            </select>
          </label>

          <label class="rounded-xl border border-slate-200 bg-slate-50 px-3 py-2 dark:border-slate-800 dark:bg-slate-900">
            <div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Linked field</div>
            <select bind:value={secondaryJoinField} class="mt-2 w-full bg-transparent text-sm outline-none">
              {#each Object.keys(secondaryObjects[0]?.properties ?? {}) as key}
                <option value={key}>{key}</option>
              {/each}
            </select>
          </label>

          <label class="rounded-xl border border-slate-200 bg-slate-50 px-3 py-2 dark:border-slate-800 dark:bg-slate-900">
            <div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Date field</div>
            <select bind:value={dateField} class="mt-2 w-full bg-transparent text-sm outline-none">
              {#each Object.keys(joinedObjects()[0] ?? {}) as key}
                <option value={key}>{key}</option>
              {/each}
            </select>
          </label>

          <label class="rounded-xl border border-slate-200 bg-slate-50 px-3 py-2 dark:border-slate-800 dark:bg-slate-900">
            <div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Metric field</div>
            <select bind:value={metricField} class="mt-2 w-full bg-transparent text-sm outline-none">
              {#each Object.keys(joinedObjects()[0] ?? {}) as key}
                <option value={key}>{key}</option>
              {/each}
            </select>
          </label>

          <label class="rounded-xl border border-slate-200 bg-slate-50 px-3 py-2 dark:border-slate-800 dark:bg-slate-900 md:col-span-2">
            <div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Group field</div>
            <select bind:value={groupField} class="mt-2 w-full bg-transparent text-sm outline-none">
              {#each Object.keys(joinedObjects()[0] ?? {}) as key}
                <option value={key}>{key}</option>
              {/each}
            </select>
          </label>
        </div>
      </section>

      <section class="rounded-[2rem] border border-slate-200 bg-white p-6 shadow-sm dark:border-slate-800 dark:bg-slate-950">
        <div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Visual functions</div>
        <h2 class="mt-1 text-xl font-semibold text-slate-950 dark:text-slate-50">Reusable analysis presets</h2>

        <div class="mt-4 flex gap-3">
          <input bind:value={visualFunctionName} class="min-w-0 flex-1 rounded-xl border border-slate-300 bg-white px-3 py-2 text-sm outline-none dark:border-slate-700 dark:bg-slate-900" placeholder="Save this lens as a visual function" />
          <button class="rounded-xl border border-slate-300 px-4 py-2 text-sm font-medium dark:border-slate-700" onclick={saveVisualFunction}>Save</button>
        </div>

        <div class="mt-4 space-y-3">
          {#if visualFunctions.length === 0}
            <div class="rounded-2xl border border-dashed border-slate-300 px-4 py-5 text-sm text-slate-500 dark:border-slate-700 dark:text-slate-400">
              Save the current lens to reuse it in dashboards, Workshop, or object views later.
            </div>
          {:else}
            {#each visualFunctions as visualFunction (visualFunction.id)}
              <button class="w-full rounded-2xl border border-slate-200 bg-slate-50 p-4 text-left hover:border-sky-300 hover:bg-sky-50 dark:border-slate-800 dark:bg-slate-900/60 dark:hover:border-sky-700 dark:hover:bg-sky-950/20" onclick={() => void applyVisualFunction(visualFunction)}>
                <div class="text-sm font-semibold text-slate-900 dark:text-slate-100">{visualFunction.name}</div>
                <div class="mt-1 text-xs text-slate-500 dark:text-slate-400">{visualFunction.metricField} by {visualFunction.groupField}</div>
              </button>
            {/each}
          {/if}
        </div>
      </section>
    </div>

    <div class="space-y-6">
      <section class="rounded-[2rem] border border-slate-200 bg-white p-6 shadow-sm dark:border-slate-800 dark:bg-slate-950">
        <div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Time series</div>
        <h2 class="mt-1 text-xl font-semibold text-slate-950 dark:text-slate-50">Metric progression over time</h2>

        <div class="mt-4 h-[320px]">
          <EChartView
            rows={timeSeriesRows().map((row) => ({ date: row.date, value: row.value }))}
            categoryKey="date"
            valueKeys={['value']}
            mode="line"
            emptyLabel={loading ? 'Loading ontology objects...' : 'Pick a type with date and numeric properties to render the time series.'}
          />
        </div>
      </section>

      <section class="rounded-[2rem] border border-slate-200 bg-white p-6 shadow-sm dark:border-slate-800 dark:bg-slate-950">
        <div class="flex items-start justify-between gap-3">
          <div>
            <div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Object analytics</div>
            <h2 class="mt-1 text-xl font-semibold text-slate-950 dark:text-slate-50">Interactive grouped lens</h2>
          </div>
          {#if selectedGroup}
            <button class="rounded-lg border border-slate-300 px-3 py-1.5 text-sm font-medium dark:border-slate-700" onclick={() => selectedGroup = ''}>
              Clear {selectedGroup}
            </button>
          {/if}
        </div>

        <div class="mt-4 h-[320px]">
          <EChartView
            rows={groupedRows().map((row) => ({ group: row.group, value: row.value }))}
            categoryKey="group"
            valueKeys={['value']}
            mode="bar"
            emptyLabel="Choose group and metric fields to build the object lens."
            onCategoryClick={(value) => selectedGroup = value}
          />
        </div>
      </section>

      <section class="rounded-[2rem] border border-slate-200 bg-white p-6 shadow-sm dark:border-slate-800 dark:bg-slate-950">
        <div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Graph navigation</div>
        <h2 class="mt-1 text-xl font-semibold text-slate-950 dark:text-slate-50">Relationship overview for the current object type</h2>

        {#if graph}
          <div class="mt-4 grid gap-3 sm:grid-cols-3">
            <div class="rounded-2xl border border-slate-200 bg-slate-50 p-4 dark:border-slate-800 dark:bg-slate-900/60">
              <div class="text-[11px] uppercase tracking-[0.22em] text-slate-500">Nodes</div>
              <div class="mt-2 text-3xl font-semibold text-slate-900 dark:text-slate-100">{graph.total_nodes}</div>
            </div>
            <div class="rounded-2xl border border-slate-200 bg-slate-50 p-4 dark:border-slate-800 dark:bg-slate-900/60">
              <div class="text-[11px] uppercase tracking-[0.22em] text-slate-500">Edges</div>
              <div class="mt-2 text-3xl font-semibold text-slate-900 dark:text-slate-100">{graph.total_edges}</div>
            </div>
            <div class="rounded-2xl border border-slate-200 bg-slate-50 p-4 dark:border-slate-800 dark:bg-slate-900/60">
              <div class="text-[11px] uppercase tracking-[0.22em] text-slate-500">Mode</div>
              <div class="mt-2 text-3xl font-semibold text-slate-900 dark:text-slate-100">{graph.mode}</div>
            </div>
          </div>

          <div class="mt-4 rounded-2xl border border-slate-200 bg-slate-50 p-4 dark:border-slate-800 dark:bg-slate-900/60">
            <div class="text-sm font-semibold text-slate-900 dark:text-slate-100">Related nodes</div>
            <div class="mt-3 flex flex-wrap gap-2">
              {#each graph.nodes.slice(0, 18) as node}
                <span class="rounded-full border border-slate-300 px-3 py-1 text-xs text-slate-600 dark:border-slate-700 dark:text-slate-300">{node.label}</span>
              {/each}
            </div>
          </div>
        {:else}
          <div class="mt-4 rounded-2xl border border-dashed border-slate-300 px-4 py-5 text-sm text-slate-500 dark:border-slate-700 dark:text-slate-400">
            Load a primary object type to inspect its relationship graph.
          </div>
        {/if}
      </section>
    </div>
  </section>
</div>
