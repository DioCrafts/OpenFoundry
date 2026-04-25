<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { page as pageStore } from '$app/stores';
  import { goto } from '$app/navigation';
  import { get } from 'svelte/store';
  import { auth } from '$stores/auth';
  import {
    exportNotepadDocument,
    getNotepadDocument,
    listNotepadPresence,
    updateNotepadDocument,
    upsertNotepadPresence,
    type NotepadDocument,
    type NotepadExportPayload,
    type NotepadPresence,
  } from '$lib/api/notepad';
  import {
    createKnowledgeDocument,
    listKnowledgeBases,
    type KnowledgeBase,
  } from '$lib/api/ai';

  type NotepadWidgetDraft = {
    id?: string;
    kind: string;
    title: string;
    summary: string;
    source_ref: string;
  };

  const documentId = $derived($pageStore.params.id ?? '');

  let document = $state<NotepadDocument | null>(null);
  let exportPayload = $state<NotepadExportPayload | null>(null);
  let presence = $state<NotepadPresence[]>([]);
  let knowledgeBases = $state<KnowledgeBase[]>([]);
  let selectedKnowledgeBaseId = $state('');
  let widgetDraft = $state<NotepadWidgetDraft>(emptyWidgetDraft());
  let loading = $state(true);
  let saving = $state(false);
  let indexing = $state(false);
  let error = $state('');
  let sessionId = $state(crypto.randomUUID());

  let heartbeatTimer: ReturnType<typeof setInterval> | null = null;
  let presenceTimer: ReturnType<typeof setInterval> | null = null;

  function emptyWidgetDraft(): NotepadWidgetDraft {
    return {
      kind: 'contour',
      title: '',
      summary: '',
      source_ref: '',
    };
  }

  function documentWidgets() {
    return Array.isArray(document?.widgets) ? document.widgets : [];
  }

  async function load() {
    loading = true;
    error = '';
    try {
      const [doc, exportResult, presenceResult, kbResult] = await Promise.all([
        getNotepadDocument(documentId),
        exportNotepadDocument(documentId),
        listNotepadPresence(documentId),
        listKnowledgeBases().catch(() => ({ data: [] as KnowledgeBase[] })),
      ]);

      document = doc;
      exportPayload = exportResult;
      presence = presenceResult.data;
      knowledgeBases = kbResult.data;
      selectedKnowledgeBaseId = knowledgeBases[0]?.id ?? '';
    } catch (cause) {
      error = cause instanceof Error ? cause.message : 'Failed to load document';
      document = null;
      exportPayload = null;
    } finally {
      loading = false;
    }
  }

  async function saveDocument() {
    if (!document) {
      return;
    }

    saving = true;
    error = '';
    try {
      document = await updateNotepadDocument(document.id, {
        title: document.title,
        description: document.description,
        content: document.content,
        widgets: document.widgets,
      });
      exportPayload = await exportNotepadDocument(document.id);
      await sendPresence('reviewing latest changes');
    } catch (cause) {
      error = cause instanceof Error ? cause.message : 'Failed to save document';
    } finally {
      saving = false;
    }
  }

  function addWidget() {
    if (!document || !widgetDraft.title.trim()) {
      return;
    }

    const next = {
      id: crypto.randomUUID(),
      kind: widgetDraft.kind,
      title: widgetDraft.title.trim(),
      summary: widgetDraft.summary.trim() || 'Live widget reference attached to the document.',
      source_ref: widgetDraft.source_ref.trim() || null,
    };

    document = {
      ...document,
      widgets: [...documentWidgets(), next],
    };
    widgetDraft = emptyWidgetDraft();
  }

  function removeWidget(widgetId: string) {
    if (!document) {
      return;
    }

    document = {
      ...document,
      widgets: documentWidgets().filter((widget) => String(widget.id ?? '') !== widgetId),
    };
  }

  async function sendPresence(cursorLabel = 'editing document') {
    const user = get(auth.user);
    if (!document || !user) {
      return;
    }

    try {
      await upsertNotepadPresence(document.id, {
        session_id: sessionId,
        display_name: user.name,
        cursor_label: cursorLabel,
        color: '#0f766e',
      });
    } catch {
      // Presence should not block editing.
    }
  }

  async function refreshPresence() {
    if (!document) {
      return;
    }

    try {
      const result = await listNotepadPresence(document.id);
      presence = result.data;
    } catch {
      // Ignore transient polling failures.
    }
  }

  async function indexInKnowledgeBase() {
    if (!document || !selectedKnowledgeBaseId) {
      return;
    }

    indexing = true;
    error = '';
    try {
      await createKnowledgeDocument(selectedKnowledgeBaseId, {
        title: document.title,
        content: [document.content, '', ...documentWidgets().map((widget) => `- ${widget.title ?? 'Widget'}: ${widget.summary ?? ''}`)].join('\n'),
        source_uri: `notepad://${document.id}`,
        metadata: {
          source: 'notepad',
          widget_count: documentWidgets().length,
        },
      });
      document = await updateNotepadDocument(document.id, {
        last_indexed_at: new Date().toISOString(),
      });
    } catch (cause) {
      error = cause instanceof Error ? cause.message : 'Failed to index document';
    } finally {
      indexing = false;
    }
  }

  function openPrintView() {
    if (!exportPayload) {
      return;
    }

    const windowRef = window.open('', '_blank', 'noopener,noreferrer');
    if (!windowRef) {
      return;
    }

    windowRef.document.write(exportPayload.html);
    windowRef.document.close();
    windowRef.focus();
    windowRef.print();
  }

  onMount(() => {
    void load();
    void sendPresence();
    heartbeatTimer = setInterval(() => void sendPresence('editing document'), 15_000);
    presenceTimer = setInterval(() => void refreshPresence(), 12_000);
  });

  onDestroy(() => {
    if (heartbeatTimer) clearInterval(heartbeatTimer);
    if (presenceTimer) clearInterval(presenceTimer);
  });
