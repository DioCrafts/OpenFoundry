<script lang="ts">
  import { onMount } from 'svelte';
  import cytoscape, { type Core } from 'cytoscape';

  import {
    getDatasetLineageImpact,
    getFullLineage,
    triggerLineageBuilds,
    type LineageBuildResult,
    type LineageGraph,
    type LineageImpactAnalysis,
    type LineageNode,
  } from '$lib/api/pipelines';
  import { notifications } from '$stores/notifications';

  let container = $state<HTMLDivElement | undefined>(undefined);
  let graph = $state<LineageGraph | null>(null);
  let loading = $state(true);
  let impactLoading = $state(false);
  let building = $state(false);
  let error = $state('');
  let selectedNode = $state<LineageNode | null>(null);
  let impact = $state<LineageImpactAnalysis | null>(null);
  let buildResult = $state<LineageBuildResult | null>(null);

  let cy = $state<Core | null>(null);

  const kindPalette: Record<string, string> = {
    dataset: '#0f766e',
    pipeline: '#2563eb',
    workflow: '#d97706',
  };

  const markingPalette: Record<string, string> = {
    public: '#a3a3a3',
    confidential: '#f97316',
    pii: '#ef4444',
  };

  function nodeColor(kind: string) {
    return kindPalette[kind] ?? '#64748b';
  }

  function markingColor(marking: string) {
    return markingPalette[marking] ?? '#a3a3a3';
  }

  async function loadGraph() {
    loading = true;
    error = '';
    try {
      graph = await getFullLineage();
      renderGraph();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : 'Failed to load lineage';
    } finally {
      loading = false;
    }
  }

  async function loadImpact(datasetId: string) {
    impactLoading = true;
    buildResult = null;
    try {
      impact = await getDatasetLineageImpact(datasetId);
    } catch (cause) {
      impact = null;
      notifications.error(cause instanceof Error ? cause.message : 'Failed to load impact analysis');
    } finally {
      impactLoading = false;
    }
  }

  async function triggerBuilds() {
    if (!selectedNode || selectedNode.kind !== 'dataset') return;
    building = true;
    try {
      buildResult = await triggerLineageBuilds(selectedNode.id, {
        include_workflows: true,
        dry_run: false,
        context: {
          initiated_from: 'lineage-explorer',
        },
      });
      notifications.success(`Triggered ${buildResult.triggered.length} downstream build(s)`);
      await loadImpact(selectedNode.id);
    } catch (cause) {
      notifications.error(cause instanceof Error ? cause.message : 'Failed to trigger builds');
    } finally {
      building = false;
    }
  }

  function renderGraph() {
    if (!container || !graph) return;

    cy?.destroy();

    const nodes = graph.nodes.map((node) => ({
      data: {
        id: node.id,
        label: node.label,
        kind: node.kind,
        marking: node.marking,
        color: nodeColor(node.kind),
        borderColor: markingColor(node.marking),
      },
    }));

    const edges = graph.edges.map((edge) => ({
      data: {
        id: edge.id,
        source: edge.source,
        target: edge.target,
        relation: edge.relation_kind,
        color: markingColor(edge.effective_marking),
      },
    }));

    const instance = cytoscape({
      container,
      elements: [...nodes, ...edges],
      style: [
        {
          selector: 'node',
          style: {
            'background-color': 'data(color)',
            label: 'data(label)',
            color: '#e5e7eb',
            'text-wrap': 'wrap',
            'text-max-width': '140',
            'text-valign': 'bottom',
            'text-margin-y': 10,
            'font-size': '11px',
            'font-weight': 600,
            width: 42,
            height: 42,
            'border-width': 3,
            'border-color': 'data(borderColor)',
          },
        },
        {
          selector: 'edge',
          style: {
            width: 2,
            label: 'data(relation)',
            'font-size': '9px',
            color: '#cbd5e1',
            'text-background-color': '#0f172a',
            'text-background-opacity': 0.7,
            'text-background-padding': '2',
            'line-color': 'data(color)',
            'target-arrow-color': 'data(color)',
            'target-arrow-shape': 'triangle',
            'curve-style': 'bezier',
          },
        },
        {
          selector: ':selected',
          style: {
            'overlay-opacity': 0,
            'border-width': 5,
          },
        },
      ],
      layout: {
        name: 'breadthfirst',
        directed: true,
        spacingFactor: 1.45,
      },
    });

    cy = instance;

    instance.on('tap', 'node', (event) => {
      const nodeId = String(event.target.id());
      selectedNode = graph?.nodes.find((node) => node.id === nodeId) ?? null;
      impact = null;
      buildResult = null;
      if (selectedNode?.kind === 'dataset') {
        void loadImpact(selectedNode.id);
      }
    });
  }

  function kindCount(kind: string) {
    return graph?.nodes.filter((node) => node.kind === kind).length ?? 0;
  }

  onMount(() => {
    void loadGraph();
    return () => cy?.destroy();
  });
