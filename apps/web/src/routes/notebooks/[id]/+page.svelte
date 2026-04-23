<script lang="ts">
  import { onMount } from 'svelte';
  import { page as pageStore } from '$app/stores';
  import CellEditor from '$components/notebook/CellEditor.svelte';
  import CellOutput from '$components/notebook/CellOutput.svelte';
  import KernelSelector from '$components/notebook/KernelSelector.svelte';
  import {
    getNotebook,
    updateNotebook,
    addCell,
    updateCell,
    deleteCell,
    executeCell,
    createSession,
    listSessions,
    stopSession,
    type Notebook,
    type Cell,
    type CellOutput as NotebookCellOutput,
    type Session,
  } from '$lib/api/notebooks';

  const notebookId = $derived($pageStore.params.id ?? '');

  let notebook = $state<Notebook | null>(null);
  let cells = $state<Cell[]>([]);
  let loading = $state(true);
  let executing = $state<Record<string, boolean>>({});
  let outputs = $state<Record<string, NotebookCellOutput>>({});
  let sessionsByKernel = $state<Record<string, Session | null>>({ python: null, sql: null });
  let activeKernel = $state<'python' | 'sql'>('python');

  function kernelKey(kernel: string): 'python' | 'sql' {
    return kernel === 'sql' ? 'sql' : 'python';
  }

  function upsertCell(nextCell: Cell) {
    cells = cells
      .map((cell) => cell.id === nextCell.id ? nextCell : cell)
      .sort((left, right) => left.position - right.position);
  }

  function updateSession(kernel: string, session: Session | null) {
    sessionsByKernel = { ...sessionsByKernel, [kernelKey(kernel)]: session };
  }

  async function loadSessionsForNotebook() {
    const res = await listSessions(notebookId);
    const next: Record<string, Session | null> = { python: null, sql: null };

    for (const session of res.data) {
      const key = kernelKey(session.kernel);
      if (!next[key]) {
        next[key] = session;
      }
    }

    sessionsByKernel = next;
  }

  async function load() {
    loading = true;
    try {
      const res = await getNotebook(notebookId);
      notebook = res.notebook;
      cells = res.cells.sort((left, right) => left.position - right.position);
      outputs = {};

      for (const cell of res.cells) {
        if (cell.last_output) {
          outputs[cell.id] = cell.last_output;
        }
      }

      activeKernel = kernelKey(res.notebook.default_kernel);
      await loadSessionsForNotebook();
    } catch {
      notebook = null;
    }
    loading = false;
  }

  async function ensureSession(kernel: string): Promise<Session> {
    const key = kernelKey(kernel);
    const existing = sessionsByKernel[key];

    if (existing && existing.status !== 'dead') {
      return existing;
    }

    const session = await createSession(notebookId, key);
    updateSession(key, session);
    return session;
  }

  async function handleKernelChange(kernel: 'python' | 'sql') {
    activeKernel = kernel;
    if (!notebook || notebook.default_kernel === kernel) {
      return;
    }

    notebook = await updateNotebook(notebookId, { default_kernel: kernel });
  }

  async function handleStartSession() {
    await ensureSession(activeKernel);
  }

  async function handleStopSession() {
    const current = sessionsByKernel[activeKernel];
    if (!current) {
      return;
    }

    const stopped = await stopSession(notebookId, current.id);
    updateSession(activeKernel, stopped);
  }

  async function handleAddCell(type: string) {
    const cell = await addCell(notebookId, {
      cell_type: type,
      kernel: type === 'code' ? activeKernel : undefined,
      source: '',
    });
    cells = [...cells, cell].sort((left, right) => left.position - right.position);
  }

  function handleSourceChange(cellId: string, source: string) {
    cells = cells.map((cell) => cell.id === cellId ? { ...cell, source } : cell);
  }

  async function handlePersistSource(cellId: string, source: string) {
    const updated = await updateCell(notebookId, cellId, { source });
    upsertCell(updated);
  }

  async function handleCellKernelChange(cellId: string, kernel: 'python' | 'sql') {
    const updated = await updateCell(notebookId, cellId, { kernel });
    upsertCell(updated);
  }

  async function handleDeleteCell(cellId: string) {
    await deleteCell(notebookId, cellId);
    cells = cells.filter((cell) => cell.id !== cellId);
  }

  async function handleExecute(cellId: string) {
    const cell = cells.find((entry) => entry.id === cellId);
    if (!cell || cell.cell_type !== 'code') {
      return;
    }

    const key = kernelKey(cell.kernel);
    executing = { ...executing, [cellId]: true };

    try {
      const session = await ensureSession(cell.kernel);
      updateSession(key, { ...session, status: 'busy' });

      const output = await executeCell(notebookId, cellId, session.id);
      outputs = { ...outputs, [cellId]: output };
      cells = cells.map((entry) => entry.id === cellId
        ? { ...entry, execution_count: output.execution_count, last_output: output }
        : entry);
      updateSession(key, {
        ...(sessionsByKernel[key] ?? session),
        status: 'idle',
        last_activity: new Date().toISOString(),
      });
    } catch (error: any) {
      outputs = {
        ...outputs,
        [cellId]: {
          output_type: 'error',
          content: error?.message ?? 'Execution failed',
          execution_count: cell.execution_count ?? 0,
        },
      };

      const session = sessionsByKernel[key];
      if (session) {
        updateSession(key, { ...session, status: 'idle' });
      }
    }

    executing = { ...executing, [cellId]: false };
  }

  async function handleRunAll() {
    for (const cell of cells) {
      if (cell.cell_type === 'code') {
        await handleExecute(cell.id);
      }
    }
  }

  onMount(() => {
    void load();
  });
