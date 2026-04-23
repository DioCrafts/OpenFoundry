<script lang="ts">
  import { listObjectTypes, deleteObjectType, type ObjectType } from '$lib/api/ontology';
  import { goto } from '$app/navigation';

  let types = $state<ObjectType[]>([]);
  let total = $state(0);
  let page = $state(1);
  let search = $state('');
  let loading = $state(true);

  async function load() {
    loading = true;
    try {
      const res = await listObjectTypes({ page, per_page: 20, search: search || undefined });
      types = res.data;
      total = res.total;
    } catch (e) {
      console.error('Failed to load object types', e);
    } finally {
      loading = false;
    }
  }

  async function handleDelete(id: string) {
    if (!confirm('Delete this object type and all its instances?')) return;
    await deleteObjectType(id);
    await load();
  }

  $effect(() => {
    load();
  });
</script>

<div class="space-y-6">
  <div class="flex items-center justify-between">
    <h1 class="text-2xl font-bold">Ontology Explorer</h1>
    <div class="flex gap-2">
      <a href="/ontology/graph" class="px-4 py-2 rounded border dark:border-gray-600 hover:bg-gray-100 dark:hover:bg-gray-700">
        Graph View
      </a>
      <a href="/ontology/types" class="px-4 py-2 rounded bg-blue-600 text-white hover:bg-blue-700">
        New Type
      </a>
    </div>
  </div>

  <div class="flex gap-4">
    <input
      type="text"
      placeholder="Search object types..."
      bind:value={search}
      oninput={() => { page = 1; load(); }}
      class="flex-1 px-3 py-2 border rounded dark:bg-gray-800 dark:border-gray-700"
    />
  </div>

  {#if loading}
    <div class="text-center py-12 text-gray-500">Loading...</div>
  {:else if types.length === 0}
    <div class="text-center py-12 text-gray-500">
      No object types found. Create your first type to build your ontology.
    </div>
  {:else}
    <div class="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
      {#each types as t (t.id)}
        <div class="border rounded-lg p-4 dark:border-gray-700 hover:shadow-md transition-shadow">
          <div class="flex items-start justify-between">
            <div class="flex items-center gap-2">
              {#if t.icon}
                <span class="text-xl">{t.icon}</span>
              {:else}
                <span class="w-8 h-8 rounded-full flex items-center justify-center text-white text-sm font-bold"
                      style="background-color: {t.color || '#6366f1'}">
                  {t.name.charAt(0).toUpperCase()}
                </span>
              {/if}
              <div>
                <h3 class="font-semibold">{t.display_name}</h3>
                <p class="text-xs text-gray-500 font-mono">{t.name}</p>
              </div>
            </div>
            <button
              onclick={() => handleDelete(t.id)}
              class="text-red-500 hover:text-red-700 text-sm"
            >Delete</button>
          </div>
          {#if t.description}
            <p class="mt-2 text-sm text-gray-600 dark:text-gray-400 line-clamp-2">{t.description}</p>
          {/if}
          <div class="mt-3 flex gap-2">
            <a href="/ontology/{t.id}" class="text-sm text-blue-600 hover:underline">Details</a>
          </div>
        </div>
      {/each}
    </div>

    {#if total > 20}
      <div class="flex justify-center gap-4 pt-4">
        <button disabled={page <= 1} onclick={() => { page--; load(); }}
                class="px-3 py-1 border rounded disabled:opacity-50 dark:border-gray-700">Previous</button>
        <span class="py-1 text-sm text-gray-500">Page {page}</span>
        <button disabled={page * 20 >= total} onclick={() => { page++; load(); }}
                class="px-3 py-1 border rounded disabled:opacity-50 dark:border-gray-700">Next</button>
      </div>
    {/if}
  {/if}
</div>
