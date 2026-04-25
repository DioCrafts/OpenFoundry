<script lang="ts">
  import { onMount } from 'svelte';
  import {
    listDatasets,
    previewDataset,
    uploadData,
    type Dataset,
  } from '$lib/api/datasets';
  import {
    listObjectTypes,
    listObjects,
    updateObject,
    type ObjectInstance,
    type ObjectType,
  } from '$lib/api/ontology';

  let mode = $state<'dataset' | 'ontology'>('dataset');
  let datasets = $state<Dataset[]>([]);
  let datasetId = $state('');
  let datasetRows = $state<Array<Record<string, unknown>>>([]);
  let objectTypes = $state<ObjectType[]>([]);
  let objectTypeId = $state('');
  let objectRows = $state<ObjectInstance[]>([]);
  let busy = $state(false);
  let error = $state('');

  async function loadDatasetRows(id: string) {
    const first = await previewDataset(id, { limit: 1000, offset: 0 });
    const total = first.total_rows ?? first.rows?.length ?? 0;
    const rows = [...(first.rows ?? [])];
    for (let offset = rows.length; offset < total; offset += 1000) {
      const next = await previewDataset(id, { limit: 1000, offset });
      rows.push(...(next.rows ?? []));
    }
    return rows;
  }

  async function loadDatasetsAndTypes() {
    busy = true;
    error = '';
    try {
      const [datasetResponse, typeResponse] = await Promise.all([
        listDatasets({ per_page: 100 }),
        listObjectTypes({ per_page: 100 }),
      ]);

      datasets = datasetResponse.data;
      objectTypes = typeResponse.data;
      datasetId = datasetId || datasets[0]?.id || '';
      objectTypeId = objectTypeId || objectTypes[0]?.id || '';

      if (datasetId) {
        datasetRows = await loadDatasetRows(datasetId);
      }

      if (objectTypeId) {
        objectRows = await loadObjectRows(objectTypeId);
      }
    } catch (cause) {
      error = cause instanceof Error ? cause.message : 'Failed to load spreadsheet sources';
    } finally {
      busy = false;
    }
  }

  async function loadObjectRows(typeId: string) {
    const rows: ObjectInstance[] = [];
    let page = 1;
    let total = 0;
    do {
      const response = await listObjects(typeId, { page, per_page: 100 });
      rows.push(...response.data);
      total = response.total;
      page += 1;
    } while (rows.length < total);
    return rows;
  }

  async function refreshDatasetSheet() {
    if (!datasetId) return;
    busy = true;
    try {
      datasetRows = await loadDatasetRows(datasetId);
    } catch (cause) {
      error = cause instanceof Error ? cause.message : 'Failed to refresh dataset sheet';
    } finally {
      busy = false;
    }
  }

  async function refreshObjectSheet() {
    if (!objectTypeId) return;
    busy = true;
    try {
      objectRows = await loadObjectRows(objectTypeId);
    } catch (cause) {
      error = cause instanceof Error ? cause.message : 'Failed to refresh ontology sheet';
    } finally {
      busy = false;
    }
  }

  function datasetColumns() {
    return Object.keys(datasetRows[0] ?? {}).slice(0, 12);
  }

  function objectColumns() {
    return Object.keys(objectRows[0]?.properties ?? {}).slice(0, 12);
  }

  function updateDatasetCell(rowIndex: number, column: string, value: string) {
    datasetRows = datasetRows.map((row, index) => index === rowIndex ? { ...row, [column]: value } : row);
  }

  async function saveDatasetSheet() {
    if (!datasetId || datasetRows.length === 0) return;

    busy = true;
    error = '';
    try {
      const payload = JSON.stringify(datasetRows, null, 2);
      const file = new File([payload], 'fusion-spreadsheet.json', { type: 'application/json' });
      await uploadData(datasetId, file);
      await refreshDatasetSheet();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : 'Failed to persist dataset spreadsheet';
    } finally {
      busy = false;
    }
  }

  function updateObjectCell(objectId: string, column: string, value: string) {
    objectRows = objectRows.map((row) => row.id === objectId
      ? { ...row, properties: { ...row.properties, [column]: value } }
      : row);
  }

  async function saveObjectRow(objectId: string) {
    if (!objectTypeId) return;
    const row = objectRows.find((entry) => entry.id === objectId);
    if (!row) return;

    busy = true;
    error = '';
    try {
      const updated = await updateObject(objectTypeId, objectId, {
        properties: row.properties,
      });
      objectRows = objectRows.map((entry) => entry.id === objectId ? updated : entry);
    } catch (cause) {
      error = cause instanceof Error ? cause.message : 'Failed to persist ontology row';
    } finally {
      busy = false;
    }
  }

  onMount(() => {
    void loadDatasetsAndTypes();
  });
</script>

