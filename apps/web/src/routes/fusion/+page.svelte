<script lang="ts">
	import { onMount } from 'svelte';

	import ClusterViewer from '$components/fusion/ClusterViewer.svelte';
	import GoldenRecordView from '$components/fusion/GoldenRecordView.svelte';
	import ManualReview from '$components/fusion/ManualReview.svelte';
	import MatchRuleBuilder from '$components/fusion/MatchRuleBuilder.svelte';
	import MergePreview from '$components/fusion/MergePreview.svelte';
	import ResolutionResults from '$components/fusion/ResolutionResults.svelte';
	import {
		createJob,
		createMergeStrategy,
		createRule,
		getCluster,
		getOverview,
		listClusters,
		listGoldenRecords,
		listJobs,
		listMergeStrategies,
		listReviewQueue,
		listRules,
		runJob,
		submitReview,
		updateMergeStrategy,
		updateRule,
		type ClusterDetail,
		type FusionJob,
		type FusionOverview,
		type GoldenRecord,
		type MatchRule,
		type MergeStrategy,
		type ResolvedCluster,
		type ReviewQueueItem,
		type RunResolutionJobResponse,
	} from '$lib/api/fusion';
	import { notifications } from '$stores/notifications';

	type MatchRuleDraft = {
		id?: string;
		name: string;
		description: string;
		status: string;
		entity_type: string;
		strategy_type: string;
		key_fields_text: string;
		window_size: number;
		bucket_count: number;
		review_threshold: number;
		auto_merge_threshold: number;
		conditions_text: string;
	};

	type MergeStrategyDraft = {
		id?: string;
		name: string;
		description: string;
		status: string;
		entity_type: string;
		default_strategy: string;
		rules_text: string;
	};

	type JobDraft = {
		name: string;
		description: string;
		status: string;
		entity_type: string;
		match_rule_id: string;
		merge_strategy_id: string;
		source_labels_text: string;
		record_count: number;
		review_sampling_rate: number;
	};

	type ReviewDraft = {
		decision: string;
		reviewed_by: string;
		notes: string;
	};

	let overview = $state<FusionOverview | null>(null);
	let rules = $state<MatchRule[]>([]);
	let mergeStrategies = $state<MergeStrategy[]>([]);
	let jobs = $state<FusionJob[]>([]);
	let clusters = $state<ResolvedCluster[]>([]);
	let reviewQueue = $state<ReviewQueueItem[]>([]);
	let goldenRecords = $state<GoldenRecord[]>([]);
	let clusterDetail = $state<ClusterDetail | null>(null);
	let lastRun = $state<RunResolutionJobResponse | null>(null);

	let selectedJobId = $state('');
	let selectedClusterId = $state('');

	let matchRuleDraft = $state<MatchRuleDraft>(createEmptyMatchRuleDraft());
	let mergeStrategyDraft = $state<MergeStrategyDraft>(createEmptyMergeStrategyDraft());
	let jobDraft = $state<JobDraft>(createEmptyJobDraft());
	let reviewDraft = $state<ReviewDraft>(createEmptyReviewDraft());

	let loading = $state(true);
	let busyAction = $state('');
	let uiError = $state('');

	const busy = $derived(loading || busyAction.length > 0);

	onMount(() => {
		void refreshAll();
	});

	function createEmptyMatchRuleDraft(): MatchRuleDraft {
		return {
			name: 'Person Resolution Rule',
			description: '',
			status: 'active',
			entity_type: 'person',
			strategy_type: 'sorted-neighborhood',
			key_fields_text: 'email, phone, display_name',
			window_size: 4,
			bucket_count: 24,
			review_threshold: 0.76,
			auto_merge_threshold: 0.9,
			conditions_text: formatJson([
				{ field: 'email', comparator: 'email_exact', weight: 0.35, threshold: 1.0, required: false },
				{ field: 'phone', comparator: 'phone_exact', weight: 0.2, threshold: 1.0, required: false },
				{ field: 'display_name', comparator: 'jaro_winkler', weight: 0.25, threshold: 0.86, required: true },
				{ field: 'display_name', comparator: 'phonetic', weight: 0.1, threshold: 0.5, required: false },
				{ field: 'company', comparator: 'fuzzy', weight: 0.1, threshold: 0.72, required: false },
			]),
		};
	}

	function createEmptyMergeStrategyDraft(): MergeStrategyDraft {
		return {
			name: 'Person Survivorship',
			description: '',
			status: 'active',
			entity_type: 'person',
			default_strategy: 'longest_non_empty',
			rules_text: formatJson([
				{ field: 'display_name', strategy: 'longest_non_empty', source_priority: ['crm', 'erp', 'support'], fallback: 'highest_confidence' },
				{ field: 'email', strategy: 'source_priority', source_priority: ['crm', 'erp', 'support'], fallback: 'most_common' },
				{ field: 'phone', strategy: 'most_common', source_priority: [], fallback: 'longest_non_empty' },
				{ field: 'company', strategy: 'most_common', source_priority: [], fallback: 'longest_non_empty' },
			]),
		};
	}

	function createEmptyJobDraft(): JobDraft {
		return {
			name: 'Customer 360 Batch',
			description: 'Resolve customer identities across CRM, ERP, and support exports.',
			status: 'draft',
			entity_type: 'person',
			match_rule_id: '',
			merge_strategy_id: '',
			source_labels_text: 'crm, erp, support',
			record_count: 12,
			review_sampling_rate: 0.25,
		};
	}

	function createEmptyReviewDraft(): ReviewDraft {
		return {
			decision: 'confirm_match',
			reviewed_by: 'reviewer@openfoundry.dev',
			notes: '',
		};
	}

	function parseCsv(value: string) {
		return value.split(',').map((entry) => entry.trim()).filter(Boolean);
	}

	function parseJson<T>(value: string, fallback: T): T {
		if (!value.trim()) return fallback;
		try {
			return JSON.parse(value) as T;
		} catch {
			throw new Error('Invalid JSON payload');
		}
	}

	function formatJson(value: unknown) {
		return JSON.stringify(value, null, 2);
	}

	function matchRuleToDraft(rule: MatchRule): MatchRuleDraft {
		return {
			id: rule.id,
			name: rule.name,
			description: rule.description,
			status: rule.status,
			entity_type: rule.entity_type,
			strategy_type: rule.blocking_strategy.strategy_type,
			key_fields_text: rule.blocking_strategy.key_fields.join(', '),
			window_size: rule.blocking_strategy.window_size,
			bucket_count: rule.blocking_strategy.bucket_count,
			review_threshold: rule.review_threshold,
			auto_merge_threshold: rule.auto_merge_threshold,
			conditions_text: formatJson(rule.conditions),
		};
	}

	function mergeStrategyToDraft(strategy: MergeStrategy): MergeStrategyDraft {
		return {
			id: strategy.id,
			name: strategy.name,
			description: strategy.description,
			status: strategy.status,
			entity_type: strategy.entity_type,
			default_strategy: strategy.default_strategy,
			rules_text: formatJson(strategy.rules),
		};
	}

	async function refreshAll() {
		loading = true;
		uiError = '';
		try {
			const [overviewResponse, ruleResponse, mergeStrategyResponse, jobResponse, clusterResponse, reviewResponse, goldenResponse] = await Promise.all([
				getOverview(),
				listRules(),
				listMergeStrategies(),
				listJobs(),
				listClusters(),
				listReviewQueue(),
				listGoldenRecords(),
			]);

			overview = overviewResponse;
			rules = ruleResponse.data;
			mergeStrategies = mergeStrategyResponse.data;
			jobs = jobResponse.data;
			clusters = clusterResponse.data;
			reviewQueue = reviewResponse.data;
			goldenRecords = goldenResponse.data;

			if (!matchRuleDraft.id && rules[0]) matchRuleDraft = matchRuleToDraft(rules[0]);
			if (!mergeStrategyDraft.id && mergeStrategies[0]) mergeStrategyDraft = mergeStrategyToDraft(mergeStrategies[0]);
			if (!jobDraft.match_rule_id && rules[0]) jobDraft = { ...jobDraft, match_rule_id: rules[0].id };
			if (!jobDraft.merge_strategy_id && mergeStrategies[0]) jobDraft = { ...jobDraft, merge_strategy_id: mergeStrategies[0].id };
			if (!selectedJobId && jobs[0]) selectedJobId = jobs[0].id;
			if (!selectedClusterId && clusters[0]) selectedClusterId = clusters[0].id;

			if (selectedClusterId) {
				await refreshClusterDetail(selectedClusterId);
			} else {
				clusterDetail = null;
			}
		} catch (cause) {
			uiError = cause instanceof Error ? cause.message : 'Failed to load Fusion data';
			notifications.error(uiError);
		} finally {
			loading = false;
		}
	}

	async function refreshClusterDetail(clusterId: string) {
		selectedClusterId = clusterId;
		clusterDetail = await getCluster(clusterId);
	}

	async function runAction(label: string, action: () => Promise<void>) {
		busyAction = label;
		uiError = '';
		try {
			await action();
		} catch (cause) {
			uiError = cause instanceof Error ? cause.message : 'Action failed';
			notifications.error(uiError);
		} finally {
			busyAction = '';
		}
	}

	async function saveRule() {
		await runAction('save-rule', async () => {
			const payload = {
				name: matchRuleDraft.name.trim(),
				description: matchRuleDraft.description,
				status: matchRuleDraft.status,
				entity_type: matchRuleDraft.entity_type,
				blocking_strategy: {
					strategy_type: matchRuleDraft.strategy_type,
					key_fields: parseCsv(matchRuleDraft.key_fields_text),
					window_size: matchRuleDraft.window_size,
					bucket_count: matchRuleDraft.bucket_count,
				},
				conditions: parseJson(matchRuleDraft.conditions_text, []),
				review_threshold: matchRuleDraft.review_threshold,
				auto_merge_threshold: matchRuleDraft.auto_merge_threshold,
			};

			const saved = matchRuleDraft.id
				? await updateRule(matchRuleDraft.id, payload)
				: await createRule(payload);
			matchRuleDraft = matchRuleToDraft(saved);
			await refreshAll();
			notifications.success('Match rule saved.');
		});
	}

	async function saveMergeStrategy() {
		await runAction('save-merge-strategy', async () => {
			const payload = {
				name: mergeStrategyDraft.name.trim(),
				description: mergeStrategyDraft.description,
				status: mergeStrategyDraft.status,
				entity_type: mergeStrategyDraft.entity_type,
				default_strategy: mergeStrategyDraft.default_strategy,
				rules: parseJson(mergeStrategyDraft.rules_text, []),
			};
			const saved = mergeStrategyDraft.id
				? await updateMergeStrategy(mergeStrategyDraft.id, payload)
				: await createMergeStrategy(payload);
			mergeStrategyDraft = mergeStrategyToDraft(saved);
			await refreshAll();
			notifications.success('Merge strategy saved.');
		});
	}

	async function saveJob() {
		await runAction('save-job', async () => {
			const saved = await createJob({
				name: jobDraft.name.trim(),
				description: jobDraft.description,
				status: jobDraft.status,
				entity_type: jobDraft.entity_type,
				match_rule_id: jobDraft.match_rule_id,
				merge_strategy_id: jobDraft.merge_strategy_id,
				config: {
					source_labels: parseCsv(jobDraft.source_labels_text),
					record_count: jobDraft.record_count,
					blocking_strategy_override: null,
					review_sampling_rate: jobDraft.review_sampling_rate,
				},
			});
			selectedJobId = saved.id;
			await refreshAll();
			notifications.success('Fusion job created.');
		});
	}

	async function runSelectedJob() {
		if (!selectedJobId) {
			notifications.warning('Select a job to run.');
			return;
		}
		await runAction('run-job', async () => {
			lastRun = await runJob(selectedJobId);
			await refreshAll();
			notifications.success('Fusion resolution run completed.');
		});
	}

	async function submitSelectedReview() {
		if (!selectedClusterId) {
			notifications.warning('Select a cluster from the review queue first.');
			return;
		}
		await runAction('submit-review', async () => {
			clusterDetail = await submitReview(selectedClusterId, {
				decision: reviewDraft.decision,
				reviewed_by: reviewDraft.reviewed_by,
				notes: reviewDraft.notes,
			});
			await refreshAll();
			notifications.success('Review decision recorded.');
		});
	}
