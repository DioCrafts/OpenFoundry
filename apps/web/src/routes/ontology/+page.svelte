<script lang="ts">
  import {
    deleteObjectType,
    listObjectTypes,
    searchOntology,
    type ObjectType,
    type SearchResult,
  } from '$lib/api/ontology';

  let types = $state<ObjectType[]>([]);
  let total = $state(0);
  let page = $state(1);
  let search = $state('');
  let loading = $state(true);

  let semanticQuery = $state('');
  let semanticKind = $state('all');
  let semanticLoading = $state(false);
  let semanticError = $state('');
  let searchResults = $state<SearchResult[]>([]);

  async function load() {
    loading = true;
    try {
      const res = await listObjectTypes({ page, per_page: 20, search: search || undefined });
      types = res.data;
      total = res.total;
    } catch (error) {
      console.error('Failed to load object types', error);
    } finally {
      loading = false;
    }
  }

  async function runSemanticSearch() {
    const query = semanticQuery.trim();
    if (!query) {
      semanticError = '';
      searchResults = [];
      return;
    }

    semanticLoading = true;
    semanticError = '';
    try {
      const response = await searchOntology({
        query,
        kind: semanticKind === 'all' ? undefined : semanticKind,
        limit: 20,
        semantic: true,
      });
      searchResults = response.data;
    } catch (error) {
      semanticError = error instanceof Error ? error.message : 'Failed to run ontology search';
    } finally {
      semanticLoading = false;
    }
  }

  async function handleDelete(id: string) {
    if (!confirm('Delete this object type and all its instances?')) return;
    await deleteObjectType(id);
    await load();
  }

  function resultBadgeClass(kind: string) {
    switch (kind) {
      case 'object_instance':
        return 'bg-emerald-50 text-emerald-700 dark:bg-emerald-950/40 dark:text-emerald-300';
      case 'action_type':
        return 'bg-sky-50 text-sky-700 dark:bg-sky-950/40 dark:text-sky-300';
      case 'interface':
        return 'bg-teal-50 text-teal-700 dark:bg-teal-950/40 dark:text-teal-300';
      case 'link_type':
        return 'bg-amber-50 text-amber-700 dark:bg-amber-950/40 dark:text-amber-300';
      default:
        return 'bg-slate-100 text-slate-700 dark:bg-slate-800 dark:text-slate-300';
    }
  }

  $effect(() => {
    load();
  });
</script>

