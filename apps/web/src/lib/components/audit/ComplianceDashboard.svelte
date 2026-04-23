<script lang="ts">
	import type { AnomalyAlert, AuditOverview, CollectorStatus, ComplianceReport } from '$lib/api/audit';

	export let overview: AuditOverview | null = null;
	export let collectors: CollectorStatus[] = [];
	export let anomalies: AnomalyAlert[] = [];
	export let reports: ComplianceReport[] = [];
</script>

<section class="rounded-3xl border border-stone-200 bg-white p-5 shadow-sm shadow-stone-200/60">
	<div>
		<p class="text-xs font-semibold uppercase tracking-[0.24em] text-teal-700">Compliance Dashboard</p>
		<h2 class="mt-2 text-2xl font-semibold text-stone-900">Integrity, anomalies, collectors, and evidence packs</h2>
		<p class="mt-1 text-sm text-stone-600">Monitor the append-only audit chain, NATS collector health, critical events, and compliance exports.</p>
	</div>

	<div class="mt-5 grid gap-4 sm:grid-cols-2 xl:grid-cols-6">
		<div class="rounded-2xl bg-stone-950 px-4 py-4 text-stone-50">
			<p class="text-xs uppercase tracking-[0.18em] text-teal-300">Events</p>
			<p class="mt-2 text-2xl font-semibold">{overview?.event_count ?? 0}</p>
		</div>
		<div class="rounded-2xl bg-rose-50 px-4 py-4 text-rose-900">
			<p class="text-xs uppercase tracking-[0.18em] text-rose-600">Critical</p>
			<p class="mt-2 text-2xl font-semibold">{overview?.critical_event_count ?? 0}</p>
		</div>
		<div class="rounded-2xl bg-sky-50 px-4 py-4 text-sky-900">
			<p class="text-xs uppercase tracking-[0.18em] text-sky-600">Collectors</p>
			<p class="mt-2 text-2xl font-semibold">{overview?.collector_count ?? 0}</p>
		</div>
		<div class="rounded-2xl bg-amber-50 px-4 py-4 text-amber-900">
			<p class="text-xs uppercase tracking-[0.18em] text-amber-600">Policies</p>
			<p class="mt-2 text-2xl font-semibold">{overview?.active_policy_count ?? 0}</p>
		</div>
		<div class="rounded-2xl bg-violet-50 px-4 py-4 text-violet-900">
			<p class="text-xs uppercase tracking-[0.18em] text-violet-600">Anomalies</p>
			<p class="mt-2 text-2xl font-semibold">{overview?.anomaly_count ?? 0}</p>
		</div>
		<div class="rounded-2xl bg-emerald-50 px-4 py-4 text-emerald-900">
			<p class="text-xs uppercase tracking-[0.18em] text-emerald-600">GDPR Subjects</p>
			<p class="mt-2 text-2xl font-semibold">{overview?.gdpr_subject_count ?? 0}</p>
		</div>
	</div>

	<div class="mt-5 grid gap-4 xl:grid-cols-[1.05fr_0.95fr]">
		<div class="rounded-2xl border border-stone-200 bg-stone-50/80 p-4">
			<div class="flex items-center justify-between gap-3">
				<p class="text-sm font-semibold text-stone-900">Collector health</p>
				<p class="text-xs uppercase tracking-[0.18em] text-stone-500">NATS subjects</p>
			</div>
			<div class="mt-3 space-y-3">
				{#each collectors as collector}
					<div class="rounded-2xl border border-stone-200 bg-white px-4 py-4">
						<div class="flex items-start justify-between gap-3">
							<div>
								<p class="font-medium text-stone-900">{collector.service_name}</p>
								<p class="text-sm text-stone-500">{collector.subject}</p>
							</div>
							<span class={`rounded-full px-3 py-1 text-xs font-semibold uppercase tracking-[0.18em] ${collector.connected ? 'bg-emerald-100 text-emerald-700' : 'bg-amber-100 text-amber-700'}`}>{collector.health}</span>
						</div>
						<div class="mt-3 flex flex-wrap gap-2 text-xs text-stone-600">
							<span class="rounded-full bg-stone-100 px-2 py-1">Backlog {collector.backlog_depth}</span>
							<span class="rounded-full bg-stone-100 px-2 py-1">Next pull {new Date(collector.next_pull_at).toLocaleTimeString()}</span>
						</div>
					</div>
				{/each}
			</div>
		</div>

		<div class="space-y-4 rounded-2xl border border-stone-200 bg-stone-950 p-4 text-stone-100">
			<div>
				<p class="text-sm font-semibold">Anomaly alerts</p>
				<div class="mt-3 space-y-3">
					{#each anomalies as anomaly}
						<div class="rounded-2xl border border-stone-700 bg-stone-900 px-4 py-4">
							<div class="flex items-start justify-between gap-3">
								<div>
									<p class="font-medium text-stone-100">{anomaly.title}</p>
									<p class="mt-1 text-sm text-stone-400">{anomaly.description}</p>
								</div>
								<span class="rounded-full bg-rose-300 px-3 py-1 text-xs font-semibold uppercase tracking-[0.18em] text-rose-950">{anomaly.severity}</span>
							</div>
							<p class="mt-3 text-xs text-stone-400">{anomaly.recommended_action}</p>
						</div>
					{/each}
				</div>
			</div>

			<div>
				<p class="text-sm font-semibold">Recent reports</p>
				<div class="mt-3 space-y-3">
					{#each reports.slice(0, 3) as report}
						<div class="rounded-2xl border border-stone-700 bg-stone-900 px-4 py-4">
							<div class="flex items-center justify-between gap-3">
								<div>
									<p class="font-medium text-stone-100">{report.title}</p>
									<p class="text-xs text-stone-400">{report.standard} • {report.scope}</p>
								</div>
								<span class="rounded-full bg-teal-300 px-3 py-1 text-xs font-semibold uppercase tracking-[0.18em] text-teal-950">{report.status}</span>
							</div>
							<p class="mt-3 text-xs text-stone-400">{report.control_summary}</p>
						</div>
					{/each}
				</div>
			</div>
		</div>
	</div>
</section>
