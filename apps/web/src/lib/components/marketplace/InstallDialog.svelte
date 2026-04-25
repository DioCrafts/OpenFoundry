<script lang="ts">
	import type { ProductFleetRecord } from '$lib/api/marketplace';

	export let versions: string[] = [];
	export let version = '';
	export let workspaceName = '';
	export let releaseChannel = 'stable';
	export let fleetId = '';
	export let enrollmentBranch = '';
	export let fleets: ProductFleetRecord[] = [];
	export let busy = false;
	export let onVersionChange: (version: string) => void;
	export let onWorkspaceNameChange: (workspaceName: string) => void;
	export let onReleaseChannelChange: (releaseChannel: string) => void;
	export let onFleetChange: (fleetId: string) => void;
	export let onEnrollmentBranchChange: (branchName: string) => void;
	export let onInstall: () => void;

	function inputValue(event: Event) {
		return (event.currentTarget as HTMLInputElement).value;
	}
</script>

<div class="rounded-2xl border border-stone-200 bg-stone-50 p-4">
	<p class="text-xs font-semibold uppercase tracking-[0.2em] text-emerald-700">One-click install</p>
	<div class="mt-4 grid gap-4 md:grid-cols-2">
		<label class="block text-sm">
			<span class="mb-2 block font-medium text-stone-700">Version</span>
			<select class="w-full rounded-2xl border border-stone-300 bg-white px-4 py-3 outline-none transition focus:border-emerald-500" value={version} onchange={(event) => onVersionChange((event.currentTarget as HTMLSelectElement).value)}>
				{#each versions as option}
					<option value={option}>{option}</option>
				{/each}
			</select>
		</label>
		<label class="block text-sm">
			<span class="mb-2 block font-medium text-stone-700">Workspace</span>
			<input class="w-full rounded-2xl border border-stone-300 bg-white px-4 py-3 outline-none transition focus:border-emerald-500" value={workspaceName} oninput={(event) => onWorkspaceNameChange(inputValue(event))} />
		</label>
		<label class="block text-sm">
			<span class="mb-2 block font-medium text-stone-700">Release channel</span>
			<input class="w-full rounded-2xl border border-stone-300 bg-white px-4 py-3 outline-none transition focus:border-emerald-500" value={releaseChannel} oninput={(event) => onReleaseChannelChange(inputValue(event))} />
		</label>
		<label class="block text-sm">
			<span class="mb-2 block font-medium text-stone-700">Fleet</span>
			<select class="w-full rounded-2xl border border-stone-300 bg-white px-4 py-3 outline-none transition focus:border-emerald-500" value={fleetId} onchange={(event) => onFleetChange((event.currentTarget as HTMLSelectElement).value)}>
				<option value="">Direct install</option>
				{#each fleets as fleet}
					<option value={fleet.id}>{fleet.name} · {fleet.release_channel}</option>
				{/each}
			</select>
		</label>
		<label class="block text-sm md:col-span-2">
			<span class="mb-2 block font-medium text-stone-700">Enrollment branch</span>
			<input class="w-full rounded-2xl border border-stone-300 bg-white px-4 py-3 outline-none transition focus:border-emerald-500" value={enrollmentBranch} oninput={(event) => onEnrollmentBranchChange(inputValue(event))} placeholder="feature/ops-branch (optional)" />
		</label>
	</div>
	<button class="mt-4 rounded-full bg-emerald-500 px-4 py-2 text-sm font-semibold text-stone-950 transition hover:bg-emerald-400 disabled:cursor-not-allowed disabled:bg-emerald-200" onclick={onInstall} disabled={busy || versions.length === 0}>Install package</button>
</div>
