<script lang="ts">
  import { createDataset, uploadData } from '$lib/api/datasets';
  import { goto } from '$app/navigation';

  let name = $state('');
  let description = $state('');
  let format = $state('parquet');
  let tags = $state('');
  let file = $state<File | null>(null);
  let uploading = $state(false);
  let error = $state('');

  function handleFileChange(e: Event) {
    const input = e.target as HTMLInputElement;
    file = input.files?.[0] ?? null;
    if (file && !name) {
      name = file.name.replace(/\.[^.]+$/, '');
    }
  }

  async function handleSubmit(e: Event) {
    e.preventDefault();
    error = '';
    uploading = true;

    try {
      const tagList = tags.split(',').map(t => t.trim()).filter(Boolean);
      const dataset = await createDataset({ name, description: description || undefined, format, tags: tagList });

      if (file) {
        await uploadData(dataset.id, file);
      }

      goto(`/datasets/${dataset.id}`);
    } catch (e: any) {
      error = e.message || 'Upload failed';
    } finally {
      uploading = false;
    }
  }
</script>

<div class="max-w-2xl mx-auto space-y-6">
  <h1 class="text-2xl font-bold">Upload Dataset</h1>

  {#if error}
    <div class="bg-red-50 dark:bg-red-900/20 text-red-600 px-4 py-3 rounded">{error}</div>
  {/if}

  <form onsubmit={handleSubmit} class="space-y-4">
    <div>
      <label for="name" class="block text-sm font-medium mb-1">Name</label>
      <input id="name" type="text" required bind:value={name}
        class="w-full px-3 py-2 border rounded dark:bg-gray-800 dark:border-gray-700" />
    </div>

    <div>
      <label for="description" class="block text-sm font-medium mb-1">Description</label>
      <textarea id="description" bind:value={description} rows="3"
        class="w-full px-3 py-2 border rounded dark:bg-gray-800 dark:border-gray-700"></textarea>
    </div>

    <div>
      <label for="format" class="block text-sm font-medium mb-1">Format</label>
      <select id="format" bind:value={format}
        class="w-full px-3 py-2 border rounded dark:bg-gray-800 dark:border-gray-700">
        <option value="parquet">Parquet</option>
        <option value="csv">CSV</option>
        <option value="json">JSON</option>
      </select>
    </div>

    <div>
      <label for="tags" class="block text-sm font-medium mb-1">Tags (comma separated)</label>
      <input id="tags" type="text" bind:value={tags}
        placeholder="finance, monthly, report"
        class="w-full px-3 py-2 border rounded dark:bg-gray-800 dark:border-gray-700" />
    </div>

    <div>
      <label for="file" class="block text-sm font-medium mb-1">File</label>
      <input id="file" type="file" onchange={handleFileChange}
        accept=".parquet,.csv,.json,.jsonl,.tsv"
        class="w-full px-3 py-2 border rounded dark:bg-gray-800 dark:border-gray-700" />
    </div>

    <div class="flex gap-4">
      <button type="submit" disabled={uploading || !name}
        class="px-6 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 disabled:opacity-50">
        {uploading ? 'Uploading...' : 'Create Dataset'}
      </button>
      <a href="/datasets" class="px-6 py-2 border rounded hover:bg-gray-50 dark:hover:bg-gray-800">Cancel</a>
    </div>
  </form>
</div>
