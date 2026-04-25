<script lang="ts">
	import type {
		AgentDefinition,
		AgentExecutionResponse,
		KnowledgeBase,
		ToolDefinition,
	} from '$lib/api/ai';

	interface AgentDraft {
		id?: string;
		name: string;
		description: string;
		status: string;
		system_prompt: string;
		objective: string;
		tool_ids: string[];
		planning_strategy: string;
		max_iterations: number;
		memory_text: string;
	}

	interface ExecutionDraft {
		user_message: string;
		objective: string;
		knowledge_base_id: string;
		context_text: string;
	}

	interface Props {
		agents: AgentDefinition[];
		tools: ToolDefinition[];
		knowledgeBases: KnowledgeBase[];
		draft: AgentDraft;
		executionDraft: ExecutionDraft;
		executionResponse: AgentExecutionResponse | null;
		busy?: boolean;
		onSelect?: (agentId: string) => void;
		onDraftChange?: (draft: AgentDraft) => void;
		onExecutionDraftChange?: (draft: ExecutionDraft) => void;
		onSave?: () => void;
		onExecute?: () => void;
		onReset?: () => void;
	}

	let {
		agents,
		tools,
		knowledgeBases,
		draft,
		executionDraft,
		executionResponse,
		busy = false,
		onSelect,
		onDraftChange,
		onExecutionDraftChange,
		onSave,
		onExecute,
		onReset,
	}: Props = $props();

	let localDraft = $state<AgentDraft>({
		id: undefined,
		name: '',
		description: '',
		status: '',
		system_prompt: '',
		objective: '',
		tool_ids: [],
		planning_strategy: '',
		max_iterations: 3,
		memory_text: '',
	});
	let localExecutionDraft = $state<ExecutionDraft>({
		user_message: '',
		objective: '',
		knowledge_base_id: '',
		context_text: '',
	});

	$effect(() => {
		localDraft = { ...draft };
	});
	$effect(() => {
		localExecutionDraft = { ...executionDraft };
	});

	function updateDraft<K extends keyof AgentDraft>(key: K, value: AgentDraft[K]) {
		const nextDraft = { ...localDraft, [key]: value };
		localDraft = nextDraft;
		onDraftChange?.(nextDraft);
	}

	function updateExecution<K extends keyof ExecutionDraft>(key: K, value: ExecutionDraft[K]) {
		const nextDraft = { ...localExecutionDraft, [key]: value };
		localExecutionDraft = nextDraft;
		onExecutionDraftChange?.(nextDraft);
	}

	function toggleTool(toolId: string) {
		const nextToolIds = localDraft.tool_ids.includes(toolId)
			? localDraft.tool_ids.filter((value) => value !== toolId)
			: [...localDraft.tool_ids, toolId];
		updateDraft('tool_ids', nextToolIds);
	}
</script>

