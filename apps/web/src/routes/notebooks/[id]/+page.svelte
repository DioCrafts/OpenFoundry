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
    listWorkspaceFiles,
    upsertWorkspaceFile,
    deleteWorkspaceFile,
    type Notebook,
    type Cell,
    type CellOutput as NotebookCellOutput,
    type Session,
    type NotebookKernel,
    type NotebookWorkspaceFile,
  } from '$lib/api/notebooks';

  const notebookId = $derived($pageStore.params.id ?? '');

  let notebook = $state<Notebook | null>(null);
  let cells = $state<Cell[]>([]);
  let loading = $state(true);
  let loadingWorkspace = $state(true);
  let executing = $state<Record<string, boolean>>({});
  let outputs = $state<Record<string, NotebookCellOutput>>({});
  let sessionsByKernel = $state<Record<NotebookKernel, Session | null>>({
    python: null,
    sql: null,
    llm: null,
    r: null,
  });
  let activeKernel = $state<NotebookKernel>('python');
  let error = $state('');
  let workspaceFiles = $state<NotebookWorkspaceFile[]>([]);
  let selectedWorkspaceFilePath = $state('');
  let newWorkspaceFilePath = $state('');
  let savingWorkspaceFile = $state<Record<string, boolean>>({});

  function kernelKey(kernel: string): NotebookKernel {
    if (kernel === 'sql' || kernel === 'llm' || kernel === 'r') {
      return kernel;
    }
    return 'python';
  }

  function upsertCell(nextCell: Cell) {
    cells = cells
      .map((cell) => cell.id === nextCell.id ? nextCell : cell)
      .sort((left, right) => left.position - right.position);
  }

  function updateSession(kernel: NotebookKernel, session: Session | null) {
    sessionsByKernel = { ...sessionsByKernel, [kernel]: session };
  }

  function selectedWorkspaceFile() {
    return workspaceFiles.find((file) => file.path === selectedWorkspaceFilePath) ?? workspaceFiles[0] ?? null;
  }

  function syncWorkspaceSelection() {
    if (workspaceFiles.length === 0) {
      selectedWorkspaceFilePath = '';
      return;
    }
    if (!workspaceFiles.some((file) => file.path === selectedWorkspaceFilePath)) {
      selectedWorkspaceFilePath = workspaceFiles[0].path;
    }
  }

  function workspaceEditorLanguage(file: NotebookWorkspaceFile | null) {
    if (!file) return 'text';
    if (file.language === 'markdown') return 'markdown';
    if (file.language === 'typescript' || file.language === 'javascript' || file.language === 'json' || file.language === 'python' || file.language === 'sql' || file.language === 'r' || file.language === 'toml') {
      return file.language;
    }
    return 'text';
  }

  async function loadSessionsForNotebook() {
    const res = await listSessions(notebookId);
    const next: Record<NotebookKernel, Session | null> = { python: null, sql: null, llm: null, r: null };

    for (const session of res.data) {
      const key = kernelKey(session.kernel);
      if (!next[key]) {
        next[key] = session;
      }
    }

    sessionsByKernel = next;
  }

  async function loadWorkspace() {
    loadingWorkspace = true;
    try {
      const res = await listWorkspaceFiles(notebookId);
      workspaceFiles = res.data;
      syncWorkspaceSelection();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : 'Failed to load notebook workspace';
      workspaceFiles = [];
    } finally {
      loadingWorkspace = false;
    }
  }

  async function load() {
    loading = true;
    error = '';
    try {
      const [res] = await Promise.all([
        getNotebook(notebookId),
        loadWorkspace(),
      ]);
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
    } catch (cause) {
      error = cause instanceof Error ? cause.message : 'Failed to load notebook';
      notebook = null;
    }
    loading = false;
  }

  async function ensureSession(kernel: NotebookKernel): Promise<Session> {
    const existing = sessionsByKernel[kernel];

    if (existing && existing.status !== 'dead') {
      return existing;
    }

    const session = await createSession(notebookId, kernel);
    updateSession(kernel, session);
    return session;
  }

  async function handleKernelChange(kernel: NotebookKernel) {
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

  async function handleAddCell(type: 'code' | 'markdown') {
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

  async function handleCellKernelChange(cellId: string, kernel: NotebookKernel) {
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
      const session = await ensureSession(key);
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
    } catch (cause: any) {
      outputs = {
        ...outputs,
        [cellId]: {
          output_type: 'error',
          content: cause?.message ?? 'Execution failed',
          execution_count: (cell.execution_count ?? 0) + 1,
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

  async function addWorkspaceFile() {
    const path = newWorkspaceFilePath.trim();
    if (!path) {
      return;
    }

    if (workspaceFiles.some((file) => file.path === path)) {
      error = 'That workspace file already exists.';
      return;
    }

    const file = await upsertWorkspaceFile(notebookId, { path, content: '' });
    workspaceFiles = [...workspaceFiles, file].sort((left, right) => left.path.localeCompare(right.path));
    selectedWorkspaceFilePath = file.path;
    newWorkspaceFilePath = '';
    error = '';
  }

  function handleWorkspaceContentChange(path: string, content: string) {
    workspaceFiles = workspaceFiles.map((file) => file.path === path ? { ...file, content } : file);
  }

  async function persistWorkspaceFile(path: string, content: string) {
    savingWorkspaceFile = { ...savingWorkspaceFile, [path]: true };
    try {
      const file = await upsertWorkspaceFile(notebookId, { path, content });
      workspaceFiles = workspaceFiles
        .map((entry) => entry.path === path ? file : entry)
        .sort((left, right) => left.path.localeCompare(right.path));
    } catch (cause) {
      error = cause instanceof Error ? cause.message : 'Failed to persist workspace file';
    } finally {
      savingWorkspaceFile = { ...savingWorkspaceFile, [path]: false };
    }
  }

  async function removeWorkspaceFile(path: string) {
    await deleteWorkspaceFile(notebookId, path);
    workspaceFiles = workspaceFiles.filter((file) => file.path !== path);
    syncWorkspaceSelection();
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
  <div class="mx-auto max-w-[1600px] p-6">
    <div class="mb-6 flex flex-wrap items-start justify-between gap-4">
      <div class="max-w-3xl">
        <div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-cyan-600">Code Workbook</div>
        <h1 class="mt-2 text-3xl font-bold">{notebook.name}</h1>
        <p class="mt-2 text-sm text-gray-500">{notebook.description || 'No description'}</p>
      </div>
      <div class="flex gap-2">
        <button class="rounded bg-green-600 px-3 py-1 text-sm text-white hover:bg-green-700" onclick={handleRunAll}>
          ▶ Run All
        </button>
        <a href="/notebooks" class="rounded border px-3 py-1 text-sm hover:bg-gray-50">Back</a>
      </div>
    </div>

    {#if error}
      <div class="mb-6 rounded-2xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-700">
        {error}
      </div>
    {/if}

    <div class="mb-6 rounded-2xl border border-slate-200 bg-slate-50 p-4 dark:border-slate-800 dark:bg-slate-900/40">
      <KernelSelector
        value={activeKernel}
        status={sessionsByKernel[activeKernel]?.status ?? null}
        onChange={handleKernelChange}
        onStart={handleStartSession}
        onStop={handleStopSession}
      />

      <div class="mt-3 flex flex-wrap gap-2 text-xs text-slate-500">
        <span class="rounded-full border border-slate-300 px-3 py-1">Default kernel: {activeKernel}</span>
        <span class="rounded-full border border-slate-300 px-3 py-1">Available kernels: Python, SQL, LLM, R</span>
        <span class="rounded-full border border-slate-300 px-3 py-1">{workspaceFiles.length} workspace file(s)</span>
      </div>
    </div>

    <div class="grid gap-6 xl:grid-cols-[minmax(0,1fr)_340px]">
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
                  onchange={(event) => handleCellKernelChange(cell.id, (event.currentTarget as HTMLSelectElement).value as NotebookKernel)}
                >
                  <option value="python">python</option>
                  <option value="sql">sql</option>
                  <option value="llm">llm</option>
                  <option value="r">r</option>
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
              language={cell.cell_type === 'markdown' ? 'markdown' : (kernelKey(cell.kernel) === 'llm' ? 'markdown' : kernelKey(cell.kernel))}
              minHeight={cell.cell_type === 'markdown' ? 120 : 180}
              onChange={(source) => handleSourceChange(cell.id, source)}
              onBlur={(source) => handlePersistSource(cell.id, source)}
            />

            <CellOutput output={outputs[cell.id] ?? cell.last_output} />
          </div>
        {/each}

        <div class="mt-4 flex gap-2">
          <button class="rounded border px-3 py-1 text-sm hover:bg-gray-50" onclick={() => handleAddCell('code')}>
            + Code Cell
          </button>
          <button class="rounded border px-3 py-1 text-sm hover:bg-gray-50" onclick={() => handleAddCell('markdown')}>
            + Markdown Cell
          </button>
        </div>
      </div>

      <aside class="space-y-4">
        <section class="rounded-2xl border border-slate-200 bg-white p-4 shadow-sm dark:border-slate-800 dark:bg-slate-950">
          <div class="flex items-center justify-between gap-3">
            <div>
              <div class="text-xs font-semibold uppercase tracking-[0.18em] text-slate-400">Workspace</div>
              <div class="mt-1 text-sm text-slate-500">Persist helper files, prompts, scripts, and notes next to the notebook.</div>
            </div>
          </div>

          <div class="mt-4 flex gap-2">
            <input type="text" bind:value={newWorkspaceFilePath} placeholder="prompts/system.md" class="min-w-0 flex-1 rounded-xl border border-slate-200 px-3 py-2 text-sm dark:border-slate-700 dark:bg-slate-950" />
            <button type="button" class="rounded-xl border border-slate-200 px-3 py-2 text-sm hover:bg-slate-100 dark:border-slate-700 dark:hover:bg-slate-800" onclick={addWorkspaceFile}>Add</button>
          </div>

          {#if loadingWorkspace}
            <div class="mt-4 text-sm text-slate-500">Loading workspace...</div>
          {:else if workspaceFiles.length === 0}
            <div class="mt-4 rounded-2xl border border-dashed border-slate-300 px-4 py-8 text-center text-sm text-slate-500 dark:border-slate-700">
              No workspace files yet.
            </div>
          {:else}
            <div class="mt-4 grid gap-4">
              <div class="space-y-2">
                {#each workspaceFiles as file (file.path)}
                  <button type="button" class={`w-full rounded-2xl border px-3 py-3 text-left text-sm ${selectedWorkspaceFilePath === file.path ? 'border-cyan-500 bg-cyan-50 dark:bg-cyan-950/20' : 'border-slate-200 bg-white hover:bg-slate-50 dark:border-slate-700 dark:bg-slate-950 dark:hover:bg-slate-900'}`} onclick={() => selectedWorkspaceFilePath = file.path}>
                    <div class="font-medium text-slate-900 dark:text-slate-100">{file.path}</div>
                    <div class="mt-1 text-xs text-slate-500">{file.language} · {file.size_bytes} bytes</div>
                  </button>
                {/each}
              </div>

              {#if selectedWorkspaceFile()}
                <div class="space-y-3">
                  <div class="flex flex-wrap items-center justify-between gap-3">
                    <div class="rounded-full border border-slate-200 px-3 py-1 text-xs text-slate-500 dark:border-slate-700">{selectedWorkspaceFile()?.path}</div>
                    <button type="button" class="rounded-xl border border-rose-200 px-3 py-2 text-xs text-rose-600 hover:bg-rose-50 dark:border-rose-900/40 dark:hover:bg-rose-950/20" onclick={() => removeWorkspaceFile(selectedWorkspaceFile()?.path ?? '')}>
                      Remove file
                    </button>
                  </div>

                  <CellEditor
                    value={selectedWorkspaceFile()?.content ?? ''}
                    language={workspaceEditorLanguage(selectedWorkspaceFile())}
                    minHeight={360}
                    onChange={(content) => handleWorkspaceContentChange(selectedWorkspaceFile()?.path ?? '', content)}
                    onBlur={(content) => persistWorkspaceFile(selectedWorkspaceFile()?.path ?? '', content)}
                  />

                  {#if selectedWorkspaceFile()}
                    <div class="text-xs text-slate-500">
                      {savingWorkspaceFile[selectedWorkspaceFile()?.path ?? ''] ? 'Saving...' : `Updated ${new Date(selectedWorkspaceFile()?.updated_at ?? '').toLocaleString()}`}
                    </div>
                  {/if}
                </div>
              {/if}
            </div>
          {/if}
        </section>
      </aside>
    </div>
  </div>
{/if}
