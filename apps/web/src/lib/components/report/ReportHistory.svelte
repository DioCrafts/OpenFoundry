<script lang="ts">
	import type { ReportExecution } from '$lib/api/reports';

	export let history: ReportExecution[] = [];
	export let onSelectExecution: (executionId: string) => void;
</script>

<section class="rounded-3xl border border-stone-200 bg-white p-5 shadow-sm shadow-stone-200/60">
	<div class="flex items-start justify-between gap-4">
		<div>
			<p class="text-xs font-semibold uppercase tracking-[0.24em] text-amber-700">History</p>
			<h3 class="mt-2 text-xl font-semibold text-stone-900">Execution timeline</h3>
		</div>
		<p class="text-sm text-stone-500">Select a run to inspect its preview and artifact metadata.</p>
	</div>

	<div class="mt-5 space-y-3">
		{#if history.length > 0}
			{#each history as execution}
				<button class="flex w-full items-start justify-between gap-3 rounded-2xl border border-stone-200 bg-stone-50 px-4 py-4 text-left transition hover:border-amber-300 hover:bg-amber-50" onclick={() => onSelectExecution(execution.id)}>
					<div>
						<p class="font-semibold text-stone-900">{execution.report_name}</p>
						<p class="text-sm text-stone-500">{execution.generator_kind} • {execution.status} • {execution.metrics.row_count.toLocaleString()} rows</p>
					</div>
					<div class="text-right text-sm text-stone-500">
						<p>{new Date(execution.generated_at).toLocaleString()}</p>
						<p>{execution.metrics.duration_ms} ms</p>
					</div>
				</button>
			{/each}
		{:else}
			<div class="rounded-2xl border border-dashed border-stone-300 bg-stone-50 p-8 text-center text-sm text-stone-500">
				No executions recorded yet for the selected report.
			</div>
		{/if}
	</div>
</section>
