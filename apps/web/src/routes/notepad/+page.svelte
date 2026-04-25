<script lang="ts">
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';
  import {
    createNotepadDocument,
    deleteNotepadDocument,
    listNotepadDocuments,
    type NotepadDocument,
  } from '$lib/api/notepad';

  type Template = {
    key: string;
    name: string;
    description: string;
    content: string;
    widgets: Array<Record<string, unknown>>;
  };

  const templates: Template[] = [
    {
      key: 'executive-brief',
      name: 'Executive Brief',
      description: 'One-page summary with highlights, decisions, and next moves.',
      content: `# Executive brief

## Situation
- Summarize the current state in plain language.

## What changed
- Highlight the biggest movement.

## Decisions
- Record approvals, blockers, and owners.

## Next week
- List the actions that need to happen next.`,
      widgets: [
        { id: crypto.randomUUID(), kind: 'contour', title: 'Top-down trend', summary: 'Embed a Contour board or exported insight snapshot.' },
      ],
    },
    {
      key: 'investigation',
      name: 'Investigation',
      description: 'Evidence-first writeup with hypotheses and findings.',
      content: `# Investigation log

## Hypothesis
- State the working theory.

## Evidence
- Capture the signals that support or contradict it.

## Findings
- List the confirmed facts.

## Follow-up
- Record the next analysis steps.`,
      widgets: [
        { id: crypto.randomUUID(), kind: 'quiver', title: 'Object/time-series lens', summary: 'Attach Quiver object analytics and relationship snapshots.' },
      ],
    },
    {
      key: 'operating-review',
      name: 'Operating Review',
      description: 'Recurring operating cadence with metrics, narrative, and actions.',
      content: `# Operating review

## KPI pulse
- Describe the current business pulse.

## Risks
- Call out material risks.

## Opportunities
- Capture upside and experiments.

## Commitments
- Make ownership explicit.`,
      widgets: [
        { id: crypto.randomUUID(), kind: 'report', title: 'Scheduled report', summary: 'Link the latest report execution or exported deck.' },
        { id: crypto.randomUUID(), kind: 'fusion', title: 'Spreadsheet decision log', summary: 'Reference Fusion edits and reconciliations.' },
      ],
    },
  ];

  let documents = $state<NotepadDocument[]>([]);
  let loading = $state(true);
  let search = $state('');
  let creating = $state(false);
  let error = $state('');

  async function load() {
    loading = true;
    error = '';
    try {
      const response = await listNotepadDocuments({ search: search.trim() || undefined, per_page: 100 });
      documents = response.data;
    } catch (cause) {
      error = cause instanceof Error ? cause.message : 'Failed to load notepad documents';
      documents = [];
    } finally {
      loading = false;
    }
  }

  async function createFromTemplate(template: Template) {
    creating = true;
    try {
      const document = await createNotepadDocument({
        title: template.name,
        description: template.description,
        content: template.content,
        template_key: template.key,
        widgets: template.widgets,
      });
      await goto(`/notepad/${document.id}`);
    } finally {
      creating = false;
    }
  }

  async function createBlankDocument() {
    creating = true;
    try {
      const document = await createNotepadDocument({
        title: 'Untitled document',
        content: '# New document\n\nStart writing here.',
        widgets: [],
      });
      await goto(`/notepad/${document.id}`);
    } finally {
      creating = false;
    }
  }

  async function removeDocument(id: string) {
    if (!confirm('Delete this notepad document?')) {
      return;
    }

    await deleteNotepadDocument(id);
    await load();
  }

  onMount(() => {
    void load();
  });
</script>

<svelte:head>
  <title>OpenFoundry — Notepad</title>
</svelte:head>

