<script lang="ts">
	import type { InstallRecord } from '$lib/api/marketplace';

	export let installs: InstallRecord[] = [];
</script>

<section class="rounded-3xl border border-stone-200 bg-white p-5 shadow-sm shadow-stone-200/60">
	<div>
		<p class="text-xs font-semibold uppercase tracking-[0.24em] text-emerald-700">Installed Packages</p>
		<h3 class="mt-2 text-xl font-semibold text-stone-900">Workspace rollout history</h3>
		<p class="mt-1 text-sm text-stone-500">Track completed installs and their dependency plans across workspaces.</p>
	</div>

	<div class="mt-5 grid gap-3 lg:grid-cols-2">
		{#each installs as install}
			<div class="rounded-2xl border border-stone-200 bg-stone-50 px-4 py-4">
				<div class="flex items-center justify-between gap-3">
					<div>
						<p class="font-semibold text-stone-900">{install.listing_name}</p>
						<p class="text-sm text-stone-500">{install.workspace_name} • {install.version} • {install.release_channel}</p>
					</div>
					<span class="rounded-full bg-emerald-100 px-3 py-1 text-xs font-semibold uppercase tracking-[0.18em] text-emerald-700">{install.status}</span>
				</div>
				<div class="mt-3 flex flex-wrap gap-2 text-xs text-stone-600">
					{#if install.fleet_name}
						<span class="rounded-full bg-white px-2 py-1">Fleet: {install.fleet_name}</span>
					{/if}
					{#if install.enrollment_branch}
						<span class="rounded-full bg-white px-2 py-1">Branch: {install.enrollment_branch}</span>
					{/if}
					{#if install.auto_upgrade_enabled}
						<span class="rounded-full bg-white px-2 py-1">Auto-upgrade</span>
					{/if}
					{#if install.maintenance_window}
						<span class="rounded-full bg-white px-2 py-1">Window: {install.maintenance_window.days.join(', ')} {install.maintenance_window.start_hour_utc}:00 UTC</span>
					{/if}
				</div>
				<div class="mt-3 flex flex-wrap gap-2">
					{#each install.dependency_plan as dependency}
						<span class="rounded-full bg-white px-2 py-1 text-xs text-stone-600">{dependency.package_slug} {dependency.version_req}</span>
					{/each}
				</div>
				{#if install.activation}
					<div class="mt-3 rounded-2xl border border-emerald-200 bg-white px-3 py-3 text-sm text-stone-600">
						<div class="font-medium text-stone-900">Activation</div>
						<div class="mt-1">{install.activation.kind} • {install.activation.status}</div>
						{#if install.activation.public_url}
							<a class="mt-2 inline-flex text-emerald-700 hover:text-emerald-800" href={install.activation.public_url}>
								Open runtime
							</a>
						{/if}
						{#if install.activation.notes}
							<p class="mt-2 text-xs text-stone-500">{install.activation.notes}</p>
						{/if}
					</div>
				{/if}
			</div>
		{/each}
	</div>
</section>
