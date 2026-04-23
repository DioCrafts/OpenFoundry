<script lang="ts">
	import type { ClusterDetail, ReviewQueueItem } from '$lib/api/fusion';

	interface ReviewDraft {
		decision: string;
		reviewed_by: string;
		notes: string;
	}

	interface Props {
		reviewQueue: ReviewQueueItem[];
		selectedClusterId: string;
		clusterDetail: ClusterDetail | null;
		draft: ReviewDraft;
		busy?: boolean;
		onSelectCluster?: (clusterId: string) => void;
		onDraftChange?: (draft: ReviewDraft) => void;
		onSubmit?: () => void;
	}

	let {
		reviewQueue,
		selectedClusterId,
		clusterDetail,
		draft,
		busy = false,
		onSelectCluster,
		onDraftChange,
		onSubmit,
	}: Props = $props();

	let localDraft = $state<ReviewDraft>({
		decision: 'confirm_match',
		reviewed_by: '',
		notes: '',
	});

	$effect(() => {
		localDraft = { ...draft };
	});

	function updateDraft<K extends keyof ReviewDraft>(key: K, value: ReviewDraft[K]) {
		const nextDraft = { ...localDraft, [key]: value };
		localDraft = nextDraft;
		onDraftChange?.(nextDraft);
	}
</script>

<section class="rounded-[28px] border border-slate-200 bg-white p-5 shadow-sm dark:border-slate-800 dark:bg-slate-950">
	<div class="flex flex-wrap items-start justify-between gap-3">
		<div>
			<div class="text-[11px] font-semibold uppercase tracking-[0.28em] text-slate-500">Manual Review</div>
			<h2 class="mt-2 text-xl font-semibold text-slate-900 dark:text-slate-100">Human-in-the-loop decisions for uncertain matches</h2>
		</div>
		<button class="rounded-full bg-slate-950 px-3 py-1.5 text-sm font-medium text-white hover:bg-slate-800 disabled:opacity-60 dark:bg-slate-100 dark:text-slate-950" onclick={() => onSubmit?.()} disabled={busy || !selectedClusterId}>Submit review</button>
	</div>

	<div class="mt-5 grid gap-5 xl:grid-cols-[minmax(0,0.8fr)_minmax(0,1.2fr)]">
		<div class="space-y-3">
			{#if reviewQueue.length === 0}
				<div class="rounded-2xl border border-dashed border-slate-300 px-4 py-5 text-sm text-slate-500 dark:border-slate-700 dark:text-slate-400">No pending reviews right now.</div>
			{:else}
				{#each reviewQueue as item}
					<button
						class={`w-full rounded-2xl border px-4 py-3 text-left transition ${selectedClusterId === item.cluster_id ? 'border-rose-400 bg-rose-50 dark:border-rose-700 dark:bg-rose-950/30' : 'border-slate-200 bg-slate-50 hover:border-slate-300 dark:border-slate-800 dark:bg-slate-900 dark:hover:border-slate-700'}`}
						onclick={() => onSelectCluster?.(item.cluster_id)}
						type="button"
						disabled={busy}
					>
						<div class="flex items-center justify-between gap-3">
							<div>
								<div class="text-sm font-semibold text-slate-900 dark:text-slate-100">{item.cluster_id}</div>
								<div class="mt-1 text-xs text-slate-500">{item.severity} • {item.recommended_action}</div>
							</div>
							<span class="rounded-full bg-white px-2 py-1 text-[11px] font-semibold uppercase tracking-[0.2em] text-slate-500 dark:bg-slate-950">{item.status}</span>
						</div>
					</button>
				{/each}
			{/if}
		</div>

		<div class="space-y-4 rounded-[24px] border border-slate-200 bg-gradient-to-br from-rose-50 to-white p-4 dark:border-slate-800 dark:from-rose-950/20 dark:to-slate-950">
			<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Review Decision</div>
			{#if clusterDetail?.review_item}
				<div class="rounded-2xl border border-rose-200 bg-white px-4 py-3 text-sm text-slate-700 dark:border-rose-900 dark:bg-slate-950 dark:text-slate-200">
					<p class="font-medium">{clusterDetail.review_item.recommended_action}</p>
					<p class="mt-2">{clusterDetail.review_item.rationale.join(' | ')}</p>
				</div>
			{/if}
			<label class="rounded-2xl border border-slate-200 bg-white px-4 py-3 dark:border-slate-800 dark:bg-slate-950">
				<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Decision</div>
				<select class="mt-2 w-full bg-transparent text-sm text-slate-900 outline-none dark:text-slate-100" value={localDraft.decision} onchange={(event) => updateDraft('decision', (event.currentTarget as HTMLSelectElement).value)}>
					<option value="confirm_match">Confirm match</option>
					<option value="manually_resolved">Manual override</option>
					<option value="split_cluster">Split cluster</option>
					<option value="reject_match">Reject match</option>
				</select>
			</label>
			<label class="rounded-2xl border border-slate-200 bg-white px-4 py-3 dark:border-slate-800 dark:bg-slate-950">
				<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Reviewed By</div>
				<input class="mt-2 w-full bg-transparent text-sm text-slate-900 outline-none dark:text-slate-100" value={localDraft.reviewed_by} oninput={(event) => updateDraft('reviewed_by', (event.currentTarget as HTMLInputElement).value)} />
			</label>
			<label class="rounded-2xl border border-slate-200 bg-white px-4 py-3 dark:border-slate-800 dark:bg-slate-950">
				<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Notes</div>
				<textarea class="mt-2 h-28 w-full bg-transparent text-sm text-slate-900 outline-none dark:text-slate-100" oninput={(event) => updateDraft('notes', (event.currentTarget as HTMLTextAreaElement).value)}>{localDraft.notes}</textarea>
			</label>
		</div>
	</div>
</section>
