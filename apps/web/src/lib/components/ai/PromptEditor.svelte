<script lang="ts">
	import type { PromptTemplate } from '$lib/api/ai';

	interface PromptDraft {
		id?: string;
		name: string;
		description: string;
		category: string;
		status: string;
		tags_text: string;
		content: string;
		input_variables_text: string;
		notes: string;
	}

	interface Props {
		prompts: PromptTemplate[];
		draft: PromptDraft;
		renderedPreview: string;
		missingVariables: string[];
		busy?: boolean;
		onSelect?: (promptId: string) => void;
		onDraftChange?: (draft: PromptDraft) => void;
		onSave?: () => void;
		onRender?: () => void;
		onReset?: () => void;
	}

	let {
		prompts,
		draft,
		renderedPreview,
		missingVariables,
		busy = false,
		onSelect,
		onDraftChange,
		onSave,
		onRender,
		onReset,
	}: Props = $props();

	let localDraft = $state<PromptDraft>({
		id: undefined,
		name: '',
		description: '',
		category: '',
		status: '',
		tags_text: '',
		content: '',
		input_variables_text: '',
		notes: '',
	});

	$effect(() => {
		localDraft = { ...draft };
	});

	function updateDraft<K extends keyof PromptDraft>(key: K, value: PromptDraft[K]) {
		const nextDraft = { ...localDraft, [key]: value };
		localDraft = nextDraft;
		onDraftChange?.(nextDraft);
	}
</script>

