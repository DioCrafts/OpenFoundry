<script lang="ts">
	import { onMount } from 'svelte';

	import { askCopilot, listKnowledgeBases, type CopilotResponse, type KnowledgeBase } from '$lib/api/ai';
	import { listDatasets, type Dataset } from '$lib/api/datasets';
	import { notifications } from '$stores/notifications';
	import { copilot } from '$stores/copilot';

	let datasets = $state<Dataset[]>([]);
	let knowledgeBases = $state<KnowledgeBase[]>([]);
	let selectedDatasetIds = $state<string[]>([]);
	let selectedKnowledgeBaseIds = $state<string[]>([]);
	let question = $state('Which provider should take over when latency spikes beyond 500ms?');
	let includeSql = $state(true);
	let includePipelinePlan = $state(true);
	let response = $state<CopilotResponse | null>(null);
	let loading = $state(false);
	let error = $state('');

	$effect(() => {
		if ($copilot.seedQuestion) {
			question = $copilot.seedQuestion;
		}
	});

	onMount(async () => {
		try {
			const [datasetResponse, knowledgeBaseResponse] = await Promise.all([
				listDatasets({ per_page: 50 }),
				listKnowledgeBases(),
			]);
			datasets = datasetResponse.data;
			knowledgeBases = knowledgeBaseResponse.data;
			selectedKnowledgeBaseIds = knowledgeBaseResponse.data.slice(0, 1).map((item) => item.id);
		} catch (cause) {
			error = cause instanceof Error ? cause.message : 'Failed to load copilot context';
		}
	});

	function toggleSelection(values: string[], id: string) {
		return values.includes(id) ? values.filter((item) => item !== id) : [...values, id];
	}

	async function submit() {
		if (!question.trim()) {
			notifications.error('Enter a question for the copilot.');
			return;
		}

		loading = true;
		error = '';

		try {
			response = await askCopilot({
				question: question.trim(),
				dataset_ids: selectedDatasetIds,
				knowledge_base_ids: selectedKnowledgeBaseIds,
				include_sql: includeSql,
				include_pipeline_plan: includePipelinePlan,
			});
			notifications.success('Copilot response updated.');
		} catch (cause) {
			error = cause instanceof Error ? cause.message : 'Copilot request failed';
			notifications.error(error);
		} finally {
			loading = false;
		}
	}
</script>

