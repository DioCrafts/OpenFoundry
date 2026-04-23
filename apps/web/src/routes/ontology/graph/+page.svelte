<script lang="ts">
  import { listObjectTypes, listLinkTypes, type ObjectType, type LinkType } from '$lib/api/ontology';
  import { onMount } from 'svelte';

  let container = $state<HTMLDivElement | undefined>(undefined);
  let types = $state<ObjectType[]>([]);
  let links = $state<LinkType[]>([]);
  let loading = $state(true);

  onMount(async () => {
    try {
      const [typesRes, linksRes] = await Promise.all([
        listObjectTypes({ per_page: 100 }),
        listLinkTypes({ per_page: 100 }),
      ]);
      types = typesRes.data;
      links = linksRes.data;
    } catch (e) {
      console.error('Failed to load ontology graph data', e);
    } finally {
      loading = false;
    }

    if (types.length === 0) return;

    // Dynamically import cytoscape
    const cytoscape = (await import('cytoscape')).default;

    const nodes = types.map((t) => ({
      data: {
        id: t.id,
        label: t.display_name || t.name,
        color: t.color || '#6366f1',
      },
    }));

    const edges = links.map((l) => ({
      data: {
        id: l.id,
        source: l.source_type_id,
        target: l.target_type_id,
        label: l.display_name || l.name,
      },
    }));

    cytoscape({
      container,
      elements: [...nodes, ...edges],
      style: [
        {
          selector: 'node',
          style: {
            'background-color': 'data(color)',
            label: 'data(label)',
            color: '#e5e7eb',
            'text-valign': 'bottom',
            'text-margin-y': 8,
            'font-size': '12px',
            width: 40,
            height: 40,
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
            label: 'data(label)',
            'font-size': '10px',
            color: '#9ca3af',
            'text-rotation': 'autorotate',
          },
        },
      ],
      layout: { name: 'cose', animate: true },
    });
  });
</script>

<div class="space-y-4">
  <div class="flex items-center justify-between">
    <h1 class="text-2xl font-bold">Ontology Graph</h1>
    <a href="/ontology" class="px-4 py-2 border rounded dark:border-gray-600 hover:bg-gray-100 dark:hover:bg-gray-700">
      List View
    </a>
  </div>

  {#if loading}
    <div class="text-center py-12 text-gray-500">Loading graph...</div>
  {:else if types.length === 0}
    <div class="text-center py-12 text-gray-500">
      No object types to visualize. Create some types first.
    </div>
  {:else}
    <div bind:this={container} class="w-full border rounded dark:border-gray-700"
         style="height: 600px; background: #111827;"></div>
    <p class="text-sm text-gray-500">{types.length} types, {links.length} link types</p>
  {/if}
</div>
