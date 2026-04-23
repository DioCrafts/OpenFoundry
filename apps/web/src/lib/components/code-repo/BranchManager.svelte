<script lang="ts">
	import type { BranchDefinition } from '$lib/api/code-repos';

	type BranchDraft = {
		name: string;
		base_branch: string;
		protected: boolean;
	};

	export let branches: BranchDefinition[] = [];
	export let draft: BranchDraft;
	export let busy = false;
	export let onDraftChange: (patch: Partial<BranchDraft>) => void;
	export let onCreateBranch: () => void;

	function inputValue(event: Event) {
		return (event.currentTarget as HTMLInputElement).value;
	}

	function boolValue(event: Event) {
		return (event.currentTarget as HTMLInputElement).checked;
	}
</script>

<section class="rounded-3xl border border-stone-200 bg-white p-5 shadow-sm shadow-stone-200/60">
	<div>
		<p class="text-xs font-semibold uppercase tracking-[0.24em] text-emerald-700">Branches</p>
		<h3 class="mt-2 text-xl font-semibold text-stone-900">Protected bases and feature streams</h3>
		<p class="mt-1 text-sm text-stone-500">Create branches off the default base and watch review pressure accumulate branch by branch.</p>
	</div>

	<div class="mt-5 grid gap-4 xl:grid-cols-[0.92fr_1.08fr]">
		<div class="space-y-3">
			{#each branches as branch}
				<div class="rounded-2xl border border-stone-200 bg-stone-50 px-4 py-4">
					<div class="flex items-center justify-between gap-3">
						<div>
							<p class="font-semibold text-stone-900">{branch.name}</p>
							<p class="text-sm text-stone-500">Head {branch.head_sha} • base {branch.base_branch ?? 'none'}</p>
						</div>
						<span class={`rounded-full px-3 py-1 text-xs font-semibold uppercase tracking-[0.18em] ${branch.is_default ? 'bg-stone-950 text-stone-50' : 'bg-emerald-100 text-emerald-700'}`}>
							{branch.is_default ? 'Default' : 'Feature'}
						</span>
					</div>
					<div class="mt-3 flex flex-wrap gap-2 text-xs text-stone-600">
						<span class="rounded-full bg-white px-2 py-1">Ahead {branch.ahead_by}</span>
						<span class="rounded-full bg-white px-2 py-1">Pending reviews {branch.pending_reviews}</span>
						<span class="rounded-full bg-white px-2 py-1">{branch.protected ? 'Protected' : 'Writable'}</span>
					</div>
				</div>
			{/each}
		</div>

		<div class="rounded-2xl border border-stone-200 bg-stone-950 p-4 text-stone-100">
			<p class="text-xs font-semibold uppercase tracking-[0.2em] text-emerald-300">Create branch</p>
			<div class="mt-4 grid gap-4 md:grid-cols-2">
				<label class="block text-sm md:col-span-2">
					<span class="mb-2 block font-medium text-stone-100">Branch name</span>
					<input class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-emerald-400" value={draft.name} oninput={(event) => onDraftChange({ name: inputValue(event) })} />
				</label>
				<label class="block text-sm">
					<span class="mb-2 block font-medium text-stone-100">Base branch</span>
					<input class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-emerald-400" value={draft.base_branch} oninput={(event) => onDraftChange({ base_branch: inputValue(event) })} />
				</label>
				<label class="inline-flex items-center gap-3 rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 text-sm text-stone-200">
					<input type="checkbox" checked={draft.protected} onchange={(event) => onDraftChange({ protected: boolValue(event) })} />
					<span>Protected branch</span>
				</label>
			</div>
			<button class="mt-4 rounded-full bg-emerald-500 px-4 py-2 text-sm font-semibold text-stone-950 transition hover:bg-emerald-400 disabled:cursor-not-allowed disabled:bg-emerald-200" onclick={onCreateBranch} disabled={busy}>Create branch</button>
		</div>
	</div>
</section>
