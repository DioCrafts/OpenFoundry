<script lang="ts">
  import { onMount } from 'svelte';
  import { page as pageStore } from '$app/stores';
  import { get } from 'svelte/store';
  import {
    getOntologyGraph,
    listObjectTypes,
    type GraphResponse,
    type ObjectType,
  } from '$lib/api/ontology';

  let container = $state<HTMLDivElement | undefined>(undefined);
  let types = $state<ObjectType[]>([]);
  let graph = $state<GraphResponse | null>(null);
  let loading = $state(true);
  let error = $state('');

  let rootObjectId = $state('');
  let rootTypeId = $state('');
  let depth = $state(2);

  let cytoscapeModule: typeof import('cytoscape') | null = null;
  let graphInstance: import('cytoscape').Core | null = null;

  async function loadGraph() {
    loading = true;
    error = '';
    try {
      graph = await getOntologyGraph({
        root_object_id: rootObjectId || undefined,
        root_type_id: rootObjectId ? undefined : rootTypeId || undefined,
        depth,
        limit: 60,
      });
    } catch (cause) {
      error = cause instanceof Error ? cause.message : 'Failed to load ontology graph';
      graph = null;
    } finally {
      loading = false;
    }
  }

  async function loadReferenceData() {
    try {
      const res = await listObjectTypes({ per_page: 100 });
      types = res.data;
    } catch (cause) {
      console.error('Failed to load ontology types for graph filters', cause);
    }
  }

  async function renderGraph() {
    if (!container || !graph || graph.nodes.length === 0) {
      graphInstance?.destroy();
      graphInstance = null;
      return;
    }

    if (!cytoscapeModule) {
      cytoscapeModule = (await import('cytoscape')).default;
    }

    graphInstance?.destroy();

    const elements = [
      ...graph.nodes.map((node) => ({
        data: {
          id: node.id,
          label: node.label,
          secondaryLabel: node.secondary_label,
          color: node.color || (node.kind === 'interface' ? '#0f766e' : '#0f172a'),
          route: node.route,
          kind: node.kind,
        },
      })),
      ...graph.edges.map((edge) => ({
        data: {
          id: edge.id,
          source: edge.source,
          target: edge.target,
          label: edge.label,
          kind: edge.kind,
        },
      })),
    ];

    graphInstance = cytoscapeModule({
      container,
      elements,
      style: [
        {
          selector: 'node',
          style: {
            'background-color': 'data(color)',
            label: 'data(label)',
            color: '#e2e8f0',
            'text-wrap': 'wrap',
            'text-max-width': '120px',
            'font-size': 11,
            'text-valign': 'bottom',
            'text-margin-y': 8,
            width: 44,
            height: 44,
            'border-width': 2,
            'border-color': '#0f172a',
          },
        },
        {
          selector: 'edge',
          style: {
            width: 2,
            'line-color': '#64748b',
            'target-arrow-color': '#64748b',
            'target-arrow-shape': 'triangle',
            'curve-style': 'bezier',
            label: 'data(label)',
            'font-size': 10,
            color: '#94a3b8',
            'text-rotation': 'autorotate',
          },
        },
      ],
      layout: {
        name: graph.mode === 'object' ? 'breadthfirst' : 'cose',
        animate: true,
        padding: 24,
      },
    });

    graphInstance.on('tap', 'node', (event) => {
      const route = event.target.data('route') as string | undefined;
      if (route) {
        window.location.href = route;
      }
    });
  }

  function applyMode(mode: 'schema' | 'object') {
    if (mode === 'schema') {
      rootObjectId = '';
    } else {
      rootTypeId = '';
    }
    loadGraph();
  }

  onMount(async () => {
    const page = get(pageStore);
    rootObjectId = page.url.searchParams.get('root_object_id') ?? '';
    rootTypeId = page.url.searchParams.get('root_type_id') ?? '';
    depth = Number(page.url.searchParams.get('depth') ?? '2') || 2;

    await Promise.all([loadReferenceData(), loadGraph()]);
  });

  $effect(() => {
    renderGraph();
  });
</script>

