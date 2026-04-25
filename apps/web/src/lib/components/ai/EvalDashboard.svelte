<script lang="ts">
	import type {
		AiPlatformOverview,
		EvaluateGuardrailsResponse,
		LlmProvider,
	} from '$lib/api/ai';

	interface Props {
		overview: AiPlatformOverview | null;
		providers: LlmProvider[];
		guardrailInput: string;
		guardrailResponse: EvaluateGuardrailsResponse | null;
		busy?: boolean;
		onGuardrailInputChange?: (value: string) => void;
		onEvaluate?: () => void;
	}

	let {
		overview,
		providers,
		guardrailInput,
		guardrailResponse,
		busy = false,
		onGuardrailInputChange,
		onEvaluate,
	}: Props = $props();
</script>

<section class="rounded-[28px] border border-slate-200 bg-white p-5 shadow-sm dark:border-slate-800 dark:bg-slate-950">
	<div class="flex flex-wrap items-start justify-between gap-3">
		<div>
			<div class="text-[11px] font-semibold uppercase tracking-[0.28em] text-slate-500">Evaluation</div>
			<h2 class="mt-2 text-xl font-semibold text-slate-900 dark:text-slate-100">Provider health, cache efficiency, and guardrails</h2>
		</div>
		<button class="rounded-full border border-cyan-300 px-3 py-1.5 text-sm text-cyan-700 hover:bg-cyan-50 dark:border-cyan-800 dark:text-cyan-300 dark:hover:bg-cyan-950/40" onclick={() => onEvaluate?.()} disabled={busy}>Evaluate guardrails</button>
	</div>

	<div class="mt-5 grid gap-4 md:grid-cols-2 xl:grid-cols-6">
		<div class="rounded-2xl bg-slate-950 px-4 py-4 text-white">
			<div class="text-[11px] uppercase tracking-[0.24em] text-slate-300">Providers</div>
			<div class="mt-2 text-3xl font-semibold">{overview?.provider_count ?? 0}</div>
		</div>
		<div class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-4 dark:border-slate-800 dark:bg-slate-900">
			<div class="text-[11px] uppercase tracking-[0.24em] text-slate-500">Prompts</div>
			<div class="mt-2 text-3xl font-semibold text-slate-900 dark:text-slate-100">{overview?.prompt_count ?? 0}</div>
		</div>
		<div class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-4 dark:border-slate-800 dark:bg-slate-900">
			<div class="text-[11px] uppercase tracking-[0.24em] text-slate-500">KB Chunks</div>
			<div class="mt-2 text-3xl font-semibold text-slate-900 dark:text-slate-100">{overview?.indexed_chunk_count ?? 0}</div>
		</div>
		<div class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-4 dark:border-slate-800 dark:bg-slate-900">
			<div class="text-[11px] uppercase tracking-[0.24em] text-slate-500">Agents</div>
			<div class="mt-2 text-3xl font-semibold text-slate-900 dark:text-slate-100">{overview?.agent_count ?? 0}</div>
		</div>
		<div class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-4 dark:border-slate-800 dark:bg-slate-900">
			<div class="text-[11px] uppercase tracking-[0.24em] text-slate-500">Cache Hit Rate</div>
			<div class="mt-2 text-3xl font-semibold text-slate-900 dark:text-slate-100">{overview ? `${Math.round(overview.cache_hit_rate * 100)}%` : '0%'}</div>
		</div>
		<div class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-4 dark:border-slate-800 dark:bg-slate-900">
			<div class="text-[11px] uppercase tracking-[0.24em] text-slate-500">LLM Cost</div>
			<div class="mt-2 text-3xl font-semibold text-slate-900 dark:text-slate-100">${(overview?.estimated_llm_cost_usd ?? 0).toFixed(2)}</div>
			<div class="mt-1 text-xs text-slate-500">{overview?.benchmark_run_count ?? 0} benchmarks</div>
		</div>
	</div>

	<div class="mt-5 grid gap-5 xl:grid-cols-[minmax(0,1.1fr)_minmax(0,0.9fr)]">
		<div class="rounded-[24px] border border-slate-200 bg-slate-50 p-4 dark:border-slate-800 dark:bg-slate-900">
			<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Provider Routing Health</div>
			<div class="mt-3 space-y-3">
				{#each providers as provider}
					<div class="rounded-2xl border border-slate-200 bg-white px-4 py-3 dark:border-slate-800 dark:bg-slate-950">
						<div class="flex items-center justify-between gap-3">
							<div>
								<div class="text-sm font-semibold text-slate-900 dark:text-slate-100">{provider.name}</div>
								<div class="mt-1 text-xs text-slate-500">{provider.provider_type} • {provider.model_name}</div>
							</div>
							<span class={`rounded-full px-2.5 py-1 text-[11px] font-semibold uppercase tracking-[0.2em] ${provider.health_state.status === 'healthy' ? 'bg-emerald-100 text-emerald-700 dark:bg-emerald-950/40 dark:text-emerald-300' : provider.health_state.status === 'degraded' ? 'bg-amber-100 text-amber-700 dark:bg-amber-950/40 dark:text-amber-300' : 'bg-rose-100 text-rose-700 dark:bg-rose-950/40 dark:text-rose-300'}`}>{provider.health_state.status}</span>
						</div>
						<div class="mt-3 grid gap-2 text-sm text-slate-600 dark:text-slate-300 md:grid-cols-3">
							<div>Latency: {provider.health_state.avg_latency_ms} ms</div>
							<div>Error rate: {(provider.health_state.error_rate * 100).toFixed(1)}%</div>
							<div>Weight: {provider.route_rules.weight}</div>
						</div>
						<div class="mt-2 grid gap-2 text-xs text-slate-500 dark:text-slate-400 md:grid-cols-3">
							<div>Network: {provider.route_rules.network_scope}</div>
							<div>Modalities: {provider.route_rules.supported_modalities.join(', ')}</div>
							<div>Cost: ${provider.route_rules.input_cost_per_1k_tokens_usd.toFixed(4)} / ${provider.route_rules.output_cost_per_1k_tokens_usd.toFixed(4)}</div>
						</div>
					</div>
				{/each}
			</div>
		</div>

		<div class="rounded-[24px] border border-slate-200 bg-gradient-to-br from-cyan-50 to-white p-4 dark:border-slate-800 dark:from-cyan-950/20 dark:to-slate-950">
			<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Guardrail Tester</div>
			<textarea class="mt-3 h-32 w-full rounded-2xl border border-slate-200 bg-white px-4 py-3 text-sm dark:border-slate-800 dark:bg-slate-950" oninput={(event) => onGuardrailInputChange?.((event.currentTarget as HTMLTextAreaElement).value)}>{guardrailInput}</textarea>
			{#if guardrailResponse}
				<div class="mt-4 space-y-3 text-sm text-slate-700 dark:text-slate-200">
					<div class="rounded-2xl border border-slate-200 bg-white px-4 py-3 dark:border-slate-800 dark:bg-slate-950">Risk score: {guardrailResponse.risk_score.toFixed(2)}</div>
					<div class="rounded-2xl border border-slate-200 bg-white px-4 py-3 dark:border-slate-800 dark:bg-slate-950">Verdict: {guardrailResponse.verdict.blocked ? 'Blocked' : 'Passed'} • {guardrailResponse.verdict.flags.length} flags</div>
					{#if guardrailResponse.recommendations.length > 0}
						<ul class="space-y-2">
							{#each guardrailResponse.recommendations as recommendation}
								<li class="rounded-2xl border border-dashed border-cyan-200 bg-white px-4 py-3 dark:border-cyan-900 dark:bg-slate-950">{recommendation}</li>
							{/each}
						</ul>
					{/if}
				</div>
			{/if}
		</div>
	</div>
</section>
