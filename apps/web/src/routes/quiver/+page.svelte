<script lang="ts">
  import { page } from '$app/stores';
  import { onMount } from 'svelte';
  import EChartView from '$components/analytics/EChartView.svelte';
  import {
    createQuiverVisualFunction,
    deleteQuiverVisualFunction,
    getOntologyGraph,
    listObjectTypes,
    listObjects,
    listQuiverVisualFunctions,
    updateQuiverVisualFunction,
    type GraphResponse,
    type ObjectInstance,
    type ObjectType,
    type QuiverChartKind,
    type QuiverVisualFunction,
  } from '$lib/api/ontology';
  import { buildQuiverVegaSpec, downloadJsonDocument } from '$lib/utils/quiver';

  let types = $state<ObjectType[]>([]);
  let primaryTypeId = $state('');
  let secondaryTypeId = $state('');
  let primaryObjects = $state<ObjectInstance[]>([]);
  let secondaryObjects = $state<ObjectInstance[]>([]);
  let graph = $state<GraphResponse | null>(null);
  let loading = $state(true);
  let loadingVisualFunctions = $state(true);
  let savingVisualFunction = $state(false);
  let deletingVisualFunction = $state(false);
  let error = $state('');
  let notice = $state('');
  let embedded = $state(false);

  let dateField = $state('');
  let metricField = $state('');
  let groupField = $state('');
  let joinField = $state('');
  let secondaryJoinField = $state('');
  let selectedGroup = $state('');

  let visualFunctions = $state<QuiverVisualFunction[]>([]);
  let selectedVisualFunctionId = $state('');
  let requestedVisualFunctionId = $state('');
  let visualFunctionName = $state('');
  let visualFunctionDescription = $state('');
  let chartKind = $state<QuiverChartKind>('line');
  let sharedVisualFunction = $state(false);

  function applySearchParams() {
    const params = $page.url.searchParams;
    embedded = params.get('embedded') === '1';
    requestedVisualFunctionId = params.get('visual_function_id') ?? '';
    primaryTypeId = params.get('primary_type_id') ?? primaryTypeId;
    secondaryTypeId = params.get('secondary_type_id') ?? secondaryTypeId;
    joinField = params.get('join_field') ?? joinField;
    secondaryJoinField = params.get('secondary_join_field') ?? secondaryJoinField;
    dateField = params.get('date_field') ?? dateField;
    metricField = params.get('metric_field') ?? metricField;
    groupField = params.get('group_field') ?? groupField;
    selectedGroup = params.get('selected_group') ?? selectedGroup;
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

  async function loadWorkspaceVisualFunctions(preferredId = selectedVisualFunctionId) {
    loadingVisualFunctions = true;
    try {
      const response = await listQuiverVisualFunctions({ per_page: 100, include_shared: true });
      visualFunctions = response.data;
      if (preferredId && !visualFunctions.some((visualFunction) => visualFunction.id === preferredId)) {
        selectedVisualFunctionId = '';
      }
    } catch (cause) {
      error = cause instanceof Error ? cause.message : 'Failed to load saved Quiver lenses';
      visualFunctions = [];
    } finally {
      loadingVisualFunctions = false;
    }
  }

  async function loadObjects(typeId: string) {
    const rows: ObjectInstance[] = [];
    let nextPage = 1;
    let total = 0;

    do {
      const response = await listObjects(typeId, { page: nextPage, per_page: 100 });
      rows.push(...response.data);
      total = response.total;
      nextPage += 1;
    } while (rows.length < total);

    return rows;
  }

  async function loadPrimaryObjects() {
    if (!primaryTypeId) {
      primaryObjects = [];
      graph = null;
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
      graph = null;
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
    if (!visualFunctionName) visualFunctionName = `${primaryTypeLabel()} lens`;
  }

  function primaryTypeLabel() {
    return types.find((type) => type.id === primaryTypeId)?.display_name ?? 'Quiver';
  }

  function secondaryTypeLabel() {
    return secondaryTypeId
      ? (types.find((type) => type.id === secondaryTypeId)?.display_name ?? 'Joined type')
      : 'No join';
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

  function currentVisualFunctionPayload() {
    return {
      name: visualFunctionName.trim() || `${primaryTypeLabel()} lens`,
      description: visualFunctionDescription.trim(),
      primary_type_id: primaryTypeId,
      secondary_type_id: secondaryTypeId || null,
      join_field: joinField,
      secondary_join_field: secondaryJoinField,
      date_field: dateField,
      metric_field: metricField,
      group_field: groupField,
      selected_group: selectedGroup || null,
      chart_kind: chartKind,
      shared: sharedVisualFunction,
    };
  }

  function currentVegaSpec() {
    return buildQuiverVegaSpec(
      {
        title: currentVisualFunctionPayload().name,
        description: currentVisualFunctionPayload().description,
        primaryTypeId: primaryTypeId,
        secondaryTypeId: secondaryTypeId,
        joinField: joinField,
        secondaryJoinField: secondaryJoinField,
        dateField: dateField,
        metricField: metricField,
        groupField: groupField,
        selectedGroup: selectedGroup,
        chartKind: chartKind,
        shared: sharedVisualFunction,
      },
      timeSeriesRows(),
      groupedRows(),
    );
  }

  async function saveVisualFunction() {
    if (!primaryTypeId || !joinField || !dateField || !metricField || !groupField) {
      error = 'Choose the primary type and the join/date/metric/group fields before saving the lens.';
      return;
    }

    savingVisualFunction = true;
    error = '';
    notice = '';

    try {
      const payload = currentVisualFunctionPayload();
      const isUpdate = Boolean(selectedVisualFunctionId);
      const saved = isUpdate
        ? await updateQuiverVisualFunction(selectedVisualFunctionId, payload)
        : await createQuiverVisualFunction(payload);

      selectedVisualFunctionId = saved.id;
      visualFunctionName = saved.name;
      visualFunctionDescription = saved.description;
      chartKind = saved.chart_kind;
      sharedVisualFunction = saved.shared;
      await loadWorkspaceVisualFunctions(saved.id);
      notice = isUpdate
        ? `Updated ${saved.name} in the Quiver workspace library.`
        : `Saved ${saved.name} to the Quiver workspace library.`;
    } catch (cause) {
      error = cause instanceof Error ? cause.message : 'Failed to save the Quiver lens';
    } finally {
      savingVisualFunction = false;
    }
  }

  async function deleteVisualFunction(id: string) {
    deletingVisualFunction = true;
    error = '';
    notice = '';

    try {
      await deleteQuiverVisualFunction(id);
      if (selectedVisualFunctionId === id) {
        resetVisualFunctionDraft();
      }
      await loadWorkspaceVisualFunctions();
      notice = 'Removed the saved Quiver lens from the workspace library.';
    } catch (cause) {
      error = cause instanceof Error ? cause.message : 'Failed to delete the Quiver lens';
    } finally {
      deletingVisualFunction = false;
    }
  }

  function resetVisualFunctionDraft() {
    selectedVisualFunctionId = '';
    visualFunctionName = `${primaryTypeLabel()} lens`;
    visualFunctionDescription = '';
    chartKind = 'line';
    sharedVisualFunction = false;
  }

  async function applyVisualFunction(visualFunction: QuiverVisualFunction) {
    selectedVisualFunctionId = visualFunction.id;
    visualFunctionName = visualFunction.name;
    visualFunctionDescription = visualFunction.description;
    primaryTypeId = visualFunction.primary_type_id;
    secondaryTypeId = visualFunction.secondary_type_id ?? '';
    joinField = visualFunction.join_field;
    secondaryJoinField = visualFunction.secondary_join_field;
    dateField = visualFunction.date_field;
    metricField = visualFunction.metric_field;
    groupField = visualFunction.group_field;
    selectedGroup = visualFunction.selected_group ?? '';
    chartKind = visualFunction.chart_kind;
    sharedVisualFunction = visualFunction.shared;
    await loadPrimaryObjects();
    if (visualFunction.secondary_type_id) {
      await loadSecondaryObjects();
    } else {
      secondaryObjects = [];
    }
    notice = `Loaded ${visualFunction.name}.`;
  }

  async function copyVegaSpec() {
    if (typeof navigator === 'undefined' || !navigator.clipboard) {
      error = 'Clipboard access is not available in this browser context.';
      return;
    }

    try {
      await navigator.clipboard.writeText(JSON.stringify(currentVegaSpec(), null, 2));
      notice = 'Copied the Vega-Lite spec to the clipboard.';
    } catch (cause) {
      error = cause instanceof Error ? cause.message : 'Failed to copy the Vega spec';
    }
  }

  function downloadVegaSpec() {
    downloadJsonDocument(
      `${(visualFunctionName.trim() || primaryTypeLabel()).toLowerCase().replace(/[^a-z0-9]+/g, '-')}-vega.json`,
      currentVegaSpec(),
    );
    notice = 'Downloaded the Vega-Lite spec JSON.';
  }

  onMount(async () => {
    applySearchParams();
    await Promise.all([loadWorkspaceVisualFunctions(requestedVisualFunctionId), loadTypes()]);
    if (requestedVisualFunctionId) {
      const requested = visualFunctions.find((visualFunction) => visualFunction.id === requestedVisualFunctionId);
      if (requested) {
        await applyVisualFunction(requested);
      }
    }
    if (!visualFunctionName) {
      visualFunctionName = `${primaryTypeLabel()} lens`;
    }
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
        Explore object sets without code, join related domains, navigate the ontology graph, persist reusable lenses in the workspace catalog, and export advanced Vega-Lite specs for downstream use.
      </p>
    </div>

    {#if error}
      <div class="mt-4 rounded-2xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-700 dark:border-rose-900 dark:bg-rose-950/40 dark:text-rose-300">
        {error}
      </div>
    {/if}

    {#if notice}
      <div class="mt-4 rounded-2xl border border-emerald-200 bg-emerald-50 px-4 py-3 text-sm text-emerald-700 dark:border-emerald-900 dark:bg-emerald-950/40 dark:text-emerald-300">
        {notice}
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
        <div class="flex items-start justify-between gap-3">
          <div>
            <div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Visual functions</div>
            <h2 class="mt-1 text-xl font-semibold text-slate-950 dark:text-slate-50">Workspace-persisted analysis presets</h2>
          </div>
          <div class="text-xs text-slate-500 dark:text-slate-400">
            {loadingVisualFunctions ? 'Syncing...' : `${visualFunctions.length} saved lens(es)`}
          </div>
        </div>

        <div class="mt-4 grid gap-3">
          <input bind:value={visualFunctionName} class="rounded-xl border border-slate-300 bg-white px-3 py-2 text-sm outline-none dark:border-slate-700 dark:bg-slate-900" placeholder="Name this Quiver lens" />
          <textarea bind:value={visualFunctionDescription} rows="3" class="rounded-xl border border-slate-300 bg-white px-3 py-2 text-sm outline-none dark:border-slate-700 dark:bg-slate-900" placeholder="Describe the lens and intended audience"></textarea>

          <div class="grid gap-3 md:grid-cols-[minmax(0,1fr)_auto]">
            <label class="rounded-xl border border-slate-200 bg-slate-50 px-3 py-2 dark:border-slate-800 dark:bg-slate-900">
              <div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Vega chart preset</div>
              <select bind:value={chartKind} class="mt-2 w-full bg-transparent text-sm outline-none">
                <option value="line">line</option>
                <option value="area">area</option>
                <option value="bar">bar</option>
                <option value="point">point</option>
              </select>
            </label>

            <label class="flex items-center gap-3 rounded-xl border border-slate-200 bg-slate-50 px-4 py-3 text-sm text-slate-700 dark:border-slate-800 dark:bg-slate-900 dark:text-slate-200">
              <input type="checkbox" bind:checked={sharedVisualFunction} />
              Share with workspace
            </label>
          </div>

          <div class="flex flex-wrap gap-2">
            <button class="rounded-xl border border-slate-300 px-4 py-2 text-sm font-medium dark:border-slate-700" onclick={resetVisualFunctionDraft}>New Draft</button>
            <button class="rounded-xl bg-slate-900 px-4 py-2 text-sm font-medium text-white disabled:opacity-60 dark:bg-slate-100 dark:text-slate-950" onclick={saveVisualFunction} disabled={savingVisualFunction}>
              {savingVisualFunction ? 'Saving...' : selectedVisualFunctionId ? 'Update Saved Lens' : 'Save Lens'}
            </button>
            {#if selectedVisualFunctionId}
              <button class="rounded-xl border border-rose-300 px-4 py-2 text-sm font-medium text-rose-700 disabled:opacity-60 dark:border-rose-800 dark:text-rose-300" onclick={() => void deleteVisualFunction(selectedVisualFunctionId)} disabled={deletingVisualFunction}>
                {deletingVisualFunction ? 'Deleting...' : 'Delete Saved Lens'}
              </button>
            {/if}
          </div>

          <div class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 text-sm text-slate-600 dark:border-slate-800 dark:bg-slate-900/60 dark:text-slate-300">
            Saved Quiver lenses now persist in `ontology-service` with their canonical Vega-Lite template, instead of living only in browser storage.
          </div>
        </div>

        <div class="mt-5 space-y-3">
          {#if visualFunctions.length === 0}
            <div class="rounded-2xl border border-dashed border-slate-300 px-4 py-5 text-sm text-slate-500 dark:border-slate-700 dark:text-slate-400">
              Save the current lens to reuse it in dashboards, Workshop, or object views later.
            </div>
          {:else}
            {#each visualFunctions as visualFunction (visualFunction.id)}
              <div class={`rounded-2xl border p-4 ${selectedVisualFunctionId === visualFunction.id ? 'border-sky-400 bg-sky-50 dark:border-sky-700 dark:bg-sky-950/30' : 'border-slate-200 bg-slate-50 dark:border-slate-800 dark:bg-slate-900/60'}`}>
                <div class="flex flex-wrap items-start justify-between gap-3">
                  <button class="min-w-0 flex-1 text-left" onclick={() => void applyVisualFunction(visualFunction)}>
                    <div class="flex flex-wrap items-center gap-2">
                      <div class="text-sm font-semibold text-slate-900 dark:text-slate-100">{visualFunction.name}</div>
                      <span class="rounded-full border border-slate-300 px-2 py-0.5 text-[11px] uppercase tracking-[0.2em] text-slate-500 dark:border-slate-700 dark:text-slate-300">{visualFunction.chart_kind}</span>
                      {#if visualFunction.shared}
                        <span class="rounded-full bg-emerald-100 px-2 py-0.5 text-[11px] font-semibold uppercase tracking-[0.2em] text-emerald-700 dark:bg-emerald-950/40 dark:text-emerald-300">shared</span>
                      {/if}
                    </div>
                    <div class="mt-1 text-xs text-slate-500 dark:text-slate-400">
                      {visualFunction.metric_field} by {visualFunction.group_field} • {new Date(visualFunction.updated_at).toLocaleString()}
                    </div>
                    {#if visualFunction.description}
                      <div class="mt-2 text-sm text-slate-600 dark:text-slate-300">{visualFunction.description}</div>
                    {/if}
                  </button>

                  {#if selectedVisualFunctionId === visualFunction.id}
                    <button class="rounded-lg border border-rose-300 px-3 py-1.5 text-sm font-medium text-rose-700 dark:border-rose-800 dark:text-rose-300" onclick={() => void deleteVisualFunction(visualFunction.id)} disabled={deletingVisualFunction}>
                      Delete
                    </button>
                  {/if}
                </div>
              </div>
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
            mode={chartKind === 'point' ? 'line' : chartKind}
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
        <div class="flex flex-wrap items-start justify-between gap-3">
          <div>
            <div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Vega plots</div>
            <h2 class="mt-1 text-xl font-semibold text-slate-950 dark:text-slate-50">Hydrated Vega-Lite export</h2>
            <p class="mt-2 max-w-2xl text-sm leading-6 text-slate-600 dark:text-slate-300">
              Quiver now emits a live Vega-Lite spec for the current lens, including time-series and grouped analytics datasets, so the same analysis can move into external notebooks, published dashboards, or other frontend surfaces.
            </p>
          </div>

          <div class="flex flex-wrap gap-2">
            <button class="rounded-xl border border-slate-300 px-4 py-2 text-sm font-medium dark:border-slate-700" onclick={copyVegaSpec}>Copy spec</button>
            <button class="rounded-xl border border-slate-300 px-4 py-2 text-sm font-medium dark:border-slate-700" onclick={downloadVegaSpec}>Download JSON</button>
          </div>
        </div>

        <div class="mt-4 grid gap-3 md:grid-cols-3">
          <div class="rounded-2xl border border-slate-200 bg-slate-50 p-4 dark:border-slate-800 dark:bg-slate-900/60">
            <div class="text-[11px] uppercase tracking-[0.22em] text-slate-500">Preset</div>
            <div class="mt-2 text-2xl font-semibold text-slate-900 dark:text-slate-100">{chartKind}</div>
          </div>
          <div class="rounded-2xl border border-slate-200 bg-slate-50 p-4 dark:border-slate-800 dark:bg-slate-900/60">
            <div class="text-[11px] uppercase tracking-[0.22em] text-slate-500">Time-series rows</div>
            <div class="mt-2 text-2xl font-semibold text-slate-900 dark:text-slate-100">{timeSeriesRows().length}</div>
          </div>
          <div class="rounded-2xl border border-slate-200 bg-slate-50 p-4 dark:border-slate-800 dark:bg-slate-900/60">
            <div class="text-[11px] uppercase tracking-[0.22em] text-slate-500">Grouped rows</div>
            <div class="mt-2 text-2xl font-semibold text-slate-900 dark:text-slate-100">{groupedRows().length}</div>
          </div>
        </div>

        <div class="mt-4 rounded-2xl border border-slate-200 bg-slate-950 p-4 dark:border-slate-800">
          <pre class="overflow-x-auto whitespace-pre-wrap text-xs leading-6 text-slate-100">{JSON.stringify(currentVegaSpec(), null, 2)}</pre>
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
              <div class="text-[11px] uppercase tracking-[0.22em] text-slate-500">Join scope</div>
              <div class="mt-2 text-lg font-semibold text-slate-900 dark:text-slate-100">{primaryTypeLabel()} → {secondaryTypeLabel()}</div>
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
