<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { page as pageStore } from '$app/stores';
  import { get } from 'svelte/store';
  import DashboardGrid from '$components/dashboard/DashboardGrid.svelte';
  import FilterBar from '$components/dashboard/FilterBar.svelte';
  import WidgetConfig from '$components/dashboard/WidgetConfig.svelte';
  import { dashboards } from '$lib/stores/dashboards';
  import {
    cloneDashboard,
    createDefaultFilters,
    createWidget,
    deserializeDashboardSnapshot,
    duplicateDashboardDefinition,
    formatDashboardTimestamp,
    serializeDashboardSnapshot,
    type DashboardDefinition,
    type DashboardFilterState,
    type DashboardWidget,
    type DashboardWidgetLayout,
  } from '$lib/utils/dashboards';

  const dashboardId = $derived($pageStore.params.id ?? '');

  let loading = $state(true);
  let dashboard = $state<DashboardDefinition | null>(null);
  let sharedDashboard = $state<DashboardDefinition | null>(null);
  let filters = $state<DashboardFilterState>(createDefaultFilters());
  let editMode = $state(false);
  let widgetEditorOpen = $state(false);
  let editorWidget = $state<DashboardWidget | null>(null);
  let feedback = $state('');

  const activeDashboard = $derived(dashboard ?? sharedDashboard);
  const readOnlySnapshot = $derived(sharedDashboard !== null && dashboard === null);

  function persist() {
    if (!dashboard) {
      return;
    }

    dashboard = cloneDashboard(dashboards.save(dashboard));
  }

  function loadDashboard() {
    dashboards.restore();
    const localDashboard = dashboards.getById(dashboardId);

    if (localDashboard) {
      dashboard = cloneDashboard(localDashboard);
      sharedDashboard = null;
      loading = false;
      return;
    }

    const snapshot = get(pageStore).url.searchParams.get('snapshot');
    if (snapshot) {
      try {
        sharedDashboard = cloneDashboard(deserializeDashboardSnapshot(snapshot));
      } catch {
        sharedDashboard = null;
      }
    }

    loading = false;
  }

  function updateMetadata(field: 'name' | 'description', value: string) {
    if (!dashboard) {
      return;
    }

    dashboard[field] = value;
  }

  function reorderWidgets(fromIndex: number, toIndex: number) {
    if (!dashboard) {
      return;
    }

    const nextWidgets = [...dashboard.widgets];
    const [moved] = nextWidgets.splice(fromIndex, 1);
    nextWidgets.splice(toIndex, 0, moved);
    dashboard.widgets = nextWidgets;
    persist();
  }

  function openNewWidget() {
    if (!dashboard) {
      return;
    }

    editorWidget = createWidget('chart');
    widgetEditorOpen = true;
  }

  function openWidgetEditor(widget: DashboardWidget) {
    editorWidget = cloneDashboard(widget);
    widgetEditorOpen = true;
  }

  function saveWidget(widget: DashboardWidget) {
    if (!dashboard) {
      return;
    }

    const existingIndex = dashboard.widgets.findIndex((entry) => entry.id === widget.id);
    if (existingIndex >= 0) {
      dashboard.widgets[existingIndex] = widget;
    } else {
      dashboard.widgets = [...dashboard.widgets, widget];
    }

    persist();
  }

  function deleteWidget(widgetId: string) {
    if (!dashboard || !confirm('Delete this widget?')) {
      return;
    }

    dashboard.widgets = dashboard.widgets.filter((widget) => widget.id !== widgetId);
    persist();
  }

  function duplicateWidget(widgetId: string) {
    if (!dashboard) {
      return;
    }

    const index = dashboard.widgets.findIndex((widget) => widget.id === widgetId);
    if (index < 0) {
      return;
    }

    const source = dashboard.widgets[index];
    const copy = cloneDashboard({
      ...source,
      id: crypto.randomUUID(),
      title: `${source.title} Copy`,
    });

    const nextWidgets = [...dashboard.widgets];
    nextWidgets.splice(index + 1, 0, copy);
    dashboard.widgets = nextWidgets;
    persist();
  }

  function resizeWidget(widgetId: string, layout: DashboardWidgetLayout) {
    if (!dashboard) {
      return;
    }

    dashboard.widgets = dashboard.widgets.map((widget) => widget.id === widgetId ? { ...widget, layout } : widget);
    persist();
  }

  function applyFilters(nextFilters: DashboardFilterState) {
    filters = nextFilters;
  }

  function resetFilters() {
    filters = createDefaultFilters();
  }

  async function duplicateDashboard() {
    if (!activeDashboard) {
      return;
    }

    const duplicate = dashboard
      ? dashboards.duplicate(dashboard.id)
      : dashboards.save(duplicateDashboardDefinition(activeDashboard));

    if (!duplicate) {
      return;
    }

    await goto(`/dashboards/${duplicate.id}`);
  }

  async function shareDashboard() {
    if (!activeDashboard || typeof window === 'undefined') {
      return;
    }

    const snapshot = serializeDashboardSnapshot(activeDashboard);
    const shareUrl = `${window.location.origin}/dashboards/${activeDashboard.id}?snapshot=${snapshot}`;

    try {
      await navigator.clipboard.writeText(shareUrl);
      feedback = 'Share link copied to clipboard.';
    } catch {
      feedback = shareUrl;
    }
  }

  async function importSnapshot() {
    if (!sharedDashboard) {
      return;
    }

    const imported = dashboards.save({
      ...duplicateDashboardDefinition(sharedDashboard),
      name: `${sharedDashboard.name} Imported`,
    });

    await goto(`/dashboards/${imported.id}`);
  }

  function closeWidgetEditor() {
    editorWidget = null;
    widgetEditorOpen = false;
  }

  onMount(() => {
    loadDashboard();
  });
