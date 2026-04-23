<script lang="ts">
  import { getFullLineage, type LineageGraph } from '$lib/api/pipelines';
  import { onMount } from 'svelte';

  let container = $state<HTMLDivElement | undefined>(undefined);
  let graph = $state<LineageGraph | null>(null);
  let loading = $state(true);
  let error = $state('');

  onMount(async () => {
    try {
      graph = await getFullLineage();
    } catch (e: any) {
      error = e.message || 'Failed to load lineage';
    } finally {
      loading = false;
    }

    if (!graph || graph.nodes.length === 0) return;

    const cytoscape = (await import('cytoscape')).default;

    const nodes = graph.nodes.map((n) => ({
      data: { id: n.dataset_id, label: n.dataset_id.slice(0, 8) + '...' },
    }));

    const edges = graph.edges.map((e, i) => ({
      data: { id: `edge_${i}`, source: e.source, target: e.target },
    }));

    cytoscape({
      container,
      elements: [...nodes, ...edges],
      style: [
        {
          selector: 'node',
          style: {
            'background-color': '#3b82f6',
            label: 'data(label)',
            color: '#e5e7eb',
            'text-valign': 'bottom',
            'text-margin-y': 8,
            'font-size': '11px',
            width: 36,
            height: 36,
          },
        },
        {
          selector: 'edge',
          style: {
            width: 2,
            'line-color': '#6b7280',
            'target-arrow-color': '#6b7280',
            'target-arrow-shape': 'triangle',
            'curve-style': 'bezier',
          },
        },
      ],
      layout: {
        name: 'breadthfirst',
        directed: true,
        spacingFactor: 1.5,
      },
    });
  });
</script>

<div class="space-y-4">
  <h1 class="text-2xl font-bold">Data Lineage</h1>

  {#if loading}
    <div class="text-center py-12 text-gray-500">Loading lineage graph...</div>
  {:else if error}
    <div class="p-3 bg-red-100 text-red-700 rounded dark:bg-red-900 dark:text-red-300">{error}</div>
  {:else if !graph || graph.nodes.length === 0}
    <div class="text-center py-12 text-gray-500">
      No lineage data yet. Run a pipeline to generate lineage.
    </div>
  {:else}
    <p class="text-sm text-gray-500">{graph.nodes.length} datasets, {graph.edges.length} edges</p>
    <div bind:this={container} class="w-full border rounded dark:border-gray-700"
         style="height: 600px; background: #111827;"></div>
  {/if}
</div>
