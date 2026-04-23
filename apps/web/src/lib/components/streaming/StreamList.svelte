<script lang="ts">
	import type { StreamDefinition } from '$lib/api/streaming';

	interface StreamDraft {
		id?: string;
		name: string;
		description: string;
		status: string;
		retention_hours: number;
		connector_type: string;
		endpoint: string;
		format: string;
		schema_text: string;
	}

	interface Props {
		streams: StreamDefinition[];
		draft: StreamDraft;
		busy?: boolean;
		onSelect?: (streamId: string) => void;
		onDraftChange?: (draft: StreamDraft) => void;
		onSave?: () => void;
		onReset?: () => void;
	}

	let { streams, draft, busy = false, onSelect, onDraftChange, onSave, onReset }: Props = $props();

	let localDraft = $state<StreamDraft>({
		id: undefined,
		name: '',
		description: '',
		status: '',
		retention_hours: 72,
		connector_type: '',
		endpoint: '',
		format: '',
		schema_text: '',
	});

	$effect(() => {
		localDraft = { ...draft };
	});

	function updateDraft<K extends keyof StreamDraft>(key: K, value: StreamDraft[K]) {
		const nextDraft = { ...localDraft, [key]: value };
		localDraft = nextDraft;
		onDraftChange?.(nextDraft);
	}
</script>

<section class="rounded-[28px] border border-slate-200 bg-white p-5 shadow-sm dark:border-slate-800 dark:bg-slate-950">
	<div class="flex flex-wrap items-start justify-between gap-3">
		<div>
			<div class="text-[11px] font-semibold uppercase tracking-[0.28em] text-slate-500">Stream Definitions</div>
			<h2 class="mt-2 text-xl font-semibold text-slate-900 dark:text-slate-100">Named streams with schemas and source connectors</h2>
		</div>
		<div class="flex gap-2">
			<button class="rounded-full border border-slate-300 px-3 py-1.5 text-sm text-slate-700 hover:bg-slate-100 dark:border-slate-700 dark:text-slate-200 dark:hover:bg-slate-900" onclick={() => onReset?.()} disabled={busy}>New</button>
			<button class="rounded-full bg-slate-950 px-3 py-1.5 text-sm font-medium text-white hover:bg-slate-800 disabled:opacity-60 dark:bg-slate-100 dark:text-slate-950" onclick={() => onSave?.()} disabled={busy}>Save</button>
		</div>
	</div>

	<div class="mt-5 grid gap-5 xl:grid-cols-[minmax(0,0.78fr)_minmax(0,1.22fr)]">
		<div class="space-y-3">
			{#if streams.length === 0}
				<div class="rounded-2xl border border-dashed border-slate-300 px-4 py-5 text-sm text-slate-500 dark:border-slate-700 dark:text-slate-400">No streams defined yet.</div>
			{:else}
				{#each streams as stream}
					<button class={`w-full rounded-2xl border px-4 py-3 text-left transition ${localDraft.id === stream.id ? 'border-sky-400 bg-sky-50 dark:border-sky-700 dark:bg-sky-950/30' : 'border-slate-200 bg-slate-50 hover:border-slate-300 dark:border-slate-800 dark:bg-slate-900 dark:hover:border-slate-700'}`} onclick={() => onSelect?.(stream.id)} type="button">
						<div class="text-sm font-semibold text-slate-900 dark:text-slate-100">{stream.name}</div>
						<div class="mt-1 text-xs text-slate-500">{stream.source_binding.connector_type} • {stream.schema.fields.length} fields • {stream.retention_hours}h retention</div>
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
					<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Status</div>
					<input class="mt-2 w-full bg-transparent text-sm text-slate-900 outline-none dark:text-slate-100" value={localDraft.status} oninput={(event) => updateDraft('status', (event.currentTarget as HTMLInputElement).value)} />
				</label>
			</div>

			<label class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 dark:border-slate-800 dark:bg-slate-900">
				<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Description</div>
				<textarea class="mt-2 h-20 w-full bg-transparent text-sm text-slate-900 outline-none dark:text-slate-100" oninput={(event) => updateDraft('description', (event.currentTarget as HTMLTextAreaElement).value)}>{localDraft.description}</textarea>
			</label>

			<div class="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
				<label class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 dark:border-slate-800 dark:bg-slate-900">
					<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Connector</div>
					<input class="mt-2 w-full bg-transparent text-sm text-slate-900 outline-none dark:text-slate-100" value={localDraft.connector_type} oninput={(event) => updateDraft('connector_type', (event.currentTarget as HTMLInputElement).value)} />
				</label>
				<label class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 dark:border-slate-800 dark:bg-slate-900">
					<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Endpoint</div>
					<input class="mt-2 w-full bg-transparent text-sm text-slate-900 outline-none dark:text-slate-100" value={localDraft.endpoint} oninput={(event) => updateDraft('endpoint', (event.currentTarget as HTMLInputElement).value)} />
				</label>
				<label class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 dark:border-slate-800 dark:bg-slate-900">
					<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Format</div>
					<input class="mt-2 w-full bg-transparent text-sm text-slate-900 outline-none dark:text-slate-100" value={localDraft.format} oninput={(event) => updateDraft('format', (event.currentTarget as HTMLInputElement).value)} />
				</label>
				<label class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 dark:border-slate-800 dark:bg-slate-900">
					<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Retention Hours</div>
					<input class="mt-2 w-full bg-transparent text-sm text-slate-900 outline-none dark:text-slate-100" type="number" value={String(localDraft.retention_hours)} oninput={(event) => updateDraft('retention_hours', Number((event.currentTarget as HTMLInputElement).value) || 72)} />
				</label>
			</div>

			<label class="rounded-2xl border border-dashed border-sky-300 bg-sky-50/60 px-4 py-3 dark:border-sky-900 dark:bg-sky-950/20">
				<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-sky-700 dark:text-sky-300">Schema JSON</div>
				<textarea class="mt-2 h-56 w-full bg-transparent text-sm text-slate-900 outline-none dark:text-slate-100" oninput={(event) => updateDraft('schema_text', (event.currentTarget as HTMLTextAreaElement).value)}>{localDraft.schema_text}</textarea>
			</label>
		</div>
	</div>
</section>
