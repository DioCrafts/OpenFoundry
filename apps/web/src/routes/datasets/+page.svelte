<script lang="ts">
  import { onMount } from 'svelte';
  import { listUsers, type UserProfile } from '$lib/api/auth';
  import {
    deleteDataset,
    getCatalogFacets,
    getDatasetQuality,
    listDatasets,
    type Dataset,
    type DatasetQualityResponse,
  } from '$lib/api/datasets';

  let datasets = $state<Dataset[]>([]);
  let qualityByDatasetId = $state<Record<string, DatasetQualityResponse>>({});
  let users = $state<UserProfile[]>([]);
  let availableTags = $state<{ value: string; count: number }[]>([]);
  let total = $state(0);
  let page = $state(1);
  let search = $state('');
  let selectedTag = $state('');
  let selectedOwnerId = $state('');
  let loading = $state(true);
  let loadingQuality = $state(false);
  let error = $state('');

  function ownerName(ownerId: string) {
    return users.find((user) => user.id === ownerId)?.name ?? ownerId.slice(0, 8);
  }

  function scoreFor(datasetId: string) {
    return qualityByDatasetId[datasetId]?.score ?? null;
  }

  function activeAlertsFor(datasetId: string) {
    return qualityByDatasetId[datasetId]?.alerts.filter((alert) => alert.status === 'active').length ?? 0;
  }

  function toneFor(score: number | null) {
    if (score === null) return 'bg-gray-100 text-gray-600 dark:bg-gray-800 dark:text-gray-300';
    if (score >= 90) return 'bg-emerald-100 text-emerald-700 dark:bg-emerald-900/40 dark:text-emerald-300';
    if (score >= 75) return 'bg-amber-100 text-amber-700 dark:bg-amber-900/40 dark:text-amber-300';
    return 'bg-rose-100 text-rose-700 dark:bg-rose-900/40 dark:text-rose-300';
  }

  function averageQualityScore() {
    const scores = datasets
      .map((dataset) => qualityByDatasetId[dataset.id]?.score)
      .filter((score): score is number => typeof score === 'number');
    if (scores.length === 0) return null;
    return scores.reduce((sum, score) => sum + score, 0) / scores.length;
  }

  function totalActiveAlerts() {
    return datasets.reduce((sum, dataset) => sum + activeAlertsFor(dataset.id), 0);
  }

  async function loadFilters() {
    try {
      const [facets, allUsers] = await Promise.all([
        getCatalogFacets(),
        listUsers().catch(() => [] as UserProfile[]),
      ]);
      availableTags = facets.tags;
      users = allUsers;
    } catch (cause) {
      console.error('Failed to load catalog filters', cause);
    }
  }

  async function loadQuality(datasetList: Dataset[]) {
    loadingQuality = true;
    try {
      const results = await Promise.allSettled(
        datasetList.map(async (dataset) => [dataset.id, await getDatasetQuality(dataset.id)] as const),
      );
      const next: Record<string, DatasetQualityResponse> = {};
      for (const result of results) {
        if (result.status === 'fulfilled') {
          const [datasetId, quality] = result.value;
          next[datasetId] = quality;
        }
      }
      qualityByDatasetId = next;
    } finally {
      loadingQuality = false;
    }
  }

  async function load() {
    loading = true;
    error = '';
    try {
      const response = await listDatasets({
        page,
        per_page: 20,
        search: search || undefined,
        tag: selectedTag || undefined,
        owner_id: selectedOwnerId || undefined,
      });
      datasets = response.data;
      total = response.total;
      await loadQuality(response.data);
    } catch (cause) {
      console.error('Failed to load datasets', cause);
      error = cause instanceof Error ? cause.message : 'Failed to load datasets';
    } finally {
      loading = false;
    }
  }

  async function handleDelete(id: string) {
    if (!confirm('Delete this dataset?')) return;
    await deleteDataset(id);
    await load();
  }

  function applyFilters() {
    page = 1;
    void load();
  }

  function clearFilters() {
    search = '';
    selectedTag = '';
    selectedOwnerId = '';
    page = 1;
    void load();
  }

  onMount(async () => {
    await loadFilters();
    await load();
  });
