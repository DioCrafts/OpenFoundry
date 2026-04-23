<script lang="ts">
	import type { MatchRule } from '$lib/api/fusion';

	interface MatchRuleDraft {
		id?: string;
		name: string;
		description: string;
		status: string;
		entity_type: string;
		strategy_type: string;
		key_fields_text: string;
		window_size: number;
		bucket_count: number;
		review_threshold: number;
		auto_merge_threshold: number;
		conditions_text: string;
	}

	interface Props {
		rules: MatchRule[];
		draft: MatchRuleDraft;
		busy?: boolean;
		onSelect?: (ruleId: string) => void;
		onDraftChange?: (draft: MatchRuleDraft) => void;
		onSave?: () => void;
		onReset?: () => void;
	}

	let { rules, draft, busy = false, onSelect, onDraftChange, onSave, onReset }: Props = $props();

	let localDraft = $state<MatchRuleDraft>({
		id: undefined,
		name: '',
		description: '',
		status: '',
		entity_type: '',
		strategy_type: '',
		key_fields_text: '',
		window_size: 4,
		bucket_count: 24,
		review_threshold: 0.76,
		auto_merge_threshold: 0.9,
		conditions_text: '',
	});

	$effect(() => {
		localDraft = { ...draft };
	});

	function updateDraft<K extends keyof MatchRuleDraft>(key: K, value: MatchRuleDraft[K]) {
		const nextDraft = { ...localDraft, [key]: value };
		localDraft = nextDraft;
		onDraftChange?.(nextDraft);
	}
</script>

<section class="rounded-[28px] border border-slate-200 bg-white p-5 shadow-sm dark:border-slate-800 dark:bg-slate-950">
	<div class="flex flex-wrap items-start justify-between gap-3">
		<div>
			<div class="text-[11px] font-semibold uppercase tracking-[0.28em] text-slate-500">Match Rule Builder</div>
			<h2 class="mt-2 text-xl font-semibold text-slate-900 dark:text-slate-100">Deterministic, fuzzy, and phonetic matching rules</h2>
		</div>
		<div class="flex gap-2">
			<button class="rounded-full border border-slate-300 px-3 py-1.5 text-sm text-slate-700 hover:bg-slate-100 dark:border-slate-700 dark:text-slate-200 dark:hover:bg-slate-900" onclick={() => onReset?.()} disabled={busy}>New</button>
			<button class="rounded-full bg-slate-950 px-3 py-1.5 text-sm font-medium text-white hover:bg-slate-800 disabled:opacity-60 dark:bg-slate-100 dark:text-slate-950" onclick={() => onSave?.()} disabled={busy}>Save</button>
		</div>
	</div>

	<div class="mt-5 grid gap-5 xl:grid-cols-[minmax(0,0.8fr)_minmax(0,1.2fr)]">
		<div class="space-y-3">
			{#if rules.length === 0}
				<div class="rounded-2xl border border-dashed border-slate-300 px-4 py-5 text-sm text-slate-500 dark:border-slate-700 dark:text-slate-400">No rules defined yet.</div>
			{:else}
				{#each rules as rule}
					<button
						class={`w-full rounded-2xl border px-4 py-3 text-left transition ${localDraft.id === rule.id ? 'border-amber-400 bg-amber-50 dark:border-amber-700 dark:bg-amber-950/30' : 'border-slate-200 bg-slate-50 hover:border-slate-300 dark:border-slate-800 dark:bg-slate-900 dark:hover:border-slate-700'}`}
						onclick={() => onSelect?.(rule.id)}
						type="button"
					>
						<div class="text-sm font-semibold text-slate-900 dark:text-slate-100">{rule.name}</div>
						<div class="mt-1 text-xs text-slate-500">{rule.entity_type} • {rule.blocking_strategy.strategy_type}</div>
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
					<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Entity Type</div>
					<input class="mt-2 w-full bg-transparent text-sm text-slate-900 outline-none dark:text-slate-100" value={localDraft.entity_type} oninput={(event) => updateDraft('entity_type', (event.currentTarget as HTMLInputElement).value)} />
				</label>
			</div>

			<label class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 dark:border-slate-800 dark:bg-slate-900">
				<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Description</div>
				<textarea class="mt-2 h-24 w-full bg-transparent text-sm text-slate-900 outline-none dark:text-slate-100" oninput={(event) => updateDraft('description', (event.currentTarget as HTMLTextAreaElement).value)}>{localDraft.description}</textarea>
			</label>

			<div class="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
				<label class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 dark:border-slate-800 dark:bg-slate-900">
					<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Blocking</div>
					<input class="mt-2 w-full bg-transparent text-sm text-slate-900 outline-none dark:text-slate-100" value={localDraft.strategy_type} oninput={(event) => updateDraft('strategy_type', (event.currentTarget as HTMLInputElement).value)} />
				</label>
				<label class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 dark:border-slate-800 dark:bg-slate-900">
					<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Key Fields</div>
					<input class="mt-2 w-full bg-transparent text-sm text-slate-900 outline-none dark:text-slate-100" value={localDraft.key_fields_text} oninput={(event) => updateDraft('key_fields_text', (event.currentTarget as HTMLInputElement).value)} />
				</label>
				<label class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 dark:border-slate-800 dark:bg-slate-900">
					<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Window</div>
					<input class="mt-2 w-full bg-transparent text-sm text-slate-900 outline-none dark:text-slate-100" type="number" value={String(localDraft.window_size)} oninput={(event) => updateDraft('window_size', Number((event.currentTarget as HTMLInputElement).value) || 4)} />
				</label>
				<label class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 dark:border-slate-800 dark:bg-slate-900">
					<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Buckets</div>
					<input class="mt-2 w-full bg-transparent text-sm text-slate-900 outline-none dark:text-slate-100" type="number" value={String(localDraft.bucket_count)} oninput={(event) => updateDraft('bucket_count', Number((event.currentTarget as HTMLInputElement).value) || 24)} />
				</label>
			</div>

			<div class="grid gap-4 md:grid-cols-2">
				<label class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 dark:border-slate-800 dark:bg-slate-900">
					<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Review Threshold</div>
					<input class="mt-2 w-full bg-transparent text-sm text-slate-900 outline-none dark:text-slate-100" type="number" step="0.01" value={String(localDraft.review_threshold)} oninput={(event) => updateDraft('review_threshold', Number((event.currentTarget as HTMLInputElement).value) || 0.76)} />
				</label>
				<label class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 dark:border-slate-800 dark:bg-slate-900">
					<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Auto-Merge Threshold</div>
					<input class="mt-2 w-full bg-transparent text-sm text-slate-900 outline-none dark:text-slate-100" type="number" step="0.01" value={String(localDraft.auto_merge_threshold)} oninput={(event) => updateDraft('auto_merge_threshold', Number((event.currentTarget as HTMLInputElement).value) || 0.9)} />
				</label>
			</div>

			<label class="rounded-2xl border border-dashed border-amber-300 bg-amber-50/60 px-4 py-3 dark:border-amber-900 dark:bg-amber-950/20">
				<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-amber-700 dark:text-amber-300">Conditions JSON</div>
				<textarea class="mt-2 h-56 w-full bg-transparent text-sm text-slate-900 outline-none dark:text-slate-100" oninput={(event) => updateDraft('conditions_text', (event.currentTarget as HTMLTextAreaElement).value)}>{localDraft.conditions_text}</textarea>
			</label>
		</div>
	</div>
</section>