</script>

<svelte:head>
	<title>Fusion | OpenFoundry</title>
</svelte:head>

<div class="space-y-6">
	<section class="overflow-hidden rounded-[36px] border border-slate-200 bg-[radial-gradient(circle_at_top_left,_rgba(251,191,36,0.26),_transparent_36%),linear-gradient(135deg,#111827_0%,#1f2937_34%,#f8fafc_100%)] p-6 text-white shadow-sm dark:border-slate-800">
		<div class="grid gap-6 xl:grid-cols-[minmax(0,1.1fr)_minmax(0,0.9fr)]">
			<div>
				<div class="text-[11px] font-semibold uppercase tracking-[0.34em] text-amber-100">Phase 4.1</div>
				<h1 class="mt-3 max-w-3xl text-4xl font-semibold leading-tight">Fusion entity resolution across rules, ML scoring, graph closure, and manual review</h1>
				<p class="mt-4 max-w-2xl text-sm leading-7 text-slate-100/85">
					Model deterministic match rules, blend them with probabilistic scoring, close clusters transitively, and curate golden records with a human review queue.
				</p>
			</div>
			<div class="rounded-[28px] border border-white/15 bg-white/10 p-5 backdrop-blur">
				<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-amber-100">Operator Loop</div>
				<div class="mt-3 grid gap-3 text-sm text-slate-100/90">
					<div class="rounded-2xl border border-white/10 bg-white/5 px-4 py-3">1. Define blocking and match conditions.</div>
					<div class="rounded-2xl border border-white/10 bg-white/5 px-4 py-3">2. Run resolution jobs to synthesize clusters and golden records.</div>
					<div class="rounded-2xl border border-white/10 bg-white/5 px-4 py-3">3. Review uncertain clusters and approve or split them.</div>
				</div>
			</div>
		</div>
	</section>

	{#if uiError}
		<div class="rounded-2xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-700 dark:border-rose-900/70 dark:bg-rose-950/40 dark:text-rose-200">{uiError}</div>
	{/if}

	{#if loading}
		<div class="rounded-[28px] border border-slate-200 bg-white px-6 py-10 text-center text-sm text-slate-500 shadow-sm dark:border-slate-800 dark:bg-slate-950 dark:text-slate-400">Loading Fusion surfaces...</div>
	{:else}
		<div class="grid gap-6 xl:grid-cols-[minmax(0,1.08fr)_minmax(0,0.92fr)]">
			<MatchRuleBuilder
				rules={rules}
				draft={matchRuleDraft}
				busy={busy}
				onSelect={(ruleId) => {
					const rule = rules.find((item) => item.id === ruleId);
					if (rule) matchRuleDraft = matchRuleToDraft(rule);
				}}
				onDraftChange={(draft) => matchRuleDraft = draft}
				onSave={saveRule}
				onReset={() => matchRuleDraft = createEmptyMatchRuleDraft()}
			/>
			<MergePreview
				strategies={mergeStrategies}
				draft={mergeStrategyDraft}
				busy={busy}
				onSelect={(strategyId) => {
					const strategy = mergeStrategies.find((item) => item.id === strategyId);
					if (strategy) mergeStrategyDraft = mergeStrategyToDraft(strategy);
				}}
				onDraftChange={(draft) => mergeStrategyDraft = draft}
				onSave={saveMergeStrategy}
				onReset={() => mergeStrategyDraft = createEmptyMergeStrategyDraft()}
			/>
		</div>

		<ResolutionResults
			overview={overview}
			jobs={jobs}
			rules={rules}
			mergeStrategies={mergeStrategies}
			draft={jobDraft}
			lastRun={lastRun}
			selectedJobId={selectedJobId}
			busy={busy}
			onSelectJob={(jobId) => selectedJobId = jobId}
			onDraftChange={(draft) => jobDraft = draft}
			onSave={saveJob}
			onRun={runSelectedJob}
			onReset={() => jobDraft = createEmptyJobDraft()}
		/>

		<ClusterViewer
			clusters={clusters}
			selectedClusterId={selectedClusterId}
			clusterDetail={clusterDetail}
			busy={busy}
			onSelectCluster={(clusterId) => void refreshClusterDetail(clusterId)}
		/>

		<div class="grid gap-6 xl:grid-cols-[minmax(0,0.95fr)_minmax(0,1.05fr)]">
			<ManualReview
				reviewQueue={reviewQueue}
				selectedClusterId={selectedClusterId}
				clusterDetail={clusterDetail}
				draft={reviewDraft}
				busy={busy}
				onSelectCluster={(clusterId) => void refreshClusterDetail(clusterId)}
				onDraftChange={(draft) => reviewDraft = draft}
				onSubmit={submitSelectedReview}
			/>
			<GoldenRecordView goldenRecords={goldenRecords} clusterDetail={clusterDetail} />
		</div>
	{/if}
</div>