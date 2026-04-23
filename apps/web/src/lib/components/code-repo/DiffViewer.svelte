<script lang="ts">
	export let availableBranches: string[] = [];
	export let branchName = '';
	export let patch = '';
	export let busy = false;
	export let onSelectBranch: (branchName: string) => void;
</script>

<section class="rounded-3xl border border-stone-200 bg-white p-5 shadow-sm shadow-stone-200/60">
	<div class="flex flex-col gap-4 lg:flex-row lg:items-start lg:justify-between">
		<div>
			<p class="text-xs font-semibold uppercase tracking-[0.24em] text-rose-700">Diff Viewer</p>
			<h3 class="mt-2 text-xl font-semibold text-stone-900">Branch patch preview</h3>
			<p class="mt-1 text-sm text-stone-500">Inspect a synthetic patch against the selected branch head to review changed files before merge.</p>
		</div>
		<select class="rounded-full border border-stone-300 bg-white px-4 py-2 text-sm outline-none transition focus:border-rose-500" value={branchName} onchange={(event) => onSelectBranch((event.currentTarget as HTMLSelectElement).value)} disabled={busy}>
			{#each availableBranches as branch}
				<option value={branch}>{branch}</option>
			{/each}
		</select>
	</div>

	<pre class="mt-5 overflow-x-auto rounded-2xl border border-stone-200 bg-stone-950 p-4 text-xs text-rose-100">{patch || 'Select a repository branch to render its patch.'}</pre>
</section>
