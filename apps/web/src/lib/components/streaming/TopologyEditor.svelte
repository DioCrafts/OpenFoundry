<script lang="ts">
	import type { StreamDefinition, TopologyDefinition, WindowDefinition } from '$lib/api/streaming';

	interface TopologyDraft {
		id?: string;
		name: string;
		description: string;
		status: string;
		state_backend: string;
		source_stream_ids_text: string;
		nodes_text: string;
		edges_text: string;
		join_definition_text: string;
		cep_definition_text: string;
		backpressure_policy_text: string;
		sink_bindings_text: string;
	}

	interface Props {
		topologies: TopologyDefinition[];
		streams: StreamDefinition[];
		windows: WindowDefinition[];
		draft: TopologyDraft;
		busy?: boolean;
		onSelect?: (topologyId: string) => void;
		onDraftChange?: (draft: TopologyDraft) => void;
		onSave?: () => void;
		onReset?: () => void;
	}

	let { topologies, streams, windows, draft, busy = false, onSelect, onDraftChange, onSave, onReset }: Props = $props();

	let localDraft = $state<TopologyDraft>({
		id: undefined,
		name: '',
		description: '',
		status: '',
		state_backend: '',
		source_stream_ids_text: '',
		nodes_text: '',
		edges_text: '',
		join_definition_text: '',
		cep_definition_text: '',
		backpressure_policy_text: '',
		sink_bindings_text: '',
	});

	$effect(() => {
		localDraft = { ...draft };
	});

	function updateDraft<K extends keyof TopologyDraft>(key: K, value: TopologyDraft[K]) {
		const nextDraft = { ...localDraft, [key]: value };
		localDraft = nextDraft;
		onDraftChange?.(nextDraft);
	}
</script>

