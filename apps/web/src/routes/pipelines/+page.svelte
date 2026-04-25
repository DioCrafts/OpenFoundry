<script lang="ts">
  import { onMount } from 'svelte';

  import {
    askCopilot,
    listKnowledgeBases,
    listProviders,
    type CopilotResponse,
    type KnowledgeBase,
    type LlmProvider,
  } from '$lib/api/ai';
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
  import {
    getRuntime,
    listTopologies,
    runTopology,
    type TopologyDefinition,
    type TopologyRuntimeSnapshot,
  } from '$lib/api/streaming';
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
  let topologies = $state<TopologyDefinition[]>([]);
  let topologyRuntime = $state<TopologyRuntimeSnapshot | null>(null);
  let llmProviders = $state<LlmProvider[]>([]);
  let knowledgeBases = $state<KnowledgeBase[]>([]);
  let loading = $state(true);
  let saving = $state(false);
  let running = $state(false);
  let loadingTopologyRuntime = $state(false);
  let runningTopologyId = $state('');
  let search = $state('');
  let selectedPipelineId = $state('');
  let selectedTopologyId = $state('');
  let error = $state('');
  let streamingError = $state('');
  let copilotQuestion = $state('Design a hybrid pipeline that enriches the latest records, validates drift, and keeps a streaming companion topology healthy.');
  let copilotLoading = $state(false);
  let copilotResponse = $state<CopilotResponse | null>(null);
  let copilotError = $state('');
  let draft = $state<PipelineDraft>(createEmptyPipeline());
  let buildMode = $state<'incremental' | 'full_rebuild'>('incremental');

  type HybridCanvasMode = 'hybrid' | 'batch';
  let canvasMode = $state<HybridCanvasMode>('hybrid');

  type SuggestedTransform = 'spark' | 'external' | 'llm' | 'pyspark';

  function makeId() {
    if (typeof crypto !== 'undefined' && 'randomUUID' in crypto) {
      return crypto.randomUUID();
    }

    return `node_${Date.now()}_${Math.floor(Math.random() * 10_000)}`;
  }

  function transformTypeLabel(transformType: string) {
    if (transformType === 'pyspark') return 'PySpark';
    if (transformType === 'llm') return 'LLM';
    if (transformType === 'wasm') return 'WASM';
    return transformType.charAt(0).toUpperCase() + transformType.slice(1);
  }

  function isSparkFamily(node: PipelineNode) {
    return node.transform_type === 'spark' || node.transform_type === 'pyspark';
  }

  function isRemoteComputeNode(node: PipelineNode) {
    return isSparkFamily(node) || node.transform_type === 'external';
  }

  function createNode(transformType = 'sql'): PipelineNode {
    const config: Record<string, unknown> =
      transformType === 'sql'
        ? { sql: 'SELECT 1 AS value' }
        : transformType === 'python'
          ? { source: 'rows_affected = 0\nresult = "python transform ready"' }
          : transformType === 'llm'
            ? {
                system_prompt: 'You are an OpenFoundry pipeline LLM transform. Produce concise, factual outputs.',
                prompt:
                  'Classify or enrich the following record and return the result.\n\n{{input_json}}',
                input_field: '',
                output_field: 'llm_response',
                response_format: 'text',
                flatten_json_output: false,
                preserve_input: true,
                fallback_enabled: true,
                max_rows: 25,
                max_tokens: 256,
                temperature: 0.2,
                knowledge_base_id: '',
                preferred_provider_id: '',
              }
          : transformType === 'wasm'
            ? { module: '(module (func (export "run") (result i32) i32.const 0))', function: 'run' }
            : transformType === 'spark' || transformType === 'pyspark'
              ? {
                  endpoint: '',
                  auth_mode: 'service_jwt',
                  job_type: 'spark-batch',
                  cluster_profile: 'shared',
                  entrypoint: 'main',
                  runtime: transformType,
                  source:
                    'from pyspark.sql import functions as F\n# Remote Spark runner receives config + prepared inputs.\n# Emit result_rows from the external compute service.',
                }
              : transformType === 'external'
                ? {
                    endpoint: '',
                    auth_mode: 'none',
                    job_type: 'external-job',
                    source: '{\n  "operation": "enrich",\n  "mode": "append"\n}',
                  }
                : { identity_columns: [] };

    return {
      id: makeId(),
      label: `${transformTypeLabel(transformType)} transform`,
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

  function topologyStatusBadge(status: string) {
    const colors: Record<string, string> = {
      draft: 'bg-gray-200 text-gray-700 dark:bg-gray-700 dark:text-gray-300',
      active: 'bg-emerald-100 text-emerald-700 dark:bg-emerald-900 dark:text-emerald-300',
      running: 'bg-sky-100 text-sky-700 dark:bg-sky-900 dark:text-sky-300',
      paused: 'bg-amber-100 text-amber-700 dark:bg-amber-900 dark:text-amber-300',
      archived: 'bg-slate-200 text-slate-700 dark:bg-slate-800 dark:text-slate-300',
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

  function stringConfig(node: PipelineNode, key: string, fallback = '') {
    return typeof node.config?.[key] === 'string' ? String(node.config[key]) : fallback;
  }

  function numericConfig(node: PipelineNode, key: string, fallback = 0) {
    const value = node.config?.[key];
    if (typeof value === 'number' && Number.isFinite(value)) return value;
    if (typeof value === 'string') {
      const parsed = Number(value);
      if (Number.isFinite(parsed)) return parsed;
    }
    return fallback;
  }

  function booleanConfig(node: PipelineNode, key: string, fallback = false) {
    const value = node.config?.[key];
    return typeof value === 'boolean' ? value : fallback;
  }

  function identityColumns(node: PipelineNode) {
    const columns = node.config?.['identity_columns'];
    return Array.isArray(columns) ? columns.filter((value): value is string => typeof value === 'string').join(', ') : '';
  }

  function nodeCodeKey(node: PipelineNode) {
    if (node.transform_type === 'sql') return 'sql';
    if (node.transform_type === 'llm') return 'prompt';
    if (node.transform_type === 'wasm') return 'module';
    return 'source';
  }

  function nodeCode(node: PipelineNode) {
    if (node.transform_type === 'sql') return stringConfig(node, 'sql');
    if (node.transform_type === 'python' || isRemoteComputeNode(node)) {
      return stringConfig(node, 'source');
    }
    if (node.transform_type === 'llm') return stringConfig(node, 'prompt');
    if (node.transform_type === 'wasm') return stringConfig(node, 'module');
    return '';
  }

  function nodeCodeLabel(node: PipelineNode) {
    if (node.transform_type === 'sql') return 'SQL';
    if (node.transform_type === 'python') return 'Python source';
    if (node.transform_type === 'llm') return 'Prompt template';
    if (node.transform_type === 'wasm') return 'WASM module';
    if (node.transform_type === 'spark') return 'Remote Spark job spec';
    if (node.transform_type === 'pyspark') return 'Remote PySpark job spec';
    if (node.transform_type === 'external') return 'External compute payload';
    return 'Passthrough config';
  }

  function wasmFunction(node: PipelineNode) {
    return typeof node.config?.['function'] === 'string' ? String(node.config['function']) : 'run';
  }

  function branchValue(node: PipelineNode, key: 'input_branch' | 'output_branch') {
    return typeof node.config?.[key] === 'string' ? String(node.config[key]) : '';
  }

  function pipelineDatasetContext() {
    return Array.from(new Set(draft.nodes.flatMap((node) => node.input_dataset_ids)));
  }

  function builderInsights() {
    return {
      batchNodes: draft.nodes.length,
      outputDatasets: new Set(draft.nodes.map((node) => node.output_dataset_id).filter(Boolean)).size,
      topologyCount: topologies.length,
      streamingEdges: topologyRuntime?.topology.edges.length ?? 0,
    };
  }

  function buildContext(run: PipelineRun) {
    const build = run.execution_context?.['build'];
    return build && typeof build === 'object' ? build as Record<string, unknown> : null;
  }

  function buildModeLabel(run: PipelineRun) {
    return buildContext(run)?.['mode'] === 'full_rebuild' ? 'Full rebuild' : 'Incremental';
  }

  function buildMetric(run: PipelineRun, key: string) {
    const value = buildContext(run)?.[key];
    return typeof value === 'number' ? value : 0;
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
    copilotResponse = null;
    copilotError = '';
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

  async function loadAiRegistry() {
    try {
      const [providerResponse, knowledgeBaseResponse] = await Promise.all([
        listProviders(),
        listKnowledgeBases(),
      ]);
      llmProviders = providerResponse.data;
      knowledgeBases = knowledgeBaseResponse.data;
    } catch (cause) {
      console.warn('Failed to load AI registry for pipeline builder', cause);
      llmProviders = [];
      knowledgeBases = [];
    }
  }

  async function loadTopologyRuntime(id: string) {
    if (!id) {
      topologyRuntime = null;
      selectedTopologyId = '';
      return;
    }

    loadingTopologyRuntime = true;
    streamingError = '';
    selectedTopologyId = id;
    try {
      topologyRuntime = await getRuntime(id);
    } catch (cause) {
      console.error('Failed to load topology runtime', cause);
      streamingError = cause instanceof Error ? cause.message : 'Failed to load topology runtime';
      topologyRuntime = null;
    } finally {
      loadingTopologyRuntime = false;
    }
  }

  async function loadStreamingRegistry() {
    streamingError = '';
    try {
      const response = await listTopologies();
      topologies = response.data;
      const nextTopologyId =
        topologies.find((topology) => topology.id === selectedTopologyId)?.id ??
        topologies[0]?.id ??
        '';
      await loadTopologyRuntime(nextTopologyId);
    } catch (cause) {
      console.error('Failed to load streaming topologies', cause);
      streamingError = cause instanceof Error ? cause.message : 'Failed to load streaming topologies';
      topologies = [];
      topologyRuntime = null;
      selectedTopologyId = '';
    }
  }

  async function runStreamingTopology(topologyId: string) {
    if (!topologyId) return;
    runningTopologyId = topologyId;
    streamingError = '';
    try {
      await runTopology(topologyId);
      notifications.success('Streaming topology run started');
      await loadTopologyRuntime(topologyId);
    } catch (cause) {
      streamingError = cause instanceof Error ? cause.message : 'Failed to run topology';
      notifications.error(streamingError);
    } finally {
      runningTopologyId = '';
    }
  }

  async function requestCopilotPlan() {
    if (!copilotQuestion.trim()) {
      notifications.error('Describe what you want the builder to generate first.');
      return;
    }

    copilotLoading = true;
    copilotError = '';
    try {
      copilotResponse = await askCopilot({
        question: copilotQuestion.trim(),
        dataset_ids: pipelineDatasetContext(),
        include_sql: true,
        include_pipeline_plan: true,
      });
      notifications.success('Builder copilot updated the draft guidance');
    } catch (cause) {
      copilotError = cause instanceof Error ? cause.message : 'Failed to query builder copilot';
      notifications.error(copilotError);
    } finally {
      copilotLoading = false;
    }
  }

  function applySuggestedSqlNode() {
    if (!copilotResponse?.suggested_sql) return;
    const node = createNode('sql');
    node.label = 'AI SQL draft';
    node.config = { ...node.config, sql: copilotResponse.suggested_sql };
    draft = { ...draft, nodes: [...draft.nodes, node] };
    notifications.success('AI SQL node added to the pipeline draft');
  }

  function applySuggestedNode(transformType: SuggestedTransform) {
    const node = createNode(transformType);
    const suggestion = copilotResponse?.pipeline_suggestions[0];
    if (suggestion) {
      node.label = suggestion.length > 48 ? `${suggestion.slice(0, 45)}...` : suggestion;
    }
    draft = { ...draft, nodes: [...draft.nodes, node] };
    notifications.success(`${transformTypeLabel(transformType)} node added`);
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
      await Promise.all([loadRegistry(), loadStreamingRegistry(), loadAiRegistry()]);
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
      await triggerRun(draft.id, {
        skip_unchanged: buildMode !== 'full_rebuild',
        context: {
          requested_build_mode: buildMode,
        },
      });
      notifications.success(buildMode === 'full_rebuild' ? 'Full rebuild started' : 'Incremental pipeline run started');
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
      await retryPipelineRun(draft.id, run.id, {
        ...(failedNode ? { from_node_id: failedNode } : {}),
        skip_unchanged: buildMode !== 'full_rebuild',
      });
      notifications.success(partial ? 'Partial re-execution started' : buildMode === 'full_rebuild' ? 'Full rebuild retry started' : 'Incremental retry started');
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
      <p class="mt-1 text-sm text-gray-500">Author hybrid batch and streaming flows with SQL, Python, PySpark, LLM transforms, external compute, in-builder AI assist, lineage, and rerun controls.</p>
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

        <div class="mt-4 space-y-3">
          <div class="flex flex-wrap items-center gap-2">
            <span class="text-sm font-medium">Build mode</span>
            <button type="button" onclick={() => buildMode = 'incremental'} class={`rounded-xl px-3 py-2 text-sm ${buildMode === 'incremental' ? 'bg-slate-900 text-white dark:bg-white dark:text-slate-900' : 'border border-slate-200 hover:bg-slate-50 dark:border-gray-700 dark:hover:bg-gray-800'}`}>Incremental</button>
            <button type="button" onclick={() => buildMode = 'full_rebuild'} class={`rounded-xl px-3 py-2 text-sm ${buildMode === 'full_rebuild' ? 'bg-slate-900 text-white dark:bg-white dark:text-slate-900' : 'border border-slate-200 hover:bg-slate-50 dark:border-gray-700 dark:hover:bg-gray-800'}`}>Full rebuild</button>
            <span class="text-xs text-gray-500">{buildMode === 'incremental' ? 'Reuse unchanged node results when fingerprints match.' : 'Force every node to re-run even if inputs are unchanged.'}</span>
          </div>

          <div class="flex flex-wrap gap-2">
            <button type="button" onclick={() => void runPipeline()} disabled={!draft.id || running} class="rounded-xl bg-slate-900 px-4 py-2 text-white disabled:opacity-50 dark:bg-white dark:text-slate-900">
              {running ? 'Running...' : buildMode === 'full_rebuild' ? 'Run full rebuild' : 'Run incremental'}
            </button>
            <button type="button" onclick={() => { buildMode = 'full_rebuild'; void runPipeline(); }} disabled={!draft.id || running || buildMode === 'full_rebuild'} class="rounded-xl border border-slate-200 px-4 py-2 disabled:opacity-50 hover:bg-slate-50 dark:border-gray-700 dark:hover:bg-gray-800">
              Force full rebuild
            </button>
            <button type="button" onclick={() => { buildMode = 'incremental'; void runPipeline(); }} disabled={!draft.id || running || buildMode === 'incremental'} class="rounded-xl border border-slate-200 px-4 py-2 disabled:opacity-50 hover:bg-slate-50 dark:border-gray-700 dark:hover:bg-gray-800">
              Force incremental
            </button>
          </div>

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
            <div class="mt-1 text-sm text-gray-500">Author hybrid batch and streaming orchestration with remote Spark, PySpark, LLM transforms, external compute, in-builder AI assist, and shared operational context.</div>
          </div>
          <div class="flex flex-wrap gap-2">
            <button type="button" onclick={() => canvasMode = 'hybrid'} class={`rounded-xl px-3 py-2 text-sm ${canvasMode === 'hybrid' ? 'bg-slate-900 text-white dark:bg-white dark:text-slate-900' : 'border border-slate-200 hover:bg-slate-50 dark:border-gray-700 dark:hover:bg-gray-800'}`}>Hybrid view</button>
            <button type="button" onclick={() => canvasMode = 'batch'} class={`rounded-xl px-3 py-2 text-sm ${canvasMode === 'batch' ? 'bg-slate-900 text-white dark:bg-white dark:text-slate-900' : 'border border-slate-200 hover:bg-slate-50 dark:border-gray-700 dark:hover:bg-gray-800'}`}>Batch only</button>
            <button type="button" onclick={() => addNode('sql')} class="rounded-xl border border-slate-200 px-3 py-2 text-sm hover:bg-slate-50 dark:border-gray-700 dark:hover:bg-gray-800">+ SQL</button>
            <button type="button" onclick={() => addNode('python')} class="rounded-xl border border-slate-200 px-3 py-2 text-sm hover:bg-slate-50 dark:border-gray-700 dark:hover:bg-gray-800">+ Python</button>
            <button type="button" onclick={() => addNode('spark')} class="rounded-xl border border-slate-200 px-3 py-2 text-sm hover:bg-slate-50 dark:border-gray-700 dark:hover:bg-gray-800">+ Spark</button>
            <button type="button" onclick={() => addNode('pyspark')} class="rounded-xl border border-slate-200 px-3 py-2 text-sm hover:bg-slate-50 dark:border-gray-700 dark:hover:bg-gray-800">+ PySpark</button>
            <button type="button" onclick={() => addNode('llm')} class="rounded-xl border border-slate-200 px-3 py-2 text-sm hover:bg-slate-50 dark:border-gray-700 dark:hover:bg-gray-800">+ LLM</button>
            <button type="button" onclick={() => addNode('external')} class="rounded-xl border border-slate-200 px-3 py-2 text-sm hover:bg-slate-50 dark:border-gray-700 dark:hover:bg-gray-800">+ External</button>
            <button type="button" onclick={() => addNode('wasm')} class="rounded-xl border border-slate-200 px-3 py-2 text-sm hover:bg-slate-50 dark:border-gray-700 dark:hover:bg-gray-800">+ WASM</button>
            <button type="button" onclick={() => addNode('passthrough')} class="rounded-xl border border-slate-200 px-3 py-2 text-sm hover:bg-slate-50 dark:border-gray-700 dark:hover:bg-gray-800">+ Passthrough</button>
          </div>
        </div>

        <div class="mt-5 grid gap-3 md:grid-cols-4">
          <div class="rounded-2xl border border-slate-200 bg-slate-50 p-4 dark:border-gray-700 dark:bg-gray-950/40">
            <div class="text-xs uppercase tracking-[0.22em] text-gray-400">Batch nodes</div>
            <div class="mt-2 text-2xl font-semibold">{builderInsights().batchNodes}</div>
            <div class="mt-1 text-xs text-gray-500">Active DAG steps in this draft.</div>
          </div>
          <div class="rounded-2xl border border-slate-200 bg-slate-50 p-4 dark:border-gray-700 dark:bg-gray-950/40">
            <div class="text-xs uppercase tracking-[0.22em] text-gray-400">Outputs</div>
            <div class="mt-2 text-2xl font-semibold">{builderInsights().outputDatasets}</div>
            <div class="mt-1 text-xs text-gray-500">Datasets materialized from the flow.</div>
          </div>
          <div class="rounded-2xl border border-slate-200 bg-slate-50 p-4 dark:border-gray-700 dark:bg-gray-950/40">
            <div class="text-xs uppercase tracking-[0.22em] text-gray-400">Streaming topologies</div>
            <div class="mt-2 text-2xl font-semibold">{builderInsights().topologyCount}</div>
            <div class="mt-1 text-xs text-gray-500">Companion topologies visible in the same builder.</div>
          </div>
          <div class="rounded-2xl border border-slate-200 bg-slate-50 p-4 dark:border-gray-700 dark:bg-gray-950/40">
            <div class="text-xs uppercase tracking-[0.22em] text-gray-400">Streaming edges</div>
            <div class="mt-2 text-2xl font-semibold">{builderInsights().streamingEdges}</div>
            <div class="mt-1 text-xs text-gray-500">Edges on the selected topology runtime view.</div>
          </div>
        </div>

        <div class="mt-5 grid gap-5 xl:grid-cols-[1.25fr,0.75fr]">
          <div class="space-y-4">
            {#each draft.nodes as node, index (node.id)}
              <div class="rounded-2xl border border-slate-200 bg-slate-50/70 p-4 shadow-sm dark:border-gray-700 dark:bg-gray-950/40">
                <div class="flex items-center justify-between gap-3">
                  <div class="flex items-center gap-3">
                    <span class="rounded-full bg-white px-2.5 py-1 text-xs font-semibold uppercase tracking-[0.16em] text-slate-600 dark:bg-gray-900 dark:text-gray-300">{transformTypeLabel(node.transform_type)}</span>
                    <input aria-label="Node label" value={node.label} oninput={(event) => setNodeField(node.id, 'label', (event.currentTarget as HTMLInputElement).value)} class="rounded-xl border border-slate-200 px-3 py-2 text-sm dark:border-gray-700 dark:bg-gray-900" />
                  </div>
                  <div class="flex items-center gap-2">
                    <select aria-label="Transform type" value={node.transform_type} oninput={(event) => setNodeField(node.id, 'transform_type', (event.currentTarget as HTMLSelectElement).value)} class="rounded-xl border border-slate-200 px-3 py-2 text-sm dark:border-gray-700 dark:bg-gray-900">
                      <option value="sql">SQL</option>
                      <option value="python">Python</option>
                      <option value="spark">Spark</option>
                      <option value="pyspark">PySpark</option>
                      <option value="llm">LLM</option>
                      <option value="external">External</option>
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
                      <label for={`node-code-${node.id}`} class="mb-1 block text-sm font-medium">{nodeCodeLabel(node)}</label>
                      {#if node.transform_type === 'passthrough'}
                        <input id={`node-code-${node.id}`} value={identityColumns(node)} oninput={(event) => setIdentityColumns(node.id, (event.currentTarget as HTMLInputElement).value)} placeholder="customer_id, order_id" class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-gray-700 dark:bg-gray-900" />
                      {:else}
                        <textarea id={`node-code-${node.id}`} rows="8" value={nodeCode(node)} oninput={(event) => setNodeConfig(node.id, nodeCodeKey(node), (event.currentTarget as HTMLTextAreaElement).value)} class="w-full rounded-xl border border-slate-200 px-3 py-2 font-mono text-sm dark:border-gray-700 dark:bg-gray-900"></textarea>
                      {/if}
                    </div>

                    {#if node.transform_type === 'wasm'}
                      <div>
                        <label for={`wasm-function-${node.id}`} class="mb-1 block text-sm font-medium">Exported function</label>
                        <input id={`wasm-function-${node.id}`} value={wasmFunction(node)} oninput={(event) => setNodeConfig(node.id, 'function', (event.currentTarget as HTMLInputElement).value)} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-gray-700 dark:bg-gray-900" />
                      </div>
                    {/if}

                    {#if isRemoteComputeNode(node)}
                      <div class="rounded-xl border border-slate-200 p-4 dark:border-gray-700">
                        <div class="grid gap-4 md:grid-cols-2">
                          <div>
                            <label for={`endpoint-${node.id}`} class="mb-1 block text-sm font-medium">Remote endpoint</label>
                            <input id={`endpoint-${node.id}`} value={stringConfig(node, 'endpoint')} oninput={(event) => setNodeConfig(node.id, 'endpoint', (event.currentTarget as HTMLInputElement).value)} placeholder="http://compute.local/jobs/run" class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-gray-700 dark:bg-gray-900" />
                          </div>
                          <div>
                            <label for={`job-type-${node.id}`} class="mb-1 block text-sm font-medium">{isSparkFamily(node) ? 'Job type' : 'External job type'}</label>
                            <input id={`job-type-${node.id}`} value={stringConfig(node, 'job_type', isSparkFamily(node) ? 'spark-batch' : 'external-job')} oninput={(event) => setNodeConfig(node.id, 'job_type', (event.currentTarget as HTMLInputElement).value)} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-gray-700 dark:bg-gray-900" />
                          </div>
                          <div>
                            <label for={`auth-mode-${node.id}`} class="mb-1 block text-sm font-medium">Auth mode</label>
                            <select id={`auth-mode-${node.id}`} value={stringConfig(node, 'auth_mode', isSparkFamily(node) ? 'service_jwt' : 'none')} oninput={(event) => setNodeConfig(node.id, 'auth_mode', (event.currentTarget as HTMLSelectElement).value)} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-gray-700 dark:bg-gray-900">
                              <option value="none">None</option>
                              <option value="service_jwt">Service JWT</option>
                            </select>
                          </div>
                          <div>
                            <label for={`profile-${node.id}`} class="mb-1 block text-sm font-medium">{isSparkFamily(node) ? 'Cluster profile' : 'Entrypoint'}</label>
                            <input id={`profile-${node.id}`} value={stringConfig(node, isSparkFamily(node) ? 'cluster_profile' : 'entrypoint', isSparkFamily(node) ? 'shared' : 'main')} oninput={(event) => setNodeConfig(node.id, isSparkFamily(node) ? 'cluster_profile' : 'entrypoint', (event.currentTarget as HTMLInputElement).value)} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-gray-700 dark:bg-gray-900" />
                          </div>
                        </div>
                        <div class="mt-3 rounded-xl bg-slate-100 px-3 py-3 text-xs text-slate-600 dark:bg-slate-950/60 dark:text-slate-300">
                          Remote compute contract: the builder posts `job_type`, full `config`, prepared input rows, and the optional `output_dataset_id`. The remote runner should return JSON with `status`, optional `output`, and optional `result_rows`.
                        </div>
                      </div>
                    {/if}

                    {#if node.transform_type === 'llm'}
                      <div class="rounded-xl border border-slate-200 p-4 dark:border-gray-700">
                        <div class="grid gap-4 md:grid-cols-2">
                          <div>
                            <label for={`llm-input-field-${node.id}`} class="mb-1 block text-sm font-medium">Input field</label>
                            <input id={`llm-input-field-${node.id}`} value={stringConfig(node, 'input_field')} oninput={(event) => setNodeConfig(node.id, 'input_field', (event.currentTarget as HTMLInputElement).value)} placeholder="review_text" class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-gray-700 dark:bg-gray-900" />
                          </div>
                          <div>
                            <label for={`llm-output-field-${node.id}`} class="mb-1 block text-sm font-medium">Output field</label>
                            <input id={`llm-output-field-${node.id}`} value={stringConfig(node, 'output_field', 'llm_response')} oninput={(event) => setNodeConfig(node.id, 'output_field', (event.currentTarget as HTMLInputElement).value)} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-gray-700 dark:bg-gray-900" />
                          </div>
                          <div>
                            <label for={`llm-provider-${node.id}`} class="mb-1 block text-sm font-medium">Preferred provider</label>
                            <select id={`llm-provider-${node.id}`} value={stringConfig(node, 'preferred_provider_id')} oninput={(event) => setNodeConfig(node.id, 'preferred_provider_id', (event.currentTarget as HTMLSelectElement).value)} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-gray-700 dark:bg-gray-900">
                              <option value="">Auto-route provider</option>
                              {#each llmProviders as provider (provider.id)}
                                <option value={provider.id}>{provider.name} · {provider.model_name}</option>
                              {/each}
                            </select>
                          </div>
                          <div>
                            <label for={`llm-kb-${node.id}`} class="mb-1 block text-sm font-medium">Knowledge base</label>
                            <select id={`llm-kb-${node.id}`} value={stringConfig(node, 'knowledge_base_id')} oninput={(event) => setNodeConfig(node.id, 'knowledge_base_id', (event.currentTarget as HTMLSelectElement).value)} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-gray-700 dark:bg-gray-900">
                              <option value="">No retrieval</option>
                              {#each knowledgeBases as knowledgeBase (knowledgeBase.id)}
                                <option value={knowledgeBase.id}>{knowledgeBase.name}</option>
                              {/each}
                            </select>
                          </div>
                        </div>

                        <div class="mt-4">
                          <label for={`llm-system-${node.id}`} class="mb-1 block text-sm font-medium">System prompt</label>
                          <textarea id={`llm-system-${node.id}`} rows="3" value={stringConfig(node, 'system_prompt')} oninput={(event) => setNodeConfig(node.id, 'system_prompt', (event.currentTarget as HTMLTextAreaElement).value)} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-gray-700 dark:bg-gray-900"></textarea>
                        </div>

                        <div class="mt-4 grid gap-4 md:grid-cols-2">
                          <div>
                            <label for={`llm-response-format-${node.id}`} class="mb-1 block text-sm font-medium">Response format</label>
                            <select id={`llm-response-format-${node.id}`} value={stringConfig(node, 'response_format', 'text')} oninput={(event) => setNodeConfig(node.id, 'response_format', (event.currentTarget as HTMLSelectElement).value)} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-gray-700 dark:bg-gray-900">
                              <option value="text">Text</option>
                              <option value="json">JSON</option>
                            </select>
                          </div>
                          <div>
                            <label for={`llm-max-rows-${node.id}`} class="mb-1 block text-sm font-medium">Max rows per run</label>
                            <input id={`llm-max-rows-${node.id}`} type="number" min="1" value={numericConfig(node, 'max_rows', 25)} oninput={(event) => setNodeConfig(node.id, 'max_rows', Number((event.currentTarget as HTMLInputElement).value || 25))} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-gray-700 dark:bg-gray-900" />
                          </div>
                          <div>
                            <label for={`llm-max-tokens-${node.id}`} class="mb-1 block text-sm font-medium">Max tokens</label>
                            <input id={`llm-max-tokens-${node.id}`} type="number" min="32" step="32" value={numericConfig(node, 'max_tokens', 256)} oninput={(event) => setNodeConfig(node.id, 'max_tokens', Number((event.currentTarget as HTMLInputElement).value || 256))} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-gray-700 dark:bg-gray-900" />
                          </div>
                          <div>
                            <label for={`llm-temperature-${node.id}`} class="mb-1 block text-sm font-medium">Temperature</label>
                            <input id={`llm-temperature-${node.id}`} type="number" min="0" max="2" step="0.1" value={numericConfig(node, 'temperature', 0.2)} oninput={(event) => setNodeConfig(node.id, 'temperature', Number((event.currentTarget as HTMLInputElement).value || 0.2))} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-gray-700 dark:bg-gray-900" />
                          </div>
                        </div>

                        <div class="mt-4 grid gap-3 md:grid-cols-3 text-sm">
                          <label class="flex items-center justify-between rounded-xl border border-slate-200 px-3 py-2 dark:border-gray-700">
                            <span>Preserve input fields</span>
                            <input type="checkbox" checked={booleanConfig(node, 'preserve_input', true)} onchange={(event) => setNodeConfig(node.id, 'preserve_input', (event.currentTarget as HTMLInputElement).checked)} />
                          </label>
                          <label class="flex items-center justify-between rounded-xl border border-slate-200 px-3 py-2 dark:border-gray-700">
                            <span>Flatten JSON output</span>
                            <input type="checkbox" checked={booleanConfig(node, 'flatten_json_output', false)} onchange={(event) => setNodeConfig(node.id, 'flatten_json_output', (event.currentTarget as HTMLInputElement).checked)} />
                          </label>
                          <label class="flex items-center justify-between rounded-xl border border-slate-200 px-3 py-2 dark:border-gray-700">
                            <span>Fallback providers</span>
                            <input type="checkbox" checked={booleanConfig(node, 'fallback_enabled', true)} onchange={(event) => setNodeConfig(node.id, 'fallback_enabled', (event.currentTarget as HTMLInputElement).checked)} />
                          </label>
                        </div>

                        <div class="mt-3 rounded-xl bg-slate-100 px-3 py-3 text-xs text-slate-600 dark:bg-slate-950/60 dark:text-slate-300">
                          Prompt placeholders: <code>{'{{input_json}}'}</code>, <code>{'{{input_text}}'}</code>, <code>{'{{dataset_name}}'}</code>, <code>{'{{dataset_id}}'}</code>, <code>{'{{dataset_alias}}'}</code>, <code>{'{{row_index}}'}</code>, <code>{'{{row_count}}'}</code>, <code>{'{{input_rows_json}}'}</code>.
                        </div>
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

          {#if canvasMode === 'hybrid'}
            <div class="space-y-4">
              <div class="rounded-2xl border border-slate-200 bg-slate-50 p-4 dark:border-gray-700 dark:bg-gray-950/40">
                <div class="flex items-center justify-between gap-3">
                  <div>
                    <div class="text-xs uppercase tracking-[0.22em] text-gray-400">AI builder assist</div>
                    <div class="mt-1 text-sm text-gray-500">Generate starter SQL plus LLM, PySpark, Spark, and external node ideas from the datasets already wired into the draft.</div>
                  </div>
                  <button type="button" onclick={() => void requestCopilotPlan()} disabled={copilotLoading} class="rounded-xl bg-slate-900 px-3 py-2 text-sm text-white disabled:opacity-50 dark:bg-white dark:text-slate-900">
                    {copilotLoading ? 'Thinking...' : 'Ask Copilot'}
                  </button>
                </div>

                <textarea rows="5" bind:value={copilotQuestion} class="mt-4 w-full rounded-2xl border border-slate-200 bg-white px-4 py-3 text-sm dark:border-gray-700 dark:bg-gray-900"></textarea>

                <div class="mt-3 text-xs text-gray-500">Dataset context: {pipelineDatasetContext().length} linked input dataset(s).</div>

                {#if copilotError}
                  <div class="mt-4 rounded-xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-700 dark:border-rose-900/40 dark:bg-rose-950/40 dark:text-rose-300">{copilotError}</div>
                {/if}

                {#if copilotResponse}
                  <div class="mt-4 space-y-4 rounded-2xl border border-slate-200 bg-white p-4 dark:border-gray-700 dark:bg-gray-900">
                    <div class="flex flex-wrap items-center gap-2 text-xs text-gray-500">
                      <span class="rounded-full bg-slate-100 px-2.5 py-1 dark:bg-gray-800">{copilotResponse.provider_name}</span>
                      <span class="rounded-full bg-slate-100 px-2.5 py-1 dark:bg-gray-800">Cache {copilotResponse.cache.hit ? 'hit' : 'miss'}</span>
                    </div>
                    <p class="text-sm leading-6 text-slate-700 dark:text-slate-200">{copilotResponse.answer}</p>
                    {#if copilotResponse.suggested_sql}
                      <div>
                        <div class="text-xs font-semibold uppercase tracking-[0.22em] text-gray-400">Suggested SQL</div>
                        <pre class="mt-2 overflow-x-auto rounded-2xl bg-slate-950 px-4 py-3 text-xs text-cyan-100">{copilotResponse.suggested_sql}</pre>
                        <button type="button" onclick={applySuggestedSqlNode} class="mt-3 rounded-xl border border-slate-200 px-3 py-2 text-sm hover:bg-slate-50 dark:border-gray-700 dark:hover:bg-gray-800">Add SQL node</button>
                      </div>
                    {/if}
                    {#if copilotResponse.pipeline_suggestions.length > 0}
                      <div>
                        <div class="text-xs font-semibold uppercase tracking-[0.22em] text-gray-400">Suggested nodes</div>
                        <div class="mt-2 space-y-2">
                          {#each copilotResponse.pipeline_suggestions as suggestion}
                            <div class="rounded-xl border border-slate-200 px-3 py-2 text-sm dark:border-gray-700">{suggestion}</div>
                          {/each}
                        </div>
                        <div class="mt-3 flex flex-wrap gap-2">
                          <button type="button" onclick={() => applySuggestedNode('llm')} class="rounded-xl border border-slate-200 px-3 py-2 text-sm hover:bg-slate-50 dark:border-gray-700 dark:hover:bg-gray-800">Add LLM node</button>
                          <button type="button" onclick={() => applySuggestedNode('pyspark')} class="rounded-xl border border-slate-200 px-3 py-2 text-sm hover:bg-slate-50 dark:border-gray-700 dark:hover:bg-gray-800">Add PySpark node</button>
                          <button type="button" onclick={() => applySuggestedNode('spark')} class="rounded-xl border border-slate-200 px-3 py-2 text-sm hover:bg-slate-50 dark:border-gray-700 dark:hover:bg-gray-800">Add Spark node</button>
                          <button type="button" onclick={() => applySuggestedNode('external')} class="rounded-xl border border-slate-200 px-3 py-2 text-sm hover:bg-slate-50 dark:border-gray-700 dark:hover:bg-gray-800">Add External node</button>
                        </div>
                      </div>
                    {/if}
                  </div>
                {/if}
              </div>

              <div class="rounded-2xl border border-slate-200 bg-slate-50 p-4 dark:border-gray-700 dark:bg-gray-950/40">
                <div class="flex items-center justify-between gap-3">
                  <div>
                    <div class="text-xs uppercase tracking-[0.22em] text-gray-400">Streaming companion</div>
                    <div class="mt-1 text-sm text-gray-500">Inspect and run real stream topologies from the same builder lane.</div>
                  </div>
                  <button type="button" onclick={() => void loadStreamingRegistry()} class="rounded-xl border border-slate-200 px-3 py-2 text-sm hover:bg-slate-50 dark:border-gray-700 dark:hover:bg-gray-800">Refresh</button>
                </div>

                {#if streamingError}
                  <div class="mt-4 rounded-xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-700 dark:border-rose-900/40 dark:bg-rose-950/40 dark:text-rose-300">{streamingError}</div>
                {/if}

                {#if topologies.length === 0}
                  <div class="mt-4 rounded-xl border border-dashed border-slate-300 px-4 py-8 text-sm text-gray-500 dark:border-gray-700">No streaming topologies yet. Open the streaming area to create one, then come back here for hybrid operations.</div>
                {:else}
                  <div class="mt-4 grid gap-4 xl:grid-cols-[0.9fr,1.1fr]">
                    <div class="space-y-2">
                      {#each topologies as topology (topology.id)}
                        <button type="button" onclick={() => void loadTopologyRuntime(topology.id)} class={`w-full rounded-xl border px-3 py-3 text-left ${selectedTopologyId === topology.id ? 'border-sky-500 bg-sky-50 dark:border-sky-400 dark:bg-sky-950/30' : 'border-slate-200 bg-white hover:bg-slate-50 dark:border-gray-700 dark:bg-gray-900 dark:hover:bg-gray-800'}`}>
                          <div class="flex items-center justify-between gap-3">
                            <div>
                              <div class="font-medium">{topology.name}</div>
                              <div class="mt-1 text-xs text-gray-500">{topology.description || 'No description'}</div>
                            </div>
                            <span class={`rounded-full px-2.5 py-1 text-xs font-medium ${topologyStatusBadge(topology.status)}`}>{topology.status}</span>
                          </div>
                          <div class="mt-2 flex flex-wrap gap-2 text-xs text-gray-500">
                            <span>{topology.nodes.length} nodes</span>
                            <span>{topology.edges.length} edges</span>
                            <span>{topology.source_stream_ids.length} streams</span>
                          </div>
                        </button>
                      {/each}
                    </div>

                    <div class="rounded-2xl border border-slate-200 bg-white p-4 dark:border-gray-700 dark:bg-gray-900">
                      {#if loadingTopologyRuntime}
                        <div class="py-10 text-center text-sm text-gray-500">Loading runtime...</div>
                      {:else if topologyRuntime}
                        <div class="space-y-4">
                          <div class="flex items-center justify-between gap-3">
                            <div>
                              <div class="font-medium">{topologyRuntime.topology.name}</div>
                              <div class="mt-1 text-xs text-gray-500">{topologyRuntime.topology.state_backend} state backend</div>
                            </div>
                            <button type="button" onclick={() => void runStreamingTopology(topologyRuntime?.topology.id ?? '')} disabled={runningTopologyId === (topologyRuntime?.topology.id ?? '')} class="rounded-xl bg-slate-900 px-3 py-2 text-sm text-white disabled:opacity-50 dark:bg-white dark:text-slate-900">
                              {runningTopologyId === (topologyRuntime?.topology.id ?? '') ? 'Running...' : 'Run topology'}
                            </button>
                          </div>

                          <div class="grid gap-3 md:grid-cols-2">
                            <div class="rounded-xl border border-slate-200 p-3 dark:border-gray-700">
                              <div class="text-xs uppercase tracking-[0.22em] text-gray-400">Latest run</div>
                              <div class="mt-2 text-sm font-medium">{topologyRuntime.latest_run?.status ?? (topologyRuntime.preview ? 'Live preview' : 'No runs yet')}</div>
                              <div class="mt-1 text-xs text-gray-500">{topologyRuntime.latest_run ? `${topologyRuntime.latest_run.metrics.input_events} input events` : topologyRuntime.preview ? `${topologyRuntime.preview.backlog_events} backlog event(s) ready to process` : 'Trigger the topology to capture runtime metrics.'}</div>
                            </div>
                            <div class="rounded-xl border border-slate-200 p-3 dark:border-gray-700">
                              <div class="text-xs uppercase tracking-[0.22em] text-gray-400">Latency</div>
                              <div class="mt-2 text-sm font-medium">{topologyRuntime.latest_run?.metrics.avg_latency_ms ?? topologyRuntime.preview?.metrics.avg_latency_ms ?? 0} ms avg</div>
                              <div class="mt-1 text-xs text-gray-500">{topologyRuntime.latest_run?.metrics.throughput_per_second ?? topologyRuntime.preview?.metrics.throughput_per_second ?? 0} events/s</div>
                            </div>
                          </div>

                          <div class="rounded-xl border border-slate-200 p-3 dark:border-gray-700">
                            <div class="text-xs uppercase tracking-[0.22em] text-gray-400">Live graph shape</div>
                            <div class="mt-2 grid gap-2 md:grid-cols-3 text-sm">
                              <div>{topologyRuntime.topology.nodes.length} nodes</div>
                              <div>{topologyRuntime.topology.edges.length} edges</div>
                              <div>{topologyRuntime.connector_statuses.length} connectors</div>
                            </div>
                          </div>

                          <div class="rounded-xl border border-slate-200 p-3 dark:border-gray-700">
                            <div class="text-xs uppercase tracking-[0.22em] text-gray-400">Recent events</div>
                            <div class="mt-2 space-y-2">
                              {#each topologyRuntime.latest_events.slice(0, 3) as event (event.id)}
                                <div class="rounded-xl bg-slate-50 px-3 py-2 text-xs dark:bg-gray-950/40">
                                  <div class="font-medium">{event.stream_name}</div>
                                  <div class="mt-1 text-gray-500">{event.connector_type} · {new Date(event.processing_time).toLocaleString()}</div>
                                </div>
                              {/each}
                              {#if topologyRuntime.latest_events.length === 0}
                                <div class="text-xs text-gray-500">No live-tail events captured yet.</div>
                              {/if}
                            </div>
                          </div>
                        </div>
                      {:else}
                        <div class="py-10 text-center text-sm text-gray-500">Select a streaming topology to load its runtime details.</div>
                      {/if}
                    </div>
                  </div>
                {/if}
              </div>
            </div>
          {/if}
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
                      <span class="rounded-full bg-slate-100 px-2.5 py-1 text-xs font-medium text-slate-700 dark:bg-gray-800 dark:text-gray-200">{buildModeLabel(run)}</span>
                    </div>
                    <div class="mt-1 text-sm text-gray-500">Attempt {run.attempt_number} · {nodeResultSummary(run)}</div>
                    <div class="mt-1 text-xs text-gray-500">Started {new Date(run.started_at).toLocaleString()}</div>
                    <div class="mt-1 text-xs text-gray-500">
                      Completed {buildMetric(run, 'completed_nodes')} · Skipped {buildMetric(run, 'skipped_nodes')} · Failed {buildMetric(run, 'failed_nodes')}
                    </div>
                    {#if run.started_from_node_id}
                      <div class="mt-1 text-xs text-gray-500">Partial from node {run.started_from_node_id}</div>
                    {/if}
                    {#if run.error_message}
                      <div class="mt-2 rounded-lg border border-rose-200 bg-rose-50 px-3 py-2 text-xs text-rose-700 dark:border-rose-900/40 dark:bg-rose-950/40 dark:text-rose-300">{run.error_message}</div>
                    {/if}
                  </div>
                  <div class="flex flex-col gap-2">
                    <button type="button" onclick={() => void rerunPipeline(run, false)} disabled={running} class="rounded-lg border border-slate-200 px-3 py-1.5 text-sm disabled:opacity-50 hover:bg-slate-50 dark:border-gray-700 dark:hover:bg-gray-800">
                      {buildMode === 'full_rebuild' ? 'Retry full rebuild' : 'Retry incremental'}
                    </button>
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
