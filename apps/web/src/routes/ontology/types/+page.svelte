<script lang="ts">
  import { createObjectType } from '$lib/api/ontology';
  import { goto } from '$app/navigation';

  let name = $state('');
  let displayName = $state('');
  let description = $state('');
  let icon = $state('');
  let color = $state('#6366f1');
  let saving = $state(false);
  let error = $state('');

  async function handleSubmit(e: Event) {
    e.preventDefault();
    if (!name.trim()) { error = 'Name is required'; return; }
    saving = true;
    error = '';
    try {
      await createObjectType({
        name: name.trim(),
        display_name: displayName.trim() || undefined,
        description: description.trim() || undefined,
        icon: icon.trim() || undefined,
        color: color || undefined,
      });
      goto('/ontology');
    } catch (err: any) {
      error = err.message || 'Failed to create object type';
    } finally {
      saving = false;
    }
  }
</script>

<div class="max-w-2xl mx-auto space-y-6">
  <h1 class="text-2xl font-bold">New Object Type</h1>

  {#if error}
    <div class="p-3 bg-red-100 text-red-700 rounded dark:bg-red-900 dark:text-red-300">{error}</div>
  {/if}

  <form onsubmit={handleSubmit} class="space-y-4">
    <div>
      <label for="object-type-name" class="block text-sm font-medium mb-1">Name (API identifier)</label>
      <input id="object-type-name" type="text" bind:value={name} required placeholder="e.g. customer, order, product"
             class="w-full px-3 py-2 border rounded dark:bg-gray-800 dark:border-gray-700 font-mono" />
    </div>

    <div>
      <label for="object-type-display-name" class="block text-sm font-medium mb-1">Display Name</label>
      <input id="object-type-display-name" type="text" bind:value={displayName} placeholder="e.g. Customer, Sales Order"
             class="w-full px-3 py-2 border rounded dark:bg-gray-800 dark:border-gray-700" />
    </div>

    <div>
      <label for="object-type-description" class="block text-sm font-medium mb-1">Description</label>
      <textarea id="object-type-description" bind:value={description} rows={3} placeholder="What does this object type represent?"
                class="w-full px-3 py-2 border rounded dark:bg-gray-800 dark:border-gray-700"></textarea>
    </div>

    <div class="flex gap-4">
      <div class="flex-1">
        <label for="object-type-icon" class="block text-sm font-medium mb-1">Icon (emoji)</label>
        <input id="object-type-icon" type="text" bind:value={icon} placeholder="👤"
               class="w-full px-3 py-2 border rounded dark:bg-gray-800 dark:border-gray-700" />
      </div>
      <div>
        <label for="object-type-color" class="block text-sm font-medium mb-1">Color</label>
        <input id="object-type-color" type="color" bind:value={color} class="h-10 w-16 rounded cursor-pointer" />
      </div>
    </div>

    <div class="flex gap-3 pt-4">
      <button type="submit" disabled={saving}
              class="px-6 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 disabled:opacity-50">
        {saving ? 'Creating...' : 'Create Type'}
      </button>
      <a href="/ontology" class="px-6 py-2 border rounded hover:bg-gray-100 dark:border-gray-700 dark:hover:bg-gray-700">
        Cancel
      </a>
    </div>
  </form>
</div>
