<script lang="ts">
	import type {
		FusionJob,
		FusionOverview,
		MatchRule,
		MergeStrategy,
		RunResolutionJobResponse,
	} from '$lib/api/fusion';

	interface JobDraft {
		name: string;
		description: string;
		status: string;
		entity_type: string;
		match_rule_id: string;
		merge_strategy_id: string;
		source_labels_text: string;
		record_count: number;
		review_sampling_rate: number;
	}

	interface Props {
		overview: FusionOverview | null;
		jobs: FusionJob[];
		rules: MatchRule[];
		mergeStrategies: MergeStrategy[];
		draft: JobDraft;
		lastRun: RunResolutionJobResponse | null;
		selectedJobId: string;
		busy?: boolean;
		onSelectJob?: (jobId: string) => void;
		onDraftChange?: (draft: JobDraft) => void;
		onSave?: () => void;
		onRun?: () => void;
		onReset?: () => void;
	}

	let {
		overview,
		jobs,
		rules,
		mergeStrategies,
		draft,
		lastRun,
		selectedJobId,
		busy = false,
		onSelectJob,
		onDraftChange,
		onSave,
		onRun,
		onReset,
	}: Props = $props();

	let localDraft = $state<JobDraft>({
		name: '',
		description: '',
		status: '',
		entity_type: '',
		match_rule_id: '',
		merge_strategy_id: '',
		source_labels_text: '',
		record_count: 12,
		review_sampling_rate: 0.25,
	});

	$effect(() => {
		localDraft = { ...draft };
	});

	function updateDraft<K extends keyof JobDraft>(key: K, value: JobDraft[K]) {
		const nextDraft = { ...localDraft, [key]: value };
		localDraft = nextDraft;
		onDraftChange?.(nextDraft);
	}
</script>