<div class="space-y-6">
  <section class="rounded-[2rem] border border-slate-200 bg-white p-6 shadow-sm dark:border-slate-800 dark:bg-slate-900">
    <div class="flex flex-wrap items-center justify-between gap-4">
      <div>
        <p class="text-xs uppercase tracking-[0.22em] text-slate-500">Ontology</p>
        <h1 class="mt-1 text-3xl font-semibold tracking-tight text-slate-950 dark:text-slate-50">Ontology Explorer</h1>
        <p class="mt-2 max-w-3xl text-sm text-slate-500">
          Browse semantic types, search objects semantically, and jump into schema or object neighborhoods.
        </p>
      </div>
      <div class="flex flex-wrap gap-2">
        <a
          href="/ontology/graph"
          class="rounded-full border border-slate-300 px-4 py-2 text-sm font-medium text-slate-700 hover:bg-slate-100 dark:border-slate-700 dark:text-slate-200 dark:hover:bg-slate-800"
        >
          Graph View
        </a>
        <a
          href="/ontology/types"
          class="rounded-full bg-teal-600 px-4 py-2 text-sm font-medium text-white hover:bg-teal-700"
        >
          New Type
        </a>
      </div>
    </div>
  </section>

  <section class="grid gap-6 xl:grid-cols-[1.05fr_0.95fr]">
    <div class="rounded-[1.75rem] border border-slate-200 bg-white p-6 shadow-sm dark:border-slate-800 dark:bg-slate-900">
      <div class="flex items-center justify-between gap-4">
        <div>
          <h2 class="text-lg font-semibold text-slate-950 dark:text-slate-50">Types</h2>
          <p class="mt-1 text-sm text-slate-500">The semantic backbone of your platform.</p>
        </div>
        <span class="rounded-full bg-slate-100 px-3 py-1 text-xs text-slate-600 dark:bg-slate-800 dark:text-slate-300">
          {total} registered
        </span>
      </div>

      <div class="mt-4 flex gap-4">
        <input
          type="text"
          placeholder="Search object types..."
          bind:value={search}
          oninput={() => {
            page = 1;
            load();
          }}
          class="flex-1 rounded-2xl border border-slate-300 px-4 py-2.5 text-sm dark:border-slate-700 dark:bg-slate-950 dark:text-slate-100"
        />
      </div>

      {#if loading}
        <div class="mt-6 rounded-2xl border border-dashed border-slate-300 px-4 py-12 text-center text-sm text-slate-500 dark:border-slate-700">
          Loading ontology types...
        </div>
      {:else if types.length === 0}
        <div class="mt-6 rounded-2xl border border-dashed border-slate-300 px-4 py-12 text-center text-sm text-slate-500 dark:border-slate-700">
          No object types found. Create your first type to start building the ontology.
        </div>
      {:else}
        <div class="mt-6 grid gap-4 md:grid-cols-2">
          {#each types as t (t.id)}
            <article class="rounded-[1.5rem] border border-slate-200 p-4 transition-shadow hover:shadow-md dark:border-slate-800">
              <div class="flex items-start justify-between gap-3">
                <div class="flex items-center gap-3">
                  {#if t.icon}
                    <span class="text-2xl">{t.icon}</span>
                  {:else}
                    <span
                      class="flex h-10 w-10 items-center justify-center rounded-2xl text-sm font-semibold text-white"
                      style={`background-color: ${t.color || '#0f766e'}`}
                    >
                      {t.name.charAt(0).toUpperCase()}
                    </span>
                  {/if}
                  <div>
                    <h3 class="font-medium text-slate-950 dark:text-slate-50">{t.display_name}</h3>
                    <p class="text-xs font-mono text-slate-500">{t.name}</p>
                  </div>
                </div>
                <button
                  onclick={() => handleDelete(t.id)}
                  class="text-sm font-medium text-rose-600 hover:text-rose-700"
                >
                  Delete
                </button>
              </div>

              {#if t.description}
                <p class="mt-3 text-sm text-slate-500">{t.description}</p>
              {/if}

              <div class="mt-4 flex flex-wrap gap-2">
                <a href="/ontology/{t.id}" class="text-sm font-medium text-teal-700 hover:underline dark:text-teal-300">
                  Open detail
                </a>
                <a href="/ontology/graph?root_type_id={t.id}" class="text-sm font-medium text-slate-600 hover:underline dark:text-slate-300">
                  Focus graph
                </a>
              </div>
            </article>
          {/each}
        </div>

        {#if total > 20}
          <div class="mt-6 flex justify-center gap-4">
            <button
              disabled={page <= 1}
              onclick={() => {
                page--;
                load();
              }}
              class="rounded-full border border-slate-300 px-4 py-2 text-sm disabled:opacity-50 dark:border-slate-700"
            >
              Previous
            </button>
            <span class="py-2 text-sm text-slate-500">Page {page}</span>
            <button
              disabled={page * 20 >= total}
              onclick={() => {
                page++;
                load();
              }}
              class="rounded-full border border-slate-300 px-4 py-2 text-sm disabled:opacity-50 dark:border-slate-700"
            >
              Next
            </button>
          </div>
        {/if}
      {/if}
    </div>

    <div class="rounded-[1.75rem] border border-slate-200 bg-white p-6 shadow-sm dark:border-slate-800 dark:bg-slate-900">
      <div>
        <h2 class="text-lg font-semibold text-slate-950 dark:text-slate-50">Semantic Search</h2>
        <p class="mt-1 text-sm text-slate-500">
          Search across types, interfaces, actions, links, and object instances using mixed fulltext and semantic ranking.
        </p>
      </div>

      <form
        class="mt-4 space-y-3"
        onsubmit={(event) => {
          event.preventDefault();
          runSemanticSearch();
        }}
      >
        <div class="grid gap-3 md:grid-cols-[1fr_auto]">
          <input
            type="text"
            bind:value={semanticQuery}
            placeholder="e.g. fraud review, customer health, analyst escalation"
            class="rounded-2xl border border-slate-300 px-4 py-2.5 text-sm dark:border-slate-700 dark:bg-slate-950 dark:text-slate-100"
          />
          <select
            bind:value={semanticKind}
            class="rounded-2xl border border-slate-300 px-4 py-2.5 text-sm dark:border-slate-700 dark:bg-slate-950 dark:text-slate-100"
          >
            <option value="all">All kinds</option>
            <option value="object_type">Object types</option>
            <option value="object_instance">Object instances</option>
            <option value="interface">Interfaces</option>
            <option value="action_type">Action types</option>
            <option value="link_type">Link types</option>
          </select>
        </div>
        <div class="flex flex-wrap items-center gap-3">
          <button
            type="submit"
            disabled={semanticLoading}
            class="rounded-full bg-sky-600 px-4 py-2 text-sm font-medium text-white hover:bg-sky-700 disabled:opacity-50"
          >
            {semanticLoading ? 'Searching...' : 'Search ontology'}
          </button>
          <button
            type="button"
            class="rounded-full border border-slate-300 px-4 py-2 text-sm font-medium text-slate-700 hover:bg-slate-100 dark:border-slate-700 dark:text-slate-200 dark:hover:bg-slate-800"
            onclick={() => {
              semanticQuery = '';
              searchResults = [];
              semanticError = '';
            }}
          >
            Clear
          </button>
        </div>
      </form>

      {#if semanticError}
        <div class="mt-4 rounded-2xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-700 dark:border-rose-900/40 dark:bg-rose-950/20 dark:text-rose-300">
          {semanticError}
        </div>
      {/if}

      {#if searchResults.length === 0}
        <div class="mt-6 rounded-2xl border border-dashed border-slate-300 px-4 py-12 text-center text-sm text-slate-500 dark:border-slate-700">
          {#if semanticQuery.trim()}
            No ontology matches yet for that query.
          {:else}
            Search results will appear here.
          {/if}
        </div>
      {:else}
        <div class="mt-6 space-y-3">
          {#each searchResults as result (result.kind + ':' + result.id)}
            <article class="rounded-2xl border border-slate-200 p-4 dark:border-slate-800">
              <div class="flex flex-wrap items-start justify-between gap-3">
                <div class="space-y-2">
                  <div class="flex flex-wrap items-center gap-2">
                    <span class={`rounded-full px-2.5 py-1 text-xs font-medium ${resultBadgeClass(result.kind)}`}>
                      {result.kind.replaceAll('_', ' ')}
                    </span>
                    <span class="text-xs text-slate-500">score {result.score.toFixed(2)}</span>
                  </div>
                  <div>
                    <h3 class="font-medium text-slate-950 dark:text-slate-50">{result.title}</h3>
                    {#if result.subtitle}
                      <p class="mt-1 text-xs font-mono text-slate-500">{result.subtitle}</p>
                    {/if}
                  </div>
                </div>
                <div class="flex flex-wrap gap-2">
                  <a
                    href={result.route}
                    class="rounded-full border border-slate-300 px-3 py-1.5 text-xs font-medium text-slate-700 hover:bg-slate-100 dark:border-slate-700 dark:text-slate-200 dark:hover:bg-slate-800"
                  >
                    Open
                  </a>
                  {#if result.kind === 'object_instance'}
                    <a
                      href="/ontology/graph?root_object_id={result.id}"
                      class="rounded-full bg-slate-900 px-3 py-1.5 text-xs font-medium text-white hover:bg-slate-700 dark:bg-slate-100 dark:text-slate-900 dark:hover:bg-slate-300"
                    >
                      Neighborhood
                    </a>
                  {/if}
                </div>
              </div>
              {#if result.snippet}
                <p class="mt-3 text-sm text-slate-500">{result.snippet}</p>
              {/if}
            </article>
          {/each}
        </div>
      {/if}
    </div>
  </section>
</div>
