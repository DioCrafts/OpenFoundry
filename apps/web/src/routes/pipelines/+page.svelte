<script lang="ts">
  import { onMount } from 'svelte';

  import { listDatasets, type Dataset } from '$lib/api/datasets';
  import {
    createPipeline,
    deletePipeline,
    getDatasetColumnLineage,
    getPipeline,
    listPipelines,
    listRuns,
    retryPipelineRun,
    runDuePipelines,
    triggerRun,
    updatePipeline,
    type ColumnLineageEdge,
    type Pipeline,
    type PipelineColumnMapping,
    type PipelineNode,
    type PipelineRetryPolicy,
    type PipelineRun,
    type PipelineScheduleConfig,
  } from '$lib/api/pipelines';
  import { notifications } from '$stores/notifications';

  type PipelineDraft = {
    id?: string;
    name: string;
    description: string;
    status: string;
    schedule_config: PipelineScheduleConfig;
    retry_policy: PipelineRetryPolicy;
    nodes: PipelineNode[];
    next_run_at?: string | null;
  };

  let pipelines = $state<Pipeline[]>([]);
  let datasets = $state<Dataset[]>([]);
  let runs = $state<PipelineRun[]>([]);
  let columnLineage = $state<ColumnLineageEdge[]>([]);
  let loading = $state(true);
  let saving = $state(false);
  let running = $state(false);
  let search = $state('');
  let selectedPipelineId = $state('');
  let error = $state('');
  let draft = $state<PipelineDraft>(createEmptyPipeline());

  function makeId() {
    if (typeof crypto !== 'undefined' && 'randomUUID' in crypto) {
      return crypto.randomUUID();
    }

    return `node_${Date.now()}_${Math.floor(Math.random() * 10_000)}`;
  }

  function createNode(transformType = 'sql'): PipelineNode {
    const config: Record<string, unknown> =
      transformType === 'sql'
        ? { sql: 'SELECT 1 AS value' }
        : transformType === 'python'
          ? { source: 'rows_affected = 0\nresult = "python transform ready"' }
          : transformType === 'wasm'
            ? { module: '(module (func (export "run") (result i32) i32.const 0))', function: 'run' }
            : { identity_columns: [] };

    return {
      id: makeId(),
      label: `${transformType.toUpperCase()} transform`,
      transform_type: transformType,
      config,
      depends_on: [],
      input_dataset_ids: [],
      output_dataset_id: null,
    };
  }

  function createEmptyPipeline(): PipelineDraft {
    return {
      name: 'New pipeline',
      description: '',
      status: 'draft',
      schedule_config: { enabled: false, cron: '0 */15 * * * *' },
      retry_policy: { max_attempts: 1, retry_on_failure: false, allow_partial_reexecution: true },
      nodes: [createNode('sql')],
      next_run_at: null,
    };
  }

  function normalizePipeline(pipeline: Pipeline): PipelineDraft {
    return {
      id: pipeline.id,
      name: pipeline.name,
      description: pipeline.description,
      status: pipeline.status,
      schedule_config: pipeline.schedule_config ?? { enabled: false, cron: null },
      retry_policy: pipeline.retry_policy ?? { max_attempts: 1, retry_on_failure: false, allow_partial_reexecution: true },
      nodes: Array.isArray(pipeline.dag) ? pipeline.dag : [],
      next_run_at: pipeline.next_run_at,
    };
  }

  function statusBadge(status: string) {
    const colors: Record<string, string> = {
      draft: 'bg-gray-200 text-gray-700 dark:bg-gray-700 dark:text-gray-300',
      active: 'bg-green-100 text-green-700 dark:bg-green-900 dark:text-green-300',
      failed: 'bg-red-100 text-red-700 dark:bg-red-900 dark:text-red-300',
      completed: 'bg-blue-100 text-blue-700 dark:bg-blue-900 dark:text-blue-300',
    };
    return colors[status] || colors.draft;
  }

  function datasetName(datasetId: string | null) {
    if (!datasetId) return 'No dataset';
    return datasets.find((dataset) => dataset.id === datasetId)?.name ?? datasetId.slice(0, 8);
  }

  function columnMappings(node: PipelineNode): PipelineColumnMapping[] {
    const mappings = node.config?.['column_mappings'];
    return Array.isArray(mappings) ? mappings as PipelineColumnMapping[] : [];
  }

  function identityColumns(node: PipelineNode) {
    const columns = node.config?.['identity_columns'];
    return Array.isArray(columns) ? columns.filter((value): value is string => typeof value === 'string').join(', ') : '';
  }

  function nodeCode(node: PipelineNode) {
    if (node.transform_type === 'sql') return typeof node.config?.['sql'] === 'string' ? String(node.config['sql']) : '';
    if (node.transform_type === 'python') return typeof node.config?.['source'] === 'string' ? String(node.config['source']) : '';
    if (node.transform_type === 'wasm') return typeof node.config?.['module'] === 'string' ? String(node.config['module']) : '';
    return '';
  }

  function wasmFunction(node: PipelineNode) {
    return typeof node.config?.['function'] === 'string' ? String(node.config['function']) : 'run';
  }

  function branchValue(node: PipelineNode, key: 'input_branch' | 'output_branch') {
    return typeof node.config?.[key] === 'string' ? String(node.config[key]) : '';
  }

  function updateNode(nodeId: string, updater: (node: PipelineNode) => PipelineNode) {
    draft = {
      ...draft,
      nodes: draft.nodes.map((node) => {
        if (node.id !== nodeId) return node;
        const next = {
          ...node,
          config: { ...node.config },
          depends_on: [...node.depends_on],
          input_dataset_ids: [...node.input_dataset_ids],
        };
        return updater(next);
      }),
    };
  }

  function newPipeline() {
    selectedPipelineId = '';
    runs = [];
    columnLineage = [];
    draft = createEmptyPipeline();
    error = '';
  }

  async function loadRegistry() {
    const [pipelineResponse, datasetResponse] = await Promise.all([
      listPipelines({ search: search || undefined, per_page: 50 }),
      listDatasets({ per_page: 100 }),
    ]);
    pipelines = pipelineResponse.data;
    datasets = datasetResponse.data;
  }

  async function loadRuns() {
    if (!selectedPipelineId) {
      runs = [];
      return;
    }

    const response = await listRuns(selectedPipelineId, { per_page: 20 });
    runs = response.data;
  }

  async function loadLineage() {
    const outputDatasetIds = Array.from(
      new Set(
        draft.nodes
          .map((node) => node.output_dataset_id)
          .filter((datasetId): datasetId is string => Boolean(datasetId)),
      ),
    );

    if (outputDatasetIds.length === 0) {
      columnLineage = [];
      return;
    }

    const results = await Promise.all(
      outputDatasetIds.slice(0, 4).map((datasetId) => getDatasetColumnLineage(datasetId).catch(() => [] as ColumnLineageEdge[])),
    );
    columnLineage = results.flat();
  }

  async function selectPipeline(id: string) {
    selectedPipelineId = id;
    const pipeline = await getPipeline(id);
    draft = normalizePipeline(pipeline);
    await Promise.all([loadRuns(), loadLineage()]);
  }

  async function load() {
    loading = true;
    error = '';
    try {
      await loadRegistry();
      if (selectedPipelineId) {
        await selectPipeline(selectedPipelineId);
      } else if (pipelines.length > 0) {
        await selectPipeline(pipelines[0].id);
      } else {
        newPipeline();
      }
    } catch (cause) {
      console.error('Failed to load pipelines', cause);
      error = cause instanceof Error ? cause.message : 'Failed to load pipelines';
    } finally {
      loading = false;
    }
  }

  async function savePipeline() {
    saving = true;
    error = '';

    try {
      const payload = {
        name: draft.name,
        description: draft.description,
        status: draft.status,
        schedule_config: {
          enabled: draft.schedule_config.enabled,
          cron: draft.schedule_config.enabled ? draft.schedule_config.cron : null,
        },
        retry_policy: draft.retry_policy,
        nodes: draft.nodes,
      };

      const pipeline = draft.id
        ? await updatePipeline(draft.id, payload)
        : await createPipeline(payload);

      notifications.success(`Pipeline ${draft.id ? 'updated' : 'created'}`);
      await loadRegistry();
      await selectPipeline(pipeline.id);
    } catch (cause) {
      error = cause instanceof Error ? cause.message : 'Failed to save pipeline';
    } finally {
      saving = false;
    }
  }

  async function removePipeline() {
    if (!draft.id || !confirm('Delete this pipeline?')) return;
    await deletePipeline(draft.id);
    notifications.success('Pipeline deleted');
    await loadRegistry();
    if (pipelines.length > 0) {
      await selectPipeline(pipelines[0].id);
    } else {
      newPipeline();
    }
  }

  function addNode(type = 'sql') {
    draft = { ...draft, nodes: [...draft.nodes, createNode(type)] };
  }

  function removeNode(nodeId: string) {
    if (draft.nodes.length <= 1) return;
    draft = {
      ...draft,
      nodes: draft.nodes
        .filter((node) => node.id !== nodeId)
        .map((node) => ({ ...node, depends_on: node.depends_on.filter((dependency) => dependency !== nodeId) })),
    };
  }

  function toggleDependency(nodeId: string, dependencyId: string) {
    updateNode(nodeId, (node) => ({
      ...node,
      depends_on: node.depends_on.includes(dependencyId)
        ? node.depends_on.filter((dependency) => dependency !== dependencyId)
        : [...node.depends_on, dependencyId],
    }));
  }

  function toggleInputDataset(nodeId: string, datasetId: string, checked: boolean) {
    updateNode(nodeId, (node) => ({
      ...node,
      input_dataset_ids: checked
        ? [...node.input_dataset_ids, datasetId]
        : node.input_dataset_ids.filter((candidate) => candidate !== datasetId),
    }));
  }

  function setNodeOutputDataset(nodeId: string, datasetId: string) {
    updateNode(nodeId, (node) => ({ ...node, output_dataset_id: datasetId || null }));
  }

  function setNodeField(nodeId: string, key: keyof PipelineNode, value: string) {
    if (key === 'label') {
      updateNode(nodeId, (node) => ({ ...node, label: value }));
      return;
    }

    if (key === 'transform_type') {
      updateNode(nodeId, (node) => ({ ...node, transform_type: value, config: createNode(value).config }));
    }
  }

  function setNodeConfig(nodeId: string, key: string, value: unknown) {
    updateNode(nodeId, (node) => ({ ...node, config: { ...node.config, [key]: value } }));
  }

  function setIdentityColumns(nodeId: string, csv: string) {
    setNodeConfig(nodeId, 'identity_columns', csv.split(',').map((value) => value.trim()).filter(Boolean));
  }

  function addColumnMapping(nodeId: string) {
    updateNode(nodeId, (node) => ({
      ...node,
      config: {
        ...node.config,
        column_mappings: [
          ...columnMappings(node),
          {
            source_dataset_id: node.input_dataset_ids[0] ?? null,
            source_column: '',
            target_column: '',
          },
        ],
      },
    }));
  }

  function updateColumnMapping(nodeId: string, index: number, key: keyof PipelineColumnMapping, value: string) {
    updateNode(nodeId, (node) => ({
      ...node,
      config: {
        ...node.config,
        column_mappings: columnMappings(node).map((mapping, mappingIndex) => {
          if (mappingIndex !== index) return mapping;
          return {
            ...mapping,
            [key]: key === 'source_dataset_id' ? (value || null) : value,
          };
        }),
      },
    }));
  }

  function removeColumnMapping(nodeId: string, index: number) {
    updateNode(nodeId, (node) => ({
      ...node,
      config: {
        ...node.config,
        column_mappings: columnMappings(node).filter((_, mappingIndex) => mappingIndex !== index),
      },
    }));
  }

  async function runPipeline() {
    if (!draft.id) return;
    running = true;
    error = '';
    try {
      await triggerRun(draft.id, {});
      notifications.success('Pipeline run started');
      await Promise.all([loadRuns(), loadLineage(), loadRegistry()]);
    } catch (cause) {
      error = cause instanceof Error ? cause.message : 'Failed to run pipeline';
    } finally {
      running = false;
    }
  }

  async function rerunPipeline(run: PipelineRun, partial: boolean) {
    if (!draft.id) return;
    running = true;
    error = '';
    try {
      const failedNode = partial ? (run.node_results ?? []).find((result) => result.status === 'failed')?.node_id : undefined;
      await retryPipelineRun(draft.id, run.id, failedNode ? { from_node_id: failedNode } : {});
      notifications.success(partial ? 'Partial re-execution started' : 'Retry started');
      await Promise.all([loadRuns(), loadLineage(), loadRegistry()]);
    } catch (cause) {
      error = cause instanceof Error ? cause.message : 'Failed to retry pipeline';
    } finally {
      running = false;
    }
  }

  async function runSchedules() {
    running = true;
    error = '';
    try {
      const response = await runDuePipelines();
      notifications.info(`Triggered ${response.triggered_runs} scheduled pipeline run(s)`);
      await Promise.all([loadRuns(), loadLineage(), loadRegistry()]);
    } catch (cause) {
      error = cause instanceof Error ? cause.message : 'Failed to evaluate schedules';
    } finally {
      running = false;
    }
  }

  function nodeResultSummary(run: PipelineRun) {
    const results = run.node_results ?? [];
    const completed = results.filter((result) => result.status === 'completed').length;
    return `${completed}/${results.length} nodes completed`;
  }

  onMount(() => {
    void load();
  });
