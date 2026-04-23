<script lang="ts">
  import { executeQuery, explainQuery, createSavedQuery, listSavedQueries, deleteSavedQuery, type QueryResult, type SavedQuery } from '$lib/api/queries';

  let sql = $state('SELECT 1 as test');
  let result = $state<QueryResult | null>(null);
  let error = $state('');
  let executing = $state(false);
  let savedQueries = $state<SavedQuery[]>([]);
  let activeTab = $state<'results' | 'saved'>('results');
  let showSaveDialog = $state(false);
  let saveName = $state('');

  async function handleExecute() {
    error = '';
    result = null;
    executing = true;
    try {
      result = await executeQuery(sql, 1000);
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
        rows: [
          [plan.logical_plan],
          ['---'],
          [plan.physical_plan],
        ],
        total_rows: 3,
        execution_time_ms: 0,
      };
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
    } catch { /* ignore */ }
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

  $effect(() => {
    loadSaved();
  });
</script>

<div class="h-full flex flex-col gap-4">
  <div class="flex items-center justify-between">
    <h1 class="text-2xl font-bold">SQL Workbench</h1>
    <div class="flex gap-2">
      <button onclick={handleExecute} disabled={executing || !sql.trim()}
        class="px-4 py-2 bg-green-600 text-white rounded hover:bg-green-700 disabled:opacity-50">
        {executing ? 'Running...' : 'Run'} <kbd class="ml-1 text-xs opacity-70">⌘↵</kbd>
      </button>
      <button onclick={handleExplain} disabled={executing || !sql.trim()}
        class="px-4 py-2 border rounded hover:bg-gray-50 dark:hover:bg-gray-800">
        Explain
      </button>
      <button onclick={() => { showSaveDialog = true; }}
        class="px-4 py-2 border rounded hover:bg-gray-50 dark:hover:bg-gray-800">
        Save
      </button>
    </div>
  </div>

  <!-- SQL Editor -->
  <div class="border rounded dark:border-gray-700">
    <textarea
      bind:value={sql}
      onkeydown={handleKeydown}
      rows="8"
      placeholder="Enter SQL query..."
      spellcheck="false"
      class="w-full p-4 font-mono text-sm bg-gray-50 dark:bg-gray-900 resize-y rounded focus:outline-none"
    ></textarea>
  </div>

  {#if showSaveDialog}
    <div class="flex gap-2 items-center">
      <input type="text" bind:value={saveName} placeholder="Query name..."
        class="flex-1 px-3 py-2 border rounded dark:bg-gray-800 dark:border-gray-700" />
      <button onclick={handleSave} class="px-4 py-2 bg-blue-600 text-white rounded">Save</button>
      <button onclick={() => showSaveDialog = false} class="px-4 py-2 border rounded">Cancel</button>
    </div>
  {/if}

  {#if error}
    <div class="bg-red-50 dark:bg-red-900/20 text-red-600 px-4 py-3 rounded font-mono text-sm">{error}</div>
  {/if}

  <!-- Tabs -->
  <div class="border-b dark:border-gray-700">
    <nav class="flex gap-4">
      <button
        class="pb-2 px-1 text-sm font-medium border-b-2 transition-colors"
        class:border-blue-600={activeTab === 'results'}
        class:text-blue-600={activeTab === 'results'}
        class:border-transparent={activeTab !== 'results'}
        onclick={() => activeTab = 'results'}
      >Results</button>
      <button
        class="pb-2 px-1 text-sm font-medium border-b-2 transition-colors"
        class:border-blue-600={activeTab === 'saved'}
        class:text-blue-600={activeTab === 'saved'}
        class:border-transparent={activeTab !== 'saved'}
        onclick={() => activeTab = 'saved'}
      >Saved Queries ({savedQueries.length})</button>
    </nav>
  </div>

  <!-- Results -->
  {#if activeTab === 'results'}
    {#if result}
      <div class="text-sm text-gray-500 mb-1">
        {result.total_rows} rows in {result.execution_time_ms}ms
      </div>
      <div class="overflow-auto border rounded dark:border-gray-700 flex-1">
        <table class="w-full text-sm">
          <thead class="bg-gray-50 dark:bg-gray-800 sticky top-0">
            <tr>
              {#each result.columns as col}
                <th class="px-3 py-2 text-left font-medium border-b dark:border-gray-700">
                  {col.name}
                  <span class="text-xs text-gray-400 font-normal ml-1">{col.data_type}</span>
                </th>
              {/each}
            </tr>
          </thead>
          <tbody>
            {#each result.rows as row, i}
              <tr class:bg-gray-50={i % 2 === 0} class:dark:bg-gray-900={i % 2 === 0}>
                {#each row as cell}
                  <td class="px-3 py-1.5 border-b dark:border-gray-800 font-mono">{cell}</td>
                {/each}
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {:else if !executing}
      <div class="flex-1 flex items-center justify-center text-gray-500">
        Run a query to see results
      </div>
    {/if}
  {:else}
    <div class="space-y-2 flex-1 overflow-auto">
      {#each savedQueries as q (q.id)}
        <div class="border rounded p-3 dark:border-gray-700 flex justify-between items-start">
          <button type="button" class="flex-1 cursor-pointer text-left" onclick={() => loadQuery(q)}>
            <div class="font-medium">{q.name}</div>
            <pre class="text-xs text-gray-500 mt-1 truncate">{q.sql}</pre>
          </button>
          <button onclick={() => handleDeleteSaved(q.id)} class="text-red-500 hover:text-red-700 text-sm ml-2">Delete</button>
        </div>
      {/each}
      {#if savedQueries.length === 0}
        <div class="text-center py-8 text-gray-500">No saved queries yet</div>
      {/if}
    </div>
  {/if}
</div>
