<script lang="ts">
  import { createPipeline, type PipelineNode } from '$lib/api/pipelines';
  import { goto } from '$app/navigation';

  let name = $state('');
  let description = $state('');
  let nodes = $state<PipelineNode[]>([]);
  let saving = $state(false);
  let error = $state('');

  let nextNodeNum = $state(1);

  function addNode(type: string) {
    const id = `node_${nextNodeNum}`;
    nextNodeNum++;
    nodes = [...nodes, {
      id,
      label: `${type} Transform ${nodes.length + 1}`,
      transform_type: type,
      config: type === 'sql' ? { sql: '' } : {},
      depends_on: [],
      input_dataset_ids: [],
      output_dataset_id: null,
    }];
  }

  function removeNode(id: string) {
    nodes = nodes.filter(n => n.id !== id).map(n => ({
      ...n,
      depends_on: n.depends_on.filter(d => d !== id),
    }));
  }

  function updateNodeSql(id: string, sql: string) {
    nodes = nodes.map(n => n.id === id ? { ...n, config: { ...n.config, sql } } : n);
  }

  function updateNodeLabel(id: string, label: string) {
    nodes = nodes.map(n => n.id === id ? { ...n, label } : n);
  }

  function toggleDependency(nodeId: string, depId: string) {
    nodes = nodes.map(n => {
      if (n.id !== nodeId) return n;
      const deps = n.depends_on.includes(depId)
        ? n.depends_on.filter(d => d !== depId)
        : [...n.depends_on, depId];
      return { ...n, depends_on: deps };
    });
  }

  async function handleSave() {
    if (!name.trim()) { error = 'Pipeline name is required'; return; }
    if (nodes.length === 0) { error = 'Add at least one node'; return; }
    saving = true;
    error = '';
    try {
      await createPipeline({
        name: name.trim(),
        description: description.trim() || undefined,
        nodes,
      });
      goto('/pipelines');
    } catch (err: any) {
      error = err.message || 'Failed to create pipeline';
    } finally {
      saving = false;
    }
  }
</script>

<div class="space-y-6 max-w-4xl mx-auto">
  <h1 class="text-2xl font-bold">New Pipeline</h1>

  {#if error}
    <div class="p-3 bg-red-100 text-red-700 rounded dark:bg-red-900 dark:text-red-300">{error}</div>
  {/if}

  <div class="grid gap-4 md:grid-cols-2">
    <div>
      <label for="pipeline-name" class="block text-sm font-medium mb-1">Pipeline Name</label>
      <input id="pipeline-name" type="text" bind:value={name} placeholder="e.g. Sales ETL"
             class="w-full px-3 py-2 border rounded dark:bg-gray-800 dark:border-gray-700" />
    </div>
    <div>
      <label for="pipeline-description" class="block text-sm font-medium mb-1">Description</label>
      <input id="pipeline-description" type="text" bind:value={description} placeholder="What does this pipeline do?"
             class="w-full px-3 py-2 border rounded dark:bg-gray-800 dark:border-gray-700" />
    </div>
  </div>

  <!-- Node palette -->
  <div class="border rounded-lg p-4 dark:border-gray-700">
    <h2 class="text-sm font-semibold mb-3 text-gray-500 uppercase tracking-wide">Add Transform</h2>
    <div class="flex gap-2">
      <button onclick={() => addNode('sql')}
              class="px-4 py-2 bg-indigo-600 text-white rounded hover:bg-indigo-700 text-sm">
        + SQL Transform
      </button>
      <button onclick={() => addNode('passthrough')}
              class="px-4 py-2 bg-gray-600 text-white rounded hover:bg-gray-700 text-sm">
        + Passthrough
      </button>
    </div>
  </div>

  <!-- DAG canvas (node list) -->
  {#if nodes.length > 0}
    <div class="space-y-4">
      <h2 class="text-sm font-semibold text-gray-500 uppercase tracking-wide">Pipeline Nodes</h2>
      {#each nodes as node, i (node.id)}
        <div class="border rounded-lg p-4 dark:border-gray-700 space-y-3">
          <div class="flex items-center justify-between">
            <div class="flex items-center gap-3">
              <span class="text-xs font-mono bg-gray-200 dark:bg-gray-700 px-2 py-0.5 rounded">{node.id}</span>
              <input type="text" bind:value={node.label}
                     oninput={(e: Event) => updateNodeLabel(node.id, (e.target as HTMLInputElement).value)}
                     class="px-2 py-1 border rounded text-sm dark:bg-gray-800 dark:border-gray-700 w-64" />
              <span class="text-xs px-2 py-0.5 rounded-full bg-indigo-100 text-indigo-700 dark:bg-indigo-900 dark:text-indigo-300">
                {node.transform_type}
              </span>
            </div>
            <button onclick={() => removeNode(node.id)}
                    class="text-red-500 hover:text-red-700 text-sm">Remove</button>
          </div>

          {#if node.transform_type === 'sql'}
            <div>
              <label for={`pipeline-node-sql-${node.id}`} class="block text-xs font-medium mb-1 text-gray-500">SQL Query</label>
              <textarea
                id={`pipeline-node-sql-${node.id}`}
                value={typeof node.config.sql === 'string' ? node.config.sql : ''}
                oninput={(e: Event) => updateNodeSql(node.id, (e.target as HTMLTextAreaElement).value)}
                rows={4}
                placeholder="SELECT * FROM ..."
                class="w-full px-3 py-2 border rounded font-mono text-sm dark:bg-gray-800 dark:border-gray-700"
              ></textarea>
            </div>
          {/if}

          {#if i > 0}
            <div>
              <div class="block text-xs font-medium mb-1 text-gray-500">Depends on</div>
              <div class="flex flex-wrap gap-2">
                {#each nodes.slice(0, i) as dep (dep.id)}
                  <button
                    onclick={() => toggleDependency(node.id, dep.id)}
                    class="text-xs px-2 py-1 rounded border {node.depends_on.includes(dep.id) ? 'bg-blue-600 text-white border-blue-600' : 'dark:border-gray-600 hover:bg-gray-100 dark:hover:bg-gray-700'}"
                  >
                    {dep.label || dep.id}
                  </button>
                {/each}
              </div>
            </div>
          {/if}
        </div>
      {/each}
    </div>
  {/if}

  <div class="flex gap-3 pt-4">
    <button onclick={handleSave} disabled={saving}
            class="px-6 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 disabled:opacity-50">
      {saving ? 'Saving...' : 'Create Pipeline'}
    </button>
    <a href="/pipelines" class="px-6 py-2 border rounded hover:bg-gray-100 dark:border-gray-700 dark:hover:bg-gray-700">
      Cancel
    </a>
  </div>
</div>