</script>

<svelte:head>
  <title>OpenFoundry — Dashboard Editor</title>
</svelte:head>

{#if loading}
  <div class="py-20 text-center text-slate-500">Loading dashboard...</div>
{:else if !activeDashboard}
  <div class="mx-auto max-w-2xl rounded-[2rem] border border-slate-200 bg-white p-10 text-center shadow-sm dark:border-slate-800 dark:bg-slate-950">
    <h1 class="text-3xl font-semibold text-slate-950 dark:text-slate-50">Dashboard not found</h1>
    <p class="mt-3 text-slate-600 dark:text-slate-300">The requested dashboard does not exist locally and no share snapshot was provided.</p>
    <a href="/dashboards" class="mt-6 inline-flex rounded-xl bg-slate-900 px-4 py-2 text-sm font-medium text-white dark:bg-slate-100 dark:text-slate-950">Back to dashboards</a>
  </div>
{:else}
  <div class="mx-auto max-w-[1500px] space-y-6">
    <section class="rounded-[2rem] border border-slate-200 bg-white p-6 shadow-sm dark:border-slate-800 dark:bg-slate-950">
      <div class="flex flex-wrap items-start justify-between gap-4">
        <div class="max-w-3xl space-y-3">
          <a href="/dashboards" class="text-sm font-medium text-slate-500 hover:text-slate-700 dark:text-slate-400 dark:hover:text-slate-200">← Back to dashboards</a>

          {#if dashboard}
            <input
              type="text"
              value={dashboard.name}
              oninput={(event) => updateMetadata('name', (event.currentTarget as HTMLInputElement).value)}
              onblur={persist}
              class="w-full bg-transparent text-4xl font-semibold tracking-tight text-slate-950 outline-none dark:text-slate-50"
            />

            <textarea
              rows="2"
              oninput={(event) => updateMetadata('description', (event.currentTarget as HTMLTextAreaElement).value)}
              onblur={persist}
              class="w-full resize-none bg-transparent text-base leading-7 text-slate-600 outline-none dark:text-slate-300"
            >{dashboard.description}</textarea>
          {:else}
            <div>
              <div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-amber-500">Shared Snapshot</div>
              <h1 class="mt-2 text-4xl font-semibold tracking-tight text-slate-950 dark:text-slate-50">{activeDashboard.name}</h1>
              <p class="mt-3 text-base leading-7 text-slate-600 dark:text-slate-300">{activeDashboard.description}</p>
            </div>
          {/if}

          <div class="flex flex-wrap gap-2 text-xs text-slate-500 dark:text-slate-400">
            <span class="rounded-full border border-slate-200 px-3 py-1 dark:border-slate-700">{activeDashboard.widgets.length} widgets</span>
            <span class="rounded-full border border-slate-200 px-3 py-1 dark:border-slate-700">Updated {formatDashboardTimestamp(activeDashboard.updatedAt)}</span>
          </div>
        </div>

        <div class="flex flex-wrap gap-2">
          {#if dashboard}
            <button class={`rounded-xl px-4 py-2 text-sm font-medium ${editMode ? 'bg-emerald-600 text-white' : 'border border-slate-300 dark:border-slate-700'}`} onclick={() => editMode = !editMode}>
              {editMode ? 'Done Layout' : 'Edit Layout'}
            </button>
            <button class="rounded-xl bg-slate-900 px-4 py-2 text-sm font-medium text-white dark:bg-slate-100 dark:text-slate-950" onclick={openNewWidget}>Add Widget</button>
          {:else}
            <button class="rounded-xl bg-amber-500 px-4 py-2 text-sm font-medium text-white hover:bg-amber-400" onclick={importSnapshot}>Save Copy</button>
          {/if}

          <button class="rounded-xl border border-slate-300 px-4 py-2 text-sm font-medium dark:border-slate-700" onclick={duplicateDashboard}>Duplicate</button>
          <button class="rounded-xl border border-slate-300 px-4 py-2 text-sm font-medium dark:border-slate-700" onclick={shareDashboard}>Share</button>
        </div>
      </div>

      {#if readOnlySnapshot}
        <div class="mt-5 rounded-2xl border border-amber-200 bg-amber-50 px-4 py-3 text-sm text-amber-700 dark:border-amber-900 dark:bg-amber-950/40 dark:text-amber-300">
          This view comes from a snapshot link. Save a copy to persist edits locally.
        </div>
      {/if}

      {#if feedback}
        <div class="mt-5 rounded-2xl border border-emerald-200 bg-emerald-50 px-4 py-3 text-sm text-emerald-700 dark:border-emerald-900 dark:bg-emerald-950/40 dark:text-emerald-300">
          {feedback}
        </div>
      {/if}
    </section>

    <FilterBar search={filters.search} dateRange={filters.dateRange} onApply={applyFilters} onReset={resetFilters} />

    <DashboardGrid
      widgets={activeDashboard.widgets}
      filters={filters}
      editing={editMode && !readOnlySnapshot}
      onReorder={reorderWidgets}
      onEditWidget={openWidgetEditor}
      onDeleteWidget={deleteWidget}
      onDuplicateWidget={duplicateWidget}
      onResizeWidget={resizeWidget}
    />

    <WidgetConfig open={widgetEditorOpen} initialWidget={editorWidget} onSave={saveWidget} onClose={closeWidgetEditor} />
  </div>
{/if}