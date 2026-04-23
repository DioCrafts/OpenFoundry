<script lang="ts">
	import type { MergeRequestDetail as MergeRequestDetailModel, MergeRequestStatus } from '$lib/api/code-repos';

	type CommentDraft = {
		author: string;
		body: string;
		file_path: string;
		line_number: string;
		resolved: boolean;
	};

	export let detail: MergeRequestDetailModel | null = null;
	export let draft: CommentDraft;
	export let busy = false;
	export let onDraftChange: (patch: Partial<CommentDraft>) => void;
	export let onCreateComment: () => void;
	export let onStatusChange: (status: MergeRequestStatus) => void;

	const statuses: MergeRequestStatus[] = ['open', 'approved', 'merged', 'closed'];

	function inputValue(event: Event) {
		return (event.currentTarget as HTMLInputElement).value;
	}

	function textValue(event: Event) {
		return (event.currentTarget as HTMLTextAreaElement).value;
	}

	function boolValue(event: Event) {
		return (event.currentTarget as HTMLInputElement).checked;
	}
</script>

<section class="rounded-3xl border border-stone-200 bg-white p-5 shadow-sm shadow-stone-200/60">
	<div class="flex flex-col gap-4 lg:flex-row lg:items-start lg:justify-between">
		<div>
			<p class="text-xs font-semibold uppercase tracking-[0.24em] text-indigo-700">Review Detail</p>
			<h3 class="mt-2 text-xl font-semibold text-stone-900">Inline comments, approvals, and lifecycle state</h3>
			<p class="mt-1 text-sm text-stone-500">Track thread count and advance the merge request through approval or merge states.</p>
		</div>
		<div class="flex flex-wrap gap-2">
			{#each statuses as status}
				<button class={`rounded-full px-4 py-2 text-sm font-medium transition ${detail?.merge_request.status === status ? 'bg-indigo-600 text-white' : 'border border-indigo-200 bg-white text-indigo-700 hover:bg-indigo-50'}`} onclick={() => onStatusChange(status)} disabled={busy || !detail}>{status}</button>
			{/each}
		</div>
	</div>

	{#if detail}
		<div class="mt-5 grid gap-4 xl:grid-cols-[0.98fr_1.02fr]">
			<div class="space-y-4 rounded-2xl border border-stone-200 bg-stone-50/80 p-4">
				<div class="rounded-2xl border border-stone-200 bg-white px-4 py-4">
					<p class="text-lg font-semibold text-stone-900">{detail.merge_request.title}</p>
					<p class="mt-1 text-sm text-stone-500">{detail.merge_request.source_branch} → {detail.merge_request.target_branch} • {detail.merge_request.author}</p>
					<p class="mt-3 text-sm text-stone-700">{detail.merge_request.description}</p>
					<div class="mt-4 flex flex-wrap gap-2">
						<span class="rounded-full bg-indigo-100 px-2 py-1 text-xs font-semibold text-indigo-700">Approvals {detail.approval_count}/{detail.merge_request.approvals_required}</span>
						<span class="rounded-full bg-stone-100 px-2 py-1 text-xs font-semibold text-stone-700">Threads {detail.thread_count}</span>
						<span class="rounded-full bg-stone-100 px-2 py-1 text-xs font-semibold text-stone-700">Changed files {detail.merge_request.changed_files}</span>
					</div>
				</div>

				<div class="space-y-3">
					{#each detail.comments as comment}
						<div class="rounded-2xl border border-stone-200 bg-white px-4 py-4">
							<div class="flex items-center justify-between gap-3">
								<div>
									<p class="font-medium text-stone-900">{comment.author}</p>
									<p class="text-xs text-stone-500">{comment.file_path || 'general comment'}{#if comment.line_number !== null} • line {comment.line_number}{/if}</p>
								</div>
								<span class={`rounded-full px-3 py-1 text-xs font-semibold uppercase tracking-[0.18em] ${comment.resolved ? 'bg-emerald-100 text-emerald-700' : 'bg-amber-100 text-amber-700'}`}>{comment.resolved ? 'Resolved' : 'Open'}</span>
							</div>
							<p class="mt-3 text-sm text-stone-700">{comment.body}</p>
						</div>
					{/each}
				</div>
			</div>

			<div class="rounded-2xl border border-stone-200 bg-stone-950 p-4 text-stone-100">
				<p class="text-xs font-semibold uppercase tracking-[0.2em] text-indigo-300">Add inline comment</p>
				<div class="mt-4 grid gap-4 md:grid-cols-2">
					<label class="block text-sm">
						<span class="mb-2 block font-medium text-stone-100">Author</span>
						<input class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-indigo-400" value={draft.author} oninput={(event) => onDraftChange({ author: inputValue(event) })} />
					</label>
					<label class="block text-sm">
						<span class="mb-2 block font-medium text-stone-100">File path</span>
						<input class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-indigo-400" value={draft.file_path} oninput={(event) => onDraftChange({ file_path: inputValue(event) })} />
					</label>
					<label class="block text-sm">
						<span class="mb-2 block font-medium text-stone-100">Line number</span>
						<input class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-indigo-400" value={draft.line_number} oninput={(event) => onDraftChange({ line_number: inputValue(event) })} />
					</label>
					<label class="inline-flex items-center gap-3 rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 text-sm text-stone-200">
						<input type="checkbox" checked={draft.resolved} onchange={(event) => onDraftChange({ resolved: boolValue(event) })} />
						<span>Mark resolved</span>
					</label>
					<label class="block text-sm md:col-span-2">
						<span class="mb-2 block font-medium text-stone-100">Comment</span>
						<textarea class="min-h-28 w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-indigo-400" oninput={(event) => onDraftChange({ body: textValue(event) })}>{draft.body}</textarea>
					</label>
				</div>
				<button class="mt-4 rounded-full bg-indigo-500 px-4 py-2 text-sm font-semibold text-stone-950 transition hover:bg-indigo-400 disabled:cursor-not-allowed disabled:bg-indigo-200" onclick={onCreateComment} disabled={busy}>Add comment</button>
			</div>
		</div>
	{:else}
		<div class="mt-5 rounded-2xl border border-dashed border-stone-300 bg-stone-50 px-4 py-8 text-center text-sm text-stone-500">Select a merge request to inspect reviewers and inline comments.</div>
	{/if}
</section>
