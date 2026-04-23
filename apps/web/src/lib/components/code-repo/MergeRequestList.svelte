<script lang="ts">
	import type { MergeRequestDefinition } from '$lib/api/code-repos';

	type MergeRequestDraft = {
		title: string;
		description: string;
		source_branch: string;
		target_branch: string;
		author: string;
		labels_text: string;
		reviewers_text: string;
		approvals_required: string;
		changed_files: string;
	};

	export let mergeRequests: MergeRequestDefinition[] = [];
	export let selectedMergeRequestId = '';
	export let branchOptions: string[] = [];
	export let draft: MergeRequestDraft;
	export let busy = false;
	export let onSelectMergeRequest: (mergeRequestId: string) => void;
	export let onDraftChange: (patch: Partial<MergeRequestDraft>) => void;
	export let onCreateMergeRequest: () => void;

	function inputValue(event: Event) {
		return (event.currentTarget as HTMLInputElement).value;
	}

	function textValue(event: Event) {
		return (event.currentTarget as HTMLTextAreaElement).value;
	}
</script>

<section class="rounded-3xl border border-stone-200 bg-white p-5 shadow-sm shadow-stone-200/60">
	<div class="flex flex-col gap-4 lg:flex-row lg:items-start lg:justify-between">
		<div>
			<p class="text-xs font-semibold uppercase tracking-[0.24em] text-fuchsia-700">Merge Requests</p>
			<h3 class="mt-2 text-xl font-semibold text-stone-900">Review queues, labels, and approvals</h3>
			<p class="mt-1 text-sm text-stone-500">Open review flows between feature and target branches, then select one to inspect comments and status.</p>
		</div>
		<button class="rounded-full bg-fuchsia-600 px-4 py-2 text-sm font-semibold text-white transition hover:bg-fuchsia-700 disabled:cursor-not-allowed disabled:bg-fuchsia-300" onclick={onCreateMergeRequest} disabled={busy}>Create MR</button>
	</div>

	<div class="mt-5 grid gap-4 xl:grid-cols-[0.92fr_1.08fr]">
		<div class="space-y-3">
			{#each mergeRequests as mergeRequest}
				<button class={`w-full rounded-2xl border px-4 py-4 text-left transition ${selectedMergeRequestId === mergeRequest.id ? 'border-fuchsia-500 bg-fuchsia-50' : 'border-stone-200 bg-stone-50 hover:border-fuchsia-300 hover:bg-fuchsia-50/60'}`} onclick={() => onSelectMergeRequest(mergeRequest.id)}>
					<div class="flex items-start justify-between gap-3">
						<div>
							<p class="font-semibold text-stone-900">{mergeRequest.title}</p>
							<p class="text-sm text-stone-500">{mergeRequest.source_branch} → {mergeRequest.target_branch}</p>
						</div>
						<span class="rounded-full bg-fuchsia-100 px-3 py-1 text-xs font-semibold uppercase tracking-[0.18em] text-fuchsia-700">{mergeRequest.status}</span>
					</div>
					<div class="mt-3 flex flex-wrap gap-2">
						{#each mergeRequest.labels as label}
							<span class="rounded-full bg-white px-2 py-1 text-xs text-stone-600">{label}</span>
						{/each}
					</div>
				</button>
			{/each}
		</div>

		<div class="rounded-2xl border border-stone-200 bg-stone-950 p-4 text-stone-100">
			<p class="text-xs font-semibold uppercase tracking-[0.2em] text-fuchsia-300">New merge request</p>
			<div class="mt-4 grid gap-4 md:grid-cols-2">
				<label class="block text-sm md:col-span-2">
					<span class="mb-2 block font-medium text-stone-100">Title</span>
					<input class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-fuchsia-400" value={draft.title} oninput={(event) => onDraftChange({ title: inputValue(event) })} />
				</label>
				<label class="block text-sm md:col-span-2">
					<span class="mb-2 block font-medium text-stone-100">Description</span>
					<textarea class="min-h-24 w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-fuchsia-400" oninput={(event) => onDraftChange({ description: textValue(event) })}>{draft.description}</textarea>
				</label>
				<label class="block text-sm">
					<span class="mb-2 block font-medium text-stone-100">Source</span>
					<select class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-fuchsia-400" value={draft.source_branch} onchange={(event) => onDraftChange({ source_branch: (event.currentTarget as HTMLSelectElement).value })}>
						{#each branchOptions as branch}
							<option value={branch}>{branch}</option>
						{/each}
					</select>
				</label>
				<label class="block text-sm">
					<span class="mb-2 block font-medium text-stone-100">Target</span>
					<select class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-fuchsia-400" value={draft.target_branch} onchange={(event) => onDraftChange({ target_branch: (event.currentTarget as HTMLSelectElement).value })}>
						{#each branchOptions as branch}
							<option value={branch}>{branch}</option>
						{/each}
					</select>
				</label>
				<label class="block text-sm">
					<span class="mb-2 block font-medium text-stone-100">Author</span>
					<input class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-fuchsia-400" value={draft.author} oninput={(event) => onDraftChange({ author: inputValue(event) })} />
				</label>
				<label class="block text-sm">
					<span class="mb-2 block font-medium text-stone-100">Reviewers</span>
					<input class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-fuchsia-400" value={draft.reviewers_text} oninput={(event) => onDraftChange({ reviewers_text: inputValue(event) })} placeholder="Elena, Marco" />
				</label>
				<label class="block text-sm md:col-span-2">
					<span class="mb-2 block font-medium text-stone-100">Labels</span>
					<input class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-fuchsia-400" value={draft.labels_text} oninput={(event) => onDraftChange({ labels_text: inputValue(event) })} placeholder="preview, widget, release" />
				</label>
				<label class="block text-sm">
					<span class="mb-2 block font-medium text-stone-100">Approvals required</span>
					<input class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-fuchsia-400" value={draft.approvals_required} oninput={(event) => onDraftChange({ approvals_required: inputValue(event) })} />
				</label>
				<label class="block text-sm">
					<span class="mb-2 block font-medium text-stone-100">Changed files</span>
					<input class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-fuchsia-400" value={draft.changed_files} oninput={(event) => onDraftChange({ changed_files: inputValue(event) })} />
				</label>
			</div>
		</div>
	</div>
</section>