<div class="mx-auto max-w-7xl space-y-6">
  <section class="rounded-[2rem] border border-slate-200 bg-white p-6 shadow-sm dark:border-slate-800 dark:bg-slate-950">
    <div class="flex flex-wrap items-start justify-between gap-4">
      <div class="max-w-3xl">
        <div class="text-[11px] font-semibold uppercase tracking-[0.28em] text-cyan-600">Notepad</div>
        <h1 class="mt-2 text-4xl font-semibold tracking-tight text-slate-950 dark:text-slate-50">Collaborative documents with live workspace embeds</h1>
        <p class="mt-3 text-base leading-7 text-slate-600 dark:text-slate-300">
          Capture narrative, decisions, and evidence in one place, then export or index the document into AIP knowledge.
        </p>
      </div>

      <button class="rounded-xl bg-slate-900 px-4 py-2 text-sm font-medium text-white dark:bg-slate-100 dark:text-slate-950" onclick={createBlankDocument} disabled={creating}>
        {creating ? 'Creating...' : 'New document'}
      </button>
    </div>
  </section>

  <section class="grid gap-6 xl:grid-cols-[minmax(0,1.1fr)_minmax(320px,0.9fr)]">
    <div class="space-y-4 rounded-[2rem] border border-slate-200 bg-white p-6 shadow-sm dark:border-slate-800 dark:bg-slate-950">
      <div class="flex flex-wrap items-center justify-between gap-3">
        <div>
          <div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Documents</div>
          <h2 class="mt-1 text-xl font-semibold text-slate-950 dark:text-slate-50">Persistent operating notes</h2>
        </div>

        <input
          class="w-full rounded-xl border border-slate-300 bg-white px-3 py-2 text-sm outline-none md:w-80 dark:border-slate-700 dark:bg-slate-900"
          bind:value={search}
          placeholder="Search title or description"
          onkeydown={(event) => event.key === 'Enter' && void load()}
        />
      </div>

      {#if error}
        <div class="rounded-2xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-700 dark:border-rose-900 dark:bg-rose-950/40 dark:text-rose-300">
          {error}
        </div>
      {/if}

      {#if loading}
        <div class="py-12 text-sm text-slate-500">Loading documents...</div>
      {:else if documents.length === 0}
        <div class="rounded-2xl border border-dashed border-slate-300 px-4 py-8 text-sm text-slate-500 dark:border-slate-700 dark:text-slate-400">
          No documents yet. Start from a template or create a blank note.
        </div>
      {:else}
        <div class="space-y-3">
          {#each documents as document (document.id)}
            <div class="rounded-2xl border border-slate-200 bg-slate-50 p-4 dark:border-slate-800 dark:bg-slate-900/60">
              <div class="flex items-start justify-between gap-3">
                <div class="min-w-0">
                  <a href={`/notepad/${document.id}`} class="text-lg font-semibold text-slate-900 hover:text-cyan-700 dark:text-slate-100 dark:hover:text-cyan-300">
                    {document.title}
                  </a>
                  <p class="mt-1 text-sm text-slate-600 dark:text-slate-300">{document.description || 'No description yet.'}</p>
                  <div class="mt-3 flex flex-wrap gap-2 text-xs text-slate-500 dark:text-slate-400">
                    {#if document.template_key}
                      <span class="rounded-full border border-slate-300 px-2.5 py-1 dark:border-slate-700">{document.template_key}</span>
                    {/if}
                    <span class="rounded-full border border-slate-300 px-2.5 py-1 dark:border-slate-700">{document.widgets.length} embeds</span>
                    {#if document.last_indexed_at}
                      <span class="rounded-full border border-emerald-300 px-2.5 py-1 text-emerald-700 dark:border-emerald-700 dark:text-emerald-300">Indexed in AIP</span>
                    {/if}
                  </div>
                </div>

                <button class="rounded-lg border border-slate-300 px-3 py-1.5 text-xs font-medium text-slate-700 hover:bg-slate-100 dark:border-slate-700 dark:text-slate-200 dark:hover:bg-slate-800" onclick={() => void removeDocument(document.id)}>
                  Delete
                </button>
              </div>
            </div>
          {/each}
        </div>
      {/if}
    </div>

    <div class="space-y-4 rounded-[2rem] border border-slate-200 bg-white p-6 shadow-sm dark:border-slate-800 dark:bg-slate-950">
      <div>
        <div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Templates</div>
        <h2 class="mt-1 text-xl font-semibold text-slate-950 dark:text-slate-50">Start from a structured playbook</h2>
      </div>

      {#each templates as template}
        <button
          class="w-full rounded-2xl border border-slate-200 bg-slate-50 p-4 text-left transition hover:border-cyan-300 hover:bg-cyan-50 dark:border-slate-800 dark:bg-slate-900/60 dark:hover:border-cyan-700 dark:hover:bg-cyan-950/20"
          onclick={() => void createFromTemplate(template)}
          disabled={creating}
        >
          <div class="text-sm font-semibold text-slate-900 dark:text-slate-100">{template.name}</div>
          <p class="mt-1 text-sm text-slate-600 dark:text-slate-300">{template.description}</p>
          <div class="mt-3 text-xs text-slate-500 dark:text-slate-400">{template.widgets.length} starter embeds</div>
        </button>
      {/each}
    </div>
  </section>
</div>
