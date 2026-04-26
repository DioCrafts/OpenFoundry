<script lang="ts">
  import Glyph from '$components/ui/Glyph.svelte';
  import {
    deleteObjectType,
    listObjectTypes,
    searchOntology,
    type ObjectType,
    type SearchResult
  } from '$lib/api/ontology';

  let types = $state<ObjectType[]>([]);
  let total = $state(0);
  let page = $state(1);
  let search = $state('');
  let loading = $state(true);

  let semanticQuery = $state('');
  let semanticKind = $state('all');
  let semanticLoading = $state(false);
  let semanticError = $state('');
  let searchResults = $state<SearchResult[]>([]);
  let activeTab = $state<'overview' | 'objects' | 'types' | 'artifacts'>('overview');

  const demoArtifacts = [
    { name: 'Passenger Exploration', type: 'Exploration', icon: 'artifact' as const },
    { name: 'Airport Ops List', type: 'List', icon: 'list' as const }
  ];

  async function load() {
    loading = true;
    try {
      const res = await listObjectTypes({ page, per_page: 24, search: search || undefined });
      types = res.data;
      total = res.total;
    } catch (error) {
      console.error('Failed to load object types', error);
    } finally {
      loading = false;
    }
  }

  async function runSemanticSearch() {
    const query = semanticQuery.trim();
    if (!query) {
      semanticError = '';
      searchResults = [];
      return;
    }

    semanticLoading = true;
    semanticError = '';
    try {
      const response = await searchOntology({
        query,
        kind: semanticKind === 'all' ? undefined : semanticKind,
        limit: 24,
        semantic: true
      });
      searchResults = response.data;
      activeTab = 'overview';
    } catch (error) {
      semanticError = error instanceof Error ? error.message : 'Failed to run ontology search';
    } finally {
      semanticLoading = false;
    }
  }

  async function handleDelete(id: string) {
    if (!confirm('Delete this object type and all its instances?')) return;
    await deleteObjectType(id);
    await load();
  }

  function resultBadgeClass(kind: string) {
    switch (kind) {
      case 'object_instance':
        return 'of-status-success';
      case 'action_type':
        return 'of-status-info';
      case 'interface':
        return 'bg-[#e7f5f3] text-[#0f766e]';
      case 'link_type':
        return 'of-status-warning';
      default:
        return 'bg-[#eef2f7] text-[var(--text-muted)]';
    }
  }

  function objectTypeGlyph(typeItem: ObjectType) {
    const key = (typeItem.name + typeItem.display_name).toLowerCase();
    if (key.includes('route') || key.includes('link')) return 'link';
    if (key.includes('customer') || key.includes('employee') || key.includes('person')) return 'object';
    if (key.includes('dataset') || key.includes('table')) return 'database';
    return 'cube';
  }

  function shouldShowGroup(group: string) {
    if (activeTab === 'overview') return true;
    if (activeTab === 'objects') return group === 'object instance';
    if (activeTab === 'types') return group === 'object type';
    return group !== 'object instance' && group !== 'object type';
  }

  const groupedResults = $derived.by(() => {
    const groups = new Map<string, SearchResult[]>();
    for (const result of searchResults) {
      const label = result.kind.replaceAll('_', ' ');
      if (!groups.has(label)) groups.set(label, []);
      groups.get(label)!.push(result);
    }
    return Array.from(groups.entries());
  });

  const objectCount = $derived(searchResults.filter((item) => item.kind === 'object_instance').length);
  const typeCount = $derived(searchResults.filter((item) => item.kind === 'object_type').length);
  const artifactCount = $derived(
    searchResults.filter((item) => ['action_type', 'interface', 'link_type'].includes(item.kind)).length
  );

  $effect(() => {
    load();
  });
</script>

