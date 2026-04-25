<script lang="ts">
  import {
    createObjectSet,
    deleteObjectSet,
    evaluateObjectSet,
    listObjectSets,
    listObjectTypes,
    materializeObjectSet,
    type ObjectSetDefinition,
    type ObjectSetEvaluationResponse,
    type ObjectType,
  } from '$lib/api/ontology';

  let objectTypes = $state<ObjectType[]>([]);
  let objectSets = $state<ObjectSetDefinition[]>([]);
  let selectedBaseTypeId = $state('');
  let name = $state('');
  let description = $state('');
  let filtersText = $state('[\n  {\n    "field": "status",\n    "operator": "equals",\n    "value": "active"\n  }\n]');
  let traversalsText = $state('[]');
  let joinText = $state('null');
  let projectionsText = $state('base.id,base.properties.status,joined.properties.name');
  let whatIfLabel = $state('');
  let policyText = $state('{\n  "allowed_markings": ["public", "confidential"],\n  "deny_guest_sessions": false\n}');
  let loading = $state(true);
  let saving = $state(false);
  let error = $state('');
  let evaluation = $state<ObjectSetEvaluationResponse | null>(null);

  async function load() {
    loading = true;
    error = '';
    try {
      const [typesResponse, setsResponse] = await Promise.all([
        listObjectTypes({ per_page: 100 }),
        listObjectSets(),
      ]);
      objectTypes = typesResponse.data;
      objectSets = setsResponse.data;
      if (!selectedBaseTypeId && objectTypes[0]) {
        selectedBaseTypeId = objectTypes[0].id;
      }
    } catch (cause) {
      error = cause instanceof Error ? cause.message : 'Failed to load object sets';
    } finally {
      loading = false;
    }
  }

  function parseJson<T>(text: string, fallback: T): T {
    const trimmed = text.trim();
    if (!trimmed) return fallback;
    return JSON.parse(trimmed) as T;
  }

  async function submit() {
    if (!selectedBaseTypeId || !name.trim()) {
      error = 'Name and base object type are required';
      return;
    }

    saving = true;
    error = '';
    try {
      await createObjectSet({
        name,
        description,
        base_object_type_id: selectedBaseTypeId,
        filters: parseJson(filtersText, []),
        traversals: parseJson(traversalsText, []),
        join: parseJson(joinText, null),
        projections: projectionsText
          .split(',')
          .map((item) => item.trim())
          .filter(Boolean),
        what_if_label: whatIfLabel.trim() || null,
        policy: parseJson(policyText, {}),
      });
      name = '';
      description = '';
      whatIfLabel = '';
      await load();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : 'Failed to create object set';
    } finally {
      saving = false;
    }
  }

  async function runEvaluation(objectSet: ObjectSetDefinition, mode: 'preview' | 'materialize') {
    error = '';
    try {
      evaluation =
        mode === 'preview'
          ? await evaluateObjectSet(objectSet.id, { limit: 100 })
          : await materializeObjectSet(objectSet.id, { limit: 500 });
      await load();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : `Failed to ${mode} object set`;
    }
  }

  async function remove(objectSet: ObjectSetDefinition) {
    if (!confirm(`Delete object set "${objectSet.name}"?`)) return;
    await deleteObjectSet(objectSet.id);
    if (evaluation?.object_set.id === objectSet.id) {
      evaluation = null;
    }
    await load();
  }

  function typeLabel(typeId: string) {
    return objectTypes.find((entry) => entry.id === typeId)?.display_name ?? typeId;
  }

  $effect(() => {
    load();
  });
</script>

