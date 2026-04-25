<script lang="ts">
	import type { EnrollmentBranchRecord, ProductFleetRecord } from '$lib/api/marketplace';

	type FleetDraft = {
		name: string;
		environment: string;
		workspace_targets_text: string;
		release_channel: string;
		auto_upgrade_enabled: boolean;
		maintenance_days_text: string;
		start_hour_utc: string;
		duration_minutes: string;
		branch_strategy: string;
		rollout_strategy: string;
	};

	type BranchDraft = {
		fleet_id: string;
		name: string;
		repository_branch: string;
		notes: string;
	};

	let {
		fleets = [],
		branches = [],
		selectedListingId = '',
		busy = false,
		fleetDraft,
		branchDraft,
		onFleetDraftChange,
		onBranchDraftChange,
		onCreateFleet,
		onCreateBranch,
		onSyncFleet,
	}: {
		fleets?: ProductFleetRecord[];
		branches?: EnrollmentBranchRecord[];
		selectedListingId?: string;
		busy?: boolean;
		fleetDraft: FleetDraft;
		branchDraft: BranchDraft;
		onFleetDraftChange: (patch: Partial<FleetDraft>) => void;
		onBranchDraftChange: (patch: Partial<BranchDraft>) => void;
		onCreateFleet: () => void;
		onCreateBranch: () => void;
		onSyncFleet: (fleetId: string) => void;
	} = $props();

	function inputValue(event: Event) {
		return (event.currentTarget as HTMLInputElement).value;
	}

	function textValue(event: Event) {
		return (event.currentTarget as HTMLTextAreaElement).value;
	}

	function boolValue(event: Event) {
		return (event.currentTarget as HTMLInputElement).checked;
	}

	const selectedListingFleets = $derived(
		selectedListingId ? fleets.filter((fleet) => fleet.listing_id === selectedListingId) : fleets,
	);
</script>

