<script lang="ts">
	import type { CiRun, CommitDefinition } from '$lib/api/code-repos';

	type CommitDraft = {
		branch_name: string;
		title: string;
		description: string;
		author_name: string;
		additions: string;
		deletions: string;
	};

	export let commits: CommitDefinition[] = [];
	export let ciRuns: CiRun[] = [];
	export let draft: CommitDraft;
	export let busy = false;
	export let onDraftChange: (patch: Partial<CommitDraft>) => void;
	export let onCreateCommit: () => void;
	export let onTriggerCi: () => void;

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
			<p class="text-xs font-semibold uppercase tracking-[0.24em] text-violet-700">Commits & CI</p>
			<h3 class="mt-2 text-xl font-semibold text-stone-900">History, pipeline triggers, and changed files</h3>
			<p class="mt-1 text-sm text-stone-500">Push synthetic commits into a branch and trigger the package validation pipeline on demand.</p>
		</div>
		<div class="flex gap-2">
			<button class="rounded-full border border-violet-300 px-4 py-2 text-sm font-medium text-violet-700 transition hover:border-violet-400 hover:bg-violet-50" onclick={onTriggerCi} disabled={busy}>Trigger CI</button>
			<button class="rounded-full bg-violet-600 px-4 py-2 text-sm font-semibold text-white transition hover:bg-violet-700 disabled:cursor-not-allowed disabled:bg-violet-300" onclick={onCreateCommit} disabled={busy}>Create commit</button>
		</div>
	</div>

	<div class="mt-5 grid gap-4 xl:grid-cols-[0.94fr_1.06fr]">
		<div class="space-y-3 rounded-2xl border border-stone-200 bg-stone-50/80 p-4">
			{#each commits as commit}
				<div class="rounded-2xl border border-stone-200 bg-white px-4 py-4">
					<div class="flex items-start justify-between gap-3">
						<div>
							<p class="font-semibold text-stone-900">{commit.title}</p>
							<p class="text-sm text-stone-500">{commit.branch_name} • {commit.sha} • {commit.author_name}</p>
						</div>
						<span class="rounded-full bg-violet-100 px-3 py-1 text-xs font-semibold uppercase tracking-[0.18em] text-violet-700">{commit.files_changed} files</span>
					</div>
					<p class="mt-3 text-sm text-stone-600">{commit.description}</p>
					<div class="mt-3 flex gap-2 text-xs text-stone-600">
						<span class="rounded-full bg-stone-100 px-2 py-1">+{commit.additions}</span>
						<span class="rounded-full bg-stone-100 px-2 py-1">-{commit.deletions}</span>
					</div>
				</div>
			{/each}
		</div>

		<div class="space-y-4 rounded-2xl border border-stone-200 bg-stone-950 p-4 text-stone-100">
			<div>
				<p class="text-xs font-semibold uppercase tracking-[0.2em] text-violet-300">Commit draft</p>
			</div>
			<div class="grid gap-4 md:grid-cols-2">
				<label class="block text-sm">
					<span class="mb-2 block font-medium text-stone-100">Branch</span>
					<input class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-violet-400" value={draft.branch_name} oninput={(event) => onDraftChange({ branch_name: inputValue(event) })} />
				</label>
				<label class="block text-sm">
					<span class="mb-2 block font-medium text-stone-100">Author</span>
					<input class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-violet-400" value={draft.author_name} oninput={(event) => onDraftChange({ author_name: inputValue(event) })} />
				</label>
				<label class="block text-sm md:col-span-2">
					<span class="mb-2 block font-medium text-stone-100">Title</span>
					<input class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-violet-400" value={draft.title} oninput={(event) => onDraftChange({ title: inputValue(event) })} />
				</label>
				<label class="block text-sm md:col-span-2">
					<span class="mb-2 block font-medium text-stone-100">Description</span>
					<textarea class="min-h-24 w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-violet-400" oninput={(event) => onDraftChange({ description: textValue(event) })}>{draft.description}</textarea>
				</label>
				<label class="block text-sm">
					<span class="mb-2 block font-medium text-stone-100">Additions</span>
					<input class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-violet-400" value={draft.additions} oninput={(event) => onDraftChange({ additions: inputValue(event) })} />
				</label>
				<label class="block text-sm">
					<span class="mb-2 block font-medium text-stone-100">Deletions</span>
					<input class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-violet-400" value={draft.deletions} oninput={(event) => onDraftChange({ deletions: inputValue(event) })} />
				</label>
			</div>

			<div>
				<p class="text-xs font-semibold uppercase tracking-[0.2em] text-violet-300">Latest CI runs</p>
				<div class="mt-3 space-y-3">
					{#each ciRuns as run}
						<div class="rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3">
							<div class="flex items-center justify-between gap-3">
								<div>
									<p class="font-medium text-stone-100">{run.pipeline_name}</p>
									<p class="text-xs text-stone-400">{run.branch_name} • {run.commit_sha} • {run.trigger}</p>
								</div>
								<span class="rounded-full bg-emerald-300 px-3 py-1 text-xs font-semibold uppercase tracking-[0.18em] text-emerald-950">{run.status}</span>
							</div>
							<div class="mt-3 flex flex-wrap gap-2">
								{#each run.checks as check}
									<span class="rounded-full bg-stone-800 px-2 py-1 text-xs text-stone-300">{check}</span>
								{/each}
							</div>
						</div>
					{/each}
				</div>
			</div>
		</div>
	</div>
</section>