<div class="space-y-6">
  <section class="rounded-[2rem] border border-slate-200 bg-white p-6 shadow-sm dark:border-slate-800 dark:bg-slate-900">
    <div class="flex flex-wrap items-center justify-between gap-4">
      <div>
        <p class="text-xs uppercase tracking-[0.22em] text-slate-500">Ontology Runtime</p>
        <h1 class="mt-1 text-3xl font-semibold tracking-tight text-slate-950 dark:text-slate-50">Object Sets</h1>
        <p class="mt-2 max-w-3xl text-sm text-slate-500">
          Persist semantic filters, traversals, projections, what-if labels, and optional joins so Quiver, apps, workflows, and agents can reuse the same governed runtime.
        </p>
      </div>
      <div class="flex flex-wrap gap-2">
        <a
          href="/ontology"
          class="rounded-full border border-slate-300 px-4 py-2 text-sm font-medium text-slate-700 hover:bg-slate-100 dark:border-slate-700 dark:text-slate-200 dark:hover:bg-slate-800"
        >
          Back to Ontology
        </a>
      </div>
    </div>
  </section>

  {#if error}
    <div class="rounded-2xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-700 dark:border-rose-900/40 dark:bg-rose-950/20 dark:text-rose-200">
      {error}
    </div>
  {/if}

  <section class="grid gap-6 xl:grid-cols-[0.95fr_1.05fr]">
    <div class="rounded-[1.75rem] border border-slate-200 bg-white p-6 shadow-sm dark:border-slate-800 dark:bg-slate-900">
      <div>
        <h2 class="text-lg font-semibold text-slate-950 dark:text-slate-50">Create Object Set</h2>
        <p class="mt-1 text-sm text-slate-500">Use JSON for the advanced parts so we can move quickly while the runtime is still evolving.</p>
      </div>

      <form
        class="mt-5 space-y-4"
        onsubmit={(event) => {
          event.preventDefault();
          submit();
        }}
      >
        <div class="grid gap-4 md:grid-cols-2">
          <label class="block text-sm">
            <span class="mb-1 block font-medium text-slate-700 dark:text-slate-200">Name</span>
            <input bind:value={name} class="w-full rounded-2xl border border-slate-300 px-4 py-2.5 dark:border-slate-700 dark:bg-slate-950 dark:text-slate-100" placeholder="high_risk_cases" />
          </label>
          <label class="block text-sm">
            <span class="mb-1 block font-medium text-slate-700 dark:text-slate-200">Base object type</span>
            <select bind:value={selectedBaseTypeId} class="w-full rounded-2xl border border-slate-300 px-4 py-2.5 dark:border-slate-700 dark:bg-slate-950 dark:text-slate-100">
              <option value="" disabled>Select a type</option>
              {#each objectTypes as objectType (objectType.id)}
                <option value={objectType.id}>{objectType.display_name}</option>
              {/each}
            </select>
          </label>
        </div>

        <label class="block text-sm">
          <span class="mb-1 block font-medium text-slate-700 dark:text-slate-200">Description</span>
          <input bind:value={description} class="w-full rounded-2xl border border-slate-300 px-4 py-2.5 dark:border-slate-700 dark:bg-slate-950 dark:text-slate-100" placeholder="Cases with a high score and governed traversal context" />
        </label>

        <label class="block text-sm">
          <span class="mb-1 block font-medium text-slate-700 dark:text-slate-200">Filters JSON</span>
          <textarea bind:value={filtersText} rows={8} class="w-full rounded-2xl border border-slate-300 bg-slate-950 px-4 py-3 font-mono text-xs text-slate-100 dark:border-slate-700" spellcheck="false"></textarea>
        </label>

        <div class="grid gap-4 lg:grid-cols-2">
          <label class="block text-sm">
            <span class="mb-1 block font-medium text-slate-700 dark:text-slate-200">Traversals JSON</span>
            <textarea bind:value={traversalsText} rows={8} class="w-full rounded-2xl border border-slate-300 bg-slate-950 px-4 py-3 font-mono text-xs text-slate-100 dark:border-slate-700" spellcheck="false"></textarea>
          </label>
          <label class="block text-sm">
            <span class="mb-1 block font-medium text-slate-700 dark:text-slate-200">Join JSON</span>
            <textarea bind:value={joinText} rows={8} class="w-full rounded-2xl border border-slate-300 bg-slate-950 px-4 py-3 font-mono text-xs text-slate-100 dark:border-slate-700" spellcheck="false"></textarea>
          </label>
        </div>

        <div class="grid gap-4 md:grid-cols-2">
          <label class="block text-sm">
            <span class="mb-1 block font-medium text-slate-700 dark:text-slate-200">Projections CSV</span>
            <input bind:value={projectionsText} class="w-full rounded-2xl border border-slate-300 px-4 py-2.5 font-mono text-xs dark:border-slate-700 dark:bg-slate-950 dark:text-slate-100" />
          </label>
          <label class="block text-sm">
            <span class="mb-1 block font-medium text-slate-700 dark:text-slate-200">What-if label</span>
            <input bind:value={whatIfLabel} class="w-full rounded-2xl border border-slate-300 px-4 py-2.5 dark:border-slate-700 dark:bg-slate-950 dark:text-slate-100" placeholder="what_if_q2_compliance" />
          </label>
        </div>

        <label class="block text-sm">
          <span class="mb-1 block font-medium text-slate-700 dark:text-slate-200">Policy JSON</span>
          <textarea bind:value={policyText} rows={6} class="w-full rounded-2xl border border-slate-300 bg-slate-950 px-4 py-3 font-mono text-xs text-slate-100 dark:border-slate-700" spellcheck="false"></textarea>
        </label>

        <button disabled={saving || loading} class="rounded-full bg-teal-600 px-4 py-2 text-sm font-medium text-white hover:bg-teal-700 disabled:opacity-60">
          {saving ? 'Creating...' : 'Create object set'}
        </button>
      </form>
    </div>

    <div class="space-y-6">
      <div class="rounded-[1.75rem] border border-slate-200 bg-white p-6 shadow-sm dark:border-slate-800 dark:bg-slate-900">
        <div class="flex items-center justify-between gap-4">
          <div>
            <h2 class="text-lg font-semibold text-slate-950 dark:text-slate-50">Catalog</h2>
            <p class="mt-1 text-sm text-slate-500">Saved object sets can be previewed or materialized into snapshot rows.</p>
          </div>
          <span class="rounded-full bg-slate-100 px-3 py-1 text-xs text-slate-600 dark:bg-slate-800 dark:text-slate-300">
            {objectSets.length} sets
          </span>
        </div>

        {#if loading}
          <div class="mt-5 rounded-2xl border border-dashed border-slate-300 px-4 py-10 text-center text-sm text-slate-500 dark:border-slate-700">
            Loading object sets...
          </div>
        {:else if objectSets.length === 0}
          <div class="mt-5 rounded-2xl border border-dashed border-slate-300 px-4 py-10 text-center text-sm text-slate-500 dark:border-slate-700">
            No object sets yet. Create the first runtime blueprint from the form on the left.
          </div>
        {:else}
          <div class="mt-5 space-y-4">
            {#each objectSets as objectSet (objectSet.id)}
              <article class="rounded-[1.5rem] border border-slate-200 p-4 dark:border-slate-800">
                <div class="flex flex-wrap items-start justify-between gap-3">
                  <div>
                    <h3 class="font-medium text-slate-950 dark:text-slate-50">{objectSet.name}</h3>
                    <p class="mt-1 text-xs text-slate-500">
                      Base type: {typeLabel(objectSet.base_object_type_id)} · Filters: {objectSet.filters.length} · Traversals: {objectSet.traversals.length} · Rows materialized: {objectSet.materialized_row_count}
                    </p>
                    {#if objectSet.description}
                      <p class="mt-2 text-sm text-slate-500">{objectSet.description}</p>
                    {/if}
                  </div>
                  <button onclick={() => remove(objectSet)} class="text-sm font-medium text-rose-600 hover:text-rose-700">
                    Delete
                  </button>
                </div>

                <div class="mt-4 flex flex-wrap gap-2">
                  <button onclick={() => runEvaluation(objectSet, 'preview')} class="rounded-full border border-slate-300 px-3 py-1.5 text-sm font-medium text-slate-700 hover:bg-slate-100 dark:border-slate-700 dark:text-slate-200 dark:hover:bg-slate-800">
                    Preview
                  </button>
                  <button onclick={() => runEvaluation(objectSet, 'materialize')} class="rounded-full bg-slate-900 px-3 py-1.5 text-sm font-medium text-white hover:bg-slate-800 dark:bg-slate-100 dark:text-slate-950 dark:hover:bg-slate-200">
                    Materialize snapshot
                  </button>
                </div>
              </article>
            {/each}
          </div>
        {/if}
      </div>

      <div class="rounded-[1.75rem] border border-slate-200 bg-white p-6 shadow-sm dark:border-slate-800 dark:bg-slate-900">
        <div class="flex items-center justify-between gap-4">
          <div>
            <h2 class="text-lg font-semibold text-slate-950 dark:text-slate-50">Evaluation</h2>
            <p class="mt-1 text-sm text-slate-500">Preview rows, joined context, and traversal blast radius from the runtime.</p>
          </div>
          {#if evaluation}
            <span class="rounded-full bg-emerald-50 px-3 py-1 text-xs text-emerald-700 dark:bg-emerald-950/40 dark:text-emerald-300">
              {evaluation.materialized ? 'Materialized' : 'Preview'}
            </span>
          {/if}
        </div>

        {#if evaluation}
          <div class="mt-5 grid gap-4 md:grid-cols-3">
            <div class="rounded-2xl bg-slate-50 p-4 dark:bg-slate-800/60">
              <p class="text-xs uppercase tracking-[0.18em] text-slate-500">Base matches</p>
              <p class="mt-2 text-2xl font-semibold text-slate-950 dark:text-slate-50">{evaluation.total_base_matches}</p>
            </div>
            <div class="rounded-2xl bg-slate-50 p-4 dark:bg-slate-800/60">
              <p class="text-xs uppercase tracking-[0.18em] text-slate-500">Rows</p>
              <p class="mt-2 text-2xl font-semibold text-slate-950 dark:text-slate-50">{evaluation.total_rows}</p>
            </div>
            <div class="rounded-2xl bg-slate-50 p-4 dark:bg-slate-800/60">
              <p class="text-xs uppercase tracking-[0.18em] text-slate-500">Traversal neighbors</p>
              <p class="mt-2 text-2xl font-semibold text-slate-950 dark:text-slate-50">{evaluation.traversal_neighbor_count}</p>
            </div>
          </div>

          <pre class="mt-5 overflow-x-auto rounded-3xl bg-slate-950 p-4 text-xs text-slate-100">{JSON.stringify(evaluation.rows, null, 2)}</pre>
        {:else}
          <div class="mt-5 rounded-2xl border border-dashed border-slate-300 px-4 py-10 text-center text-sm text-slate-500 dark:border-slate-700">
            Run a preview or materialization from the catalog to inspect the runtime output.
          </div>
        {/if}
      </div>
    </div>
  </section>
</div>
