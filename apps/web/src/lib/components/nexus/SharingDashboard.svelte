<script lang="ts">
	import type { AuditBridgeSummary, NexusOverview, ReplicationPlan } from '$lib/api/nexus';

	export let overview: NexusOverview | null = null;
	export let auditBridge: AuditBridgeSummary | null = null;
	export let replicationPlans: ReplicationPlan[] = [];
</script>

<section class="rounded-3xl border border-stone-200 bg-white p-5 shadow-sm shadow-stone-200/60">
	<div>
		<p class="text-xs font-semibold uppercase tracking-[0.24em] text-cyan-700">Nexus Dashboard</p>
		<h2 class="mt-2 text-2xl font-semibold text-stone-900">Cross-org trust, contracts, replication, and audit exchange</h2>
		<p class="mt-1 text-sm text-stone-600">Monitor partner authentication, data sharing posture, selective replication readiness, and cross-org audit cursors.</p>
	</div>

	<div class="mt-5 grid gap-4 sm:grid-cols-2 xl:grid-cols-6">
		<div class="rounded-2xl bg-stone-950 px-4 py-4 text-stone-50">
			<p class="text-xs uppercase tracking-[0.18em] text-cyan-300">Peers</p>
			<p class="mt-2 text-2xl font-semibold">{overview?.peer_count ?? 0}</p>
		</div>
		<div class="rounded-2xl bg-emerald-50 px-4 py-4 text-emerald-900">
			<p class="text-xs uppercase tracking-[0.18em] text-emerald-600">Authenticated</p>
			<p class="mt-2 text-2xl font-semibold">{overview?.active_peer_count ?? 0}</p>
		</div>
		<div class="rounded-2xl bg-sky-50 px-4 py-4 text-sky-900">
			<p class="text-xs uppercase tracking-[0.18em] text-sky-600">Contracts</p>
			<p class="mt-2 text-2xl font-semibold">{overview?.contract_count ?? 0}</p>
		</div>
		<div class="rounded-2xl bg-fuchsia-50 px-4 py-4 text-fuchsia-900">
			<p class="text-xs uppercase tracking-[0.18em] text-fuchsia-600">Shares</p>
			<p class="mt-2 text-2xl font-semibold">{overview?.share_count ?? 0}</p>
		</div>
		<div class="rounded-2xl bg-amber-50 px-4 py-4 text-amber-900">
			<p class="text-xs uppercase tracking-[0.18em] text-amber-600">Replication Ready</p>
			<p class="mt-2 text-2xl font-semibold">{overview?.replication_ready_count ?? 0}</p>
		</div>
		<div class="rounded-2xl bg-violet-50 px-4 py-4 text-violet-900">
			<p class="text-xs uppercase tracking-[0.18em] text-violet-600">Encrypted</p>
			<p class="mt-2 text-2xl font-semibold">{overview?.encrypted_share_count ?? 0}</p>
		</div>
	</div>

	<div class="mt-5 grid gap-4 xl:grid-cols-[1fr_1fr]">
		<div class="rounded-2xl border border-stone-200 bg-stone-50/80 p-4">
			<div class="flex items-center justify-between gap-3">
				<p class="text-sm font-semibold text-stone-900">Audit bridge</p>
				<span class={`rounded-full px-3 py-1 text-xs font-semibold uppercase tracking-[0.18em] ${auditBridge?.bridge_status === 'healthy' ? 'bg-emerald-100 text-emerald-700' : 'bg-amber-100 text-amber-700'}`}>{auditBridge?.bridge_status ?? 'pending'}</span>
			</div>
			<div class="mt-3 space-y-3">
				{#each auditBridge?.entries ?? [] as entry}
					<div class="rounded-2xl border border-stone-200 bg-white px-4 py-4">
						<div class="flex items-start justify-between gap-3">
							<div>
								<p class="font-medium text-stone-900">{entry.dataset_name}</p>
								<p class="text-sm text-stone-500">{entry.peer_name} • {entry.contract_name}</p>
							</div>
							<span class="rounded-full bg-stone-100 px-3 py-1 text-xs font-semibold uppercase tracking-[0.18em] text-stone-700">{entry.status}</span>
						</div>
						<p class="mt-3 font-mono text-xs text-stone-500">{entry.audit_cursor}</p>
					</div>
				{/each}
			</div>
		</div>

		<div class="rounded-2xl border border-stone-200 bg-stone-950 p-4 text-stone-100">
			<p class="text-sm font-semibold">Selective replication plans</p>
			<div class="mt-3 space-y-3">
				{#each replicationPlans as plan}
					<div class="rounded-2xl border border-stone-700 bg-stone-900 px-4 py-4">
						<div class="flex items-start justify-between gap-3">
							<div>
								<p class="font-medium text-stone-100">{plan.dataset_name}</p>
								<p class="text-sm text-stone-400">{plan.mode} • backlog {plan.backlog_rows}</p>
							</div>
							<span class={`rounded-full px-3 py-1 text-xs font-semibold uppercase tracking-[0.18em] ${plan.encrypted ? 'bg-cyan-300 text-cyan-950' : 'bg-rose-300 text-rose-950'}`}>{plan.status}</span>
						</div>
						<p class="mt-3 text-xs text-stone-400">Filter {JSON.stringify(plan.selective_filter)}</p>
					</div>
				{/each}
			</div>
		</div>
	</div>
</section>
