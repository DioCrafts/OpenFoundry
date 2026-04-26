<script lang="ts">
  import Glyph from '$components/ui/Glyph.svelte';
  import {
    executeQuery,
    explainQuery,
    createSavedQuery,
    listSavedQueries,
    deleteSavedQuery,
    type QueryResult,
    type SavedQuery
  } from '$lib/api/queries';
  import { listObjectTypes, listProperties, type ObjectType, type Property } from '$lib/api/ontology';

  let sql = $state('SELECT *\nFROM `[Example]`');
  let result = $state<QueryResult | null>(null);
  let error = $state('');
  let executing = $state(false);
  let savedQueries = $state<SavedQuery[]>([]);
  let activeTab = $state<'results' | 'saved'>('results');
  let showSaveDialog = $state(false);
  let saveName = $state('');

  let objectTypes = $state<ObjectType[]>([]);
  let selectedTypeId = $state('');
  let selectedType = $state<ObjectType | null>(null);
  let properties = $state<Property[]>([]);
  let objectFilter = $state('');
  let propertyFilter = $state('');
  let loadingCatalog = $state(true);

  async function handleExecute() {
    error = '';
    result = null;
    executing = true;
    try {
      result = await executeQuery(sql, 1000);
      activeTab = 'results';
    } catch (e: any) {
      error = e.message || 'Query failed';
    } finally {
      executing = false;
    }
  }

  async function handleExplain() {
    error = '';
    executing = true;
    try {
      const plan = await explainQuery(sql);
      result = {
        columns: [{ name: 'plan', data_type: 'Utf8' }],
        rows: [[plan.logical_plan], ['---'], [plan.physical_plan]],
        total_rows: 3,
        execution_time_ms: 0
      };
      activeTab = 'results';
    } catch (e: any) {
      error = e.message || 'Explain failed';
    } finally {
      executing = false;
    }
  }

  async function handleSave() {
    if (!saveName.trim()) return;
    try {
      await createSavedQuery({ name: saveName, sql });
      showSaveDialog = false;
      saveName = '';
      await loadSaved();
    } catch (e: any) {
      error = e.message || 'Save failed';
    }
  }

  async function handleDeleteSaved(id: string) {
    await deleteSavedQuery(id);
    await loadSaved();
  }

  async function loadSaved() {
    try {
      const res = await listSavedQueries();
      savedQueries = res.data;
    } catch {
      // ignore
    }
  }

  async function loadCatalog() {
    loadingCatalog = true;
    try {
      const res = await listObjectTypes({ per_page: 100 });
      objectTypes = res.data;
      if (!selectedTypeId && res.data.length > 0) {
        selectedTypeId = res.data[0].id;
      }
    } finally {
      loadingCatalog = false;
    }
  }

  async function loadSelectedType(typeId: string) {
    selectedType = objectTypes.find((item) => item.id === typeId) ?? null;
    if (!typeId) {
      properties = [];
      return;
    }
    try {
      properties = await listProperties(typeId);
    } catch {
      properties = [];
    }
  }

  function loadQuery(q: SavedQuery) {
    sql = q.sql;
    activeTab = 'results';
  }

  function handleKeydown(e: KeyboardEvent) {
    if ((e.metaKey || e.ctrlKey) && e.key === 'Enter') {
      e.preventDefault();
      handleExecute();
    }
  }

  function insertText(text: string) {
    sql = `${sql.trimEnd()}\n${text}`;
  }

  const filteredTypes = $derived.by(() => {
    const query = objectFilter.trim().toLowerCase();
    if (!query) return objectTypes;
    return objectTypes.filter((item) =>
      `${item.display_name} ${item.name} ${item.description}`.toLowerCase().includes(query)
    );
  });

  const filteredProperties = $derived.by(() => {
    const query = propertyFilter.trim().toLowerCase();
    if (!query) return properties;
    return properties.filter((item) =>
      `${item.display_name} ${item.name} ${item.property_type}`.toLowerCase().includes(query)
    );
  });

  $effect(() => {
    loadSaved();
    loadCatalog();
  });

  $effect(() => {
    if (selectedTypeId) {
      loadSelectedType(selectedTypeId);
    }
  });
</script>

