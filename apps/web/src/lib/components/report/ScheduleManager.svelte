<script lang="ts">
	import type { ScheduleBoard } from '$lib/api/reports';

	export let board: ScheduleBoard | null = null;
	export let selectedReportId = '';
	export let busy = false;
	export let onSelectReport: (reportId: string) => void;
	export let onGenerate: () => void;
</script>

<section class="rounded-3xl border border-stone-200 bg-white p-5 shadow-sm shadow-stone-200/60">
	<div class="flex items-start justify-between gap-4">
		<div>
			<p class="text-xs font-semibold uppercase tracking-[0.24em] text-amber-700">Schedule Manager</p>
			<h3 class="mt-2 text-xl font-semibold text-stone-900">Upcoming runs and recent deliveries</h3>
		</div>
		<button class="rounded-full bg-stone-900 px-4 py-2 text-sm font-semibold text-white transition hover:bg-stone-800 disabled:cursor-not-allowed disabled:bg-stone-400" onclick={onGenerate} disabled={busy || !selectedReportId}>
			Run selected report
		</button>
	</div>

	{#if board}
		<div class="mt-5 grid gap-4 md:grid-cols-3">
			<div class="rounded-2xl border border-stone-200 bg-amber-50 p-4">
				<p class="text-xs font-semibold uppercase tracking-[0.18em] text-amber-700">Active schedules</p>
				<p class="mt-3 text-3xl font-semibold text-stone-900">{board.active_schedules}</p>
			</div>
			<div class="rounded-2xl border border-stone-200 bg-stone-50 p-4">
				<p class="text-xs font-semibold uppercase tracking-[0.18em] text-stone-500">Paused drafts</p>
				<p class="mt-3 text-3xl font-semibold text-stone-900">{board.paused_reports}</p>
			</div>
			<div class="rounded-2xl border border-stone-200 bg-emerald-50 p-4">
				<p class="text-xs font-semibold uppercase tracking-[0.18em] text-emerald-700">Recent executions</p>
				<p class="mt-3 text-3xl font-semibold text-stone-900">{board.recent_executions.length}</p>
			</div>
		</div>

		<div class="mt-5 grid gap-4 xl:grid-cols-2">
			<div class="rounded-2xl border border-stone-200 bg-stone-50 p-4">
				<p class="text-sm font-semibold text-stone-800">Upcoming queue</p>
				<div class="mt-3 space-y-3">
					{#each board.upcoming as run}
						<button class="flex w-full items-start justify-between gap-3 rounded-2xl border border-stone-200 bg-white px-4 py-3 text-left transition hover:border-amber-300 hover:bg-amber-50" onclick={() => onSelectReport(run.report_id)}>
							<div>
								<p class="font-medium text-stone-900">{run.report_name}</p>
								<p class="text-sm text-stone-500">{run.cadence} • {run.generator_kind} • {run.recipient_count} targets</p>
							</div>
							<p class="text-right text-sm text-stone-600">{new Date(run.next_run_at).toLocaleString()}</p>
						</button>
					{/each}
				</div>
			</div>

			<div class="rounded-2xl border border-stone-200 bg-stone-50 p-4">
				<p class="text-sm font-semibold text-stone-800">Last five executions</p>
				<div class="mt-3 space-y-3">
					{#each board.recent_executions as execution}
						<div class="rounded-2xl border border-stone-200 bg-white px-4 py-3">
							<div class="flex items-start justify-between gap-3">
								<div>
									<p class="font-medium text-stone-900">{execution.report_name}</p>
									<p class="text-sm text-stone-500">{execution.generator_kind} • {execution.triggered_by}</p>
								</div>
								<p class="text-sm text-stone-600">{new Date(execution.generated_at).toLocaleString()}</p>
							</div>
						</div>
					{/each}
				</div>
			</div>
		</div>
	{:else}
		<p class="mt-5 text-sm text-stone-500">Schedule telemetry will appear after the first load.</p>
	{/if}
</section>
