<script lang="ts">
  import { onMount } from 'svelte';
  import { page as pageStore } from '$app/stores';
  import { get } from 'svelte/store';
  import Glyph from '$components/ui/Glyph.svelte';
  import {
    getOntologyGraph,
    listObjectTypes,
    type GraphResponse,
    type ObjectType
  } from '$lib/api/ontology';

  let container = $state<HTMLDivElement | undefined>(undefined);
  let types = $state<ObjectType[]>([]);
  let graph = $state<GraphResponse | null>(null);
  let loading = $state(true);
  let error = $state('');

  let rootObjectId = $state('');
  let rootTypeId = $state('');
  let depth = $state(2);
  let mode = $state<'schema' | 'object'>('schema');

  let cytoscapeModule: typeof import('cytoscape') | null = null;
  let graphInstance: import('cytoscape').Core | null = null;

  async function loadGraph() {
    loading = true;
    error = '';
    try {
      graph = await getOntologyGraph({
        root_object_id: mode === 'object' ? rootObjectId || undefined : undefined,
        root_type_id: mode === 'schema' ? rootTypeId || undefined : undefined,
        depth,
        limit: 60
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
          color: node.color || '#4d8cf0',
          route: node.route,
          kind: node.kind
        }
      })),
      ...graph.edges.map((edge) => ({
        data: {
          id: edge.id,
          source: edge.source,
          target: edge.target,
          label: edge.label,
          kind: edge.kind
        }
      }))
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
            color: '#334155',
            'text-wrap': 'wrap',
            'text-max-width': '120px',
            'font-size': 11,
            'font-family': 'Helvetica Neue, Arial, sans-serif',
            'text-valign': 'center',
            'text-halign': 'center',
            width: 26,
            height: 26,
            shape: 'round-rectangle',
            padding: '16px',
            'border-width': 1,
            'border-color': '#d7dee9'
          }
        },
        {
          selector: 'edge',
          style: {
            width: 1.6,
            'line-color': '#9fb0c8',
            'target-arrow-color': '#9fb0c8',
            'target-arrow-shape': 'triangle',
            'curve-style': 'bezier',
            label: 'data(label)',
            'font-size': 9,
            color: '#64748b',
            'text-rotation': 'autorotate'
          }
        }
      ],
      layout: {
        name: graph.mode === 'object' ? 'breadthfirst' : 'cose',
        animate: true,
        padding: 32
      }
    });

    graphInstance.on('tap', 'node', (event) => {
      const route = event.target.data('route') as string | undefined;
      if (route) {
        window.location.href = route;
      }
    });
  }

  function countEntries(entries: Record<string, number> | undefined) {
    return Object.entries(entries ?? {}).sort((left, right) => right[1] - left[1]);
  }

  onMount(async () => {
    const page = get(pageStore);
    rootObjectId = page.url.searchParams.get('root_object_id') ?? '';
    rootTypeId = page.url.searchParams.get('root_type_id') ?? '';
    depth = Number(page.url.searchParams.get('depth') ?? '2') || 2;
    mode = rootObjectId ? 'object' : 'schema';

    await Promise.all([loadReferenceData(), loadGraph()]);
  });

  $effect(() => {
    renderGraph();
  });
</script>

