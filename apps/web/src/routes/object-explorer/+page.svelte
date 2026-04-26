<script lang="ts">
  import { browser } from '$app/environment';
  import { onMount } from 'svelte';
  import Glyph from '$components/ui/Glyph.svelte';
  import {
    createObjectSet,
    evaluateObjectSet,
    getObjectView,
    listObjectSets,
    listObjectTypes,
    searchOntology,
    type ObjectSetDefinition,
    type ObjectType,
    type ObjectViewResponse,
    type SearchResult
  } from '$lib/api/ontology';

  type ExplorerTab = 'all' | 'objects' | 'object_types' | 'artifacts';
  type SearchMode = 'semantic' | 'lexical';

  interface SavedExploration {
    id: string;
    name: string;
    query: string;
    objectTypeId: string;
    tab: ExplorerTab;
    createdAt: string;
  }

  const tabOptions: { id: ExplorerTab; label: string }[] = [
    { id: 'all', label: 'All' },
    { id: 'objects', label: 'Objects' },
    { id: 'object_types', label: 'Object types' },
    { id: 'artifacts', label: 'Artifacts' }
  ];

  const syntaxExamples = [
    'airport delay risk',
    'status:active owner:east',
    'priority high escalation',
    'region europe aircraft'
  ];

  let objectTypes = $state<ObjectType[]>([]);
  let objectSets = $state<ObjectSetDefinition[]>([]);
  let loadingCatalog = $state(true);
  let loadError = $state('');

  let query = $state('');
  let selectedTypeId = $state('');
  let activeTab = $state<ExplorerTab>('all');
  let searchMode = $state<SearchMode>('semantic');
  let searchLoading = $state(false);
  let searchError = $state('');
  let results = $state<SearchResult[]>([]);
  let selectedResultId = $state('');

  let previewLoading = $state(false);
  let previewError = $state('');
  let preview = $state<ObjectViewResponse | null>(null);

  let saveName = $state('');
  let saveError = $state('');
  let savingExploration = $state(false);
  let creatingList = $state(false);
  let savedExplorations = $state<SavedExploration[]>([]);

  let comparisonLeftId = $state('');
  let comparisonRightId = $state('');
  let comparisonLoading = $state(false);
  let comparisonError = $state('');
  let comparisonLeftRows = $state(0);
  let comparisonRightRows = $state(0);

  const objectTypeMap = $derived.by(() => {
    const map = new Map<string, ObjectType>();
    for (const item of objectTypes) {
      map.set(item.id, item);
    }
    return map;
  });

  const selectedType = $derived(selectedTypeId ? objectTypeMap.get(selectedTypeId) ?? null : null);

  const resultCounts = $derived.by(() => {
    const counts: Record<ExplorerTab, number> = {
      all: results.length,
      objects: 0,
      object_types: 0,
      artifacts: 0
    };

    for (const result of results) {
      if (result.kind === 'object_instance') {
        counts.objects += 1;
      } else if (result.kind === 'object_type') {
        counts.object_types += 1;
      } else {
        counts.artifacts += 1;
      }
    }

    return counts;
  });

  const visibleResults = $derived.by(() => {
    if (activeTab === 'all') return results;
    if (activeTab === 'objects') return results.filter((result) => result.kind === 'object_instance');
    if (activeTab === 'object_types') return results.filter((result) => result.kind === 'object_type');
    return results.filter((result) => result.kind !== 'object_instance' && result.kind !== 'object_type');
  });

  const groupedResults = $derived.by(() => {
    const groups = new Map<string, SearchResult[]>();
    for (const result of results) {
      const label = kindLabel(result.kind);
      if (!groups.has(label)) groups.set(label, []);
      groups.get(label)!.push(result);
    }
    return Array.from(groups.entries());
  });

  const objectTypeCounts = $derived.by(() => {
    const counts = new Map<string, number>();
    for (const result of results) {
      if (result.kind !== 'object_instance' || !result.object_type_id) continue;
      counts.set(result.object_type_id, (counts.get(result.object_type_id) ?? 0) + 1);
    }
    return Array.from(counts.entries())
      .map(([id, count]) => ({ id, count, label: objectTypeMap.get(id)?.display_name ?? id }))
      .sort((left, right) => right.count - left.count)
      .slice(0, 8);
  });

  const objectResults = $derived(results.filter((result) => result.kind === 'object_instance'));

  const chartBuckets = $derived.by(() => {
    const total = objectResults.length || 1;
    return objectTypeCounts.map((item) => ({
      ...item,
      width: Math.max(8, Math.round((item.count / total) * 100))
    }));
  });

  const selectedResult = $derived(results.find((result) => result.id === selectedResultId) ?? null);

  const shareUrl = $derived.by(() => {
    if (!browser) return '';
    const url = new URL(window.location.href);
    if (query.trim()) {
      url.searchParams.set('q', query.trim());
    } else {
      url.searchParams.delete('q');
    }
    if (selectedTypeId) {
      url.searchParams.set('type', selectedTypeId);
    } else {
      url.searchParams.delete('type');
    }
    url.searchParams.set('tab', activeTab);
    url.searchParams.set('mode', searchMode);
    return url.toString();
  });

  function kindLabel(kind: string) {
    return kind.replaceAll('_', ' ');
  }

  function objectTypeLabel(typeId: string | null | undefined) {
    if (!typeId) return 'Unknown type';
    return objectTypeMap.get(typeId)?.display_name ?? typeId;
  }

  function objectGlyph(kind: string) {
    if (kind === 'object_instance') return 'object';
    if (kind === 'object_type') return 'cube';
    if (kind === 'action_type') return 'run';
    if (kind === 'link_type') return 'link';
    return 'artifact';
  }

  function resultTone(kind: string) {
    if (kind === 'object_instance') return 'bg-[#e8f2ff] text-[#2458b8]';
    if (kind === 'object_type') return 'bg-[#eef5e8] text-[#356b3c]';
    return 'bg-[#fff1df] text-[#a35a11]';
  }

  function formatSnippet(text: string | null | undefined) {
    return text?.trim() || 'No snippet available for this result.';
  }

  function normalizeValue(value: unknown) {
    if (value === null || value === undefined) return '—';
    if (Array.isArray(value)) return value.length ? value.join(', ') : '—';
    if (typeof value === 'object') return JSON.stringify(value);
    return String(value);
  }

  function objectLabelFromProperties(properties: Record<string, unknown>) {
    const preferredKeys = ['name', 'title', 'display_name', 'label', 'code', 'identifier', 'id'];
    for (const key of preferredKeys) {
      const value = properties[key];
      if (typeof value === 'string' && value.trim()) return value;
    }

    const firstString = Object.values(properties).find((value) => typeof value === 'string' && value.trim());
    if (typeof firstString === 'string' && firstString.trim()) return firstString;
    return 'Linked object';
  }

  function syncUrl() {
    if (!browser) return;
    const url = new URL(window.location.href);
    if (query.trim()) url.searchParams.set('q', query.trim());
    else url.searchParams.delete('q');
    if (selectedTypeId) url.searchParams.set('type', selectedTypeId);
    else url.searchParams.delete('type');
    url.searchParams.set('tab', activeTab);
    url.searchParams.set('mode', searchMode);
    window.history.replaceState({}, '', url);
  }

  function hydrateFromUrl() {
    if (!browser) return;
    const params = new URLSearchParams(window.location.search);
    query = params.get('q') ?? '';
    selectedTypeId = params.get('type') ?? '';
    const nextTab = params.get('tab');
    if (nextTab === 'all' || nextTab === 'objects' || nextTab === 'object_types' || nextTab === 'artifacts') {
      activeTab = nextTab;
    }
    const nextMode = params.get('mode');
    if (nextMode === 'lexical' || nextMode === 'semantic') {
      searchMode = nextMode;
    }
  }

  function loadSavedExplorations() {
    if (!browser) return;
    try {
      const parsed = JSON.parse(window.localStorage.getItem('of.objectExplorer.saved') ?? '[]');
      savedExplorations = Array.isArray(parsed) ? parsed : [];
    } catch {
      savedExplorations = [];
    }
  }

  function persistSavedExplorations() {
    if (!browser) return;
    window.localStorage.setItem('of.objectExplorer.saved', JSON.stringify(savedExplorations));
  }

  async function loadCatalog() {
    loadingCatalog = true;
    loadError = '';
    try {
      const [typesResponse, setsResponse] = await Promise.all([
        listObjectTypes({ per_page: 200 }),
        listObjectSets()
      ]);
      objectTypes = typesResponse.data;
      objectSets = setsResponse.data;
      if (!selectedTypeId && objectTypes[0]) {
        selectedTypeId = objectTypes[0].id;
      }
    } catch (cause) {
      loadError = cause instanceof Error ? cause.message : 'Failed to load Object Explorer catalog';
    } finally {
      loadingCatalog = false;
    }
  }

  async function runSearch() {
    const trimmed = query.trim();
    syncUrl();
    if (!trimmed) {
      results = [];
      selectedResultId = '';
      preview = null;
      searchError = '';
      return;
    }

    searchLoading = true;
    searchError = '';
    preview = null;
    previewError = '';

    try {
      const response = await searchOntology({
        query: trimmed,
        object_type_id: selectedTypeId || undefined,
        limit: 60,
        semantic: searchMode === 'semantic'
      });
      results = response.data;
      selectedResultId = response.data[0]?.id ?? '';
      if (response.data[0]?.kind === 'object_instance') {
        await loadObjectPreview(response.data[0]);
      }
    } catch (cause) {
      results = [];
      selectedResultId = '';
      preview = null;
      searchError = cause instanceof Error ? cause.message : 'Search failed';
    } finally {
      searchLoading = false;
    }
  }

  async function loadObjectPreview(result: SearchResult) {
    if (result.kind !== 'object_instance' || !result.object_type_id) {
      preview = null;
      previewError = '';
      return;
    }

    previewLoading = true;
    previewError = '';
    try {
      preview = await getObjectView(result.object_type_id, result.id);
    } catch (cause) {
      preview = null;
      previewError = cause instanceof Error ? cause.message : 'Failed to load object preview';
    } finally {
      previewLoading = false;
    }
  }

  async function selectResult(result: SearchResult) {
    selectedResultId = result.id;
    if (result.kind === 'object_instance') {
      await loadObjectPreview(result);
    } else {
      preview = null;
      previewError = '';
    }
  }

  async function saveExploration() {
    if (!query.trim() || !saveName.trim()) {
      saveError = 'Name and query are required';
      return;
    }

    savingExploration = true;
    saveError = '';
    try {
      savedExplorations = [
        {
          id: crypto.randomUUID(),
          name: saveName.trim(),
          query: query.trim(),
          objectTypeId: selectedTypeId,
          tab: activeTab,
          createdAt: new Date().toISOString()
        },
        ...savedExplorations
      ];
      persistSavedExplorations();
      saveName = '';
    } catch (cause) {
      saveError = cause instanceof Error ? cause.message : 'Failed to save exploration';
    } finally {
      savingExploration = false;
    }
  }

  function openSavedExploration(item: SavedExploration) {
    query = item.query;
    selectedTypeId = item.objectTypeId;
    activeTab = item.tab;
    void runSearch();
  }

  function deleteSavedExploration(id: string) {
    savedExplorations = savedExplorations.filter((item) => item.id !== id);
    persistSavedExplorations();
  }

  async function createGovernedList() {
    if (!selectedTypeId) {
      saveError = 'Choose an object type before creating a governed list';
      return;
    }

    creatingList = true;
    saveError = '';
    try {
      const seededName = `${selectedType?.display_name ?? 'object'}_${new Date().toISOString().slice(0, 10)}`;
      await createObjectSet({
        name: seededName.toLowerCase().replaceAll(/\s+/g, '_'),
        description: `Seeded from Object Explorer query "${query.trim() || 'all results'}". Refine filters in Object Sets for a governed reusable list.`,
        base_object_type_id: selectedTypeId,
        filters: [],
        traversals: [],
        projections: ['base.id']
      });
      const setsResponse = await listObjectSets();
      objectSets = setsResponse.data;
    } catch (cause) {
      saveError = cause instanceof Error ? cause.message : 'Failed to create governed list';
    } finally {
      creatingList = false;
    }
  }

  async function runComparison() {
    if (!comparisonLeftId || !comparisonRightId) {
      comparisonError = 'Choose two saved lists to compare';
      return;
    }

    comparisonLoading = true;
    comparisonError = '';
    try {
      const [left, right] = await Promise.all([
        evaluateObjectSet(comparisonLeftId, { limit: 250 }),
        evaluateObjectSet(comparisonRightId, { limit: 250 })
      ]);
      comparisonLeftRows = left.total_rows;
      comparisonRightRows = right.total_rows;
    } catch (cause) {
      comparisonError = cause instanceof Error ? cause.message : 'Failed to compare object sets';
    } finally {
      comparisonLoading = false;
    }
  }

  async function pivotToLinkedObject(typeId: string, title: string) {
    selectedTypeId = typeId;
    activeTab = 'objects';
    query = title;
    await runSearch();
  }

  async function copyShareLink() {
    if (!browser || !shareUrl) return;
    await navigator.clipboard.writeText(shareUrl);
  }

  function handleSearchKeydown(event: KeyboardEvent) {
    if (event.key === 'Enter') {
      event.preventDefault();
      void runSearch();
    }
  }

  function formatDate(date: string) {
    return new Intl.DateTimeFormat('en', { dateStyle: 'medium', timeStyle: 'short' }).format(new Date(date));
  }

  onMount(async () => {
    hydrateFromUrl();
    loadSavedExplorations();
    await loadCatalog();
    if (query.trim()) {
      await runSearch();
    }
  });