</script>

<div class="space-y-6">
  <div class="flex items-center justify-between gap-4">
    <div>
      <h1 class="text-2xl font-bold">Operational Lineage</h1>
      <p class="mt-1 text-sm text-gray-500">Explore dataset, pipeline, and workflow dependencies, inspect propagated markings, and trigger downstream rebuilds from the graph.</p>
    </div>
    <button onclick={() => void loadGraph()} class="rounded-xl border border-slate-200 px-4 py-2 hover:bg-slate-50 dark:border-gray-700 dark:hover:bg-gray-800">
      Refresh graph
    </button>
  </div>

  {#if error}
    <div class="rounded-xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-700 dark:border-rose-900/40 dark:bg-rose-950/40 dark:text-rose-300">{error}</div>
  {/if}

  <div class="grid gap-4 sm:grid-cols-3">
    <div class="rounded-2xl border border-slate-200 bg-white p-4 shadow-sm dark:border-gray-700 dark:bg-gray-900">
      <div class="text-xs uppercase tracking-[0.22em] text-gray-400">Datasets</div>
      <div class="mt-3 text-3xl font-semibold text-teal-700 dark:text-teal-300">{kindCount('dataset')}</div>
    </div>
    <div class="rounded-2xl border border-slate-200 bg-white p-4 shadow-sm dark:border-gray-700 dark:bg-gray-900">
      <div class="text-xs uppercase tracking-[0.22em] text-gray-400">Pipelines</div>
      <div class="mt-3 text-3xl font-semibold text-blue-700 dark:text-blue-300">{kindCount('pipeline')}</div>
    </div>
    <div class="rounded-2xl border border-slate-200 bg-white p-4 shadow-sm dark:border-gray-700 dark:bg-gray-900">
      <div class="text-xs uppercase tracking-[0.22em] text-gray-400">Workflows</div>
      <div class="mt-3 text-3xl font-semibold text-amber-700 dark:text-amber-300">{kindCount('workflow')}</div>
    </div>
  </div>

  <div class="grid gap-6 xl:grid-cols-[1.45fr,0.95fr]">
    <section class="rounded-2xl border border-slate-200 bg-white p-5 shadow-sm dark:border-gray-700 dark:bg-gray-900">
      {#if loading}
        <div class="py-20 text-center text-gray-500">Loading lineage graph...</div>
      {:else if !graph || graph.nodes.length === 0}
        <div class="py-20 text-center text-gray-500">No lineage data yet. Run a pipeline or workflow to populate the graph.</div>
      {:else}
        <div class="mb-4 flex items-center justify-between gap-3 text-sm text-gray-500">
          <div>{graph.nodes.length} nodes, {graph.edges.length} relations</div>
          <div class="flex gap-3">
            <span class="inline-flex items-center gap-2"><span class="h-2.5 w-2.5 rounded-full bg-teal-700"></span>Dataset</span>
            <span class="inline-flex items-center gap-2"><span class="h-2.5 w-2.5 rounded-full bg-blue-700"></span>Pipeline</span>
            <span class="inline-flex items-center gap-2"><span class="h-2.5 w-2.5 rounded-full bg-amber-700"></span>Workflow</span>
          </div>
        </div>
        <div bind:this={container} class="w-full rounded-2xl border border-slate-200 dark:border-gray-700" style="height: 680px; background: #0f172a;"></div>
      {/if}
    </section>

    <section class="space-y-4 rounded-2xl border border-slate-200 bg-white p-5 shadow-sm dark:border-gray-700 dark:bg-gray-900">
      {#if !selectedNode}
        <div class="py-16 text-center text-sm text-gray-500">Select a node in the graph to inspect metadata, impact, and build candidates.</div>
      {:else}
        <div>
          <div class="flex items-center justify-between gap-3">
            <div>
              <div class="text-xs uppercase tracking-[0.22em] text-gray-400">{selectedNode.kind}</div>
              <h2 class="mt-1 text-xl font-semibold">{selectedNode.label}</h2>
            </div>
            <span class="rounded-full px-3 py-1 text-xs font-semibold uppercase tracking-[0.18em]" style={`background:${markingColor(selectedNode.marking)}22; color:${markingColor(selectedNode.marking)};`}>
              {selectedNode.marking}
            </span>
          </div>
          <div class="mt-3 text-xs font-mono text-gray-500">{selectedNode.id}</div>
        </div>

        <div class="rounded-xl bg-slate-50 p-4 dark:bg-gray-950">
          <div class="text-xs uppercase tracking-[0.22em] text-gray-400">Node metadata</div>
          <div class="mt-3 space-y-2 text-sm">
            {#if Object.keys(selectedNode.metadata ?? {}).length === 0}
              <div class="text-gray-500">No metadata captured yet.</div>
            {:else}
              {#each Object.entries(selectedNode.metadata ?? {}) as [key, value]}
                <div class="flex items-start justify-between gap-3">
                  <div class="font-medium text-slate-700 dark:text-gray-200">{key}</div>
                  <div class="max-w-[60%] break-words text-right text-gray-500">{typeof value === 'string' ? value : JSON.stringify(value)}</div>
                </div>
              {/each}
            {/if}
          </div>
        </div>

        {#if selectedNode.kind === 'dataset'}
          <div class="flex gap-2">
            <button onclick={() => selectedNode?.id && void loadImpact(selectedNode.id)} disabled={impactLoading} class="rounded-xl border border-slate-200 px-4 py-2 hover:bg-slate-50 disabled:opacity-50 dark:border-gray-700 dark:hover:bg-gray-800">
              {impactLoading ? 'Refreshing impact...' : 'Refresh impact'}
            </button>
            <button onclick={() => void triggerBuilds()} disabled={building || impactLoading} class="rounded-xl bg-blue-600 px-4 py-2 text-white hover:bg-blue-700 disabled:opacity-50">
              {building ? 'Triggering builds...' : 'Trigger downstream builds'}
            </button>
          </div>

          {#if impactLoading}
            <div class="rounded-xl border border-dashed border-slate-300 px-4 py-8 text-center text-sm text-gray-500 dark:border-gray-700">Loading impact analysis...</div>
          {:else if impact}
            <div class="space-y-4">
              <div class="grid gap-3 sm:grid-cols-3">
                <div class="rounded-xl bg-slate-50 p-4 dark:bg-gray-950">
                  <div class="text-xs uppercase tracking-[0.22em] text-gray-400">Upstream</div>
                  <div class="mt-2 text-2xl font-semibold">{impact.upstream.length}</div>
                </div>
                <div class="rounded-xl bg-slate-50 p-4 dark:bg-gray-950">
                  <div class="text-xs uppercase tracking-[0.22em] text-gray-400">Downstream</div>
                  <div class="mt-2 text-2xl font-semibold">{impact.downstream.length}</div>
                </div>
                <div class="rounded-xl bg-slate-50 p-4 dark:bg-gray-950">
                  <div class="text-xs uppercase tracking-[0.22em] text-gray-400">Propagated marking</div>
                  <div class="mt-2 text-2xl font-semibold">{impact.propagated_marking}</div>
                </div>
              </div>

              <div class="rounded-xl bg-slate-50 p-4 dark:bg-gray-950">
                <div class="text-xs uppercase tracking-[0.22em] text-gray-400">Build candidates</div>
                <div class="mt-3 space-y-3">
                  {#if impact.build_candidates.length === 0}
                    <div class="text-sm text-gray-500">No downstream pipelines or workflows are currently reachable from this dataset.</div>
                  {:else}
                    {#each impact.build_candidates as candidate}
                      <div class="rounded-xl border border-slate-200 px-3 py-3 dark:border-gray-800">
                        <div class="flex items-center justify-between gap-3">
                          <div>
                            <div class="font-medium">{candidate.label}</div>
                            <div class="mt-1 text-xs uppercase tracking-[0.18em] text-gray-400">{candidate.kind} · distance {candidate.distance}</div>
                          </div>
                          <div class="text-right">
                            <div class={`text-sm font-medium ${candidate.triggerable ? 'text-emerald-600 dark:text-emerald-400' : 'text-gray-500'}`}>
                              {candidate.status ?? 'unknown'}
                            </div>
                            <div class="text-xs text-gray-400">{candidate.marking}</div>
                          </div>
                        </div>
                      </div>
                    {/each}
                  {/if}
                </div>
              </div>

              <div class="grid gap-4 lg:grid-cols-2">
                <div class="rounded-xl bg-slate-50 p-4 dark:bg-gray-950">
                  <div class="text-xs uppercase tracking-[0.22em] text-gray-400">Upstream path</div>
                  <div class="mt-3 space-y-3">
                    {#if impact.upstream.length === 0}
                      <div class="text-sm text-gray-500">No upstream dependencies captured yet.</div>
                    {:else}
                      {#each impact.upstream.slice(0, 6) as item}
                        <div class="rounded-xl border border-slate-200 px-3 py-3 dark:border-gray-800">
                          <div class="font-medium">{item.label}</div>
                          <div class="mt-1 text-xs uppercase tracking-[0.18em] text-gray-400">{item.kind} · distance {item.distance} · {item.marking}</div>
                        </div>
                      {/each}
                    {/if}
                  </div>
                </div>

                <div class="rounded-xl bg-slate-50 p-4 dark:bg-gray-950">
                  <div class="text-xs uppercase tracking-[0.22em] text-gray-400">Downstream impact</div>
                  <div class="mt-3 space-y-3">
                    {#if impact.downstream.length === 0}
                      <div class="text-sm text-gray-500">No downstream dependencies captured yet.</div>
                    {:else}
                      {#each impact.downstream.slice(0, 6) as item}
                        <div class="rounded-xl border border-slate-200 px-3 py-3 dark:border-gray-800">
                          <div class="font-medium">{item.label}</div>
                          <div class="mt-1 text-xs uppercase tracking-[0.18em] text-gray-400">{item.kind} · distance {item.distance} · {item.marking}</div>
                        </div>
                      {/each}
                    {/if}
                  </div>
                </div>
              </div>
            </div>
          {/if}

          {#if buildResult}
            <div class="rounded-xl bg-slate-50 p-4 dark:bg-gray-950">
              <div class="text-xs uppercase tracking-[0.22em] text-gray-400">Last build dispatch</div>
              <div class="mt-3 space-y-3">
                <div class="text-sm text-gray-500">{buildResult.triggered.length} triggered · {buildResult.skipped.length} skipped</div>
                {#each [...buildResult.triggered, ...buildResult.skipped] as item}
                  <div class="rounded-xl border border-slate-200 px-3 py-3 dark:border-gray-800">
                    <div class="flex items-center justify-between gap-3">
                      <div>
                        <div class="font-medium">{item.label}</div>
                        <div class="mt-1 text-xs uppercase tracking-[0.18em] text-gray-400">{item.kind}</div>
                      </div>
                      <div class="text-right">
                        <div class="font-medium">{item.status}</div>
                        <div class="text-xs text-gray-400">{item.run_id ?? item.message ?? 'No extra details'}</div>
                      </div>
                    </div>
                  </div>
                {/each}
              </div>
            </div>
          {/if}
        {/if}
      {/if}
    </section>
  </div>
</div>
