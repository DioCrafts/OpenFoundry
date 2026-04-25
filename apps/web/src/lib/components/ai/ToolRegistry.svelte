<script lang="ts">
	import type { ToolDefinition } from '$lib/api/ai';

	interface ToolDraft {
		id?: string;
		name: string;
		description: string;
		category: string;
		execution_mode: string;
		execution_config_text: string;
		status: string;
		input_schema_text: string;
		output_schema_text: string;
		tags_text: string;
	}

	interface Props {
		tools: ToolDefinition[];
		draft: ToolDraft;
		busy?: boolean;
		onSelect?: (toolId: string) => void;
		onDraftChange?: (draft: ToolDraft) => void;
		onSave?: () => void;
		onReset?: () => void;
	}

	let { tools, draft, busy = false, onSelect, onDraftChange, onSave, onReset }: Props = $props();
	let localDraft = $state<ToolDraft>({
		id: undefined,
		name: '',
		description: '',
		category: '',
		execution_mode: '',
		execution_config_text: '',
		status: '',
		input_schema_text: '',
		output_schema_text: '',
		tags_text: '',
	});

	$effect(() => {
		localDraft = { ...draft };
	});

	function updateDraft<K extends keyof ToolDraft>(key: K, value: ToolDraft[K]) {
		const nextDraft = { ...localDraft, [key]: value };
		localDraft = nextDraft;
		onDraftChange?.(nextDraft);
	}
</script>

<section class="rounded-[28px] border border-slate-200 bg-white p-5 shadow-sm dark:border-slate-800 dark:bg-slate-950">
	<div class="flex flex-wrap items-start justify-between gap-3">
		<div>
			<div class="text-[11px] font-semibold uppercase tracking-[0.28em] text-slate-500">Tool Registry</div>
			<h2 class="mt-2 text-xl font-semibold text-slate-900 dark:text-slate-100">Manage callable tools for agent execution</h2>
		</div>
		<div class="flex gap-2">
			<button class="rounded-full border border-slate-300 px-3 py-1.5 text-sm text-slate-700 hover:bg-slate-100 dark:border-slate-700 dark:text-slate-200 dark:hover:bg-slate-900" onclick={() => onReset?.()} disabled={busy}>New</button>
			<button class="rounded-full bg-slate-950 px-3 py-1.5 text-sm font-medium text-white hover:bg-slate-800 disabled:opacity-60 dark:bg-slate-100 dark:text-slate-950" onclick={() => onSave?.()} disabled={busy}>Save</button>
		</div>
	</div>

	<div class="mt-5 grid gap-5 xl:grid-cols-[minmax(0,0.8fr)_minmax(0,1.2fr)]">
		<div class="space-y-3">
			{#if tools.length === 0}
				<div class="rounded-2xl border border-dashed border-slate-300 px-4 py-5 text-sm text-slate-500 dark:border-slate-700 dark:text-slate-400">No tools registered yet.</div>
			{:else}
				{#each tools as tool}
					<button class={`w-full rounded-2xl border px-4 py-3 text-left transition ${localDraft.id === tool.id ? 'border-cyan-400 bg-cyan-50 dark:border-cyan-700 dark:bg-cyan-950/30' : 'border-slate-200 bg-slate-50 hover:border-slate-300 dark:border-slate-800 dark:bg-slate-900 dark:hover:border-slate-700'}`} onclick={() => onSelect?.(tool.id)} type="button">
						<div class="text-sm font-semibold text-slate-900 dark:text-slate-100">{tool.name}</div>
						<div class="mt-1 text-xs text-slate-500">{tool.category} • {tool.execution_mode}</div>
					</button>
				{/each}
			{/if}
		</div>

		<div class="grid gap-4">
			<div class="grid gap-4 md:grid-cols-2">
				<input class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 text-sm dark:border-slate-800 dark:bg-slate-900" value={localDraft.name} oninput={(event) => updateDraft('name', (event.currentTarget as HTMLInputElement).value)} placeholder="SQL Generator" />
				<input class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 text-sm dark:border-slate-800 dark:bg-slate-900" value={localDraft.category} oninput={(event) => updateDraft('category', (event.currentTarget as HTMLInputElement).value)} placeholder="analysis" />
			</div>
			<div class="grid gap-4 md:grid-cols-2">
				<select class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 text-sm dark:border-slate-800 dark:bg-slate-900" value={localDraft.execution_mode} onchange={(event) => updateDraft('execution_mode', (event.currentTarget as HTMLSelectElement).value)}>
					<option value="native_sql">native_sql</option>
					<option value="native_dataset">native_dataset</option>
					<option value="native_ontology">native_ontology</option>
					<option value="native_pipeline">native_pipeline</option>
					<option value="native_report">native_report</option>
					<option value="native_workflow">native_workflow</option>
					<option value="native_code_repo">native_code_repo</option>
					<option value="knowledge_search">knowledge_search</option>
					<option value="openfoundry_api">openfoundry_api</option>
					<option value="http_json">http_json</option>
					<option value="simulated">simulated</option>
				</select>
				<input class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 text-sm dark:border-slate-800 dark:bg-slate-900" value={localDraft.tags_text} oninput={(event) => updateDraft('tags_text', (event.currentTarget as HTMLInputElement).value)} placeholder="sql, copilot" />
			</div>
			<textarea class="h-24 rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 text-sm dark:border-slate-800 dark:bg-slate-900" oninput={(event) => updateDraft('description', (event.currentTarget as HTMLTextAreaElement).value)}>{localDraft.description}</textarea>
			<textarea class="h-40 rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 font-mono text-xs dark:border-slate-800 dark:bg-slate-900" oninput={(event) => updateDraft('execution_config_text', (event.currentTarget as HTMLTextAreaElement).value)}>{localDraft.execution_config_text}</textarea>
			<div class="grid gap-4 md:grid-cols-2">
				<textarea class="h-40 rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 text-sm dark:border-slate-800 dark:bg-slate-900" oninput={(event) => updateDraft('input_schema_text', (event.currentTarget as HTMLTextAreaElement).value)}>{localDraft.input_schema_text}</textarea>
				<textarea class="h-40 rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 text-sm dark:border-slate-800 dark:bg-slate-900" oninput={(event) => updateDraft('output_schema_text', (event.currentTarget as HTMLTextAreaElement).value)}>{localDraft.output_schema_text}</textarea>
			</div>
			<p class="text-xs text-slate-500">`native_dataset`, `native_sql`, `native_ontology`, `native_pipeline`, `native_report`, `native_workflow`, `native_code_repo` y `knowledge_search` se ejecutan dentro del runtime del agente. `openfoundry_api` y `http_json` aceptan `execution_config` para rutas, auth y payloads.</p>
		</div>
	</div>
</section>
