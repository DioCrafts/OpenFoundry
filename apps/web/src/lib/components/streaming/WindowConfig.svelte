<script lang="ts">
	import type { WindowDefinition } from '$lib/api/streaming';

	interface WindowDraft {
		id?: string;
		name: string;
		description: string;
		status: string;
		window_type: string;
		duration_seconds: number;
		slide_seconds: number;
		session_gap_seconds: number;
		allowed_lateness_seconds: number;
		aggregation_keys_text: string;
		measure_fields_text: string;
	}

	interface Props {
		windows: WindowDefinition[];
		draft: WindowDraft;
		busy?: boolean;
		onSelect?: (windowId: string) => void;
		onDraftChange?: (draft: WindowDraft) => void;
		onSave?: () => void;
		onReset?: () => void;
	}

	let { windows, draft, busy = false, onSelect, onDraftChange, onSave, onReset }: Props = $props();

	let localDraft = $state<WindowDraft>({
		id: undefined,
		name: '',
		description: '',
		status: '',
		window_type: '',
		duration_seconds: 300,
		slide_seconds: 300,
		session_gap_seconds: 180,
		allowed_lateness_seconds: 30,
		aggregation_keys_text: '',
		measure_fields_text: '',
	});

	$effect(() => {
		localDraft = { ...draft };
	});

	function updateDraft<K extends keyof WindowDraft>(key: K, value: WindowDraft[K]) {
		const nextDraft = { ...localDraft, [key]: value };
		localDraft = nextDraft;
		onDraftChange?.(nextDraft);
	}
</script>

<section class="rounded-[28px] border border-slate-200 bg-white p-5 shadow-sm dark:border-slate-800 dark:bg-slate-950">
	<div class="flex flex-wrap items-start justify-between gap-3">
		<div>
			<div class="text-[11px] font-semibold uppercase tracking-[0.28em] text-slate-500">Windowing</div>
			<h2 class="mt-2 text-xl font-semibold text-slate-900 dark:text-slate-100">Tumbling, sliding, and session window controls</h2>
		</div>
		<div class="flex gap-2">
			<button class="rounded-full border border-slate-300 px-3 py-1.5 text-sm text-slate-700 hover:bg-slate-100 dark:border-slate-700 dark:text-slate-200 dark:hover:bg-slate-900" onclick={() => onReset?.()} disabled={busy}>New</button>
			<button class="rounded-full bg-slate-950 px-3 py-1.5 text-sm font-medium text-white hover:bg-slate-800 disabled:opacity-60 dark:bg-slate-100 dark:text-slate-950" onclick={() => onSave?.()} disabled={busy}>Save</button>
		</div>
	</div>

	<div class="mt-5 grid gap-5 xl:grid-cols-[minmax(0,0.78fr)_minmax(0,1.22fr)]">
		<div class="space-y-3">
			{#if windows.length === 0}
				<div class="rounded-2xl border border-dashed border-slate-300 px-4 py-5 text-sm text-slate-500 dark:border-slate-700 dark:text-slate-400">No window definitions yet.</div>
			{:else}
				{#each windows as window}
					<button class={`w-full rounded-2xl border px-4 py-3 text-left transition ${localDraft.id === window.id ? 'border-violet-400 bg-violet-50 dark:border-violet-700 dark:bg-violet-950/30' : 'border-slate-200 bg-slate-50 hover:border-slate-300 dark:border-slate-800 dark:bg-slate-900 dark:hover:border-slate-700'}`} onclick={() => onSelect?.(window.id)} type="button">
						<div class="text-sm font-semibold text-slate-900 dark:text-slate-100">{window.name}</div>
						<div class="mt-1 text-xs text-slate-500">{window.window_type} • {window.duration_seconds}s / {window.slide_seconds}s</div>
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
					<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Type</div>
					<input class="mt-2 w-full bg-transparent text-sm text-slate-900 outline-none dark:text-slate-100" value={localDraft.window_type} oninput={(event) => updateDraft('window_type', (event.currentTarget as HTMLInputElement).value)} />
				</label>
			</div>

			<label class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 dark:border-slate-800 dark:bg-slate-900">
				<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Description</div>
				<textarea class="mt-2 h-20 w-full bg-transparent text-sm text-slate-900 outline-none dark:text-slate-100" oninput={(event) => updateDraft('description', (event.currentTarget as HTMLTextAreaElement).value)}>{localDraft.description}</textarea>
			</label>

			<div class="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
				<label class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 dark:border-slate-800 dark:bg-slate-900">
					<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Duration</div>
					<input class="mt-2 w-full bg-transparent text-sm text-slate-900 outline-none dark:text-slate-100" type="number" value={String(localDraft.duration_seconds)} oninput={(event) => updateDraft('duration_seconds', Number((event.currentTarget as HTMLInputElement).value) || 300)} />
				</label>
				<label class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 dark:border-slate-800 dark:bg-slate-900">
					<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Slide</div>
					<input class="mt-2 w-full bg-transparent text-sm text-slate-900 outline-none dark:text-slate-100" type="number" value={String(localDraft.slide_seconds)} oninput={(event) => updateDraft('slide_seconds', Number((event.currentTarget as HTMLInputElement).value) || 300)} />
				</label>
				<label class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 dark:border-slate-800 dark:bg-slate-900">
					<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Session Gap</div>
					<input class="mt-2 w-full bg-transparent text-sm text-slate-900 outline-none dark:text-slate-100" type="number" value={String(localDraft.session_gap_seconds)} oninput={(event) => updateDraft('session_gap_seconds', Number((event.currentTarget as HTMLInputElement).value) || 180)} />
				</label>
				<label class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 dark:border-slate-800 dark:bg-slate-900">
					<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Allowed Lateness</div>
					<input class="mt-2 w-full bg-transparent text-sm text-slate-900 outline-none dark:text-slate-100" type="number" value={String(localDraft.allowed_lateness_seconds)} oninput={(event) => updateDraft('allowed_lateness_seconds', Number((event.currentTarget as HTMLInputElement).value) || 30)} />
				</label>
			</div>

			<div class="grid gap-4 md:grid-cols-2">
				<label class="rounded-2xl border border-dashed border-violet-300 bg-violet-50/60 px-4 py-3 dark:border-violet-900 dark:bg-violet-950/20">
					<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-violet-700 dark:text-violet-300">Aggregation Keys</div>
					<textarea class="mt-2 h-32 w-full bg-transparent text-sm text-slate-900 outline-none dark:text-slate-100" oninput={(event) => updateDraft('aggregation_keys_text', (event.currentTarget as HTMLTextAreaElement).value)}>{localDraft.aggregation_keys_text}</textarea>
				</label>
				<label class="rounded-2xl border border-dashed border-violet-300 bg-violet-50/60 px-4 py-3 dark:border-violet-900 dark:bg-violet-950/20">
					<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-violet-700 dark:text-violet-300">Measure Fields</div>
					<textarea class="mt-2 h-32 w-full bg-transparent text-sm text-slate-900 outline-none dark:text-slate-100" oninput={(event) => updateDraft('measure_fields_text', (event.currentTarget as HTMLTextAreaElement).value)}>{localDraft.measure_fields_text}</textarea>
				</label>
			</div>
		</div>
	</div>
</section>