<div class="space-y-5">
  <section class="of-hero-strip">
    <div class="flex flex-wrap items-start justify-between gap-4">
      <div>
        <div class="of-heading-xl">Ontology graph</div>
        <div class="mt-2 max-w-3xl text-[15px] text-[var(--text-muted)]">
          Switch between schema-level topology and a live object neighborhood while keeping the same
          compact explorer chrome.
        </div>
      </div>
      <a href="/ontology" class="of-btn">
        <Glyph name="graph" size={16} />
        <span>Back to explorer</span>
      </a>
    </div>
  </section>

  <section class="of-panel p-5">
    <div class="flex flex-wrap items-end gap-4">
      <div class="min-w-[220px] flex-1">
        <label class="mb-1 block text-sm font-medium text-[var(--text-default)]" for="root-type-id">
          Object type
        </label>
        <select id="root-type-id" bind:value={rootTypeId} disabled={mode === 'object'} class="of-select">
          <option value="">All types</option>
          {#each types as typeItem (typeItem.id)}
            <option value={typeItem.id}>{typeItem.display_name}</option>
          {/each}
        </select>
      </div>

      <div class="min-w-[260px] flex-[1.2]">
        <label class="mb-1 block text-sm font-medium text-[var(--text-default)]" for="root-object-id">
          Root object
        </label>
        <input
          id="root-object-id"
          bind:value={rootObjectId}
          placeholder="Paste object UUID to inspect neighbors"
          class="of-input"
        />
      </div>

      <div class="w-[130px]">
        <label class="mb-1 block text-sm font-medium text-[var(--text-default)]" for="graph-depth">Depth</label>
        <input id="graph-depth" type="number" bind:value={depth} min="1" max="4" class="of-input" />
      </div>

      <div class="flex gap-2">
        <div class="of-pill-toggle">
          <button type="button" data-active={mode === 'schema'} onclick={() => mode = 'schema'}>List</button>
          <button type="button" data-active={mode === 'object'} onclick={() => mode = 'object'}>Graph</button>
        </div>
        <button class="of-btn of-btn-primary" type="button" onclick={loadGraph}>
          <Glyph name="run" size={15} />
          <span>Load</span>
        </button>
      </div>
    </div>

    {#if graph}
      <div class="mt-4 flex flex-wrap gap-2">
        <span class="of-chip">Nodes {graph.total_nodes}</span>
        <span class="of-chip">Edges {graph.total_edges}</span>
        <span class="of-chip">Depth {graph.summary.max_hops_reached}</span>
        <span class="of-chip">Sensitive {graph.summary.sensitive_objects}</span>
      </div>
    {/if}
  </section>

  {#if error}
    <div class="of-inline-note">{error}</div>
  {/if}

  <section class="of-panel overflow-hidden">
    <div class="flex items-center justify-between border-b border-[var(--border-subtle)] px-5 py-4">
      <div>
        <div class="of-heading-sm">Graph canvas</div>
        <div class="mt-1 text-sm text-[var(--text-muted)]">
          {#if graph}
            {graph.mode} · {graph.total_nodes} nodes · {graph.total_edges} edges
          {:else}
            Load a schema or object graph to begin.
          {/if}
        </div>
      </div>
      <div class="of-pill-toggle">
        <button type="button" data-active={mode === 'schema'} onclick={() => mode = 'schema'}>Schema</button>
        <button type="button" data-active={mode === 'object'} onclick={() => mode = 'object'}>Object</button>
      </div>
    </div>

    {#if loading}
      <div class="px-4 py-16 text-center text-sm text-[var(--text-muted)]">Loading graph...</div>
    {:else if !graph || graph.nodes.length === 0}
      <div class="px-4 py-16 text-center text-sm text-[var(--text-muted)]">
        No graph data available for the current selection.
      </div>
    {:else}
      <div class="relative bg-[#eef3f8]">
        <div class="absolute left-5 top-5 z-10 flex flex-col gap-2">
          <button class="of-btn h-9 w-9 px-0" type="button" aria-label="Zoom in">
            <Glyph name="plus" size={16} />
          </button>
          <button class="of-btn h-9 w-9 px-0" type="button" aria-label="Search">
            <Glyph name="search" size={16} />
          </button>
          <button class="of-btn h-9 w-9 px-0" type="button" aria-label="Reset">
            <Glyph name="history" size={16} />
          </button>
        </div>

        <div class="absolute right-5 top-5 z-10 rounded-[6px] border border-[var(--border-default)] bg-white px-4 py-3">
          <label class="flex items-center gap-2 text-sm text-[var(--text-default)]">
            <input type="checkbox" />
            <span>Object type</span>
          </label>
          <label class="mt-2 flex items-center gap-2 text-sm text-[var(--text-default)]">
            <input type="checkbox" checked />
            <span>Object type group</span>
          </label>
        </div>

        <div bind:this={container} style="height: 720px;"></div>
      </div>

      {#if countEntries(graph.summary.object_types).length > 0}
        <div class="border-t border-[var(--border-subtle)] px-5 py-4">
          <div class="of-heading-sm">Types in scope</div>
          <div class="mt-3 flex flex-wrap gap-2">
            {#each countEntries(graph.summary.object_types) as [label, count]}
              <span class="of-chip">{label} · {count}</span>
            {/each}
          </div>
        </div>
      {/if}
    {/if}
  </section>
</div>