<section class="rounded-[28px] border border-slate-200 bg-white p-5 shadow-sm dark:border-slate-800 dark:bg-slate-950">
  <div class="flex flex-wrap items-start justify-between gap-4">
    <div>
      <div class="text-[11px] font-semibold uppercase tracking-[0.28em] text-slate-500">Fusion Spreadsheet</div>
      <h2 class="mt-2 text-xl font-semibold text-slate-900 dark:text-slate-100">Bidirectional grid for datasets and ontology objects</h2>
      <p class="mt-2 max-w-3xl text-sm leading-6 text-slate-600 dark:text-slate-300">
        Edit dataset rows in bulk, or patch ontology objects inline, without leaving the Fusion workspace.
      </p>
    </div>

    <div class="flex gap-2">
      <button class={`rounded-xl px-4 py-2 text-sm font-medium ${mode === 'dataset' ? 'bg-slate-900 text-white dark:bg-slate-100 dark:text-slate-950' : 'border border-slate-300 dark:border-slate-700'}`} onclick={() => mode = 'dataset'}>
        Dataset sheet
      </button>
      <button class={`rounded-xl px-4 py-2 text-sm font-medium ${mode === 'ontology' ? 'bg-slate-900 text-white dark:bg-slate-100 dark:text-slate-950' : 'border border-slate-300 dark:border-slate-700'}`} onclick={() => mode = 'ontology'}>
        Ontology sheet
      </button>
    </div>
  </div>

  {#if error}
    <div class="mt-4 rounded-2xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-700 dark:border-rose-900/70 dark:bg-rose-950/40 dark:text-rose-200">{error}</div>
  {/if}

  {#if mode === 'dataset'}
    <div class="mt-5 space-y-4">
      <div class="flex flex-wrap items-center gap-3">
        <select bind:value={datasetId} class="rounded-xl border border-slate-300 bg-white px-3 py-2 text-sm outline-none dark:border-slate-700 dark:bg-slate-900" onchange={() => void refreshDatasetSheet()}>
          {#each datasets as dataset}
            <option value={dataset.id}>{dataset.name}</option>
          {/each}
        </select>
        <button class="rounded-xl border border-slate-300 px-4 py-2 text-sm font-medium dark:border-slate-700" onclick={() => void refreshDatasetSheet()} disabled={busy}>Refresh</button>
        <button class="rounded-xl bg-slate-900 px-4 py-2 text-sm font-medium text-white disabled:opacity-60 dark:bg-slate-100 dark:text-slate-950" onclick={saveDatasetSheet} disabled={busy || datasetRows.length === 0}>
          {busy ? 'Working...' : 'Save dataset'}
        </button>
      </div>

      <div class="overflow-x-auto rounded-2xl border border-slate-200 dark:border-slate-800">
        <table class="min-w-full text-sm">
          <thead class="bg-slate-50 dark:bg-slate-900/60">
            <tr>
              {#each datasetColumns() as column}
                <th class="px-3 py-2 text-left font-semibold text-slate-600 dark:text-slate-300">{column}</th>
              {/each}
            </tr>
          </thead>
          <tbody>
            {#each datasetRows.slice(0, 40) as row, rowIndex}
              <tr class="border-t border-slate-100 dark:border-slate-900">
                {#each datasetColumns() as column}
                  <td class="px-3 py-2">
                    <input
                      class="w-full rounded-lg border border-slate-200 bg-white px-2 py-1 text-sm outline-none dark:border-slate-700 dark:bg-slate-950"
                      value={String(row[column] ?? '')}
                      oninput={(event) => updateDatasetCell(rowIndex, column, (event.currentTarget as HTMLInputElement).value)}
                    />
                  </td>
                {/each}
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    </div>
  {:else}
    <div class="mt-5 space-y-4">
      <div class="flex flex-wrap items-center gap-3">
        <select bind:value={objectTypeId} class="rounded-xl border border-slate-300 bg-white px-3 py-2 text-sm outline-none dark:border-slate-700 dark:bg-slate-900" onchange={() => void refreshObjectSheet()}>
          {#each objectTypes as type}
            <option value={type.id}>{type.display_name}</option>
          {/each}
        </select>
        <button class="rounded-xl border border-slate-300 px-4 py-2 text-sm font-medium dark:border-slate-700" onclick={() => void refreshObjectSheet()} disabled={busy}>Refresh</button>
      </div>

      <div class="overflow-x-auto rounded-2xl border border-slate-200 dark:border-slate-800">
        <table class="min-w-full text-sm">
          <thead class="bg-slate-50 dark:bg-slate-900/60">
            <tr>
              {#each objectColumns() as column}
                <th class="px-3 py-2 text-left font-semibold text-slate-600 dark:text-slate-300">{column}</th>
              {/each}
              <th class="px-3 py-2 text-left font-semibold text-slate-600 dark:text-slate-300">action</th>
            </tr>
          </thead>
          <tbody>
            {#each objectRows.slice(0, 40) as row}
              <tr class="border-t border-slate-100 dark:border-slate-900">
                {#each objectColumns() as column}
                  <td class="px-3 py-2">
                    <input
                      class="w-full rounded-lg border border-slate-200 bg-white px-2 py-1 text-sm outline-none dark:border-slate-700 dark:bg-slate-950"
                      value={String(row.properties[column] ?? '')}
                      oninput={(event) => updateObjectCell(row.id, column, (event.currentTarget as HTMLInputElement).value)}
                    />
                  </td>
                {/each}
                <td class="px-3 py-2">
                  <button class="rounded-lg border border-slate-300 px-3 py-1.5 text-xs font-medium dark:border-slate-700" onclick={() => void saveObjectRow(row.id)} disabled={busy}>
                    Save row
                  </button>
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    </div>
  {/if}
</section>