<div class="space-y-5">
  <section class="of-panel overflow-hidden">
    <div class="flex items-center justify-between border-b border-[var(--border-subtle)] px-5 py-4">
      <div>
        <div class="of-heading-lg">SQL Preview</div>
        <div class="mt-1 text-sm text-[var(--text-muted)]">
          Query ontology-backed tables and inspect available object types without leaving the workbench.
        </div>
      </div>

      <div class="flex gap-2">
        <button class="of-btn" type="button" onclick={() => {
          sql = 'SELECT *\nFROM `[Example]`';
          result = null;
          error = '';
        }}>
          <Glyph name="history" size={16} />
          <span>Reset</span>
        </button>
        <button class="of-btn" type="button" onclick={() => showSaveDialog = true}>
          <Glyph name="bookmark" size={16} />
          <span>Save</span>
        </button>
        <button class="of-btn" type="button" onclick={handleExplain} disabled={executing || !sql.trim()}>
          <Glyph name="sparkles" size={16} />
          <span>Explain</span>
        </button>
        <button class="of-btn of-btn-primary" type="button" onclick={handleExecute} disabled={executing || !sql.trim()}>
          <Glyph name="run" size={15} />
          <span>{executing ? 'Running...' : 'Run'}</span>
        </button>
      </div>
    </div>

    <div class="grid gap-0 xl:grid-cols-[420px_minmax(0,1fr)]">
      <aside class="border-r border-[var(--border-subtle)] bg-[#fbfcfe]">
        <div class="border-b border-[var(--border-subtle)] px-4 py-3">
          <div class="flex flex-wrap gap-2">
            <button class="of-chip of-chip-active">All ontologies</button>
            <button class="of-chip">Group</button>
            <button class="of-chip">Status</button>
          </div>
        </div>

        <div class="grid min-h-[640px] grid-cols-[220px_minmax(0,1fr)]">
          <div class="border-r border-[var(--border-subtle)] p-3">
            <div class="mb-3 of-heading-sm">Recently used</div>
            <div class="space-y-1">
              {#if loadingCatalog}
                <div class="px-2 py-8 text-sm text-[var(--text-muted)]">Loading object types...</div>
              {:else}
                {#each filteredTypes as item (item.id)}
                  <button
                    type="button"
                    class={`flex w-full items-start gap-2 rounded-[4px] px-2 py-2 text-left ${
                      selectedTypeId === item.id ? 'bg-[#dce8fb]' : 'hover:bg-[var(--bg-hover)]'
                    }`}
                    onclick={() => selectedTypeId = item.id}
                  >
                    <span class="of-icon-box h-8 w-8 shrink-0 bg-[#e9f1ff] text-[var(--status-info)]">
                      <Glyph name="cube" size={15} />
                    </span>
                    <span class="min-w-0">
                      <span class="block truncate text-[14px] text-[var(--text-strong)]">{item.display_name}</span>
                      <span class="block truncate text-xs text-[var(--text-muted)]">{item.name}</span>
                    </span>
                  </button>
                {/each}
              {/if}
            </div>
          </div>

          <div class="p-4">
            {#if selectedType}
              <div class="flex items-start gap-4">
                <span class="of-icon-box h-11 w-11 shrink-0 bg-[#eaf1fe] text-[var(--status-info)]">
                  <Glyph name="cube" size={20} />
                </span>
                <div class="min-w-0">
                  <div class="text-[15px] font-semibold text-[var(--text-strong)]">{selectedType.display_name}</div>
                  <div class="text-sm text-[var(--text-muted)]">{selectedType.description || 'No description'}</div>
                  <div class="mt-1 text-sm text-[var(--text-muted)]">Ontology • {properties.length} properties</div>
                </div>
              </div>

              <div class="mt-4 rounded-[6px] border border-[var(--border-default)]">
                <div class="border-b border-[var(--border-subtle)] px-4 py-3">
                  <div class="of-heading-sm">Properties ({properties.length})</div>
                </div>
                <div class="max-h-[420px] overflow-auto">
                  {#each filteredProperties as property (property.id)}
                    <button
                      type="button"
                      class="flex w-full items-center justify-between border-b border-[var(--border-subtle)] px-4 py-3 text-left hover:bg-[var(--bg-hover)]"
                      onclick={() => insertText(`-- ${property.display_name}\nSELECT ${property.name}\nFROM \`${selectedType?.display_name ?? 'ObjectType'}\`;`)}
                    >
                      <span class="flex items-center gap-3">
                        <span class="rounded-[4px] border border-[var(--border-default)] bg-white px-1.5 py-0.5 text-[10px] font-semibold text-[var(--text-muted)]">
                          {property.property_type}
                        </span>
                        <span>
                          <span class="block text-[14px] text-[var(--text-strong)]">{property.display_name}</span>
                          <span class="block text-xs text-[var(--text-muted)]">{property.name}</span>
                        </span>
                      </span>
                      <Glyph name="plus" size={15} />
                    </button>
                  {/each}
                </div>
              </div>
            {:else}
              <div class="px-2 py-12 text-sm text-[var(--text-muted)]">Select an object type to inspect its properties.</div>
            {/if}
          </div>
        </div>
      </aside>

      <div class="min-w-0">
        <div class="border-b border-[var(--border-subtle)] px-5 py-3">
          <div class="flex items-center justify-between">
            <div class="of-tabbar border-b-0">
              <button type="button" class={`of-tab ${activeTab === 'results' ? 'of-tab-active' : ''}`} onclick={() => activeTab = 'results'}>
                Code
              </button>
              <button type="button" class={`of-tab ${activeTab === 'saved' ? 'of-tab-active' : ''}`} onclick={() => activeTab = 'saved'}>
                History <span class="of-badge ml-2">{savedQueries.length}</span>
              </button>
            </div>
            <div class="text-xs text-[var(--text-muted)]">Run with ⌘/Ctrl + Enter</div>
          </div>
        </div>

        <div class="p-5">
          <div class="rounded-[6px] border border-[var(--border-default)] bg-white">
            <textarea
              bind:value={sql}
              onkeydown={handleKeydown}
              rows="10"
              placeholder="Enter SQL query..."
              spellcheck="false"
              class="of-textarea min-h-[220px] border-0 bg-[#fbfcfe] font-mono text-[16px] leading-8"
            ></textarea>
          </div>

          {#if showSaveDialog}
            <div class="mt-4 flex gap-2 rounded-[6px] border border-[var(--border-default)] bg-[#fbfcfe] p-3">
              <input
                type="text"
                bind:value={saveName}
                placeholder="Query name..."
                class="of-input flex-1"
              />
              <button class="of-btn of-btn-primary" type="button" onclick={handleSave}>Save</button>
              <button class="of-btn" type="button" onclick={() => showSaveDialog = false}>Cancel</button>
            </div>
          {/if}

          {#if error}
            <div class="mt-4 rounded-[6px] border border-[#efc1c1] bg-[#fff3f3] px-4 py-3 font-mono text-sm text-[var(--status-danger)]">
              {error}
            </div>
          {/if}

          {#if activeTab === 'results'}
            {#if result}
              <div class="mt-4 flex items-center justify-between text-sm text-[var(--text-muted)]">
                <span>{result.total_rows} rows in {result.execution_time_ms}ms</span>
                <span>{result.columns.length} columns</span>
              </div>

              <div class="mt-3 overflow-auto rounded-[6px] border border-[var(--border-default)]">
                <table class="of-table">
                  <thead>
                    <tr>
                      {#each result.columns as col}
                        <th>
                          {col.name}
                          <span class="ml-1 font-normal text-[10px] normal-case text-[var(--text-soft)]">
                            {col.data_type}
                          </span>
                        </th>
                      {/each}
                    </tr>
                  </thead>
                  <tbody>
                    {#each result.rows as row}
                      <tr>
                        {#each row as cell}
                          <td class="font-mono text-[13px]">{cell}</td>
                        {/each}
                      </tr>
                    {/each}
                  </tbody>
                </table>
              </div>
            {:else if !executing}
              <div class="mt-5 rounded-[6px] border border-dashed border-[var(--border-default)] px-4 py-12 text-center text-sm text-[var(--text-muted)]">
                Run a query to see results.
              </div>
            {/if}
          {:else}
            <div class="mt-4 space-y-2">
              {#each savedQueries as q (q.id)}
                <div class="rounded-[6px] border border-[var(--border-default)] bg-white p-3">
                  <div class="flex items-start justify-between gap-3">
                    <button type="button" class="min-w-0 flex-1 text-left" onclick={() => loadQuery(q)}>
                      <div class="text-[14px] font-medium text-[var(--text-strong)]">{q.name}</div>
                      <pre class="mt-1 truncate text-xs text-[var(--text-muted)]">{q.sql}</pre>
                    </button>
                    <button type="button" class="text-sm text-[var(--status-danger)]" onclick={() => handleDeleteSaved(q.id)}>
                      Delete
                    </button>
                  </div>
                </div>
              {/each}
              {#if savedQueries.length === 0}
                <div class="rounded-[6px] border border-dashed border-[var(--border-default)] px-4 py-12 text-center text-sm text-[var(--text-muted)]">
                  No saved queries yet.
                </div>
              {/if}
            </div>
          {/if}
        </div>
      </div>
    </div>
  </section>
</div>
