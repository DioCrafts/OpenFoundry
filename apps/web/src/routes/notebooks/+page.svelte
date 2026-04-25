<script lang="ts">
  import { onMount } from 'svelte';
  import {
    listNotebooks,
    createNotebook,
    deleteNotebook,
    type Notebook,
    type NotebookKernel,
  } from '$lib/api/notebooks';
  import { goto } from '$app/navigation';

  let notebooks = $state<Notebook[]>([]);
  let total = $state(0);
  let page = $state(1);
  let search = $state('');
  let loading = $state(true);
  let showCreate = $state(false);
  let newName = $state('');
  let newDescription = $state('');
  let newKernel = $state<NotebookKernel>('python');

  async function load() {
    loading = true;
    try {
      const res = await listNotebooks({ page, per_page: 20, search });
      notebooks = res.data;
      total = res.total;
    } catch {
      notebooks = [];
    }
    loading = false;
  }

  async function handleCreate() {
    if (!newName.trim()) return;
    const res = await createNotebook({ name: newName, description: newDescription, default_kernel: newKernel });
    goto(`/notebooks/${res.id}`);
  }

  async function handleDelete(id: string) {
    if (!confirm('Delete this notebook?')) return;
    await deleteNotebook(id);
    load();
  }

  onMount(() => {
    void load();
  });
</script>

<div class="p-6 max-w-5xl mx-auto">
  <div class="flex justify-between items-center mb-6">
    <h1 class="text-2xl font-bold">Notebooks</h1>
    <button class="bg-blue-600 text-white px-4 py-2 rounded hover:bg-blue-700"
            onclick={() => showCreate = !showCreate}>
      + New Notebook
    </button>
  </div>

  {#if showCreate}
    <div class="bg-gray-50 border rounded p-4 mb-6 space-y-3">
      <input class="border rounded px-3 py-2 w-full" placeholder="Name" bind:value={newName} />
      <input class="border rounded px-3 py-2 w-full" placeholder="Description" bind:value={newDescription} />
      <select class="border rounded px-3 py-2" bind:value={newKernel}>
        <option value="python">Python</option>
        <option value="sql">SQL</option>
        <option value="llm">LLM</option>
        <option value="r">R</option>
      </select>
      <button class="bg-green-600 text-white px-4 py-2 rounded hover:bg-green-700" onclick={handleCreate}>
        Create
      </button>
    </div>
  {/if}

  <div class="mb-4">
    <input class="border rounded px-3 py-2 w-full" placeholder="Search notebooks..."
           bind:value={search} oninput={() => { page = 1; load(); }} />
  </div>

  {#if loading}
    <p class="text-gray-500">Loading...</p>
  {:else if notebooks.length === 0}
    <p class="text-gray-500">No notebooks found.</p>
  {:else}
    <div class="space-y-2">
      {#each notebooks as nb (nb.id)}
        <div class="border rounded p-4 flex justify-between items-center hover:bg-gray-50">
          <a href="/notebooks/{nb.id}" class="flex-1">
            <div class="font-semibold">{nb.name}</div>
            <div class="text-sm text-gray-500">{nb.description || 'No description'} · {nb.default_kernel} · {new Date(nb.updated_at).toLocaleDateString()}</div>
          </a>
          <button class="text-red-500 hover:text-red-700 text-sm ml-4" onclick={() => handleDelete(nb.id)}>
            Delete
          </button>
        </div>
      {/each}
    </div>

    {#if total > 20}
      <div class="flex gap-2 mt-4">
        <button disabled={page <= 1} class="px-3 py-1 border rounded disabled:opacity-50"
                onclick={() => { page--; load(); }}>Prev</button>
        <span class="px-3 py-1">Page {page}</span>
        <button disabled={page * 20 >= total} class="px-3 py-1 border rounded disabled:opacity-50"
                onclick={() => { page++; load(); }}>Next</button>
      </div>
    {/if}
  {/if}
</div>
