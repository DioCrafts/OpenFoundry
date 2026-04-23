<script lang="ts">
	import type { MergeStrategy } from '$lib/api/fusion';

	interface MergeStrategyDraft {
		id?: string;
		name: string;
		description: string;
		status: string;
		entity_type: string;
		default_strategy: string;
		rules_text: string;
	}

	interface Props {
		strategies: MergeStrategy[];
		draft: MergeStrategyDraft;
		busy?: boolean;
		onSelect?: (strategyId: string) => void;
		onDraftChange?: (draft: MergeStrategyDraft) => void;
		onSave?: () => void;
		onReset?: () => void;
	}

	let { strategies, draft, busy = false, onSelect, onDraftChange, onSave, onReset }: Props = $props();

	let localDraft = $state<MergeStrategyDraft>({
		id: undefined,
		name: '',
		description: '',
		status: '',
		entity_type: '',
		default_strategy: '',
		rules_text: '',
	});

	$effect(() => {
		localDraft = { ...draft };
	});

	function updateDraft<K extends keyof MergeStrategyDraft>(key: K, value: MergeStrategyDraft[K]) {
		const nextDraft = { ...localDraft, [key]: value };
		localDraft = nextDraft;
		onDraftChange?.(nextDraft);
	}
</script>

<section class="rounded-[28px] border border-slate-200 bg-white p-5 shadow-sm dark:border-slate-800 dark:bg-slate-950">
	<div class="flex flex-wrap items-start justify-between gap-3">
		<div>
			<div class="text-[11px] font-semibold uppercase tracking-[0.28em] text-slate-500">Merge Strategy</div>
			<h2 class="mt-2 text-xl font-semibold text-slate-900 dark:text-slate-100">Golden record survivorship and field precedence</h2>
		</div>
		<div class="flex gap-2">
			<button class="rounded-full border border-slate-300 px-3 py-1.5 text-sm text-slate-700 hover:bg-slate-100 dark:border-slate-700 dark:text-slate-200 dark:hover:bg-slate-900" onclick={() => onReset?.()} disabled={busy}>New</button>
			<button class="rounded-full bg-slate-950 px-3 py-1.5 text-sm font-medium text-white hover:bg-slate-800 disabled:opacity-60 dark:bg-slate-100 dark:text-slate-950" onclick={() => onSave?.()} disabled={busy}>Save</button>
		</div>
	</div>

	<div class="mt-5 grid gap-5 xl:grid-cols-[minmax(0,0.8fr)_minmax(0,1.2fr)]">
		<div class="space-y-3">
			{#if strategies.length === 0}
				<div class="rounded-2xl border border-dashed border-slate-300 px-4 py-5 text-sm text-slate-500 dark:border-slate-700 dark:text-slate-400">No merge strategies defined yet.</div>
			{:else}
				{#each strategies as strategy}
					<button
						class={`w-full rounded-2xl border px-4 py-3 text-left transition ${localDraft.id === strategy.id ? 'border-cyan-400 bg-cyan-50 dark:border-cyan-700 dark:bg-cyan-950/30' : 'border-slate-200 bg-slate-50 hover:border-slate-300 dark:border-slate-800 dark:bg-slate-900 dark:hover:border-slate-700'}`}
						onclick={() => onSelect?.(strategy.id)}
						type="button"
					>
						<div class="text-sm font-semibold text-slate-900 dark:text-slate-100">{strategy.name}</div>
						<div class="mt-1 text-xs text-slate-500">{strategy.default_strategy} • {strategy.rules.length} rules</div>
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
					<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Default Strategy</div>
					<input class="mt-2 w-full bg-transparent text-sm text-slate-900 outline-none dark:text-slate-100" value={localDraft.default_strategy} oninput={(event) => updateDraft('default_strategy', (event.currentTarget as HTMLInputElement).value)} />
				</label>
			</div>

			<label class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 dark:border-slate-800 dark:bg-slate-900">
				<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Description</div>
				<textarea class="mt-2 h-24 w-full bg-transparent text-sm text-slate-900 outline-none dark:text-slate-100" oninput={(event) => updateDraft('description', (event.currentTarget as HTMLTextAreaElement).value)}>{localDraft.description}</textarea>
			</label>

			<label class="rounded-2xl border border-dashed border-cyan-300 bg-cyan-50/60 px-4 py-3 dark:border-cyan-900 dark:bg-cyan-950/20">
				<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-cyan-700 dark:text-cyan-300">Survivorship Rules JSON</div>
				<textarea class="mt-2 h-56 w-full bg-transparent text-sm text-slate-900 outline-none dark:text-slate-100" oninput={(event) => updateDraft('rules_text', (event.currentTarget as HTMLTextAreaElement).value)}>{localDraft.rules_text}</textarea>
			</label>
		</div>
	</div>
</section>