</script>

<div class="space-y-6">
  <div class="flex items-center justify-between">
    <div>
      <h1 class="text-2xl font-bold">Data Catalog</h1>
      <p class="mt-1 text-sm text-gray-500">Search datasets by text, tag, and owner, then inspect the latest quality snapshot.</p>
    </div>
    <a href="/datasets/upload" class="rounded-xl bg-blue-600 px-4 py-2 text-white hover:bg-blue-700">
      Upload Dataset
    </a>
  </div>

  <div class="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
    <div class="rounded-2xl border border-slate-200 bg-white p-4 shadow-sm dark:border-gray-700 dark:bg-gray-900">
      <div class="text-xs uppercase tracking-[0.22em] text-gray-400">Datasets</div>
      <div class="mt-3 text-3xl font-semibold">{total}</div>
    </div>
    <div class="rounded-2xl border border-slate-200 bg-white p-4 shadow-sm dark:border-gray-700 dark:bg-gray-900">
      <div class="text-xs uppercase tracking-[0.22em] text-gray-400">Indexed Tags</div>
      <div class="mt-3 text-3xl font-semibold">{availableTags.length}</div>
    </div>
    <div class="rounded-2xl border border-slate-200 bg-white p-4 shadow-sm dark:border-gray-700 dark:bg-gray-900">
      <div class="text-xs uppercase tracking-[0.22em] text-gray-400">Average Quality</div>
      <div class="mt-3 text-3xl font-semibold">
        {#if averageQualityScore() !== null}
          {averageQualityScore()?.toFixed(1)}
        {:else}
          --
        {/if}
      </div>
    </div>
    <div class="rounded-2xl border border-slate-200 bg-white p-4 shadow-sm dark:border-gray-700 dark:bg-gray-900">
      <div class="text-xs uppercase tracking-[0.22em] text-gray-400">Active Alerts</div>
      <div class="mt-3 text-3xl font-semibold">{totalActiveAlerts()}</div>
    </div>
  </div>

  <div class="rounded-2xl border border-slate-200 bg-white p-5 shadow-sm dark:border-gray-700 dark:bg-gray-900">
    <div class="grid gap-4 lg:grid-cols-[2fr,1fr,1fr,auto,auto]">
      <input
        type="text"
        placeholder="Full-text search by dataset name or description"
        bind:value={search}
        class="rounded-xl border border-slate-200 px-3 py-2 dark:border-gray-700 dark:bg-gray-800"
      />
      <select bind:value={selectedOwnerId} class="rounded-xl border border-slate-200 px-3 py-2 dark:border-gray-700 dark:bg-gray-800">
        <option value="">All owners</option>
        {#each users as user (user.id)}
          <option value={user.id}>{user.name}</option>
        {/each}
      </select>
      <select bind:value={selectedTag} class="rounded-xl border border-slate-200 px-3 py-2 dark:border-gray-700 dark:bg-gray-800">
        <option value="">All tags</option>
        {#each availableTags as tag (tag.value)}
          <option value={tag.value}>{tag.value} ({tag.count})</option>
        {/each}
      </select>
      <button onclick={applyFilters} class="rounded-xl bg-blue-600 px-4 py-2 text-white hover:bg-blue-700">Apply</button>
      <button onclick={clearFilters} class="rounded-xl border border-slate-200 px-4 py-2 hover:bg-slate-50 dark:border-gray-700 dark:hover:bg-gray-800">Reset</button>
    </div>

    {#if selectedTag || selectedOwnerId}
      <div class="mt-4 flex flex-wrap gap-2">
        {#if selectedOwnerId}
          <span class="rounded-full bg-slate-100 px-3 py-1 text-xs font-medium text-slate-700 dark:bg-gray-800 dark:text-gray-200">
            Owner: {ownerName(selectedOwnerId)}
          </span>
        {/if}
        {#if selectedTag}
          <span class="rounded-full bg-blue-100 px-3 py-1 text-xs font-medium text-blue-700 dark:bg-blue-900/40 dark:text-blue-300">
            Tag: {selectedTag}
          </span>
        {/if}
      </div>
    {/if}
  </div>

  {#if error}
    <div class="rounded-xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-700 dark:border-rose-900/40 dark:bg-rose-950/40 dark:text-rose-300">
      {error}
    </div>
  {/if}

  {#if loading}
    <div class="text-center py-12 text-gray-500">Loading...</div>
  {:else if datasets.length === 0}
    <div class="text-center py-12 text-gray-500">
      No datasets matched the current catalog filters.
    </div>
  {:else}
    <div class="grid gap-4">
      {#each datasets as dataset (dataset.id)}
        <div class="rounded-2xl border border-slate-200 bg-white p-5 shadow-sm transition-colors hover:border-blue-500 dark:border-gray-700 dark:bg-gray-900">
          <div class="flex flex-col gap-4 lg:flex-row lg:items-start lg:justify-between">
            <div class="space-y-3">
              <div class="flex flex-wrap items-center gap-2">
                <a href="/datasets/{dataset.id}" class="text-lg font-semibold hover:text-blue-600">{dataset.name}</a>
                <span class={`rounded-full px-2.5 py-1 text-xs font-medium ${toneFor(scoreFor(dataset.id))}`}>
                  {#if scoreFor(dataset.id) !== null}
                    Quality {scoreFor(dataset.id)?.toFixed(1)}
                  {:else if loadingQuality}
                    Profiling...
                  {:else}
                    No profile
                  {/if}
                </span>
                {#if activeAlertsFor(dataset.id) > 0}
                  <span class="rounded-full bg-rose-100 px-2.5 py-1 text-xs font-medium text-rose-700 dark:bg-rose-900/40 dark:text-rose-300">
                    {activeAlertsFor(dataset.id)} active alert{activeAlertsFor(dataset.id) === 1 ? '' : 's'}
                  </span>
                {/if}
              </div>

              <p class="text-sm text-gray-500">{dataset.description || 'No description'}</p>

              <div class="flex flex-wrap gap-2 text-xs text-gray-500">
                <span class="rounded-full bg-slate-100 px-2 py-1 dark:bg-gray-800">{dataset.format}</span>
                <span>Owner {ownerName(dataset.owner_id)}</span>
                <span>v{dataset.current_version}</span>
                <span>{dataset.row_count.toLocaleString()} rows</span>
                <span>{(dataset.size_bytes / 1024).toFixed(1)} KB</span>
              </div>

              <div class="flex flex-wrap gap-2">
                {#each dataset.tags as tag}
                  <span class="rounded-full bg-blue-100 px-2.5 py-1 text-xs font-medium text-blue-700 dark:bg-blue-900/40 dark:text-blue-300">{tag}</span>
                {/each}
                {#if dataset.tags.length === 0}
                  <span class="text-xs text-gray-400">No tags</span>
                {/if}
              </div>
            </div>

            <div class="flex items-start gap-3 lg:flex-col lg:items-end">
              <a href="/datasets/{dataset.id}" class="rounded-xl border border-slate-200 px-3 py-2 text-sm hover:bg-slate-50 dark:border-gray-700 dark:hover:bg-gray-800">Open</a>
              <button onclick={() => handleDelete(dataset.id)} class="rounded-xl border border-rose-200 px-3 py-2 text-sm text-rose-600 hover:bg-rose-50 dark:border-rose-900/40 dark:hover:bg-rose-950/30">Delete</button>
            </div>
          </div>
        </div>
      {/each}
    </div>

    {#if total > 20}
      <div class="flex justify-center gap-2">
        <button
          disabled={page <= 1}
          onclick={() => {
            page -= 1;
            void load();
          }}
          class="px-3 py-1 border rounded disabled:opacity-50"
        >Prev</button>
        <span class="px-3 py-1">Page {page}</span>
        <button
          disabled={datasets.length < 20}
          onclick={() => {
            page += 1;
            void load();
          }}
          class="px-3 py-1 border rounded disabled:opacity-50"
        >Next</button>
      </div>
    {/if}
  {/if}
</div>