{#if !$copilot.open}
	<button
		class="fixed bottom-6 right-6 z-40 inline-flex h-14 w-14 items-center justify-center rounded-full bg-slate-950 text-sm font-semibold text-white shadow-2xl shadow-cyan-500/20 transition hover:-translate-y-0.5 hover:bg-cyan-500"
		onclick={() => copilot.open()}
	>
		AI
	</button>
{/if}

<div class={`fixed inset-y-0 right-0 z-50 w-full max-w-xl transform border-l border-slate-200 bg-white/96 shadow-2xl shadow-slate-950/10 backdrop-blur transition duration-300 dark:border-slate-800 dark:bg-slate-950/96 ${$copilot.open ? 'translate-x-0' : 'translate-x-full'}`}>
	<div class="flex h-full flex-col">
		<div class="border-b border-slate-200 bg-gradient-to-r from-cyan-500 via-sky-500 to-slate-900 p-5 text-white dark:border-slate-800">
			<div class="flex items-start justify-between gap-4">
				<div>
					<div class="text-[11px] font-semibold uppercase tracking-[0.3em] text-cyan-100">Platform Copilot</div>
					<h2 class="mt-2 text-2xl font-semibold">Ask for SQL, pipeline steps, or ontology hints</h2>
					<p class="mt-2 max-w-md text-sm text-cyan-50/90">
						The drawer stays available across the app and routes requests through the AIP backend.
					</p>
				</div>
				<button
					class="rounded-full border border-white/30 px-3 py-1 text-sm font-medium text-white transition hover:bg-white/10"
					onclick={() => copilot.close()}
				>
					Close
				</button>
			</div>
		</div>

		<div class="flex-1 space-y-5 overflow-y-auto p-5">
			<label class="block space-y-2">
				<span class="text-xs font-semibold uppercase tracking-[0.24em] text-slate-500">Question</span>
				<textarea
					rows="5"
					value={question}
					oninput={(event) => question = (event.currentTarget as HTMLTextAreaElement).value}
					class="w-full rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 text-sm text-slate-900 outline-none transition focus:border-cyan-500 focus:ring-2 focus:ring-cyan-200 dark:border-slate-800 dark:bg-slate-900 dark:text-slate-100"
					placeholder="Explain the failing workflow, ask for starter SQL, or request ontology mapping help."
				></textarea>
			</label>

			<div class="grid gap-4 lg:grid-cols-2">
				<div class="rounded-2xl border border-slate-200 bg-slate-50 p-4 dark:border-slate-800 dark:bg-slate-900">
					<div class="text-xs font-semibold uppercase tracking-[0.24em] text-slate-500">Datasets</div>
					<div class="mt-3 space-y-2">
						{#if datasets.length === 0}
							<p class="text-sm text-slate-500">No datasets available.</p>
						{:else}
							{#each datasets.slice(0, 6) as dataset}
								<label class="flex items-center gap-2 text-sm text-slate-700 dark:text-slate-300">
									<input
										type="checkbox"
										checked={selectedDatasetIds.includes(dataset.id)}
										onchange={() => selectedDatasetIds = toggleSelection(selectedDatasetIds, dataset.id)}
									/>
									<span>{dataset.name}</span>
								</label>
							{/each}
						{/if}
					</div>
				</div>

				<div class="rounded-2xl border border-slate-200 bg-slate-50 p-4 dark:border-slate-800 dark:bg-slate-900">
					<div class="text-xs font-semibold uppercase tracking-[0.24em] text-slate-500">Knowledge Bases</div>
					<div class="mt-3 space-y-2">
						{#if knowledgeBases.length === 0}
							<p class="text-sm text-slate-500">No knowledge bases available.</p>
						{:else}
							{#each knowledgeBases as knowledgeBase}
								<label class="flex items-center gap-2 text-sm text-slate-700 dark:text-slate-300">
									<input
										type="checkbox"
										checked={selectedKnowledgeBaseIds.includes(knowledgeBase.id)}
										onchange={() => selectedKnowledgeBaseIds = toggleSelection(selectedKnowledgeBaseIds, knowledgeBase.id)}
									/>
									<span>{knowledgeBase.name}</span>
								</label>
							{/each}
						{/if}
					</div>
				</div>
			</div>

			<div class="grid gap-3 rounded-2xl border border-slate-200 bg-white p-4 dark:border-slate-800 dark:bg-slate-900/70">
				<label class="flex items-center gap-3 text-sm text-slate-700 dark:text-slate-300">
					<input type="checkbox" checked={includeSql} onchange={() => includeSql = !includeSql} />
					<span>Include starter SQL</span>
				</label>
				<label class="flex items-center gap-3 text-sm text-slate-700 dark:text-slate-300">
					<input type="checkbox" checked={includePipelinePlan} onchange={() => includePipelinePlan = !includePipelinePlan} />
					<span>Include pipeline suggestions</span>
				</label>
			</div>

			<button
				class="inline-flex w-full items-center justify-center rounded-2xl bg-slate-950 px-4 py-3 text-sm font-semibold text-white transition hover:bg-cyan-500 disabled:cursor-not-allowed disabled:opacity-60 dark:bg-slate-100 dark:text-slate-950"
				onclick={submit}
				disabled={loading}
			>
				{loading ? 'Thinking...' : 'Ask Copilot'}
			</button>

			{#if error}
				<div class="rounded-2xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-700 dark:border-rose-900/70 dark:bg-rose-950/40 dark:text-rose-200">
					{error}
				</div>
			{/if}

			{#if response}
				<div class="space-y-4 rounded-[28px] border border-slate-200 bg-gradient-to-br from-slate-50 to-cyan-50 p-5 dark:border-slate-800 dark:from-slate-900 dark:to-slate-950">
					<div class="flex flex-wrap items-center gap-2 text-xs text-slate-500 dark:text-slate-400">
						<span class="rounded-full bg-white/80 px-2.5 py-1 font-medium dark:bg-slate-800">{response.provider_name}</span>
						<span class="rounded-full bg-white/80 px-2.5 py-1 font-medium dark:bg-slate-800">Cache {response.cache.hit ? 'hit' : 'miss'}</span>
					</div>
					<div>
						<div class="text-xs font-semibold uppercase tracking-[0.24em] text-slate-500">Answer</div>
						<p class="mt-2 whitespace-pre-wrap text-sm leading-6 text-slate-700 dark:text-slate-200">{response.answer}</p>
					</div>

					{#if response.suggested_sql}
						<div>
							<div class="text-xs font-semibold uppercase tracking-[0.24em] text-slate-500">Suggested SQL</div>
							<pre class="mt-2 overflow-x-auto rounded-2xl bg-slate-950 px-4 py-3 text-xs text-cyan-100">{response.suggested_sql}</pre>
						</div>
					{/if}

					{#if response.pipeline_suggestions.length > 0}
						<div>
							<div class="text-xs font-semibold uppercase tracking-[0.24em] text-slate-500">Pipeline Suggestions</div>
							<ul class="mt-2 space-y-2 text-sm text-slate-700 dark:text-slate-200">
								{#each response.pipeline_suggestions as suggestion}
									<li class="rounded-2xl border border-slate-200 bg-white px-3 py-2 dark:border-slate-800 dark:bg-slate-900">{suggestion}</li>
								{/each}
							</ul>
						</div>
					{/if}

					{#if response.ontology_hints.length > 0}
						<div>
							<div class="text-xs font-semibold uppercase tracking-[0.24em] text-slate-500">Ontology Hints</div>
							<ul class="mt-2 space-y-2 text-sm text-slate-700 dark:text-slate-200">
								{#each response.ontology_hints as hint}
									<li class="rounded-2xl border border-dashed border-cyan-200 bg-white px-3 py-2 dark:border-cyan-900 dark:bg-slate-900">{hint}</li>
								{/each}
							</ul>
						</div>
					{/if}
				</div>
			{/if}
		</div>
	</div>
</div>