</script>

<svelte:head>
  <title>OpenFoundry — Notepad Document</title>
</svelte:head>

{#if loading}
  <div class="py-20 text-center text-slate-500">Loading document...</div>
{:else if !document}
  <div class="mx-auto max-w-2xl rounded-[2rem] border border-slate-200 bg-white p-10 text-center shadow-sm dark:border-slate-800 dark:bg-slate-950">
    <h1 class="text-3xl font-semibold text-slate-950 dark:text-slate-50">Document not found</h1>
    <a href="/notepad" class="mt-6 inline-flex rounded-xl bg-slate-900 px-4 py-2 text-sm font-medium text-white dark:bg-slate-100 dark:text-slate-950">Back to Notepad</a>
  </div>
{:else}
  <div class="mx-auto max-w-[1600px] space-y-6">
    <section class="rounded-[2rem] border border-slate-200 bg-white p-6 shadow-sm dark:border-slate-800 dark:bg-slate-950">
      <div class="flex flex-wrap items-start justify-between gap-4">
        <div class="max-w-3xl">
          <a href="/notepad" class="text-sm font-medium text-slate-500 hover:text-slate-700 dark:text-slate-400 dark:hover:text-slate-200">← Back to notepad</a>
          <input
            bind:value={document.title}
            class="mt-3 w-full bg-transparent text-4xl font-semibold tracking-tight text-slate-950 outline-none dark:text-slate-50"
            placeholder="Document title"
          />
          <textarea
            bind:value={document.description}
            rows="2"
            class="mt-3 w-full resize-none bg-transparent text-base leading-7 text-slate-600 outline-none dark:text-slate-300"
            placeholder="What should readers understand after opening this document?"
          ></textarea>
          <div class="mt-3 flex flex-wrap gap-2 text-xs text-slate-500 dark:text-slate-400">
            {#if document.template_key}
              <span class="rounded-full border border-slate-300 px-3 py-1 dark:border-slate-700">{document.template_key}</span>
            {/if}
            <span class="rounded-full border border-slate-300 px-3 py-1 dark:border-slate-700">{documentWidgets().length} embeds</span>
            {#if document.last_indexed_at}
              <span class="rounded-full border border-emerald-300 px-3 py-1 text-emerald-700 dark:border-emerald-700 dark:text-emerald-300">Indexed in AIP</span>
            {/if}
          </div>
        </div>

        <div class="flex flex-wrap gap-2">
          <button class="rounded-xl border border-slate-300 px-4 py-2 text-sm font-medium dark:border-slate-700" onclick={() => goto('/notepad')}>Close</button>
          <button class="rounded-xl border border-slate-300 px-4 py-2 text-sm font-medium dark:border-slate-700" onclick={openPrintView}>Print</button>
          <button class="rounded-xl bg-slate-900 px-4 py-2 text-sm font-medium text-white dark:bg-slate-100 dark:text-slate-950" onclick={saveDocument} disabled={saving}>
            {saving ? 'Saving...' : 'Save'}
          </button>
        </div>
      </div>

      {#if error}
        <div class="mt-4 rounded-2xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-700 dark:border-rose-900 dark:bg-rose-950/40 dark:text-rose-300">
          {error}
        </div>
      {/if}
    </section>

    <div class="grid gap-6 xl:grid-cols-[minmax(0,1.1fr)_minmax(380px,0.9fr)]">
      <div class="space-y-6">
        <section class="rounded-[2rem] border border-slate-200 bg-white p-6 shadow-sm dark:border-slate-800 dark:bg-slate-950">
          <div class="flex items-center justify-between gap-3">
            <div>
              <div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Document body</div>
              <h2 class="mt-1 text-xl font-semibold text-slate-950 dark:text-slate-50">Markdown-first collaborative note</h2>
            </div>
            <div class="text-xs text-slate-500 dark:text-slate-400">{presence.length} active collaborators</div>
          </div>

          <textarea
            bind:value={document.content}
            rows="24"
            class="mt-4 min-h-[520px] w-full rounded-2xl border border-slate-300 bg-slate-50 px-4 py-4 font-mono text-sm leading-6 outline-none dark:border-slate-700 dark:bg-slate-900"
            placeholder="# Narrative&#10;&#10;Write the decision memo here."
            onfocus={() => void sendPresence('editing body')}
          ></textarea>
        </section>

        <section class="rounded-[2rem] border border-slate-200 bg-white p-6 shadow-sm dark:border-slate-800 dark:bg-slate-950">
          <div class="flex items-center justify-between gap-3">
            <div>
              <div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Embeds</div>
              <h2 class="mt-1 text-xl font-semibold text-slate-950 dark:text-slate-50">Attach live workspace context</h2>
            </div>
          </div>

          <div class="mt-4 grid gap-3 md:grid-cols-2">
            <label class="rounded-xl border border-slate-200 bg-slate-50 px-3 py-2 dark:border-slate-800 dark:bg-slate-900">
              <div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Kind</div>
              <select bind:value={widgetDraft.kind} class="mt-2 w-full bg-transparent text-sm outline-none">
                <option value="contour">Contour</option>
                <option value="quiver">Quiver</option>
                <option value="report">Report</option>
                <option value="fusion">Fusion</option>
              </select>
            </label>

            <label class="rounded-xl border border-slate-200 bg-slate-50 px-3 py-2 dark:border-slate-800 dark:bg-slate-900">
              <div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Title</div>
              <input bind:value={widgetDraft.title} class="mt-2 w-full bg-transparent text-sm outline-none" placeholder="Executive trend board" />
            </label>

            <label class="rounded-xl border border-slate-200 bg-slate-50 px-3 py-2 dark:border-slate-800 dark:bg-slate-900 md:col-span-2">
              <div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Summary</div>
              <input bind:value={widgetDraft.summary} class="mt-2 w-full bg-transparent text-sm outline-none" placeholder="Why this widget matters in the narrative." />
            </label>

            <label class="rounded-xl border border-slate-200 bg-slate-50 px-3 py-2 dark:border-slate-800 dark:bg-slate-900 md:col-span-2">
              <div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Source reference</div>
              <input bind:value={widgetDraft.source_ref} class="mt-2 w-full bg-transparent text-sm outline-none" placeholder="/contour or report execution id" />
            </label>
          </div>

          <div class="mt-4 flex justify-end">
            <button class="rounded-xl border border-slate-300 px-4 py-2 text-sm font-medium dark:border-slate-700" onclick={addWidget}>Add embed</button>
          </div>

          <div class="mt-4 space-y-3">
            {#if documentWidgets().length === 0}
              <div class="rounded-2xl border border-dashed border-slate-300 px-4 py-5 text-sm text-slate-500 dark:border-slate-700 dark:text-slate-400">
                No embedded widgets yet.
              </div>
            {:else}
              {#each documentWidgets() as widget}
                <div class="rounded-2xl border border-slate-200 bg-slate-50 p-4 dark:border-slate-800 dark:bg-slate-900/60">
                  <div class="flex items-start justify-between gap-3">
                    <div>
                      <div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-cyan-600">{widget.kind ?? 'widget'}</div>
                      <div class="mt-1 text-sm font-semibold text-slate-900 dark:text-slate-100">{widget.title ?? 'Untitled widget'}</div>
                      <p class="mt-1 text-sm text-slate-600 dark:text-slate-300">{widget.summary ?? 'No summary.'}</p>
                    </div>
                    <button class="rounded-lg border border-slate-300 px-3 py-1.5 text-xs font-medium dark:border-slate-700" onclick={() => removeWidget(String(widget.id ?? ''))}>
                      Remove
                    </button>
                  </div>
                </div>
              {/each}
            {/if}
          </div>
        </section>
      </div>

      <div class="space-y-6">
        <section class="rounded-[2rem] border border-slate-200 bg-white p-6 shadow-sm dark:border-slate-800 dark:bg-slate-950">
          <div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Presence</div>
          <h2 class="mt-1 text-xl font-semibold text-slate-950 dark:text-slate-50">Who is in the document</h2>

          <div class="mt-4 space-y-3">
            {#if presence.length === 0}
              <div class="rounded-2xl border border-dashed border-slate-300 px-4 py-5 text-sm text-slate-500 dark:border-slate-700 dark:text-slate-400">
                No active collaborators right now.
              </div>
            {:else}
              {#each presence as collaborator (collaborator.id)}
                <div class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 dark:border-slate-800 dark:bg-slate-900/60">
                  <div class="flex items-center gap-3">
                    <span class="h-3 w-3 rounded-full" style={`background:${collaborator.color};`}></span>
                    <div>
                      <div class="text-sm font-semibold text-slate-900 dark:text-slate-100">{collaborator.display_name}</div>
                      <div class="text-xs text-slate-500 dark:text-slate-400">{collaborator.cursor_label || 'Browsing the document'}</div>
                    </div>
                  </div>
                </div>
              {/each}
            {/if}
          </div>
        </section>

        <section class="rounded-[2rem] border border-slate-200 bg-white p-6 shadow-sm dark:border-slate-800 dark:bg-slate-950">
          <div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">AIP Assist</div>
          <h2 class="mt-1 text-xl font-semibold text-slate-950 dark:text-slate-50">Index the document into knowledge</h2>

          <div class="mt-4 space-y-3">
            <label class="rounded-xl border border-slate-200 bg-slate-50 px-3 py-2 dark:border-slate-800 dark:bg-slate-900">
              <div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Knowledge base</div>
              <select bind:value={selectedKnowledgeBaseId} class="mt-2 w-full bg-transparent text-sm outline-none">
                {#each knowledgeBases as knowledgeBase}
                  <option value={knowledgeBase.id}>{knowledgeBase.name}</option>
                {/each}
              </select>
            </label>

            <button
              class="w-full rounded-xl bg-slate-900 px-4 py-2 text-sm font-medium text-white disabled:opacity-60 dark:bg-slate-100 dark:text-slate-950"
              disabled={!selectedKnowledgeBaseId || indexing}
              onclick={indexInKnowledgeBase}
            >
              {indexing ? 'Indexing...' : 'Index in AIP'}
            </button>
          </div>
        </section>

        <section class="rounded-[2rem] border border-slate-200 bg-white p-6 shadow-sm dark:border-slate-800 dark:bg-slate-950">
          <div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Preview</div>
          <h2 class="mt-1 text-xl font-semibold text-slate-950 dark:text-slate-50">Rendered export</h2>

          {#if exportPayload}
            <iframe title="Notepad preview" class="mt-4 h-[540px] w-full rounded-2xl border border-slate-200 bg-white dark:border-slate-800" srcdoc={exportPayload.html}></iframe>
          {:else}
            <div class="mt-4 rounded-2xl border border-dashed border-slate-300 px-4 py-5 text-sm text-slate-500 dark:border-slate-700 dark:text-slate-400">
              Save the document to refresh the rendered preview.
            </div>
          {/if}
        </section>
      </div>
    </div>
  </div>
{/if}