<div class="space-y-5">
  <section class="of-hero-strip">
    <div class="flex flex-wrap items-start justify-between gap-5">
      <div>
        <div class="of-heading-xl">Explore your data</div>
        <div class="mt-2 max-w-3xl text-[15px] text-[var(--text-muted)]">
          Select an object type, search across ontology entities, and jump into graph, schema or artifacts
          using the same enterprise visual language.
        </div>
      </div>
      <div class="flex gap-2">
        <a href="/ontology/graph" class="of-btn">
          <Glyph name="graph" size={16} />
          <span>Graph</span>
        </a>
        <a href="/ontology/types" class="of-btn of-btn-primary">
          <Glyph name="plus" size={16} />
          <span>Create type</span>
        </a>
      </div>
    </div>

    <form
      class="mt-5 space-y-4"
      onsubmit={(event) => {
        event.preventDefault();
        runSemanticSearch();
      }}
    >
      <div class="of-search-shell">
        <button type="button" class="of-search-filter">
          <span>{semanticKind === 'all' ? 'All' : semanticKind.replaceAll('_', ' ')}</span>
          <Glyph name="chevron-down" size={14} />
        </button>
        <div class="of-search-input-wrap">
          <Glyph name="search" size={18} />
          <input
            bind:value={semanticQuery}
            class="of-search-input"
            placeholder="Search object types and properties..."
          />
        </div>
        <button type="button" class="px-4 text-sm text-[var(--text-muted)]" onclick={() => {
          semanticQuery = '';
          searchResults = [];
          semanticError = '';
        }}>
          Clear
        </button>
      </div>

      <div class="of-tabbar">
        <button type="button" class={`of-tab ${activeTab === 'overview' ? 'of-tab-active' : ''}`} onclick={() => activeTab = 'overview'}>
          All <span class="of-badge ml-2">{searchResults.length || total}</span>
        </button>
        <button type="button" class={`of-tab ${activeTab === 'objects' ? 'of-tab-active' : ''}`} onclick={() => activeTab = 'objects'}>
          Objects <span class="of-badge ml-2">{objectCount}</span>
        </button>
        <button type="button" class={`of-tab ${activeTab === 'types' ? 'of-tab-active' : ''}`} onclick={() => activeTab = 'types'}>
          Object types <span class="of-badge ml-2">{typeCount || total}</span>
        </button>
        <button type="button" class={`of-tab ${activeTab === 'artifacts' ? 'of-tab-active' : ''}`} onclick={() => activeTab = 'artifacts'}>
          Artifacts <span class="of-badge ml-2">{artifactCount || demoArtifacts.length}</span>
        </button>
      </div>
    </form>
  </section>

  {#if semanticError}
    <div class="of-inline-note">{semanticError}</div>
  {/if}

  <section class="of-content-grid of-content-grid-2">
    <aside class="of-panel overflow-hidden">
      <div class="border-b border-[var(--border-subtle)] bg-[#f7faff] px-4 py-3">
        <div class="flex items-center justify-between">
          <div class="text-[15px] font-semibold text-[var(--text-strong)]">All results</div>
          <span class="of-badge">{searchResults.length || total}</span>
        </div>
      </div>

      <div class="space-y-5 p-4">
        <div>
          <div class="of-heading-sm">Object type filters</div>
          <div class="mt-3 space-y-2">
            {#each types.slice(0, 4) as typeItem (typeItem.id)}
              <button
                type="button"
                class="flex w-full items-center justify-between rounded-[4px] border border-transparent px-2 py-2 text-left hover:bg-[var(--bg-hover)]"
                onclick={() => {
                  search = typeItem.display_name;
                  page = 1;
                  load();
                }}
              >
                <span class="flex items-center gap-2 text-sm text-[var(--text-default)]">
                  <span class="of-icon-box h-6 w-6 bg-[#efe5d4] text-[#9a6c2f]">
                    <Glyph name={objectTypeGlyph(typeItem)} size={13} />
                  </span>
                  <span class="truncate">{typeItem.display_name}</span>
                </span>
                <span class="of-badge">1</span>
              </button>
            {/each}
          </div>
        </div>

        <div class="of-divider"></div>

        <div>
          <div class="of-heading-sm">Object type groups</div>
          <div class="mt-3 space-y-2 text-sm">
            <div class="flex items-center justify-between rounded-[4px] px-2 py-2 hover:bg-[var(--bg-hover)]">
              <span>Favorites</span>
              <span class="of-badge">{Math.min(6, total)}</span>
            </div>
            <div class="flex items-center justify-between rounded-[4px] px-2 py-2 hover:bg-[var(--bg-hover)]">
              <span>Operations</span>
              <span class="of-badge">{Math.min(4, total)}</span>
            </div>
          </div>
        </div>

        <div class="of-divider"></div>

        <div>
          <div class="of-heading-sm">Artifacts</div>
          <div class="mt-3 space-y-2 text-sm">
            <div class="flex items-center justify-between rounded-[4px] px-2 py-2 hover:bg-[var(--bg-hover)]">
              <span>Explorations &amp; Lists</span>
              <span class="of-badge">{demoArtifacts.length}</span>
            </div>
            <div class="flex items-center justify-between rounded-[4px] px-2 py-2 text-[var(--text-soft)]">
              <span>Modules</span>
              <span class="of-badge">0</span>
            </div>
          </div>
        </div>
      </div>
    </aside>

    <div class="space-y-4">
      {#if activeTab === 'artifacts'}
        <section class="of-panel px-5 py-4">
          <div class="flex items-center justify-between border-b border-[var(--border-subtle)] pb-3">
            <div class="of-kicker">Artifacts</div>
            <a href="/ontology/object-sets" class="of-link">View all artifacts</a>
          </div>
          <div class="mt-4 grid gap-4 md:grid-cols-2">
            {#each demoArtifacts as artifact}
              <article class="of-card">
                <div class="flex items-center gap-3">
                  <span class="of-icon-box h-10 w-10 bg-[#f0e7ff] text-[#7c4ad8]">
                    <Glyph name={artifact.icon} size={18} />
                  </span>
                  <div>
                    <div class="text-[15px] font-medium text-[var(--text-strong)]">{artifact.name}</div>
                    <div class="text-xs text-[var(--text-muted)]">{artifact.type}</div>
                  </div>
                </div>
              </article>
            {/each}
          </div>
        </section>
      {:else if semanticQuery.trim() && searchResults.length > 0}
        {#each groupedResults as [group, items]}
          {#if shouldShowGroup(group)}
            <section class="of-panel px-5 py-4">
              <div class="flex items-center justify-between border-b border-[var(--border-subtle)] pb-3">
                <div class="of-kicker">{group}</div>
                <a href="/ontology" class="of-link">Explore all</a>
              </div>
              <div class="mt-4 space-y-3">
                {#each items as result (result.kind + ':' + result.id)}
                  <article class="rounded-[6px] border border-[var(--border-default)] bg-[#fbfcfe] px-4 py-3">
                    <div class="flex items-start justify-between gap-4">
                      <div class="min-w-0">
                        <div class="flex flex-wrap items-center gap-2">
                          <span class={`of-chip ${resultBadgeClass(result.kind)}`}>{result.kind.replaceAll('_', ' ')}</span>
                          <span class="text-xs text-[var(--text-muted)]">score {result.score.toFixed(2)}</span>
                        </div>
                        <div class="mt-2 text-[15px] font-medium text-[var(--text-strong)]">{result.title}</div>
                        {#if result.subtitle}
                          <div class="mt-1 text-sm text-[var(--text-muted)]">{result.subtitle}</div>
                        {/if}
                      </div>
                      <a href={result.route} class="of-btn text-[13px]">Open</a>
                    </div>
                  </article>
                {/each}
              </div>
            </section>
          {/if}
        {/each}
      {:else}
        {#if activeTab === 'overview' || activeTab === 'types'}
          <section class="of-panel px-5 py-4">
            <div class="flex items-center justify-between border-b border-[var(--border-subtle)] pb-3">
              <div class="of-kicker">Object types</div>
              <a href="/ontology/types" class="of-link">Create new object type</a>
            </div>

            {#if loading}
              <div class="px-2 py-12 text-center text-sm text-[var(--text-muted)]">Loading ontology types...</div>
            {:else if types.length === 0}
              <div class="px-2 py-12 text-center text-sm text-[var(--text-muted)]">
                No object types found.
              </div>
            {:else}
              <div class="mt-4 grid gap-4 xl:grid-cols-2">
                {#each types as typeItem (typeItem.id)}
                  <article class="of-card">
                    <div class="flex items-start justify-between gap-3">
                      <div class="flex min-w-0 items-start gap-3">
                        <span
                          class="flex h-11 w-11 shrink-0 items-center justify-center rounded-[6px] text-white"
                          style={`background:${typeItem.color || '#4d8cf0'}`}
                        >
                          <Glyph name={objectTypeGlyph(typeItem)} size={18} />
                        </span>
                        <div class="min-w-0">
                          <div class="flex items-center gap-2">
                            <h3 class="truncate text-[15px] font-medium text-[var(--text-strong)]">
                              {typeItem.display_name}
                            </h3>
                            <span class="of-badge">1</span>
                          </div>
                          <div class="mt-1 truncate font-mono text-xs text-[var(--text-muted)]">{typeItem.name}</div>
                          <div class="mt-2 text-sm text-[var(--text-muted)]">
                            {typeItem.description || 'Semantic object type available for exploration and graph traversal.'}
                          </div>
                        </div>
                      </div>
                      <button type="button" class="text-sm text-[#b42318]" onclick={() => handleDelete(typeItem.id)}>
                        Delete
                      </button>
                    </div>
                    <div class="flex flex-wrap gap-2 pt-1">
                      <a href="/ontology/{typeItem.id}" class="of-link text-sm">Open detail</a>
                      <span class="text-[var(--text-soft)]">•</span>
                      <a href="/ontology/graph?root_type_id={typeItem.id}" class="of-link text-sm">Explore graph</a>
                    </div>
                  </article>
                {/each}
              </div>
            {/if}
          </section>
        {/if}

        {#if activeTab === 'overview' || activeTab === 'objects'}
          <section class="of-panel px-5 py-4">
            <div class="flex items-center justify-between border-b border-[var(--border-subtle)] pb-3">
              <div class="of-kicker">Objects</div>
              <a href="/ontology/object-sets" class="of-link">View all objects</a>
            </div>
            <div class="mt-4 rounded-[6px] border border-[var(--border-default)] bg-[#fbfcfe] p-4">
              <div class="flex items-start gap-3">
                <span class="of-icon-box h-10 w-10 bg-[#efe5d4] text-[#9a6c2f]">
                  <Glyph name="link" size={17} />
                </span>
                <div>
                  <div class="text-[15px] font-medium text-[var(--text-strong)]">Operational runway graph</div>
                  <div class="mt-1 text-sm text-[var(--text-muted)]">
                    Example object section used as a landing area before users pivot into linked object sets and
                    object views.
                  </div>
                </div>
              </div>
              <div class="mt-4 grid gap-3">
                {#each types.slice(0, 2) as item (item.id)}
                  <div class="rounded-[4px] border border-[var(--border-subtle)] bg-white px-4 py-3">
                    <div class="text-sm font-medium text-[var(--text-strong)]">{item.display_name}</div>
                    <div class="mt-1 text-xs text-[var(--text-muted)]">{item.description || item.name}</div>
                  </div>
                {/each}
              </div>
            </div>
          </section>
        {/if}
      {/if}
    </div>
  </section>
</div>