<section class="rounded-[28px] border border-slate-200 bg-white p-5 shadow-sm dark:border-slate-800 dark:bg-slate-950">
	<div class="flex flex-wrap items-start justify-between gap-3">
		<div>
			<div class="text-[11px] font-semibold uppercase tracking-[0.28em] text-slate-500">Topology Editor</div>
			<h2 class="mt-2 text-xl font-semibold text-slate-900 dark:text-slate-100">DAG-based processing, joins, CEP, sinks, and backpressure policy</h2>
		</div>
		<div class="flex gap-2">
			<button class="rounded-full border border-slate-300 px-3 py-1.5 text-sm text-slate-700 hover:bg-slate-100 dark:border-slate-700 dark:text-slate-200 dark:hover:bg-slate-900" onclick={() => onReset?.()} disabled={busy}>New</button>
			<button class="rounded-full bg-slate-950 px-3 py-1.5 text-sm font-medium text-white hover:bg-slate-800 disabled:opacity-60 dark:bg-slate-100 dark:text-slate-950" onclick={() => onSave?.()} disabled={busy}>Save</button>
		</div>
	</div>

	<div class="mt-5 grid gap-5 xl:grid-cols-[minmax(0,0.76fr)_minmax(0,1.24fr)]">
		<div class="space-y-3">
			<div class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-4 text-sm text-slate-600 dark:border-slate-800 dark:bg-slate-900 dark:text-slate-300">
				{streams.length} streams available • {windows.length} windows available
			</div>
			{#if topologies.length === 0}
				<div class="rounded-2xl border border-dashed border-slate-300 px-4 py-5 text-sm text-slate-500 dark:border-slate-700 dark:text-slate-400">No topologies defined yet.</div>
			{:else}
				{#each topologies as topology}
					<button class={`w-full rounded-2xl border px-4 py-3 text-left transition ${localDraft.id === topology.id ? 'border-emerald-400 bg-emerald-50 dark:border-emerald-700 dark:bg-emerald-950/30' : 'border-slate-200 bg-slate-50 hover:border-slate-300 dark:border-slate-800 dark:bg-slate-900 dark:hover:border-slate-700'}`} onclick={() => onSelect?.(topology.id)} type="button">
						<div class="text-sm font-semibold text-slate-900 dark:text-slate-100">{topology.name}</div>
						<div class="mt-1 text-xs text-slate-500">{topology.nodes.length} nodes • {topology.edges.length} edges • {topology.state_backend}</div>
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
					<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">State Backend</div>
					<input class="mt-2 w-full bg-transparent text-sm text-slate-900 outline-none dark:text-slate-100" value={localDraft.state_backend} oninput={(event) => updateDraft('state_backend', (event.currentTarget as HTMLInputElement).value)} />
				</label>
			</div>

			<label class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 dark:border-slate-800 dark:bg-slate-900">
				<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Description</div>
				<textarea class="mt-2 h-20 w-full bg-transparent text-sm text-slate-900 outline-none dark:text-slate-100" oninput={(event) => updateDraft('description', (event.currentTarget as HTMLTextAreaElement).value)}>{localDraft.description}</textarea>
			</label>

			<div class="grid gap-4 md:grid-cols-2">
				<label class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 dark:border-slate-800 dark:bg-slate-900">
					<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Status</div>
					<input class="mt-2 w-full bg-transparent text-sm text-slate-900 outline-none dark:text-slate-100" value={localDraft.status} oninput={(event) => updateDraft('status', (event.currentTarget as HTMLInputElement).value)} />
				</label>
				<label class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 dark:border-slate-800 dark:bg-slate-900">
					<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Source Stream IDs</div>
					<input class="mt-2 w-full bg-transparent text-sm text-slate-900 outline-none dark:text-slate-100" value={localDraft.source_stream_ids_text} oninput={(event) => updateDraft('source_stream_ids_text', (event.currentTarget as HTMLInputElement).value)} placeholder="uuid-1, uuid-2" />
				</label>
			</div>

			<div class="grid gap-4 md:grid-cols-2">
				<label class="rounded-2xl border border-dashed border-emerald-300 bg-emerald-50/60 px-4 py-3 dark:border-emerald-900 dark:bg-emerald-950/20">
					<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-emerald-700 dark:text-emerald-300">Nodes JSON</div>
					<textarea class="mt-2 h-44 w-full bg-transparent text-sm text-slate-900 outline-none dark:text-slate-100" oninput={(event) => updateDraft('nodes_text', (event.currentTarget as HTMLTextAreaElement).value)}>{localDraft.nodes_text}</textarea>
				</label>
				<label class="rounded-2xl border border-dashed border-emerald-300 bg-emerald-50/60 px-4 py-3 dark:border-emerald-900 dark:bg-emerald-950/20">
					<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-emerald-700 dark:text-emerald-300">Edges JSON</div>
					<textarea class="mt-2 h-44 w-full bg-transparent text-sm text-slate-900 outline-none dark:text-slate-100" oninput={(event) => updateDraft('edges_text', (event.currentTarget as HTMLTextAreaElement).value)}>{localDraft.edges_text}</textarea>
				</label>
			</div>

			<div class="grid gap-4 md:grid-cols-2">
				<label class="rounded-2xl border border-dashed border-emerald-300 bg-emerald-50/60 px-4 py-3 dark:border-emerald-900 dark:bg-emerald-950/20">
					<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-emerald-700 dark:text-emerald-300">Join Definition JSON</div>
					<textarea class="mt-2 h-36 w-full bg-transparent text-sm text-slate-900 outline-none dark:text-slate-100" oninput={(event) => updateDraft('join_definition_text', (event.currentTarget as HTMLTextAreaElement).value)}>{localDraft.join_definition_text}</textarea>
				</label>
				<label class="rounded-2xl border border-dashed border-emerald-300 bg-emerald-50/60 px-4 py-3 dark:border-emerald-900 dark:bg-emerald-950/20">
					<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-emerald-700 dark:text-emerald-300">CEP Definition JSON</div>
					<textarea class="mt-2 h-36 w-full bg-transparent text-sm text-slate-900 outline-none dark:text-slate-100" oninput={(event) => updateDraft('cep_definition_text', (event.currentTarget as HTMLTextAreaElement).value)}>{localDraft.cep_definition_text}</textarea>
				</label>
			</div>

			<div class="grid gap-4 md:grid-cols-2">
				<label class="rounded-2xl border border-dashed border-emerald-300 bg-emerald-50/60 px-4 py-3 dark:border-emerald-900 dark:bg-emerald-950/20">
					<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-emerald-700 dark:text-emerald-300">Backpressure Policy JSON</div>
					<textarea class="mt-2 h-32 w-full bg-transparent text-sm text-slate-900 outline-none dark:text-slate-100" oninput={(event) => updateDraft('backpressure_policy_text', (event.currentTarget as HTMLTextAreaElement).value)}>{localDraft.backpressure_policy_text}</textarea>
				</label>
				<label class="rounded-2xl border border-dashed border-emerald-300 bg-emerald-50/60 px-4 py-3 dark:border-emerald-900 dark:bg-emerald-950/20">
					<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-emerald-700 dark:text-emerald-300">Sink Bindings JSON</div>
					<textarea class="mt-2 h-32 w-full bg-transparent text-sm text-slate-900 outline-none dark:text-slate-100" oninput={(event) => updateDraft('sink_bindings_text', (event.currentTarget as HTMLTextAreaElement).value)}>{localDraft.sink_bindings_text}</textarea>
				</label>
			</div>
		</div>
	</div>
</section>