<section class="rounded-[28px] border border-slate-200 bg-white p-5 shadow-sm dark:border-slate-800 dark:bg-slate-950">
	<div class="flex flex-wrap items-start justify-between gap-3">
		<div>
			<div class="text-[11px] font-semibold uppercase tracking-[0.28em] text-slate-500">Resolution Jobs</div>
			<h2 class="mt-2 text-xl font-semibold text-slate-900 dark:text-slate-100">Run blocking, scoring, clustering, and golden record generation</h2>
		</div>
		<div class="flex gap-2">
			<button class="rounded-full border border-slate-300 px-3 py-1.5 text-sm text-slate-700 hover:bg-slate-100 dark:border-slate-700 dark:text-slate-200 dark:hover:bg-slate-900" onclick={() => onReset?.()} disabled={busy}>New job</button>
			<button class="rounded-full border border-amber-300 px-3 py-1.5 text-sm text-amber-700 hover:bg-amber-50 dark:border-amber-800 dark:text-amber-300 dark:hover:bg-amber-950/40" onclick={() => onSave?.()} disabled={busy}>Create job</button>
			<button class="rounded-full bg-slate-950 px-3 py-1.5 text-sm font-medium text-white hover:bg-slate-800 disabled:opacity-60 dark:bg-slate-100 dark:text-slate-950" onclick={() => onRun?.()} disabled={busy || !selectedJobId}>Run selected</button>
		</div>
	</div>

	<div class="mt-5 grid gap-4 md:grid-cols-2 xl:grid-cols-7">
		<div class="rounded-2xl bg-slate-950 px-4 py-4 text-white">
			<div class="text-[11px] uppercase tracking-[0.24em] text-slate-300">Rules</div>
			<div class="mt-2 text-3xl font-semibold">{overview?.rule_count ?? 0}</div>
		</div>
		<div class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-4 dark:border-slate-800 dark:bg-slate-900">
			<div class="text-[11px] uppercase tracking-[0.24em] text-slate-500">Active Jobs</div>
			<div class="mt-2 text-3xl font-semibold text-slate-900 dark:text-slate-100">{overview?.active_job_count ?? 0}</div>
		</div>
		<div class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-4 dark:border-slate-800 dark:bg-slate-900">
			<div class="text-[11px] uppercase tracking-[0.24em] text-slate-500">Completed</div>
			<div class="mt-2 text-3xl font-semibold text-slate-900 dark:text-slate-100">{overview?.completed_job_count ?? 0}</div>
		</div>
		<div class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-4 dark:border-slate-800 dark:bg-slate-900">
			<div class="text-[11px] uppercase tracking-[0.24em] text-slate-500">Clusters</div>
			<div class="mt-2 text-3xl font-semibold text-slate-900 dark:text-slate-100">{overview?.cluster_count ?? 0}</div>
		</div>
		<div class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-4 dark:border-slate-800 dark:bg-slate-900">
			<div class="text-[11px] uppercase tracking-[0.24em] text-slate-500">Review Queue</div>
			<div class="mt-2 text-3xl font-semibold text-slate-900 dark:text-slate-100">{overview?.pending_review_count ?? 0}</div>
		</div>
		<div class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-4 dark:border-slate-800 dark:bg-slate-900">
			<div class="text-[11px] uppercase tracking-[0.24em] text-slate-500">Golden Records</div>
			<div class="mt-2 text-3xl font-semibold text-slate-900 dark:text-slate-100">{overview?.golden_record_count ?? 0}</div>
		</div>
		<div class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-4 dark:border-slate-800 dark:bg-slate-900">
			<div class="text-[11px] uppercase tracking-[0.24em] text-slate-500">Auto-Merged</div>
			<div class="mt-2 text-3xl font-semibold text-slate-900 dark:text-slate-100">{overview?.auto_merged_cluster_count ?? 0}</div>
		</div>
	</div>

	<div class="mt-5 grid gap-5 xl:grid-cols-[minmax(0,0.8fr)_minmax(0,1.2fr)]">
		<div class="space-y-3">
			{#if jobs.length === 0}
				<div class="rounded-2xl border border-dashed border-slate-300 px-4 py-5 text-sm text-slate-500 dark:border-slate-700 dark:text-slate-400">No jobs yet.</div>
			{:else}
				{#each jobs as job}
					<button
						class={`w-full rounded-2xl border px-4 py-3 text-left transition ${selectedJobId === job.id ? 'border-amber-400 bg-amber-50 dark:border-amber-700 dark:bg-amber-950/30' : 'border-slate-200 bg-slate-50 hover:border-slate-300 dark:border-slate-800 dark:bg-slate-900 dark:hover:border-slate-700'}`}
						onclick={() => onSelectJob?.(job.id)}
						type="button"
					>
						<div class="text-sm font-semibold text-slate-900 dark:text-slate-100">{job.name}</div>
						<div class="mt-1 text-xs text-slate-500">{job.status} • {job.metrics.cluster_count} clusters • {job.metrics.review_pairs} review pairs</div>
						<p class="mt-2 text-sm text-slate-600 dark:text-slate-300">{job.last_run_summary}</p>
					</button>
				{/each}
			{/if}
		</div>

		<div class="grid gap-4">
			<div class="grid gap-4 md:grid-cols-2">
				<label class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 dark:border-slate-800 dark:bg-slate-900">
					<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Name</div>
					<input class="mt-2 w-full bg-transparent text-sm text-slate-900 outline-none dark:text-slate-100" value={localDraft.name} oninput={(event) => updateDraft('name', (event.currentTarget as HTMLInputElement).value)} />
				</label>
				<label class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 dark:border-slate-800 dark:bg-slate-900">
					<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Entity Type</div>
					<input class="mt-2 w-full bg-transparent text-sm text-slate-900 outline-none dark:text-slate-100" value={localDraft.entity_type} oninput={(event) => updateDraft('entity_type', (event.currentTarget as HTMLInputElement).value)} />
				</label>
			</div>

			<label class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 dark:border-slate-800 dark:bg-slate-900">
				<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Description</div>
				<textarea class="mt-2 h-24 w-full bg-transparent text-sm text-slate-900 outline-none dark:text-slate-100" oninput={(event) => updateDraft('description', (event.currentTarget as HTMLTextAreaElement).value)}>{localDraft.description}</textarea>
			</label>

			<div class="grid gap-4 md:grid-cols-2">
				<label class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 dark:border-slate-800 dark:bg-slate-900">
					<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Match Rule</div>
					<select class="mt-2 w-full bg-transparent text-sm text-slate-900 outline-none dark:text-slate-100" value={localDraft.match_rule_id} onchange={(event) => updateDraft('match_rule_id', (event.currentTarget as HTMLSelectElement).value)}>
						<option value="">Select match rule</option>
						{#each rules as rule}
							<option value={rule.id}>{rule.name}</option>
						{/each}
					</select>
				</label>
				<label class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 dark:border-slate-800 dark:bg-slate-900">
					<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Merge Strategy</div>
					<select class="mt-2 w-full bg-transparent text-sm text-slate-900 outline-none dark:text-slate-100" value={localDraft.merge_strategy_id} onchange={(event) => updateDraft('merge_strategy_id', (event.currentTarget as HTMLSelectElement).value)}>
						<option value="">Select merge strategy</option>
						{#each mergeStrategies as strategy}
							<option value={strategy.id}>{strategy.name}</option>
						{/each}
					</select>
				</label>
			</div>

			<div class="grid gap-4 md:grid-cols-2 xl:grid-cols-3">
				<label class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 dark:border-slate-800 dark:bg-slate-900">
					<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Source Labels</div>
					<input class="mt-2 w-full bg-transparent text-sm text-slate-900 outline-none dark:text-slate-100" value={localDraft.source_labels_text} oninput={(event) => updateDraft('source_labels_text', (event.currentTarget as HTMLInputElement).value)} />
				</label>
				<label class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 dark:border-slate-800 dark:bg-slate-900">
					<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Record Count</div>
					<input class="mt-2 w-full bg-transparent text-sm text-slate-900 outline-none dark:text-slate-100" type="number" value={String(localDraft.record_count)} oninput={(event) => updateDraft('record_count', Number((event.currentTarget as HTMLInputElement).value) || 12)} />
				</label>
				<label class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 dark:border-slate-800 dark:bg-slate-900">
					<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Review Sampling Rate</div>
					<input class="mt-2 w-full bg-transparent text-sm text-slate-900 outline-none dark:text-slate-100" type="number" step="0.01" value={String(localDraft.review_sampling_rate)} oninput={(event) => updateDraft('review_sampling_rate', Number((event.currentTarget as HTMLInputElement).value) || 0.25)} />
				</label>
			</div>

			{#if lastRun}
				<div class="rounded-[24px] border border-dashed border-amber-300 bg-amber-50/60 p-4 dark:border-amber-900 dark:bg-amber-950/20">
					<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-amber-700 dark:text-amber-300">Latest Resolution Run</div>
					<div class="mt-3 grid gap-3 md:grid-cols-3 text-sm text-slate-700 dark:text-slate-200">
						<div class="rounded-2xl border border-amber-200 bg-white px-3 py-2 dark:border-amber-900 dark:bg-slate-950">Clusters {lastRun.cluster_ids.length}</div>
						<div class="rounded-2xl border border-amber-200 bg-white px-3 py-2 dark:border-amber-900 dark:bg-slate-950">Golden {lastRun.golden_record_ids.length}</div>
						<div class="rounded-2xl border border-amber-200 bg-white px-3 py-2 dark:border-amber-900 dark:bg-slate-950">Review {lastRun.review_queue_item_ids.length}</div>
					</div>
					<p class="mt-3 text-sm text-slate-600 dark:text-slate-300">Executed at {new Date(lastRun.executed_at).toLocaleString()}</p>
				</div>
			{/if}
		</div>
	</div>
</section>
