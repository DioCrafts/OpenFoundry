<script lang="ts">
	import type { KnowledgeBase, KnowledgeDocument, KnowledgeSearchResult } from '$lib/api/ai';

	interface KnowledgeBaseDraft {
		id?: string;
		name: string;
		description: string;
		status: string;
		embedding_provider: string;
		chunking_strategy: string;
		tags_text: string;
	}

	interface DocumentDraft {
		title: string;
		content: string;
		source_uri: string;
		metadata_text: string;
	}

	interface SearchDraft {
		query: string;
		top_k: number;
		min_score: number;
	}

	interface Props {
		knowledgeBases: KnowledgeBase[];
		documents: KnowledgeDocument[];
		selectedKnowledgeBaseId: string;
		knowledgeBaseDraft: KnowledgeBaseDraft;
		documentDraft: DocumentDraft;
		searchDraft: SearchDraft;
		searchResults: KnowledgeSearchResult[];
		busy?: boolean;
		onSelectKnowledgeBase?: (knowledgeBaseId: string) => void;
		onKnowledgeBaseDraftChange?: (draft: KnowledgeBaseDraft) => void;
		onDocumentDraftChange?: (draft: DocumentDraft) => void;
		onSearchDraftChange?: (draft: SearchDraft) => void;
		onSaveKnowledgeBase?: () => void;
		onSaveDocument?: () => void;
		onSearch?: () => void;
		onResetKnowledgeBase?: () => void;
	}

	let {
		knowledgeBases,
		documents,
		selectedKnowledgeBaseId,
		knowledgeBaseDraft,
		documentDraft,
		searchDraft,
		searchResults,
		busy = false,
		onSelectKnowledgeBase,
		onKnowledgeBaseDraftChange,
		onDocumentDraftChange,
		onSearchDraftChange,
		onSaveKnowledgeBase,
		onSaveDocument,
		onSearch,
		onResetKnowledgeBase,
	}: Props = $props();

	let localKnowledgeBaseDraft = $state<KnowledgeBaseDraft>({
		id: undefined,
		name: '',
		description: '',
		status: '',
		embedding_provider: '',
		chunking_strategy: '',
		tags_text: '',
	});
	let localDocumentDraft = $state<DocumentDraft>({
		title: '',
		content: '',
		source_uri: '',
		metadata_text: '',
	});
	let localSearchDraft = $state<SearchDraft>({
		query: '',
		top_k: 4,
		min_score: 0.55,
	});

	$effect(() => {
		localKnowledgeBaseDraft = { ...knowledgeBaseDraft };
	});
	$effect(() => {
		localDocumentDraft = { ...documentDraft };
	});
	$effect(() => {
		localSearchDraft = { ...searchDraft };
	});

	function updateKnowledgeBase<K extends keyof KnowledgeBaseDraft>(key: K, value: KnowledgeBaseDraft[K]) {
		const nextDraft = { ...localKnowledgeBaseDraft, [key]: value };
		localKnowledgeBaseDraft = nextDraft;
		onKnowledgeBaseDraftChange?.(nextDraft);
	}

	function updateDocument<K extends keyof DocumentDraft>(key: K, value: DocumentDraft[K]) {
		const nextDraft = { ...localDocumentDraft, [key]: value };
		localDocumentDraft = nextDraft;
		onDocumentDraftChange?.(nextDraft);
	}

	function updateSearch<K extends keyof SearchDraft>(key: K, value: SearchDraft[K]) {
		const nextDraft = { ...localSearchDraft, [key]: value };
		localSearchDraft = nextDraft;
		onSearchDraftChange?.(nextDraft);
	}
</script>