</script>

<div class="space-y-6">
  <div class="flex items-center justify-between">
    <div>
      <h1 class="text-2xl font-bold">Pipeline Enhancements</h1>
      <p class="mt-1 text-sm text-gray-500">Author SQL, Python, and WASM transforms, schedule runs, track column lineage, and rerun only failed slices.</p>
    </div>
    <div class="flex gap-2">
      <button type="button" onclick={newPipeline} class="rounded-xl border border-slate-200 px-4 py-2 hover:bg-slate-50 dark:border-gray-700 dark:hover:bg-gray-800">New pipeline</button>
      <button type="button" onclick={savePipeline} disabled={saving} class="rounded-xl bg-blue-600 px-4 py-2 text-white disabled:opacity-50 hover:bg-blue-700">
        {saving ? 'Saving...' : draft.id ? 'Save changes' : 'Create pipeline'}
      </button>
    </div>
  </div>

  {#if error}
    <div class="rounded-xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-700 dark:border-rose-900/40 dark:bg-rose-950/40 dark:text-rose-300">{error}</div>
  {/if}

  <div class="grid gap-6 xl:grid-cols-[0.95fr,1.05fr]">
    <section class="space-y-4 rounded-2xl border border-slate-200 bg-white p-5 shadow-sm dark:border-gray-700 dark:bg-gray-900">
      <div class="flex items-center justify-between">
        <div>
          <div class="text-xs uppercase tracking-[0.22em] text-gray-400">Pipeline registry</div>
          <div class="mt-1 text-sm text-gray-500">Existing DAGs, schedule state, and retry policy summary.</div>
        </div>
        <span class="rounded-full bg-slate-100 px-3 py-1 text-xs font-medium text-slate-700 dark:bg-gray-800 dark:text-gray-200">{pipelines.length} pipelines</span>
      </div>

      <input
        id="pipeline-search"
        type="text"
        placeholder="Search pipelines..."
        bind:value={search}
        oninput={() => void loadRegistry()}
        class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-gray-700 dark:bg-gray-800"
      />

      {#if loading}
        <div class="py-10 text-center text-gray-500">Loading pipelines...</div>
      {:else if pipelines.length === 0}
        <div class="rounded-xl border border-dashed border-slate-300 px-4 py-10 text-center text-sm text-gray-500 dark:border-gray-700">
          No pipelines yet. Create the first one from the builder.
        </div>
      {:else}
        <div class="space-y-3">
          {#each pipelines as pipeline (pipeline.id)}
            <button
              type="button"
              onclick={() => void selectPipeline(pipeline.id)}
              class={`w-full rounded-xl border p-4 text-left transition-colors ${selectedPipelineId === pipeline.id ? 'border-blue-500 bg-blue-50/70 dark:border-blue-400 dark:bg-blue-950/30' : 'border-slate-200 hover:bg-slate-50 dark:border-gray-700 dark:hover:bg-gray-800'}`}
            >
              <div class="flex items-center justify-between gap-3">
                <div>
                  <div class="font-medium">{pipeline.name}</div>
                  <div class="mt-1 text-sm text-gray-500">{pipeline.description || 'No description'}</div>
                </div>
                <span class={`rounded-full px-2.5 py-1 text-xs font-medium ${statusBadge(pipeline.status)}`}>{pipeline.status}</span>
              </div>
              <div class="mt-3 flex flex-wrap gap-2 text-xs text-gray-500">
                <span>{Array.isArray(pipeline.dag) ? pipeline.dag.length : 0} nodes</span>
                <span>{pipeline.schedule_config?.enabled ? pipeline.schedule_config.cron : 'manual only'}</span>
                <span>{pipeline.retry_policy?.max_attempts ?? 1} attempt(s)</span>
              </div>
            </button>
          {/each}
        </div>
      {/if}
    </section>

    <section class="space-y-6">
      <div class="rounded-2xl border border-slate-200 bg-white p-5 shadow-sm dark:border-gray-700 dark:bg-gray-900">
        <div class="grid gap-4 md:grid-cols-2">
          <div>
            <label for="pipeline-name" class="mb-1 block text-sm font-medium">Pipeline name</label>
            <input id="pipeline-name" bind:value={draft.name} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-gray-700 dark:bg-gray-800" />
          </div>
          <div>
            <label for="pipeline-status" class="mb-1 block text-sm font-medium">Status</label>
            <select id="pipeline-status" bind:value={draft.status} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-gray-700 dark:bg-gray-800">
              <option value="draft">Draft</option>
              <option value="active">Active</option>
              <option value="paused">Paused</option>
            </select>
          </div>
        </div>

        <div class="mt-4">
          <label for="pipeline-description" class="mb-1 block text-sm font-medium">Description</label>
          <textarea id="pipeline-description" bind:value={draft.description} rows="3" class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-gray-700 dark:bg-gray-800"></textarea>
        </div>

        <div class="mt-4 grid gap-4 md:grid-cols-2">
          <div class="rounded-xl border border-slate-200 p-4 dark:border-gray-700">
            <div class="flex items-center justify-between">
              <div>
                <div class="font-medium">Scheduling</div>
                <div class="text-sm text-gray-500">Cron-based execution for active pipelines.</div>
              </div>
              <input type="checkbox" bind:checked={draft.schedule_config.enabled} />
            </div>
            <div class="mt-3">
              <label for="pipeline-cron" class="mb-1 block text-sm font-medium">Cron expression</label>
              <input id="pipeline-cron" bind:value={draft.schedule_config.cron} disabled={!draft.schedule_config.enabled} class="w-full rounded-xl border border-slate-200 px-3 py-2 font-mono text-sm disabled:opacity-50 dark:border-gray-700 dark:bg-gray-800" />
            </div>
            <div class="mt-2 text-xs text-gray-500">Next run {draft.next_run_at ? new Date(draft.next_run_at).toLocaleString() : 'not scheduled'}</div>
          </div>

          <div class="rounded-xl border border-slate-200 p-4 dark:border-gray-700">
            <div class="font-medium">Retry policy</div>
            <div class="mt-3 space-y-3 text-sm">
              <label class="flex items-center justify-between rounded-lg border border-slate-200 px-3 py-2 dark:border-gray-700">
                <span>Retry on failure</span>
                <input type="checkbox" bind:checked={draft.retry_policy.retry_on_failure} />
              </label>
              <label class="flex items-center justify-between rounded-lg border border-slate-200 px-3 py-2 dark:border-gray-700">
                <span>Allow partial re-execution</span>
                <input type="checkbox" bind:checked={draft.retry_policy.allow_partial_reexecution} />
              </label>
              <div>
                <label for="pipeline-max-attempts" class="mb-1 block text-sm font-medium">Max attempts</label>
                <input id="pipeline-max-attempts" type="number" min="1" bind:value={draft.retry_policy.max_attempts} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-gray-700 dark:bg-gray-800" />
              </div>
            </div>
          </div>
        </div>

        <div class="mt-4 flex flex-wrap gap-2">
          <button type="button" onclick={() => void runPipeline()} disabled={!draft.id || running} class="rounded-xl bg-slate-900 px-4 py-2 text-white disabled:opacity-50 dark:bg-white dark:text-slate-900">
            {running ? 'Running...' : 'Run now'}
          </button>
          <button type="button" onclick={() => void runSchedules()} disabled={running} class="rounded-xl border border-slate-200 px-4 py-2 disabled:opacity-50 hover:bg-slate-50 dark:border-gray-700 dark:hover:bg-gray-800">
            Evaluate due schedules
          </button>
          {#if draft.id}
            <button type="button" onclick={removePipeline} class="rounded-xl border border-rose-200 px-4 py-2 text-rose-600 hover:bg-rose-50 dark:border-rose-900/40 dark:hover:bg-rose-950/30">
              Delete pipeline
            </button>
          {/if}
        </div>
      </div>

      <div class="rounded-2xl border border-slate-200 bg-white p-5 shadow-sm dark:border-gray-700 dark:bg-gray-900">
        <div class="flex items-center justify-between">
          <div>
            <div class="text-xs uppercase tracking-[0.22em] text-gray-400">Pipeline builder</div>
            <div class="mt-1 text-sm text-gray-500">Mix SQL, Python, WASM, and passthrough steps, then wire dataset inputs, branches, and lineage mappings.</div>
          </div>
          <div class="flex gap-2">
            <button type="button" onclick={() => addNode('sql')} class="rounded-xl border border-slate-200 px-3 py-2 text-sm hover:bg-slate-50 dark:border-gray-700 dark:hover:bg-gray-800">+ SQL</button>
            <button type="button" onclick={() => addNode('python')} class="rounded-xl border border-slate-200 px-3 py-2 text-sm hover:bg-slate-50 dark:border-gray-700 dark:hover:bg-gray-800">+ Python</button>
            <button type="button" onclick={() => addNode('wasm')} class="rounded-xl border border-slate-200 px-3 py-2 text-sm hover:bg-slate-50 dark:border-gray-700 dark:hover:bg-gray-800">+ WASM</button>
            <button type="button" onclick={() => addNode('passthrough')} class="rounded-xl border border-slate-200 px-3 py-2 text-sm hover:bg-slate-50 dark:border-gray-700 dark:hover:bg-gray-800">+ Passthrough</button>
          </div>
        </div>

        <div class="mt-5 space-y-4">
          {#each draft.nodes as node, index (node.id)}
            <div class="rounded-2xl border border-slate-200 bg-slate-50/70 p-4 shadow-sm dark:border-gray-700 dark:bg-gray-950/40">
              <div class="flex items-center justify-between gap-3">
                <div class="flex items-center gap-3">
                  <span class="rounded-full bg-white px-2.5 py-1 text-xs font-semibold uppercase tracking-[0.16em] text-slate-600 dark:bg-gray-900 dark:text-gray-300">{node.transform_type}</span>
                  <input aria-label="Node label" value={node.label} oninput={(event) => setNodeField(node.id, 'label', (event.currentTarget as HTMLInputElement).value)} class="rounded-xl border border-slate-200 px-3 py-2 text-sm dark:border-gray-700 dark:bg-gray-900" />
                </div>
                <div class="flex items-center gap-2">
                  <select aria-label="Transform type" value={node.transform_type} oninput={(event) => setNodeField(node.id, 'transform_type', (event.currentTarget as HTMLSelectElement).value)} class="rounded-xl border border-slate-200 px-3 py-2 text-sm dark:border-gray-700 dark:bg-gray-900">
                    <option value="sql">SQL</option>
                    <option value="python">Python</option>
                    <option value="wasm">WASM</option>
                    <option value="passthrough">Passthrough</option>
                  </select>
                  <button type="button" onclick={() => removeNode(node.id)} class="text-sm text-rose-600 hover:underline">Remove</button>
                </div>
              </div>

              <div class="mt-4 grid gap-4 xl:grid-cols-[0.9fr,1.1fr]">
                <div class="space-y-4">
                  <div>
                    <div class="mb-2 text-sm font-medium">Input datasets</div>
                    <div class="grid gap-2 md:grid-cols-2">
                      {#each datasets as dataset (dataset.id)}
                        <label class="flex items-center gap-2 rounded-xl border border-slate-200 px-3 py-2 text-sm dark:border-gray-700">
                          <input type="checkbox" checked={node.input_dataset_ids.includes(dataset.id)} onchange={(event) => toggleInputDataset(node.id, dataset.id, (event.currentTarget as HTMLInputElement).checked)} />
                          <span>{dataset.name}</span>
                        </label>
                      {/each}
                    </div>
                  </div>

                  <div>
                    <label for={`output-dataset-${node.id}`} class="mb-1 block text-sm font-medium">Output dataset</label>
                    <select id={`output-dataset-${node.id}`} value={node.output_dataset_id ?? ''} oninput={(event) => setNodeOutputDataset(node.id, (event.currentTarget as HTMLSelectElement).value)} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-gray-700 dark:bg-gray-900">
                      <option value="">Select output dataset</option>
                      {#each datasets as dataset (dataset.id)}
                        <option value={dataset.id}>{dataset.name}</option>
                      {/each}
                    </select>
                  </div>

                  <div class="grid gap-4 md:grid-cols-2">
                    <div>
                      <label for={`input-branch-${node.id}`} class="mb-1 block text-sm font-medium">Input branch</label>
                      <input id={`input-branch-${node.id}`} value={branchValue(node, 'input_branch')} oninput={(event) => setNodeConfig(node.id, 'input_branch', (event.currentTarget as HTMLInputElement).value)} placeholder="main" class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-gray-700 dark:bg-gray-900" />
                    </div>
                    <div>
                      <label for={`output-branch-${node.id}`} class="mb-1 block text-sm font-medium">Output branch</label>
                      <input id={`output-branch-${node.id}`} value={branchValue(node, 'output_branch')} oninput={(event) => setNodeConfig(node.id, 'output_branch', (event.currentTarget as HTMLInputElement).value)} placeholder="feature-branch" class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-gray-700 dark:bg-gray-900" />
                    </div>
                  </div>

                  {#if index > 0}
                    <div>
                      <div class="mb-2 text-sm font-medium">Dependencies</div>
                      <div class="flex flex-wrap gap-2">
                        {#each draft.nodes.filter((candidate) => candidate.id !== node.id) as dependency (dependency.id)}
                          <button type="button" onclick={() => toggleDependency(node.id, dependency.id)} class={`rounded-full border px-3 py-1 text-xs ${node.depends_on.includes(dependency.id) ? 'border-blue-600 bg-blue-600 text-white' : 'border-slate-200 dark:border-gray-700'}`}>
                            {dependency.label}
                          </button>
                        {/each}
                      </div>
                    </div>
                  {/if}
                </div>

                <div class="space-y-4">
                  <div>
                    <label for={`node-code-${node.id}`} class="mb-1 block text-sm font-medium">{node.transform_type === 'sql' ? 'SQL' : node.transform_type === 'python' ? 'Python source' : node.transform_type === 'wasm' ? 'WASM module' : 'Passthrough config'}</label>
                    {#if node.transform_type === 'passthrough'}
                      <input id={`node-code-${node.id}`} value={identityColumns(node)} oninput={(event) => setIdentityColumns(node.id, (event.currentTarget as HTMLInputElement).value)} placeholder="customer_id, order_id" class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-gray-700 dark:bg-gray-900" />
                    {:else}
                      <textarea id={`node-code-${node.id}`} rows="8" value={nodeCode(node)} oninput={(event) => setNodeConfig(node.id, node.transform_type === 'sql' ? 'sql' : node.transform_type === 'python' ? 'source' : 'module', (event.currentTarget as HTMLTextAreaElement).value)} class="w-full rounded-xl border border-slate-200 px-3 py-2 font-mono text-sm dark:border-gray-700 dark:bg-gray-900"></textarea>
                    {/if}
                  </div>

                  {#if node.transform_type === 'wasm'}
                    <div>
                      <label for={`wasm-function-${node.id}`} class="mb-1 block text-sm font-medium">Exported function</label>
                      <input id={`wasm-function-${node.id}`} value={wasmFunction(node)} oninput={(event) => setNodeConfig(node.id, 'function', (event.currentTarget as HTMLInputElement).value)} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-gray-700 dark:bg-gray-900" />
                    </div>
                  {/if}

                  <div class="rounded-xl border border-slate-200 p-4 dark:border-gray-700">
                    <div class="flex items-center justify-between">
                      <div class="text-sm font-medium">Column mappings</div>
                      <button type="button" onclick={() => addColumnMapping(node.id)} class="text-xs text-blue-600 hover:underline">Add mapping</button>
                    </div>
                    <div class="mt-3 space-y-3">
                      {#each columnMappings(node) as mapping, mappingIndex}
                        <div class="rounded-xl border border-slate-200 p-3 dark:border-gray-700">
                          <div class="grid gap-3 md:grid-cols-3">
                            <select aria-label="Source dataset" value={mapping.source_dataset_id ?? ''} oninput={(event) => updateColumnMapping(node.id, mappingIndex, 'source_dataset_id', (event.currentTarget as HTMLSelectElement).value)} class="rounded-lg border border-slate-200 px-3 py-2 text-sm dark:border-gray-700 dark:bg-gray-900">
                              <option value="">Auto source dataset</option>
                              {#each datasets as dataset (dataset.id)}
                                <option value={dataset.id}>{dataset.name}</option>
                              {/each}
                            </select>
                            <input aria-label="Source column" value={mapping.source_column} oninput={(event) => updateColumnMapping(node.id, mappingIndex, 'source_column', (event.currentTarget as HTMLInputElement).value)} placeholder="source column" class="rounded-lg border border-slate-200 px-3 py-2 text-sm dark:border-gray-700 dark:bg-gray-900" />
                            <input aria-label="Target column" value={mapping.target_column} oninput={(event) => updateColumnMapping(node.id, mappingIndex, 'target_column', (event.currentTarget as HTMLInputElement).value)} placeholder="target column" class="rounded-lg border border-slate-200 px-3 py-2 text-sm dark:border-gray-700 dark:bg-gray-900" />
                          </div>
                          <div class="mt-2 flex justify-end">
                            <button type="button" onclick={() => removeColumnMapping(node.id, mappingIndex)} class="text-xs text-rose-600 hover:underline">Remove</button>
                          </div>
                        </div>
                      {/each}
                      {#if columnMappings(node).length === 0}
                        <div class="text-xs text-gray-500">No explicit column mappings yet.</div>
                      {/if}
                    </div>
                  </div>
                </div>
              </div>
            </div>
          {/each}
        </div>
      </div>

      <div class="grid gap-6 xl:grid-cols-[0.95fr,1.05fr]">
        <div class="rounded-2xl border border-slate-200 bg-white p-5 shadow-sm dark:border-gray-700 dark:bg-gray-900">
          <div class="flex items-center justify-between">
            <div>
              <div class="text-xs uppercase tracking-[0.22em] text-gray-400">Run history</div>
              <div class="mt-1 text-sm text-gray-500">Trigger source, retry attempts, and partial reruns.</div>
            </div>
            <span class="rounded-full bg-slate-100 px-3 py-1 text-xs font-medium text-slate-700 dark:bg-gray-800 dark:text-gray-200">{runs.length} runs</span>
          </div>

          <div class="mt-4 space-y-3">
            {#each runs as run (run.id)}
              <div class="rounded-xl border border-slate-200 p-4 dark:border-gray-700">
                <div class="flex items-center justify-between gap-3">
                  <div>
                    <div class="flex items-center gap-2">
                      <div class="font-medium">{run.trigger_type} run</div>
                      <span class={`rounded-full px-2.5 py-1 text-xs font-medium ${statusBadge(run.status)}`}>{run.status}</span>
                    </div>
                    <div class="mt-1 text-sm text-gray-500">Attempt {run.attempt_number} · {nodeResultSummary(run)}</div>
                    <div class="mt-1 text-xs text-gray-500">Started {new Date(run.started_at).toLocaleString()}</div>
                    {#if run.started_from_node_id}
                      <div class="mt-1 text-xs text-gray-500">Partial from node {run.started_from_node_id}</div>
                    {/if}
                    {#if run.error_message}
                      <div class="mt-2 rounded-lg border border-rose-200 bg-rose-50 px-3 py-2 text-xs text-rose-700 dark:border-rose-900/40 dark:bg-rose-950/40 dark:text-rose-300">{run.error_message}</div>
                    {/if}
                  </div>
                  <div class="flex flex-col gap-2">
                    <button type="button" onclick={() => void rerunPipeline(run, false)} disabled={running} class="rounded-lg border border-slate-200 px-3 py-1.5 text-sm disabled:opacity-50 hover:bg-slate-50 dark:border-gray-700 dark:hover:bg-gray-800">Retry full</button>
                    <button type="button" onclick={() => void rerunPipeline(run, true)} disabled={running || !draft.retry_policy.allow_partial_reexecution} class="rounded-lg border border-slate-200 px-3 py-1.5 text-sm disabled:opacity-50 hover:bg-slate-50 dark:border-gray-700 dark:hover:bg-gray-800">Retry failed slice</button>
                  </div>
                </div>
              </div>
            {/each}
            {#if runs.length === 0}
              <div class="text-sm text-gray-500">No runs yet.</div>
            {/if}
          </div>
        </div>

        <div class="rounded-2xl border border-slate-200 bg-white p-5 shadow-sm dark:border-gray-700 dark:bg-gray-900">
          <div class="flex items-center justify-between">
            <div>
              <div class="text-xs uppercase tracking-[0.22em] text-gray-400">Column lineage</div>
              <div class="mt-1 text-sm text-gray-500">Recorded source-to-target column flow for the output datasets touched by this pipeline.</div>
            </div>
            <span class="rounded-full bg-slate-100 px-3 py-1 text-xs font-medium text-slate-700 dark:bg-gray-800 dark:text-gray-200">{columnLineage.length} edges</span>
          </div>

          <div class="mt-4 space-y-3">
            {#each columnLineage as edge (edge.id)}
              <div class="rounded-xl border border-slate-200 p-3 text-sm dark:border-gray-700">
                <div class="font-medium">{edge.source_column} → {edge.target_column}</div>
                <div class="mt-1 text-xs text-gray-500">
                  {datasetName(edge.source_dataset_id)} → {datasetName(edge.target_dataset_id)}
                </div>
                <div class="mt-1 text-xs text-gray-500">Node {edge.node_id ?? 'n/a'} · {new Date(edge.created_at).toLocaleString()}</div>
              </div>
            {/each}
            {#if columnLineage.length === 0}
              <div class="text-sm text-gray-500">Run the pipeline after defining output datasets and column mappings to populate lineage.</div>
            {/if}
          </div>
        </div>
      </div>
    </section>
  </div>
</div>