</script>

{#if loading}
  <div class="p-6 text-gray-500">Loading...</div>
{:else if !notebook}
  <div class="p-6 text-red-500">Notebook not found.</div>
{:else}
  <div class="mx-auto max-w-6xl p-6">
    <div class="mb-6 flex items-center justify-between gap-4">
      <div>
        <h1 class="text-2xl font-bold">{notebook.name}</h1>
        <p class="text-sm text-gray-500">{notebook.description || 'No description'}</p>
      </div>
      <div class="flex gap-2">
        <button class="rounded bg-green-600 px-3 py-1 text-sm text-white hover:bg-green-700" onclick={handleRunAll}>
          ▶ Run All
        </button>
        <a href="/notebooks" class="rounded border px-3 py-1 text-sm hover:bg-gray-50">Back</a>
      </div>
    </div>

    <div class="mb-6">
      <KernelSelector
        value={activeKernel}
        status={sessionsByKernel[activeKernel]?.status ?? null}
        onChange={handleKernelChange}
        onStart={handleStartSession}
        onStop={handleStopSession}
      />
    </div>

    <div class="space-y-4">
      {#each cells as cell (cell.id)}
        <div class="overflow-hidden rounded-2xl border border-slate-200 bg-white shadow-sm dark:border-slate-800 dark:bg-slate-900">
          <div class="flex items-center gap-2 border-b border-slate-200 bg-slate-50 px-3 py-2 text-sm dark:border-slate-800 dark:bg-slate-950/60">
            <span class="text-gray-400">In [{cell.execution_count ?? ' '}]</span>
            <span class="text-xs text-gray-400">{cell.cell_type}</span>

            {#if cell.cell_type === 'code'}
              <select
                class="rounded-md border border-slate-300 bg-white px-2 py-1 text-xs font-mono dark:border-slate-700 dark:bg-slate-800"
                value={kernelKey(cell.kernel)}
                onchange={(event) => handleCellKernelChange(cell.id, (event.currentTarget as HTMLSelectElement).value as 'python' | 'sql')}
              >
                <option value="python">python</option>
                <option value="sql">sql</option>
              </select>
            {:else}
              <span class="rounded-md bg-slate-200 px-2 py-1 text-xs font-mono text-slate-600 dark:bg-slate-800 dark:text-slate-300">markdown</span>
            {/if}

            <div class="flex-1"></div>

            {#if cell.cell_type === 'code'}
              <button
                class="text-green-600 hover:text-green-800"
                disabled={executing[cell.id]}
                onclick={() => handleExecute(cell.id)}
              >
                {executing[cell.id] ? '⏳' : '▶'}
              </button>
            {/if}

            <button class="text-red-400 hover:text-red-600" onclick={() => handleDeleteCell(cell.id)}>✕</button>
          </div>

          <CellEditor
            value={cell.source}
            language={cell.cell_type === 'markdown' ? 'markdown' : kernelKey(cell.kernel)}
            minHeight={cell.cell_type === 'markdown' ? 120 : 180}
            onChange={(source) => handleSourceChange(cell.id, source)}
            onBlur={(source) => handlePersistSource(cell.id, source)}
          />

          <CellOutput output={outputs[cell.id] ?? cell.last_output} />
        </div>
      {/each}
    </div>

    <div class="mt-4 flex gap-2">
      <button class="rounded border px-3 py-1 text-sm hover:bg-gray-50" onclick={() => handleAddCell('code')}>
        + Code Cell
      </button>
      <button class="rounded border px-3 py-1 text-sm hover:bg-gray-50" onclick={() => handleAddCell('markdown')}>
        + Markdown Cell
      </button>
    </div>
  </div>
{/if}