</script>

<svelte:head>
  <title>OpenFoundry - Object Explorer</title>
</svelte:head>

<div class="space-y-5">
  <section class="overflow-hidden rounded-[30px] border border-[var(--border-default)] bg-[linear-gradient(135deg,#f8fbff_0%,#eef5ff_45%,#edf8f0_100%)] shadow-[var(--shadow-panel)]">
    <div class="grid gap-6 px-6 py-6 lg:grid-cols-[minmax(0,1.35fr)_330px] lg:px-8">
      <div>
        <div class="of-eyebrow">Object Explorer</div>
        <h1 class="mt-3 max-w-4xl text-[34px] font-semibold tracking-[-0.03em] text-[var(--text-strong)]">
          Search, compare, and pivot across ontology objects from a dedicated walk-up surface.
        </h1>
        <p class="mt-4 max-w-3xl text-[15px] leading-7 text-[var(--text-muted)]">
          Object Explorer brings global ontology search, chart-led exploration, governed lists, SQL
          analysis, and linked-object pivoting into a product-level experience instead of scattering
          them across separate workbenches.
        </p>

        <div class="mt-6 rounded-[22px] border border-white/80 bg-white/85 p-4 backdrop-blur">
          <div class="grid gap-3 xl:grid-cols-[minmax(0,1fr)_190px_170px_auto]">
            <label class="block text-sm">
              <span class="mb-1 block font-medium text-[var(--text-default)]">Search the ontology</span>
              <input
                bind:value={query}
                onkeydown={handleSearchKeydown}
                placeholder="Find airports, cases, customers, alerts, saved lists, or modules"
                class="of-input"
              />
            </label>

            <label class="block text-sm">
              <span class="mb-1 block font-medium text-[var(--text-default)]">Object type</span>
              <select bind:value={selectedTypeId} class="of-select">
                <option value="">All types</option>
                {#each objectTypes as item (item.id)}
                  <option value={item.id}>{item.display_name}</option>
                {/each}
              </select>
            </label>

            <div class="block text-sm">
              <span class="mb-1 block font-medium text-[var(--text-default)]">Search mode</span>
              <div class="of-pill-toggle">
                <button type="button" data-active={searchMode === 'semantic'} onclick={() => searchMode = 'semantic'}>Semantic</button>
                <button type="button" data-active={searchMode === 'lexical'} onclick={() => searchMode = 'lexical'}>Lexical</button>
              </div>
            </div>

            <div class="flex items-end gap-2">
              <button class="of-btn of-btn-primary" type="button" onclick={runSearch} disabled={searchLoading}>
                <Glyph name="search" size={16} />
                <span>{searchLoading ? 'Searching...' : 'Search'}</span>
              </button>
              <button class="of-btn" type="button" onclick={copyShareLink} disabled={!shareUrl}>
                <Glyph name="link" size={16} />
                <span>Copy URL</span>
              </button>
            </div>
          </div>

          <div class="mt-3 flex flex-wrap gap-2">
            {#each syntaxExamples as example}
              <button type="button" class="of-chip" onclick={() => query = example}>{example}</button>
            {/each}
          </div>
        </div>
      </div>

      <aside class="rounded-[22px] border border-white/80 bg-white/78 p-5 backdrop-blur">
        <div class="of-heading-sm">Explorer status</div>
        <div class="mt-4 grid gap-3">
          <article class="rounded-[18px] border border-[var(--border-subtle)] bg-white/90 px-4 py-3">
            <div class="text-[11px] font-semibold uppercase tracking-[0.18em] text-[var(--text-soft)]">Saved lists</div>
            <div class="mt-1 text-2xl font-semibold text-[var(--text-strong)]">{objectSets.length}</div>
            <div class="text-sm text-[var(--text-muted)]">Governed lists ready for compare and reuse.</div>
          </article>
          <article class="rounded-[18px] border border-[var(--border-subtle)] bg-white/90 px-4 py-3">
            <div class="text-[11px] font-semibold uppercase tracking-[0.18em] text-[var(--text-soft)]">Saved explorations</div>
            <div class="mt-1 text-2xl font-semibold text-[var(--text-strong)]">{savedExplorations.length}</div>
            <div class="text-sm text-[var(--text-muted)]">Reusable search states stored for walk-up return.</div>
          </article>
          <article class="rounded-[18px] border border-[var(--border-subtle)] bg-white/90 px-4 py-3">
            <div class="text-[11px] font-semibold uppercase tracking-[0.18em] text-[var(--text-soft)]">Mode</div>
            <div class="mt-1 text-2xl font-semibold capitalize text-[var(--text-strong)]">{searchMode}</div>
            <div class="text-sm text-[var(--text-muted)]">Switch between semantic recall and exact matching.</div>
          </article>
        </div>
      </aside>
    </div>
  </section>

  {#if loadError || searchError || previewError || saveError || comparisonError}
    <div class="of-inline-note">
      {loadError || searchError || previewError || saveError || comparisonError}
    </div>
  {/if}

  <section class="grid gap-5 xl:grid-cols-[280px_minmax(0,1.2fr)_360px]">
    <aside class="of-panel p-5">
      <div class="flex items-center justify-between">
        <div class="of-heading-sm">Navigate</div>
        <span class="of-chip">{results.length} results</span>
      </div>

      <div class="mt-4 space-y-2">
        {#each tabOptions as tab}
          <button
            type="button"
            class={`flex w-full items-center justify-between rounded-[14px] px-3 py-2 text-left text-sm ${
              activeTab === tab.id ? 'bg-[#dfeafb] text-[#204d9b]' : 'hover:bg-[var(--bg-hover)]'
            }`}
            onclick={() => activeTab = tab.id}
          >
            <span>{tab.label}</span>
            <span class="text-xs text-[var(--text-muted)]">{resultCounts[tab.id]}</span>
          </button>
        {/each}
      </div>

      <div class="mt-6 border-t border-[var(--border-subtle)] pt-5">
        <div class="of-heading-sm">Object type filters</div>
        <div class="mt-3 space-y-2">
          {#if objectTypeCounts.length === 0}
            <div class="rounded-[16px] border border-dashed border-[var(--border-default)] px-3 py-5 text-sm text-[var(--text-muted)]">
              Run a search to see prominent object types.
            </div>
          {:else}
            {#each objectTypeCounts as item}
              <button
                type="button"
                class={`flex w-full items-center justify-between rounded-[14px] px-3 py-2 text-left text-sm ${
                  selectedTypeId === item.id ? 'bg-[#edf5e8] text-[#356b3c]' : 'hover:bg-[var(--bg-hover)]'
                }`}
                onclick={async () => {
                  selectedTypeId = item.id;
                  activeTab = 'objects';
                  await runSearch();
                }}
              >
                <span class="truncate pr-3">{item.label}</span>
                <span class="text-xs text-[var(--text-muted)]">{item.count}</span>
              </button>
            {/each}
          {/if}
        </div>
      </div>

      <div class="mt-6 border-t border-[var(--border-subtle)] pt-5">
        <div class="of-heading-sm">Open with</div>
        <div class="mt-3 space-y-2 text-sm">
          <a href="/queries" class="flex items-center justify-between rounded-[14px] px-3 py-2 hover:bg-[var(--bg-hover)]">
            <span>Analyze using SQL</span>
            <Glyph name="database" size={15} />
          </a>
          <a href="/ontology/graph" class="flex items-center justify-between rounded-[14px] px-3 py-2 hover:bg-[var(--bg-hover)]">
            <span>Open graph exploration</span>
            <Glyph name="graph" size={15} />
          </a>
          <a href="/quiver" class="flex items-center justify-between rounded-[14px] px-3 py-2 hover:bg-[var(--bg-hover)]">
            <span>Open in Quiver</span>
            <Glyph name="artifact" size={15} />
          </a>
          <a href="/ontology/object-sets" class="flex items-center justify-between rounded-[14px] px-3 py-2 hover:bg-[var(--bg-hover)]">
            <span>Manage lists</span>
            <Glyph name="bookmark" size={15} />
          </a>
        </div>
      </div>
    </aside>

    <div class="space-y-5">
      <section class="of-panel overflow-hidden">
        <div class="flex flex-wrap items-center justify-between gap-3 border-b border-[var(--border-subtle)] px-5 py-4">
          <div>
            <div class="of-heading-sm">Results</div>
            <div class="mt-1 text-sm text-[var(--text-muted)]">
              {#if query.trim()}
                {results.length} matches for "{query.trim()}" {#if selectedType}in {selectedType.display_name}{/if}
              {:else}
                Start with a keyword or property-style query to search objects and ontology artifacts.
              {/if}
            </div>
          </div>

          <div class="flex gap-2">
            <button class="of-btn" type="button" onclick={saveExploration} disabled={savingExploration || !query.trim() || !saveName.trim()}>
              <Glyph name="bookmark" size={15} />
              <span>{savingExploration ? 'Saving...' : 'Save exploration'}</span>
            </button>
            <button class="of-btn" type="button" onclick={createGovernedList} disabled={creatingList || !selectedTypeId}>
              <Glyph name="plus" size={15} />
              <span>{creatingList ? 'Creating...' : 'Seed governed list'}</span>
            </button>
          </div>
        </div>

        <div class="grid gap-4 border-b border-[var(--border-subtle)] bg-[#fbfcfe] px-5 py-4 lg:grid-cols-[minmax(0,1fr)_280px]">
          <label class="block text-sm">
            <span class="mb-1 block font-medium text-[var(--text-default)]">Exploration name</span>
            <input bind:value={saveName} placeholder="Eastern airports with risk flags" class="of-input" />
          </label>
          <div class="rounded-[16px] border border-[var(--border-subtle)] bg-white px-4 py-3 text-sm text-[var(--text-muted)]">
            Saving an exploration stores the current search, type filter, tab, and shareable URL. Seeding a governed list creates a reusable Object Set from the current type so you can refine filters later.
          </div>
        </div>

        {#if searchLoading}
          <div class="px-5 py-16 text-center text-sm text-[var(--text-muted)]">Searching ontology objects...</div>
        {:else if results.length === 0}
          <div class="px-5 py-16 text-center text-sm text-[var(--text-muted)]">
            {#if query.trim()}
              No results matched the current query.
            {:else}
              Search results, saved artifacts, and walk-up exploration will appear here.
            {/if}
          </div>
        {:else if activeTab === 'all'}
          <div class="space-y-6 px-5 py-5">
            {#each groupedResults as [groupLabel, groupItems]}
              <section>
                <div class="mb-3 flex items-center justify-between">
                  <div class="of-heading-sm capitalize">{groupLabel}</div>
                  <span class="of-chip">{groupItems.length}</span>
                </div>
                <div class="space-y-3">
                  {#each groupItems.slice(0, 6) as result (result.kind + result.id)}
                    <button
                      type="button"
                      class={`w-full rounded-[18px] border p-4 text-left transition ${
                        selectedResultId === result.id
                          ? 'border-[#9ec0f4] bg-[#f6f9ff]'
                          : 'border-[var(--border-default)] hover:border-[#c0d5f5] hover:bg-[#fbfdff]'
                      }`}
                      onclick={() => selectResult(result)}
                    >
                      <div class="flex items-start gap-3">
                        <span class={`of-icon-box h-10 w-10 shrink-0 ${resultTone(result.kind)}`}>
                          <Glyph name={objectGlyph(result.kind)} size={18} />
                        </span>
                        <span class="min-w-0 flex-1">
                          <span class="flex flex-wrap items-center gap-2">
                            <span class="truncate text-[15px] font-semibold text-[var(--text-strong)]">{result.title}</span>
                            <span class="rounded-full bg-[var(--bg-hover)] px-2 py-0.5 text-[11px] capitalize text-[var(--text-muted)]">{kindLabel(result.kind)}</span>
                          </span>
                          {#if result.subtitle}
                            <span class="mt-1 block text-sm text-[var(--text-muted)]">{result.subtitle}</span>
                          {/if}
                          <span class="mt-2 block text-sm text-[var(--text-default)]">{formatSnippet(result.snippet)}</span>
                        </span>
                        <span class="text-xs text-[var(--text-soft)]">Score {result.score.toFixed(2)}</span>
                      </div>
                    </button>
                  {/each}
                </div>
              </section>
            {/each}
          </div>
        {:else}
          <div class="space-y-3 px-5 py-5">
            {#each visibleResults as result (result.kind + result.id)}
              <button
                type="button"
                class={`w-full rounded-[18px] border p-4 text-left transition ${
                  selectedResultId === result.id
                    ? 'border-[#9ec0f4] bg-[#f6f9ff]'
                    : 'border-[var(--border-default)] hover:border-[#c0d5f5] hover:bg-[#fbfdff]'
                }`}
                onclick={() => selectResult(result)}
              >
                <div class="flex items-start gap-3">
                  <span class={`of-icon-box h-10 w-10 shrink-0 ${resultTone(result.kind)}`}>
                    <Glyph name={objectGlyph(result.kind)} size={18} />
                  </span>
                  <span class="min-w-0 flex-1">
                    <span class="flex flex-wrap items-center gap-2">
                      <span class="truncate text-[15px] font-semibold text-[var(--text-strong)]">{result.title}</span>
                      {#if result.object_type_id}
                        <span class="rounded-full bg-[var(--bg-hover)] px-2 py-0.5 text-[11px] text-[var(--text-muted)]">
                          {objectTypeLabel(result.object_type_id)}
                        </span>
                      {/if}
                    </span>
                    {#if result.subtitle}
                      <span class="mt-1 block text-sm text-[var(--text-muted)]">{result.subtitle}</span>
                    {/if}
                    <span class="mt-2 block text-sm text-[var(--text-default)]">{formatSnippet(result.snippet)}</span>
                  </span>
                  <span class="text-xs text-[var(--text-soft)]">Score {result.score.toFixed(2)}</span>
                </div>
              </button>
            {/each}
          </div>
        {/if}
      </section>

      <section class="grid gap-5 lg:grid-cols-[minmax(0,1fr)_360px]">
        <div class="of-panel p-5">
          <div class="flex items-center justify-between">
            <div>
              <div class="of-heading-sm">Explore with charts</div>
              <div class="mt-1 text-sm text-[var(--text-muted)]">A quick distribution view for the current object results.</div>
            </div>
            <span class="of-chip">{objectResults.length} objects</span>
          </div>

          {#if chartBuckets.length === 0}
            <div class="mt-4 rounded-[18px] border border-dashed border-[var(--border-default)] px-4 py-10 text-center text-sm text-[var(--text-muted)]">
              Search for object instances to populate the exploration charts.
            </div>
          {:else}
            <div class="mt-4 space-y-3">
              {#each chartBuckets as bucket}
                <article class="rounded-[18px] border border-[var(--border-subtle)] bg-white p-4">
                  <div class="flex items-center justify-between gap-3">
                    <div class="truncate text-sm font-medium text-[var(--text-strong)]">{bucket.label}</div>
                    <div class="text-xs text-[var(--text-muted)]">{bucket.count} matches</div>
                  </div>
                  <div class="mt-3 h-2 rounded-full bg-[#edf1f6]">
                    <div class="h-full rounded-full bg-[linear-gradient(90deg,#376fd5_0%,#6ea6ff_100%)]" style={`width:${bucket.width}%`}></div>
                  </div>
                </article>
              {/each}
            </div>
          {/if}
        </div>

        <div class="of-panel p-5">
          <div class="of-heading-sm">Compare object sets</div>
          <div class="mt-1 text-sm text-[var(--text-muted)]">
            Compare two saved lists side-by-side before opening them in a deeper workflow.
          </div>

          <div class="mt-4 space-y-3">
            <label class="block text-sm">
              <span class="mb-1 block font-medium text-[var(--text-default)]">Left set</span>
              <select bind:value={comparisonLeftId} class="of-select">
                <option value="">Choose saved list</option>
                {#each objectSets as item (item.id)}
                  <option value={item.id}>{item.name}</option>
                {/each}
              </select>
            </label>
            <label class="block text-sm">
              <span class="mb-1 block font-medium text-[var(--text-default)]">Right set</span>
              <select bind:value={comparisonRightId} class="of-select">
                <option value="">Choose comparison list</option>
                {#each objectSets as item (item.id)}
                  <option value={item.id}>{item.name}</option>
                {/each}
              </select>
            </label>
            <button class="of-btn of-btn-primary" type="button" onclick={runComparison} disabled={comparisonLoading}>
              <Glyph name="graph" size={15} />
              <span>{comparisonLoading ? 'Comparing...' : 'Compare sets'}</span>
            </button>
          </div>

          {#if comparisonLeftRows || comparisonRightRows}
            <div class="mt-4 grid gap-3 sm:grid-cols-2">
              <article class="rounded-[18px] border border-[var(--border-subtle)] bg-[#fbfcfe] px-4 py-4">
                <div class="text-[11px] font-semibold uppercase tracking-[0.18em] text-[var(--text-soft)]">Left</div>
                <div class="mt-1 text-2xl font-semibold text-[var(--text-strong)]">{comparisonLeftRows}</div>
                <div class="text-sm text-[var(--text-muted)]">rows in evaluation preview</div>
              </article>
              <article class="rounded-[18px] border border-[var(--border-subtle)] bg-[#fbfcfe] px-4 py-4">
                <div class="text-[11px] font-semibold uppercase tracking-[0.18em] text-[var(--text-soft)]">Right</div>
                <div class="mt-1 text-2xl font-semibold text-[var(--text-strong)]">{comparisonRightRows}</div>
                <div class="text-sm text-[var(--text-muted)]">rows in evaluation preview</div>
              </article>
            </div>
          {/if}
        </div>
      </section>
    </div>

    <aside class="space-y-5">
      <section class="of-panel overflow-hidden">
        <div class="border-b border-[var(--border-subtle)] px-5 py-4">
          <div class="of-heading-sm">Preview</div>
          <div class="mt-1 text-sm text-[var(--text-muted)]">
            Object View preview, linked-object pivots, and runtime context for the selected result.
          </div>
        </div>

        {#if previewLoading}
          <div class="px-5 py-12 text-center text-sm text-[var(--text-muted)]">Loading object preview...</div>
        {:else if preview && selectedResult}
          <div class="space-y-5 px-5 py-5">
            <div>
              <div class="flex items-start gap-3">
                <span class="of-icon-box h-11 w-11 shrink-0 bg-[#e8f2ff] text-[#2458b8]">
                  <Glyph name="object" size={20} />
                </span>
                <div class="min-w-0">
                  <div class="text-[16px] font-semibold text-[var(--text-strong)]">{selectedResult.title}</div>
                  <div class="mt-1 text-sm text-[var(--text-muted)]">{objectTypeLabel(selectedResult.object_type_id)}</div>
                  <div class="mt-1 text-sm text-[var(--text-default)]">{formatSnippet(selectedResult.snippet)}</div>
                </div>
              </div>
            </div>

            <div class="grid gap-3 sm:grid-cols-2">
              <article class="rounded-[16px] border border-[var(--border-subtle)] bg-[#fbfcfe] px-4 py-3">
                <div class="text-[11px] font-semibold uppercase tracking-[0.18em] text-[var(--text-soft)]">Actions</div>
                <div class="mt-1 text-xl font-semibold text-[var(--text-strong)]">{preview.applicable_actions.length}</div>
              </article>
              <article class="rounded-[16px] border border-[var(--border-subtle)] bg-[#fbfcfe] px-4 py-3">
                <div class="text-[11px] font-semibold uppercase tracking-[0.18em] text-[var(--text-soft)]">Linked objects</div>
                <div class="mt-1 text-xl font-semibold text-[var(--text-strong)]">{preview.neighbors.length}</div>
              </article>
            </div>

            <div>
              <div class="mb-2 of-heading-sm">Summary</div>
              <div class="space-y-2">
                {#each Object.entries(preview.summary).slice(0, 8) as [key, value]}
                  <div class="flex items-start justify-between gap-3 rounded-[14px] border border-[var(--border-subtle)] px-3 py-2 text-sm">
                    <span class="text-[var(--text-muted)]">{key}</span>
                    <span class="max-w-[60%] text-right text-[var(--text-default)]">{normalizeValue(value)}</span>
                  </div>
                {/each}
              </div>
            </div>

            <div>
              <div class="mb-2 of-heading-sm">Linked objects</div>
              <div class="space-y-2">
                {#if preview.neighbors.length === 0}
                  <div class="rounded-[16px] border border-dashed border-[var(--border-default)] px-3 py-5 text-sm text-[var(--text-muted)]">
                    No linked objects exposed in the preview.
                  </div>
                {:else}
                  {#each preview.neighbors.slice(0, 8) as neighbor (neighbor.link_id)}
                    <article class="rounded-[16px] border border-[var(--border-subtle)] p-3">
                      <div class="flex items-start justify-between gap-3">
                        <div class="min-w-0">
                          <div class="truncate text-sm font-medium text-[var(--text-strong)]">
                            {objectLabelFromProperties(neighbor.object.properties)}
                          </div>
                          <div class="mt-1 text-xs text-[var(--text-muted)]">
                            {neighbor.direction} via {neighbor.link_name}
                          </div>
                        </div>
                        <button
                          type="button"
                          class="of-btn px-3 py-1.5 text-xs"
                          onclick={() => pivotToLinkedObject(neighbor.object.object_type_id, objectLabelFromProperties(neighbor.object.properties))}
                        >
                          Pivot
                        </button>
                      </div>
                    </article>
                  {/each}
                {/if}
              </div>
            </div>

            <div class="flex flex-wrap gap-2">
              <a href={selectedResult.route} class="of-btn of-btn-primary">
                <Glyph name="object" size={15} />
                <span>Open Object View</span>
              </a>
              {#if selectedResult.object_type_id}
                <a href={`/ontology/graph?root_object_id=${selectedResult.id}`} class="of-btn">
                  <Glyph name="graph" size={15} />
                  <span>Open graph</span>
                </a>
              {/if}
            </div>
          </div>
        {:else if selectedResult}
          <div class="space-y-4 px-5 py-5">
            <div class="flex items-start gap-3">
              <span class={`of-icon-box h-11 w-11 shrink-0 ${resultTone(selectedResult.kind)}`}>
                <Glyph name={objectGlyph(selectedResult.kind)} size={20} />
              </span>
              <div>
                <div class="text-[16px] font-semibold text-[var(--text-strong)]">{selectedResult.title}</div>
                <div class="mt-1 text-sm text-[var(--text-muted)] capitalize">{kindLabel(selectedResult.kind)}</div>
                <div class="mt-2 text-sm text-[var(--text-default)]">{formatSnippet(selectedResult.snippet)}</div>
              </div>
            </div>
            <a href={selectedResult.route} class="of-btn of-btn-primary">
              <Glyph name="link" size={15} />
              <span>Open result</span>
            </a>
          </div>
        {:else}
          <div class="px-5 py-12 text-center text-sm text-[var(--text-muted)]">
            Select a result to inspect linked objects, summary metrics, and object-level actions.
          </div>
        {/if}
      </section>

      <section class="of-panel p-5">
        <div class="flex items-center justify-between">
          <div>
            <div class="of-heading-sm">Saved explorations</div>
            <div class="mt-1 text-sm text-[var(--text-muted)]">Reusable search states and quick-return lists.</div>
          </div>
          <span class="of-chip">{savedExplorations.length}</span>
        </div>

        <div class="mt-4 space-y-3">
          {#if savedExplorations.length === 0}
            <div class="rounded-[16px] border border-dashed border-[var(--border-default)] px-3 py-8 text-center text-sm text-[var(--text-muted)]">
              Save an exploration to create a walk-up entry point for recurring questions.
            </div>
          {:else}
            {#each savedExplorations as item (item.id)}
              <article class="rounded-[16px] border border-[var(--border-subtle)] p-3">
                <div class="flex items-start justify-between gap-3">
                  <div class="min-w-0">
                    <div class="truncate text-sm font-medium text-[var(--text-strong)]">{item.name}</div>
                    <div class="mt-1 text-xs text-[var(--text-muted)]">{item.query}</div>
                    <div class="mt-1 text-xs text-[var(--text-soft)]">{formatDate(item.createdAt)}</div>
                  </div>
                  <button type="button" class="text-xs font-medium text-rose-600 hover:text-rose-700" onclick={() => deleteSavedExploration(item.id)}>
                    Delete
                  </button>
                </div>
                <div class="mt-3 flex gap-2">
                  <button type="button" class="of-btn px-3 py-1.5 text-xs" onclick={() => openSavedExploration(item)}>
                    Open
                  </button>
                  <button type="button" class="of-btn px-3 py-1.5 text-xs" onclick={copyShareLink}>
                    Copy URL
                  </button>
                </div>
              </article>
            {/each}
          {/if}
        </div>
      </section>
    </aside>
  </section>
</div>
