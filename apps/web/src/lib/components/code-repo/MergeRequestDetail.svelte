<script lang="ts">
	import type {
		CiRun,
		MergeRequestDetail as MergeRequestDetailModel,
		MergeRequestStatus,
	} from '$lib/api/code-repos';

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
	export let mergeBlockers: string[] = [];
	export let latestSourceCi: CiRun | null = null;
	export let targetBranchProtected = false;
	export let onDraftChange: (patch: Partial<CommentDraft>) => void;
	export let onCreateComment: () => void;
	export let onStatusChange: (status: MergeRequestStatus) => void;
	export let onReviewerStateChange: (reviewer: string, approved: boolean, state: string) => void;
	export let onMerge: () => void;

	const statuses: MergeRequestStatus[] = ['open', 'closed'];

	function inputValue(event: Event) {
		return (event.currentTarget as HTMLInputElement).value;
	}

	function textValue(event: Event) {
		return (event.currentTarget as HTMLTextAreaElement).value;
	}

	function boolValue(event: Event) {
		return (event.currentTarget as HTMLInputElement).checked;
	}

	function mergeEnabled() {
		return Boolean(detail) && mergeBlockers.length === 0 && !busy;
	}
</script>

<section class="rounded-3xl border border-stone-200 bg-white p-5 shadow-sm shadow-stone-200/60">
	<div class="flex flex-col gap-4 lg:flex-row lg:items-start lg:justify-between">
		<div>
			<p class="text-xs font-semibold uppercase tracking-[0.24em] text-indigo-700">Review Detail</p>
			<h3 class="mt-2 text-xl font-semibold text-stone-900">Inline comments, approvals, and lifecycle state</h3>
			<p class="mt-1 text-sm text-stone-500">Track reviewer decisions, CI readiness, and branch protection gates before merging for real.</p>
		</div>
		<div class="flex flex-wrap gap-2">
			{#each statuses as status}
				<button class={`rounded-full px-4 py-2 text-sm font-medium transition ${detail?.merge_request.status === status ? 'bg-indigo-600 text-white' : 'border border-indigo-200 bg-white text-indigo-700 hover:bg-indigo-50'}`} onclick={() => onStatusChange(status)} disabled={busy || !detail}>{status}</button>
			{/each}
			<button class={`rounded-full px-4 py-2 text-sm font-semibold transition ${mergeEnabled() ? 'bg-emerald-600 text-white hover:bg-emerald-700' : 'border border-emerald-200 bg-white text-emerald-700'}`} onclick={onMerge} disabled={!mergeEnabled()}>
				Merge now
			</button>
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
						<span class="rounded-full bg-stone-100 px-2 py-1 text-xs font-semibold text-stone-700">{targetBranchProtected ? 'Protected target' : 'Writable target'}</span>
					</div>
				</div>

				<div class="rounded-2xl border border-stone-200 bg-white px-4 py-4">
					<div class="flex items-start justify-between gap-3">
						<div>
							<p class="font-semibold text-stone-900">Merge policy</p>
							<p class="mt-1 text-sm text-stone-500">Protected targets require approvals and a green CI run on the latest source head.</p>
						</div>
						<span class={`rounded-full px-3 py-1 text-xs font-semibold uppercase tracking-[0.18em] ${mergeBlockers.length === 0 ? 'bg-emerald-100 text-emerald-700' : 'bg-amber-100 text-amber-700'}`}>
							{mergeBlockers.length === 0 ? 'Ready' : 'Blocked'}
						</span>
					</div>

					<div class="mt-4 grid gap-3 md:grid-cols-2">
						<div class="rounded-2xl bg-stone-50 px-3 py-3">
							<div class="text-xs uppercase tracking-[0.18em] text-stone-500">Latest source CI</div>
							{#if latestSourceCi}
								<div class="mt-2 text-sm font-medium text-stone-900">{latestSourceCi.status}</div>
								<div class="mt-1 text-xs text-stone-500">{latestSourceCi.branch_name} • {latestSourceCi.commit_sha}</div>
							{:else}
								<div class="mt-2 text-sm font-medium text-stone-900">No runs yet</div>
								<div class="mt-1 text-xs text-stone-500">Trigger CI or push a commit to produce a status check.</div>
							{/if}
						</div>
						<div class="rounded-2xl bg-stone-50 px-3 py-3">
							<div class="text-xs uppercase tracking-[0.18em] text-stone-500">Reviewer state</div>
							<div class="mt-2 text-sm font-medium text-stone-900">{detail.approval_count} approved</div>
							<div class="mt-1 text-xs text-stone-500">{detail.merge_request.reviewers.length} assigned reviewer(s)</div>
						</div>
					</div>

					{#if mergeBlockers.length > 0}
						<div class="mt-4 space-y-2">
							{#each mergeBlockers as blocker}
								<div class="rounded-2xl border border-amber-200 bg-amber-50 px-3 py-2 text-sm text-amber-800">{blocker}</div>
							{/each}
						</div>
					{/if}
				</div>

				<div class="rounded-2xl border border-stone-200 bg-white px-4 py-4">
					<p class="font-semibold text-stone-900">Reviewers</p>
					<div class="mt-4 space-y-3">
						{#each detail.merge_request.reviewers as reviewer}
							<div class="rounded-2xl border border-stone-200 px-3 py-3">
								<div class="flex flex-col gap-3 lg:flex-row lg:items-center lg:justify-between">
									<div>
										<p class="font-medium text-stone-900">{reviewer.reviewer}</p>
										<p class="text-xs text-stone-500">{reviewer.state}</p>
									</div>
									<div class="flex flex-wrap gap-2">
										<button class={`rounded-full px-3 py-1.5 text-xs font-semibold transition ${reviewer.approved ? 'bg-emerald-600 text-white' : 'border border-emerald-200 bg-white text-emerald-700 hover:bg-emerald-50'}`} onclick={() => onReviewerStateChange(reviewer.reviewer, true, 'approved')} disabled={busy}>
											Approve
										</button>
										<button class={`rounded-full px-3 py-1.5 text-xs font-semibold transition ${reviewer.state === 'changes_requested' ? 'bg-rose-600 text-white' : 'border border-rose-200 bg-white text-rose-700 hover:bg-rose-50'}`} onclick={() => onReviewerStateChange(reviewer.reviewer, false, 'changes_requested')} disabled={busy}>
											Request changes
										</button>
										<button class={`rounded-full px-3 py-1.5 text-xs font-semibold transition ${reviewer.state === 'commented' ? 'bg-stone-900 text-white' : 'border border-stone-200 bg-white text-stone-700 hover:bg-stone-50'}`} onclick={() => onReviewerStateChange(reviewer.reviewer, false, 'commented')} disabled={busy}>
											Comment only
										</button>
									</div>
								</div>
							</div>
						{/each}
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