<section class="rounded-3xl border border-slate-200 bg-white p-5 shadow-sm shadow-slate-200/60">
	<div class="flex flex-col gap-4 lg:flex-row lg:items-start lg:justify-between">
		<div>
			<p class="text-xs font-semibold uppercase tracking-[0.24em] text-emerald-700">Foundry DevOps</p>
			<h3 class="mt-2 text-xl font-semibold text-slate-900">Fleet rollout, maintenance windows, and enrollment branches</h3>
			<p class="mt-1 text-sm text-slate-500">Package product resources, target release channels, orchestrate upgrades across fleets, and open feature branches at the enrollment layer.</p>
		</div>
		<div class="grid grid-cols-3 gap-3">
			<div class="rounded-2xl bg-slate-950 px-4 py-3 text-white">
				<p class="text-xs uppercase tracking-[0.18em] text-emerald-200">Fleets</p>
				<p class="mt-2 text-2xl font-semibold">{fleets.length}</p>
			</div>
			<div class="rounded-2xl bg-emerald-50 px-4 py-3 text-emerald-900">
				<p class="text-xs uppercase tracking-[0.18em] text-emerald-600">Branches</p>
				<p class="mt-2 text-2xl font-semibold">{branches.length}</p>
			</div>
			<div class="rounded-2xl bg-amber-50 px-4 py-3 text-amber-900">
				<p class="text-xs uppercase tracking-[0.18em] text-amber-600">Pending</p>
				<p class="mt-2 text-2xl font-semibold">{fleets.reduce((total, fleet) => total + fleet.pending_upgrade_count, 0)}</p>
			</div>
		</div>
	</div>

	<div class="mt-6 grid gap-6 xl:grid-cols-[1.05fr_0.95fr]">
		<div class="space-y-4 rounded-2xl border border-slate-200 bg-slate-50/80 p-4">
			<div class="flex items-center justify-between gap-3">
				<div>
					<p class="text-xs font-semibold uppercase tracking-[0.18em] text-slate-500">Product fleets</p>
					<p class="mt-1 text-sm text-slate-500">Each fleet can auto-track a release channel and enforce a maintenance window.</p>
				</div>
			</div>
			<div class="space-y-3">
				{#each selectedListingFleets as fleet}
					<div class="rounded-2xl border border-slate-200 bg-white px-4 py-4">
						<div class="flex flex-col gap-3 lg:flex-row lg:items-start lg:justify-between">
							<div>
								<p class="font-semibold text-slate-900">{fleet.name}</p>
								<p class="text-sm text-slate-500">{fleet.environment} · channel {fleet.release_channel} · {fleet.workspace_targets.length} workspaces</p>
							</div>
							<div class="flex flex-wrap gap-2">
								<span class="rounded-full bg-emerald-100 px-3 py-1 text-xs font-semibold uppercase tracking-[0.18em] text-emerald-700">{fleet.status}</span>
								<button class="rounded-full border border-emerald-300 px-3 py-1 text-xs font-semibold text-emerald-700 transition hover:bg-emerald-50 disabled:cursor-not-allowed disabled:border-emerald-100 disabled:text-emerald-300" onclick={() => onSyncFleet(fleet.id)} disabled={busy}>Sync fleet</button>
							</div>
						</div>
						<div class="mt-3 flex flex-wrap gap-2 text-xs text-slate-600">
							<span class="rounded-full bg-slate-100 px-2 py-1">Current: {fleet.current_version ?? 'none'}</span>
							<span class="rounded-full bg-slate-100 px-2 py-1">Target: {fleet.target_version ?? 'none'}</span>
							<span class="rounded-full bg-slate-100 px-2 py-1">Pending: {fleet.pending_upgrade_count}</span>
							<span class="rounded-full bg-slate-100 px-2 py-1">Cells: {fleet.deployment_cells.length}</span>
							<span class="rounded-full bg-slate-100 px-2 py-1">Gates passed: {fleet.promotion_gate_summary.passed}/{fleet.promotion_gate_summary.total}</span>
							{#if fleet.promotion_gate_summary.blocking > 0}
								<span class="rounded-full bg-rose-100 px-2 py-1 text-rose-700">Blocking gates: {fleet.promotion_gate_summary.blocking}</span>
							{/if}
							<span class="rounded-full bg-slate-100 px-2 py-1">{fleet.auto_upgrade_enabled ? 'Auto-upgrade on' : 'Manual upgrades'}</span>
							<span class="rounded-full bg-slate-100 px-2 py-1">Window: {fleet.maintenance_window.days.join(', ')} {fleet.maintenance_window.start_hour_utc}:00 UTC</span>
						</div>
						<div class="mt-3 flex flex-wrap gap-2">
							{#each fleet.workspace_targets as workspace}
								<span class="rounded-full bg-white px-2 py-1 text-xs text-slate-600">{workspace}</span>
							{/each}
						</div>
						{#if fleet.deployment_cells.length > 0}
							<div class="mt-3 flex flex-wrap gap-2 text-xs text-slate-600">
								{#each fleet.deployment_cells as cell}
									<span class="rounded-full bg-white px-2 py-1">{cell.name} · {cell.cloud}/{cell.region} · {cell.status}</span>
								{/each}
							</div>
						{/if}
					</div>
				{/each}
				{#if selectedListingFleets.length === 0}
					<div class="rounded-2xl border border-dashed border-slate-300 bg-white px-4 py-8 text-center text-sm text-slate-500">No fleets yet for the selected listing.</div>
				{/if}
			</div>
		</div>

		<div class="space-y-4 rounded-2xl border border-slate-200 bg-slate-950 p-4 text-white">
			<div>
				<p class="text-xs font-semibold uppercase tracking-[0.18em] text-emerald-300">Create fleet</p>
				<div class="mt-4 grid gap-4 md:grid-cols-2">
					<label class="block text-sm md:col-span-2">
						<span class="mb-2 block font-medium text-white">Fleet name</span>
						<input class="w-full rounded-2xl border border-slate-700 bg-slate-900 px-4 py-3 outline-none transition focus:border-emerald-400" value={fleetDraft.name} oninput={(event) => onFleetDraftChange({ name: inputValue(event) })} />
					</label>
					<label class="block text-sm">
						<span class="mb-2 block font-medium text-white">Environment</span>
						<input class="w-full rounded-2xl border border-slate-700 bg-slate-900 px-4 py-3 outline-none transition focus:border-emerald-400" value={fleetDraft.environment} oninput={(event) => onFleetDraftChange({ environment: inputValue(event) })} />
					</label>
					<label class="block text-sm">
						<span class="mb-2 block font-medium text-white">Release channel</span>
						<input class="w-full rounded-2xl border border-slate-700 bg-slate-900 px-4 py-3 outline-none transition focus:border-emerald-400" value={fleetDraft.release_channel} oninput={(event) => onFleetDraftChange({ release_channel: inputValue(event) })} />
					</label>
					<label class="block text-sm md:col-span-2">
						<span class="mb-2 block font-medium text-white">Workspace targets</span>
						<input class="w-full rounded-2xl border border-slate-700 bg-slate-900 px-4 py-3 outline-none transition focus:border-emerald-400" value={fleetDraft.workspace_targets_text} oninput={(event) => onFleetDraftChange({ workspace_targets_text: inputValue(event) })} placeholder="Ops Center - EU, Ops Center - US" />
					</label>
					<label class="inline-flex items-center gap-3 rounded-2xl border border-slate-700 bg-slate-900 px-4 py-3 text-sm text-slate-200 md:col-span-2">
						<input type="checkbox" checked={fleetDraft.auto_upgrade_enabled} onchange={(event) => onFleetDraftChange({ auto_upgrade_enabled: boolValue(event) })} />
						<span>Enable auto-upgrade during maintenance windows</span>
					</label>
					<label class="block text-sm">
						<span class="mb-2 block font-medium text-white">Maintenance days</span>
						<input class="w-full rounded-2xl border border-slate-700 bg-slate-900 px-4 py-3 outline-none transition focus:border-emerald-400" value={fleetDraft.maintenance_days_text} oninput={(event) => onFleetDraftChange({ maintenance_days_text: inputValue(event) })} placeholder="sun, wed" />
					</label>
					<label class="block text-sm">
						<span class="mb-2 block font-medium text-white">Start hour UTC</span>
						<input class="w-full rounded-2xl border border-slate-700 bg-slate-900 px-4 py-3 outline-none transition focus:border-emerald-400" value={fleetDraft.start_hour_utc} oninput={(event) => onFleetDraftChange({ start_hour_utc: inputValue(event) })} />
					</label>
					<label class="block text-sm">
						<span class="mb-2 block font-medium text-white">Duration minutes</span>
						<input class="w-full rounded-2xl border border-slate-700 bg-slate-900 px-4 py-3 outline-none transition focus:border-emerald-400" value={fleetDraft.duration_minutes} oninput={(event) => onFleetDraftChange({ duration_minutes: inputValue(event) })} />
					</label>
					<label class="block text-sm">
						<span class="mb-2 block font-medium text-white">Branch strategy</span>
						<input class="w-full rounded-2xl border border-slate-700 bg-slate-900 px-4 py-3 outline-none transition focus:border-emerald-400" value={fleetDraft.branch_strategy} oninput={(event) => onFleetDraftChange({ branch_strategy: inputValue(event) })} />
					</label>
					<label class="block text-sm md:col-span-2">
						<span class="mb-2 block font-medium text-white">Rollout strategy</span>
						<input class="w-full rounded-2xl border border-slate-700 bg-slate-900 px-4 py-3 outline-none transition focus:border-emerald-400" value={fleetDraft.rollout_strategy} oninput={(event) => onFleetDraftChange({ rollout_strategy: inputValue(event) })} />
					</label>
				</div>
				<button class="mt-4 rounded-full bg-emerald-500 px-4 py-2 text-sm font-semibold text-slate-950 transition hover:bg-emerald-400 disabled:cursor-not-allowed disabled:bg-emerald-200" onclick={onCreateFleet} disabled={busy || !selectedListingId}>Create fleet for selected listing</button>
			</div>

			<div class="border-t border-slate-800 pt-4">
				<p class="text-xs font-semibold uppercase tracking-[0.18em] text-amber-300">Enrollment branch</p>
				<div class="mt-4 grid gap-4">
					<label class="block text-sm">
						<span class="mb-2 block font-medium text-white">Fleet</span>
						<select class="w-full rounded-2xl border border-slate-700 bg-slate-900 px-4 py-3 outline-none transition focus:border-amber-400" value={branchDraft.fleet_id} onchange={(event) => onBranchDraftChange({ fleet_id: (event.currentTarget as HTMLSelectElement).value })}>
							<option value="">Select fleet</option>
							{#each fleets as fleet}
								<option value={fleet.id}>{fleet.name} · {fleet.listing_name}</option>
							{/each}
						</select>
					</label>
					<label class="block text-sm">
						<span class="mb-2 block font-medium text-white">Branch name</span>
						<input class="w-full rounded-2xl border border-slate-700 bg-slate-900 px-4 py-3 outline-none transition focus:border-amber-400" value={branchDraft.name} oninput={(event) => onBranchDraftChange({ name: inputValue(event) })} />
					</label>
					<label class="block text-sm">
						<span class="mb-2 block font-medium text-white">Repository branch override</span>
						<input class="w-full rounded-2xl border border-slate-700 bg-slate-900 px-4 py-3 outline-none transition focus:border-amber-400" value={branchDraft.repository_branch} oninput={(event) => onBranchDraftChange({ repository_branch: inputValue(event) })} placeholder="release/ops-center/feature-x" />
					</label>
					<label class="block text-sm">
						<span class="mb-2 block font-medium text-white">Notes</span>
						<textarea class="min-h-24 w-full rounded-2xl border border-slate-700 bg-slate-900 px-4 py-3 outline-none transition focus:border-amber-400" oninput={(event) => onBranchDraftChange({ notes: textValue(event) })}>{branchDraft.notes}</textarea>
					</label>
				</div>
				<button class="mt-4 rounded-full bg-amber-400 px-4 py-2 text-sm font-semibold text-slate-950 transition hover:bg-amber-300 disabled:cursor-not-allowed disabled:bg-amber-200" onclick={onCreateBranch} disabled={busy || !branchDraft.fleet_id}>Create enrollment branch</button>
			</div>
		</div>
	</div>

	<div class="mt-6 rounded-2xl border border-slate-200 bg-slate-50/80 p-4">
		<p class="text-xs font-semibold uppercase tracking-[0.18em] text-slate-500">Active enrollment branches</p>
		<div class="mt-3 grid gap-3 lg:grid-cols-2">
			{#each branches as branch}
				<div class="rounded-2xl border border-slate-200 bg-white px-4 py-4">
					<div class="flex items-start justify-between gap-3">
						<div>
							<p class="font-semibold text-slate-900">{branch.name}</p>
							<p class="text-sm text-slate-500">{branch.fleet_name} · {branch.source_release_channel} · {branch.source_version ?? 'no version yet'}</p>
						</div>
						<span class="rounded-full bg-amber-100 px-3 py-1 text-xs font-semibold uppercase tracking-[0.18em] text-amber-700">{branch.status}</span>
					</div>
					<p class="mt-3 text-sm text-slate-600">{branch.repository_branch}</p>
					{#if branch.notes}
						<p class="mt-2 text-sm text-slate-500">{branch.notes}</p>
					{/if}
					<div class="mt-3 flex flex-wrap gap-2">
						{#each branch.workspace_targets as workspace}
							<span class="rounded-full bg-slate-100 px-2 py-1 text-xs text-slate-600">{workspace}</span>
						{/each}
					</div>
				</div>
			{/each}
			{#if branches.length === 0}
				<div class="rounded-2xl border border-dashed border-slate-300 bg-white px-4 py-8 text-center text-sm text-slate-500 lg:col-span-2">No enrollment branches created yet.</div>
			{/if}
		</div>
	</div>
</section>