<section class="rounded-[28px] border border-slate-200 bg-white p-5 shadow-sm dark:border-slate-800 dark:bg-slate-950">
	<div class="flex flex-wrap items-start justify-between gap-3">
		<div>
			<div class="text-[11px] font-semibold uppercase tracking-[0.28em] text-slate-500">Prompt Management</div>
			<h2 class="mt-2 text-xl font-semibold text-slate-900 dark:text-slate-100">Versioned prompt templates with live rendering</h2>
		</div>
		<div class="flex flex-wrap gap-2">
			<button class="rounded-full border border-slate-300 px-3 py-1.5 text-sm text-slate-700 hover:bg-slate-100 dark:border-slate-700 dark:text-slate-200 dark:hover:bg-slate-900" onclick={() => onReset?.()} disabled={busy}>New</button>
			<button class="rounded-full border border-cyan-300 px-3 py-1.5 text-sm text-cyan-700 hover:bg-cyan-50 dark:border-cyan-800 dark:text-cyan-300 dark:hover:bg-cyan-950/40" onclick={() => onRender?.()} disabled={busy}>Render</button>
			<button class="rounded-full bg-slate-950 px-3 py-1.5 text-sm font-medium text-white hover:bg-slate-800 disabled:opacity-60 dark:bg-slate-100 dark:text-slate-950" onclick={() => onSave?.()} disabled={busy}>Save</button>
		</div>
	</div>

	<div class="mt-5 grid gap-5 xl:grid-cols-[minmax(0,0.8fr)_minmax(0,1.2fr)]">
		<div class="space-y-3">
			{#if prompts.length === 0}
				<div class="rounded-2xl border border-dashed border-slate-300 px-4 py-5 text-sm text-slate-500 dark:border-slate-700 dark:text-slate-400">No prompts yet.</div>
			{:else}
				{#each prompts as prompt}
					<button
						class={`w-full rounded-2xl border px-4 py-3 text-left transition ${localDraft.id === prompt.id ? 'border-cyan-400 bg-cyan-50 dark:border-cyan-700 dark:bg-cyan-950/30' : 'border-slate-200 bg-slate-50 hover:border-slate-300 dark:border-slate-800 dark:bg-slate-900 dark:hover:border-slate-700'}`}
						onclick={() => onSelect?.(prompt.id)}
						type="button"
					>
						<div class="flex items-center justify-between gap-3">
							<div>
								<div class="text-sm font-semibold text-slate-900 dark:text-slate-100">{prompt.name}</div>
								<div class="mt-1 text-xs text-slate-500">{prompt.category} • v{prompt.current_version.version_number}</div>
							</div>
							<span class="rounded-full bg-white px-2 py-1 text-[11px] font-medium uppercase tracking-[0.2em] text-slate-500 dark:bg-slate-950">{prompt.status}</span>
						</div>
					</button>
				{/each}
			{/if}
		</div>

		<div class="grid gap-4">
			<div class="grid gap-4 md:grid-cols-2">
				<label class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 dark:border-slate-800 dark:bg-slate-900">
					<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Name</div>
					<input class="mt-2 w-full bg-transparent text-sm text-slate-900 outline-none dark:text-slate-100" value={localDraft.name} oninput={(event) => updateDraft('name', (event.currentTarget as HTMLInputElement).value)} />
				</label>
				<label class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 dark:border-slate-800 dark:bg-slate-900">
					<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Category</div>
					<input class="mt-2 w-full bg-transparent text-sm text-slate-900 outline-none dark:text-slate-100" value={localDraft.category} oninput={(event) => updateDraft('category', (event.currentTarget as HTMLInputElement).value)} />
				</label>
			</div>

			<div class="grid gap-4 md:grid-cols-2">
				<label class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 dark:border-slate-800 dark:bg-slate-900">
					<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Description</div>
					<input class="mt-2 w-full bg-transparent text-sm text-slate-900 outline-none dark:text-slate-100" value={localDraft.description} oninput={(event) => updateDraft('description', (event.currentTarget as HTMLInputElement).value)} />
				</label>
				<label class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 dark:border-slate-800 dark:bg-slate-900">
					<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Tags</div>
					<input class="mt-2 w-full bg-transparent text-sm text-slate-900 outline-none dark:text-slate-100" value={localDraft.tags_text} oninput={(event) => updateDraft('tags_text', (event.currentTarget as HTMLInputElement).value)} placeholder="copilot, operations" />
				</label>
			</div>

			<label class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 dark:border-slate-800 dark:bg-slate-900">
				<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Template</div>
				<textarea class="mt-2 h-40 w-full bg-transparent text-sm text-slate-900 outline-none dark:text-slate-100" oninput={(event) => updateDraft('content', (event.currentTarget as HTMLTextAreaElement).value)}>{localDraft.content}</textarea>
			</label>

			<div class="grid gap-4 md:grid-cols-2">
				<label class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 dark:border-slate-800 dark:bg-slate-900">
					<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Input Variables</div>
					<input class="mt-2 w-full bg-transparent text-sm text-slate-900 outline-none dark:text-slate-100" value={localDraft.input_variables_text} oninput={(event) => updateDraft('input_variables_text', (event.currentTarget as HTMLInputElement).value)} placeholder="team_name, region" />
				</label>
				<label class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 dark:border-slate-800 dark:bg-slate-900">
					<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Version Notes</div>
					<input class="mt-2 w-full bg-transparent text-sm text-slate-900 outline-none dark:text-slate-100" value={localDraft.notes} oninput={(event) => updateDraft('notes', (event.currentTarget as HTMLInputElement).value)} />
				</label>
			</div>

			<div class="rounded-2xl border border-dashed border-slate-300 bg-white px-4 py-4 dark:border-slate-700 dark:bg-slate-950">
				<div class="flex items-center justify-between gap-3">
					<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Rendered Preview</div>
					{#if missingVariables.length > 0}
						<div class="text-xs text-amber-600 dark:text-amber-300">Missing: {missingVariables.join(', ')}</div>
					{/if}
				</div>
				<pre class="mt-3 whitespace-pre-wrap text-sm leading-6 text-slate-700 dark:text-slate-200">{renderedPreview || 'Render a template to preview interpolation.'}</pre>
			</div>
		</div>
	</div>
</section>