<section class="rounded-[28px] border border-slate-200 bg-white p-5 shadow-sm dark:border-slate-800 dark:bg-slate-950">
	<div class="flex flex-wrap items-start justify-between gap-3">
		<div>
			<div class="text-[11px] font-semibold uppercase tracking-[0.28em] text-slate-500">Knowledge Manager</div>
			<h2 class="mt-2 text-xl font-semibold text-slate-900 dark:text-slate-100">Index playbooks, upload docs, and test retrieval</h2>
		</div>
		<div class="flex gap-2">
			<button class="rounded-full border border-slate-300 px-3 py-1.5 text-sm text-slate-700 hover:bg-slate-100 dark:border-slate-700 dark:text-slate-200 dark:hover:bg-slate-900" onclick={() => onResetKnowledgeBase?.()} disabled={busy}>New KB</button>
			<button class="rounded-full bg-slate-950 px-3 py-1.5 text-sm font-medium text-white hover:bg-slate-800 disabled:opacity-60 dark:bg-slate-100 dark:text-slate-950" onclick={() => onSaveKnowledgeBase?.()} disabled={busy}>Save KB</button>
		</div>
	</div>

	<div class="mt-5 grid gap-5 xl:grid-cols-[minmax(0,0.8fr)_minmax(0,1.2fr)]">
		<div class="space-y-3">
			{#each knowledgeBases as knowledgeBase}
				<button class={`w-full rounded-2xl border px-4 py-3 text-left transition ${selectedKnowledgeBaseId === knowledgeBase.id ? 'border-cyan-400 bg-cyan-50 dark:border-cyan-700 dark:bg-cyan-950/30' : 'border-slate-200 bg-slate-50 hover:border-slate-300 dark:border-slate-800 dark:bg-slate-900 dark:hover:border-slate-700'}`} onclick={() => onSelectKnowledgeBase?.(knowledgeBase.id)} type="button">
					<div class="flex items-center justify-between gap-3">
						<div>
							<div class="text-sm font-semibold text-slate-900 dark:text-slate-100">{knowledgeBase.name}</div>
							<div class="mt-1 text-xs text-slate-500">{knowledgeBase.document_count} docs • {knowledgeBase.chunk_count} chunks</div>
						</div>
						<span class="rounded-full bg-white px-2 py-1 text-[11px] uppercase tracking-[0.2em] text-slate-500 dark:bg-slate-950">{knowledgeBase.status}</span>
					</div>
				</button>
			{/each}
		</div>

		<div class="grid gap-4">
			<div class="grid gap-4 md:grid-cols-2">
				<label class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 dark:border-slate-800 dark:bg-slate-900">
					<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Name</div>
					<input class="mt-2 w-full bg-transparent text-sm text-slate-900 outline-none dark:text-slate-100" value={localKnowledgeBaseDraft.name} oninput={(event) => updateKnowledgeBase('name', (event.currentTarget as HTMLInputElement).value)} />
				</label>
				<label class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 dark:border-slate-800 dark:bg-slate-900">
					<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Embedding Provider</div>
					<input class="mt-2 w-full bg-transparent text-sm text-slate-900 outline-none dark:text-slate-100" value={localKnowledgeBaseDraft.embedding_provider} oninput={(event) => updateKnowledgeBase('embedding_provider', (event.currentTarget as HTMLInputElement).value)} />
				</label>
			</div>

			<div class="grid gap-4 md:grid-cols-2">
				<label class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 dark:border-slate-800 dark:bg-slate-900">
					<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Description</div>
					<input class="mt-2 w-full bg-transparent text-sm text-slate-900 outline-none dark:text-slate-100" value={localKnowledgeBaseDraft.description} oninput={(event) => updateKnowledgeBase('description', (event.currentTarget as HTMLInputElement).value)} />
				</label>
				<label class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 dark:border-slate-800 dark:bg-slate-900">
					<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Chunk Strategy</div>
					<input class="mt-2 w-full bg-transparent text-sm text-slate-900 outline-none dark:text-slate-100" value={localKnowledgeBaseDraft.chunking_strategy} oninput={(event) => updateKnowledgeBase('chunking_strategy', (event.currentTarget as HTMLInputElement).value)} />
				</label>
			</div>

			<label class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 dark:border-slate-800 dark:bg-slate-900">
				<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Tags</div>
				<input class="mt-2 w-full bg-transparent text-sm text-slate-900 outline-none dark:text-slate-100" value={localKnowledgeBaseDraft.tags_text} oninput={(event) => updateKnowledgeBase('tags_text', (event.currentTarget as HTMLInputElement).value)} />
			</label>

			<div class="grid gap-4 lg:grid-cols-[minmax(0,1fr)_minmax(0,1fr)]">
				<div class="rounded-2xl border border-slate-200 bg-slate-50 p-4 dark:border-slate-800 dark:bg-slate-900">
					<div class="flex items-center justify-between gap-3">
						<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Documents</div>
						<button class="rounded-full border border-slate-300 px-3 py-1 text-xs text-slate-700 hover:bg-slate-100 dark:border-slate-700 dark:text-slate-200 dark:hover:bg-slate-800" onclick={() => onSaveDocument?.()} disabled={busy || !selectedKnowledgeBaseId}>Add document</button>
					</div>
					<div class="mt-3 space-y-3">
						<input class="w-full rounded-xl border border-slate-200 bg-white px-3 py-2 text-sm dark:border-slate-800 dark:bg-slate-950" value={localDocumentDraft.title} oninput={(event) => updateDocument('title', (event.currentTarget as HTMLInputElement).value)} placeholder="Incident Triage Checklist" />
						<input class="w-full rounded-xl border border-slate-200 bg-white px-3 py-2 text-sm dark:border-slate-800 dark:bg-slate-950" value={localDocumentDraft.source_uri} oninput={(event) => updateDocument('source_uri', (event.currentTarget as HTMLInputElement).value)} placeholder="kb://source" />
						<textarea class="h-24 w-full rounded-xl border border-slate-200 bg-white px-3 py-2 text-sm dark:border-slate-800 dark:bg-slate-950" oninput={(event) => updateDocument('content', (event.currentTarget as HTMLTextAreaElement).value)}>{localDocumentDraft.content}</textarea>
						<textarea class="h-20 w-full rounded-xl border border-slate-200 bg-white px-3 py-2 text-sm dark:border-slate-800 dark:bg-slate-950" oninput={(event) => updateDocument('metadata_text', (event.currentTarget as HTMLTextAreaElement).value)}>{localDocumentDraft.metadata_text}</textarea>
					</div>
					<div class="mt-4 space-y-2">
						{#if documents.length === 0}
							<p class="text-sm text-slate-500">No documents loaded for this knowledge base.</p>
						{:else}
							{#each documents as document}
								<div class="rounded-xl border border-slate-200 bg-white px-3 py-2 dark:border-slate-800 dark:bg-slate-950">
									<div class="text-sm font-medium text-slate-900 dark:text-slate-100">{document.title}</div>
									<div class="mt-1 text-xs text-slate-500">{document.chunk_count} chunks • {document.status}</div>
								</div>
							{/each}
						{/if}
					</div>
				</div>

				<div class="rounded-2xl border border-slate-200 bg-slate-50 p-4 dark:border-slate-800 dark:bg-slate-900">
					<div class="flex items-center justify-between gap-3">
						<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Semantic Retrieval</div>
						<button class="rounded-full border border-cyan-300 px-3 py-1 text-xs text-cyan-700 hover:bg-cyan-50 dark:border-cyan-800 dark:text-cyan-300 dark:hover:bg-cyan-950/40" onclick={() => onSearch?.()} disabled={busy || !selectedKnowledgeBaseId}>Run search</button>
					</div>
					<div class="mt-3 grid gap-3">
						<input class="w-full rounded-xl border border-slate-200 bg-white px-3 py-2 text-sm dark:border-slate-800 dark:bg-slate-950" value={localSearchDraft.query} oninput={(event) => updateSearch('query', (event.currentTarget as HTMLInputElement).value)} placeholder="How should providers fail over?" />
						<div class="grid gap-3 md:grid-cols-2">
							<input class="w-full rounded-xl border border-slate-200 bg-white px-3 py-2 text-sm dark:border-slate-800 dark:bg-slate-950" type="number" value={String(localSearchDraft.top_k)} oninput={(event) => updateSearch('top_k', Number((event.currentTarget as HTMLInputElement).value) || 4)} />
							<input class="w-full rounded-xl border border-slate-200 bg-white px-3 py-2 text-sm dark:border-slate-800 dark:bg-slate-950" type="number" step="0.01" value={String(localSearchDraft.min_score)} oninput={(event) => updateSearch('min_score', Number((event.currentTarget as HTMLInputElement).value) || 0.55)} />
						</div>
					</div>
					<div class="mt-4 space-y-2">
						{#if searchResults.length === 0}
							<p class="text-sm text-slate-500">Search results will appear here.</p>
						{:else}
							{#each searchResults as result}
								<div class="rounded-xl border border-slate-200 bg-white px-3 py-3 dark:border-slate-800 dark:bg-slate-950">
									<div class="flex items-center justify-between gap-3">
										<div class="text-sm font-medium text-slate-900 dark:text-slate-100">{result.document_title}</div>
										<div class="text-xs text-cyan-700 dark:text-cyan-300">score {result.score.toFixed(2)}</div>
									</div>
									<p class="mt-2 text-sm leading-6 text-slate-600 dark:text-slate-300">{result.excerpt}</p>
								</div>
							{/each}
						{/if}
					</div>
				</div>
			</div>
		</div>
	</div>
</section>