<div class="space-y-6">
  <section class="rounded-[2rem] border border-slate-200 bg-white p-6 shadow-sm dark:border-slate-800 dark:bg-slate-900">
    <div class="flex flex-wrap items-center justify-between gap-4">
      <div>
        <p class="text-xs uppercase tracking-[0.22em] text-slate-500">Vertex</p>
        <h1 class="mt-1 text-3xl font-semibold tracking-tight text-slate-950 dark:text-slate-50">
          Ontology Graph
        </h1>
        <p class="mt-2 max-w-3xl text-sm text-slate-500">
          Explore either the schema graph or a live object neighborhood built from link instances.
        </p>
      </div>
      <a
        href="/ontology"
        class="rounded-full border border-slate-300 px-4 py-2 text-sm font-medium text-slate-700 hover:bg-slate-100 dark:border-slate-700 dark:text-slate-200 dark:hover:bg-slate-800"
      >
        Back to explorer
      </a>
    </div>
  </section>

  <section class="rounded-[1.75rem] border border-slate-200 bg-white p-6 shadow-sm dark:border-slate-800 dark:bg-slate-900">
    <div class="grid gap-4 lg:grid-cols-[1.1fr_1.1fr_0.55fr_auto]">
      <div>
        <label class="mb-1 block text-sm font-medium text-slate-700 dark:text-slate-200" for="root-type-id">
          Focus schema by type
        </label>
        <select
          id="root-type-id"
          bind:value={rootTypeId}
          disabled={!!rootObjectId}
          class="w-full rounded-2xl border border-slate-300 px-4 py-2.5 text-sm disabled:opacity-50 dark:border-slate-700 dark:bg-slate-950 dark:text-slate-100"
        >
          <option value="">All types</option>
          {#each types as typeItem (typeItem.id)}
            <option value={typeItem.id}>{typeItem.display_name}</option>
          {/each}
        </select>
      </div>

      <div>
        <label class="mb-1 block text-sm font-medium text-slate-700 dark:text-slate-200" for="root-object-id">
          Focus object neighborhood
        </label>
        <input
          id="root-object-id"
          bind:value={rootObjectId}
          placeholder="Paste object UUID to inspect neighbors"
          class="w-full rounded-2xl border border-slate-300 px-4 py-2.5 text-sm dark:border-slate-700 dark:bg-slate-950 dark:text-slate-100"
        />
      </div>

      <div>
        <label class="mb-1 block text-sm font-medium text-slate-700 dark:text-slate-200" for="graph-depth">
          Depth
        </label>
        <input
          id="graph-depth"
          type="number"
          bind:value={depth}
          min="1"
          max="4"
          class="w-full rounded-2xl border border-slate-300 px-4 py-2.5 text-sm dark:border-slate-700 dark:bg-slate-950 dark:text-slate-100"
        />
      </div>

      <div class="flex items-end gap-2">
        <button
          type="button"
          onclick={() => applyMode(rootObjectId ? 'object' : 'schema')}
          class="rounded-full bg-teal-600 px-4 py-2 text-sm font-medium text-white hover:bg-teal-700"
        >
          Load graph
        </button>
      </div>
    </div>

    <div class="mt-4 flex flex-wrap gap-2 text-sm">
      <button
        type="button"
        onclick={() => applyMode('schema')}
        class="rounded-full border border-slate-300 px-3 py-1.5 text-slate-700 hover:bg-slate-100 dark:border-slate-700 dark:text-slate-200 dark:hover:bg-slate-800"
      >
        Schema mode
      </button>
      <button
        type="button"
        onclick={() => applyMode('object')}
        class="rounded-full border border-slate-300 px-3 py-1.5 text-slate-700 hover:bg-slate-100 dark:border-slate-700 dark:text-slate-200 dark:hover:bg-slate-800"
      >
        Object mode
      </button>
    </div>
  </section>

  {#if error}
    <div class="rounded-[1.75rem] border border-rose-200 bg-rose-50 px-6 py-4 text-sm text-rose-700 dark:border-rose-900/40 dark:bg-rose-950/20 dark:text-rose-300">
      {error}
    </div>
  {/if}

  <section class="rounded-[1.75rem] border border-slate-200 bg-white p-6 shadow-sm dark:border-slate-800 dark:bg-slate-900">
    <div class="flex flex-wrap items-center justify-between gap-4">
      <div>
        <h2 class="text-lg font-semibold text-slate-950 dark:text-slate-50">Graph canvas</h2>
        <p class="mt-1 text-sm text-slate-500">
          {#if graph}
            Mode: <span class="font-medium text-slate-700 dark:text-slate-300">{graph.mode}</span>
            · {graph.total_nodes} nodes · {graph.total_edges} edges
          {:else}
            Load a schema or object graph to begin.
          {/if}
        </p>
      </div>
      {#if graph?.root_object_id}
        <span class="rounded-full bg-emerald-50 px-3 py-1 text-xs text-emerald-700 dark:bg-emerald-950/40 dark:text-emerald-300">
          root object {graph.root_object_id}
        </span>
      {/if}
    </div>

    {#if loading}
      <div class="mt-6 rounded-2xl border border-dashed border-slate-300 px-4 py-16 text-center text-sm text-slate-500 dark:border-slate-700">
        Loading graph...
      </div>
    {:else if !graph || graph.nodes.length === 0}
      <div class="mt-6 rounded-2xl border border-dashed border-slate-300 px-4 py-16 text-center text-sm text-slate-500 dark:border-slate-700">
        No graph data available for the current selection.
      </div>
    {:else}
      <div
        bind:this={container}
        class="mt-6 w-full rounded-[1.5rem] border border-slate-200 dark:border-slate-800"
        style="height: 680px; background: radial-gradient(circle at top, #1e293b, #020617 68%);"
      ></div>
      <p class="mt-3 text-xs text-slate-500">
        Click nodes to navigate. Type nodes open their detail page; object nodes jump to the anchored object card.
      </p>
    {/if}
  </section>
</div>