<section class="rounded-[28px] border border-slate-200 bg-white p-5 shadow-sm dark:border-slate-800 dark:bg-slate-950">
	<div class="flex flex-wrap items-start justify-between gap-3">
		<div>
			<div class="text-[11px] font-semibold uppercase tracking-[0.28em] text-slate-500">Agent Builder</div>
			<h2 class="mt-2 text-xl font-semibold text-slate-900 dark:text-slate-100">Configure plan-act-observe agents and execute traces</h2>
		</div>
		<div class="flex gap-2">
			<button class="rounded-full border border-slate-300 px-3 py-1.5 text-sm text-slate-700 hover:bg-slate-100 dark:border-slate-700 dark:text-slate-200 dark:hover:bg-slate-900" onclick={() => onReset?.()} disabled={busy}>New</button>
			<button class="rounded-full bg-slate-950 px-3 py-1.5 text-sm font-medium text-white hover:bg-slate-800 disabled:opacity-60 dark:bg-slate-100 dark:text-slate-950" onclick={() => onSave?.()} disabled={busy}>Save</button>
		</div>
	</div>

	<div class="mt-5 grid gap-5 xl:grid-cols-[minmax(0,0.8fr)_minmax(0,1.2fr)]">
		<div class="space-y-3">
			{#each agents as agent}
				<button class={`w-full rounded-2xl border px-4 py-3 text-left transition ${localDraft.id === agent.id ? 'border-cyan-400 bg-cyan-50 dark:border-cyan-700 dark:bg-cyan-950/30' : 'border-slate-200 bg-slate-50 hover:border-slate-300 dark:border-slate-800 dark:bg-slate-900 dark:hover:border-slate-700'}`} onclick={() => onSelect?.(agent.id)} type="button">
					<div class="text-sm font-semibold text-slate-900 dark:text-slate-100">{agent.name}</div>
					<div class="mt-1 text-xs text-slate-500">{agent.planning_strategy} • {agent.max_iterations} iterations</div>
				</button>
			{/each}
		</div>

		<div class="grid gap-4">
			<div class="grid gap-4 md:grid-cols-2">
				<input class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 text-sm dark:border-slate-800 dark:bg-slate-900" value={localDraft.name} oninput={(event) => updateDraft('name', (event.currentTarget as HTMLInputElement).value)} placeholder="Platform Analyst" />
				<input class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 text-sm dark:border-slate-800 dark:bg-slate-900" value={localDraft.planning_strategy} oninput={(event) => updateDraft('planning_strategy', (event.currentTarget as HTMLInputElement).value)} placeholder="plan-act-observe" />
			</div>
			<textarea class="h-24 rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 text-sm dark:border-slate-800 dark:bg-slate-900" oninput={(event) => updateDraft('description', (event.currentTarget as HTMLTextAreaElement).value)}>{localDraft.description}</textarea>
			<textarea class="h-24 rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 text-sm dark:border-slate-800 dark:bg-slate-900" oninput={(event) => updateDraft('system_prompt', (event.currentTarget as HTMLTextAreaElement).value)}>{localDraft.system_prompt}</textarea>
			<textarea class="h-24 rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 text-sm dark:border-slate-800 dark:bg-slate-900" oninput={(event) => updateDraft('objective', (event.currentTarget as HTMLTextAreaElement).value)}>{localDraft.objective}</textarea>
			<input class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 text-sm dark:border-slate-800 dark:bg-slate-900" type="number" value={String(localDraft.max_iterations)} oninput={(event) => updateDraft('max_iterations', Number((event.currentTarget as HTMLInputElement).value) || 3)} />
			<div class="rounded-2xl border border-slate-200 bg-slate-50 p-4 dark:border-slate-800 dark:bg-slate-900">
				<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Tool Access</div>
				<div class="mt-3 grid gap-2 md:grid-cols-2">
					{#each tools as tool}
						<label class="flex items-center gap-2 rounded-xl border border-slate-200 bg-white px-3 py-2 text-sm dark:border-slate-800 dark:bg-slate-950">
							<input type="checkbox" checked={localDraft.tool_ids.includes(tool.id)} onchange={() => toggleTool(tool.id)} />
							<span>{tool.name}</span>
						</label>
					{/each}
				</div>
			</div>
			<textarea class="h-28 rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 text-sm dark:border-slate-800 dark:bg-slate-900" oninput={(event) => updateDraft('memory_text', (event.currentTarget as HTMLTextAreaElement).value)}>{localDraft.memory_text}</textarea>

			<div class="rounded-[24px] border border-dashed border-cyan-300 bg-cyan-50/60 p-4 dark:border-cyan-900 dark:bg-cyan-950/20">
				<div class="flex items-center justify-between gap-3">
					<div>
						<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-cyan-700 dark:text-cyan-300">Execution Sandbox</div>
						<p class="mt-1 text-sm text-slate-600 dark:text-slate-300">Run the current agent against a selected knowledge base.</p>
					</div>
					<button class="rounded-full bg-slate-950 px-3 py-1.5 text-sm font-medium text-white hover:bg-slate-800 disabled:opacity-60 dark:bg-slate-100 dark:text-slate-950" onclick={() => onExecute?.()} disabled={busy || !localDraft.id}>Execute</button>
				</div>
				<div class="mt-4 grid gap-3">
					<textarea class="h-24 rounded-2xl border border-slate-200 bg-white px-4 py-3 text-sm dark:border-slate-800 dark:bg-slate-950" oninput={(event) => updateExecution('user_message', (event.currentTarget as HTMLTextAreaElement).value)}>{localExecutionDraft.user_message}</textarea>
					<input class="rounded-2xl border border-slate-200 bg-white px-4 py-3 text-sm dark:border-slate-800 dark:bg-slate-950" value={localExecutionDraft.objective} oninput={(event) => updateExecution('objective', (event.currentTarget as HTMLInputElement).value)} placeholder="Investigate provider failover" />
					<select class="rounded-2xl border border-slate-200 bg-white px-4 py-3 text-sm dark:border-slate-800 dark:bg-slate-950" value={localExecutionDraft.knowledge_base_id} onchange={(event) => updateExecution('knowledge_base_id', (event.currentTarget as HTMLSelectElement).value)}>
						<option value="">No knowledge base</option>
						{#each knowledgeBases as knowledgeBase}
							<option value={knowledgeBase.id}>{knowledgeBase.name}</option>
						{/each}
					</select>
					<textarea class="h-40 rounded-2xl border border-slate-200 bg-white px-4 py-3 font-mono text-xs dark:border-slate-800 dark:bg-slate-950" oninput={(event) => updateExecution('context_text', (event.currentTarget as HTMLTextAreaElement).value)}>{localExecutionDraft.context_text}</textarea>
				</div>

				{#if executionResponse}
					<div class="mt-4 space-y-3">
						<div class="rounded-2xl border border-slate-200 bg-white px-4 py-3 dark:border-slate-800 dark:bg-slate-950">
							<div class="text-xs font-semibold uppercase tracking-[0.24em] text-slate-500">Final Response</div>
							<p class="mt-2 text-sm leading-6 text-slate-700 dark:text-slate-200">{executionResponse.final_response}</p>
						</div>
						{#each executionResponse.traces as trace}
							<div class="rounded-2xl border border-slate-200 bg-white px-4 py-3 dark:border-slate-800 dark:bg-slate-950">
								<div class="flex items-center justify-between gap-3 text-sm">
									<span class="font-semibold text-slate-900 dark:text-slate-100">{trace.title}</span>
									<span class="text-xs text-slate-500">{trace.tool_name ?? 'reasoning'}</span>
								</div>
								<p class="mt-2 text-sm text-slate-600 dark:text-slate-300">{trace.observation}</p>
								<pre class="mt-3 overflow-x-auto rounded-2xl bg-slate-950 px-3 py-3 text-xs text-cyan-100">{JSON.stringify(trace.output, null, 2)}</pre>
							</div>
						{/each}
					</div>
				{/if}
			</div>
		</div>
	</div>
</section>
