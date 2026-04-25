<script lang="ts">
	import { onMount } from 'svelte';

	import ReportDesigner from '$components/report/ReportDesigner.svelte';
	import ReportHistory from '$components/report/ReportHistory.svelte';
	import ReportPreview from '$components/report/ReportPreview.svelte';
	import ScheduleManager from '$components/report/ScheduleManager.svelte';
	import TemplateLibrary from '$components/report/TemplateLibrary.svelte';
	import {
		createReport,
		generateReport,
		getCatalog,
		getDownload,
		getExecution,
		getOverview,
		getScheduleBoard,
		listHistory,
		listReports,
		updateReport,
		type DistributionRecipient,
		type DownloadPayload,
		type GeneratorKind,
		type ReportCatalog,
		type ReportDefinition,
		type ReportExecution,
		type ReportOverview,
		type ReportSchedule,
		type ReportTemplate,
		type ScheduleBoard,
	} from '$lib/api/reports';
	import { notifications } from '$lib/stores/notifications';

	type ReportDraft = {
		id?: string;
		name: string;
		description: string;
		owner: string;
		generator_kind: GeneratorKind;
		dataset_name: string;
		active: boolean;
		tags_text: string;
		schedule_text: string;
		template_text: string;
		recipients_text: string;
	};

	let overview = $state<ReportOverview | null>(null);
	let catalog = $state<ReportCatalog | null>(null);
	let reports = $state<ReportDefinition[]>([]);
	let scheduleBoard = $state<ScheduleBoard | null>(null);
	let history = $state<ReportExecution[]>([]);
	let selectedReportId = $state('');
	let latestExecution = $state<ReportExecution | null>(null);
	let downloadPayload = $state<DownloadPayload | null>(null);
	let draft = $state<ReportDraft>(createEmptyReportDraft());
	let loading = $state(true);
	let busyAction = $state('');
	let uiError = $state('');
	const busy = $derived(loading || busyAction.length > 0);

	onMount(() => {
		void refreshAll();
	});

	function createEmptyReportDraft(): ReportDraft {
		return {
			name: 'Executive Revenue Pulse',
			description: 'Weekly executive digest with KPI cards, regional split, and map hotspots.',
			owner: 'Revenue Operations',
			generator_kind: 'pdf',
			dataset_name: 'sales_fact_daily',
			active: true,
			tags_text: 'executive, weekly, revenue',
			schedule_text: formatJson({
				cadence: 'weekly',
				expression: '0 9 * * MON',
				timezone: 'UTC',
				anchor_time: '09:00',
				interval_minutes: 10080,
				enabled: true,
				next_run_at: null,
			}),
			template_text: formatJson({
				title: 'Executive Revenue Pulse',
				subtitle: 'Weekly commercial operating review',
				theme: 'copper',
				layout: 'briefing',
				sections: [
					{
						id: 'gross-margin',
						title: 'Gross Margin',
						kind: 'kpi',
						query: 'select margin from revenue_kpis',
						description: 'Margin headline for leadership',
						config: { unit: '%' },
					},
					{
						id: 'regional-split',
						title: 'Regional Revenue',
						kind: 'table',
						query: 'select region, revenue from regional_revenue',
						description: 'Regional split by operating market',
						config: { sortBy: 'value' },
					},
					{
						id: 'geo-hotspots',
						title: 'Pipeline Hotspots',
						kind: 'map',
						query: 'select lat, lon, value from opportunities',
						description: 'Map section used in the preview and PPTX deck',
						config: { layer: 'heatmap' },
					},
				],
			}),
			recipients_text: formatJson([
				{
					id: 'exec-email',
					channel: 'email',
					target: 'exec-team@openfoundry.dev',
					label: 'Executive distribution',
					config: { subject: 'Weekly revenue pulse' },
				},
				{
					id: 'revops-slack',
					channel: 'slack',
					target: '#revops',
					label: 'RevOps room',
					config: { webhook: 'revops-webhook' },
				},
				{
					id: 'exec-teams',
					channel: 'teams',
					target: 'Operations leadership',
					label: 'Operations leadership',
					config: { webhook: 'teams-webhook' },
				},
			]),
		};
	}

	function formatJson(value: unknown) {
		return JSON.stringify(value, null, 2);
	}

	function parseJson<T>(value: string): T {
		return JSON.parse(value) as T;
	}

	function parseCsv(value: string) {
		return value.split(',').map((entry) => entry.trim()).filter(Boolean);
	}

	function reportToDraft(report: ReportDefinition): ReportDraft {
		return {
			id: report.id,
			name: report.name,
			description: report.description,
			owner: report.owner,
			generator_kind: report.generator_kind,
			dataset_name: report.dataset_name,
			active: report.active,
			tags_text: report.tags.join(', '),
			schedule_text: formatJson(report.schedule),
			template_text: formatJson(report.template),
			recipients_text: formatJson(report.recipients),
		};
	}

	async function refreshAll(preferredReportId?: string) {
		loading = true;
		uiError = '';
		try {
			const [overviewResponse, catalogResponse, reportsResponse, boardResponse] = await Promise.all([
				getOverview(),
				getCatalog(),
				listReports(),
				getScheduleBoard(),
			]);

			overview = overviewResponse;
			catalog = catalogResponse;
			reports = reportsResponse.items;
			scheduleBoard = boardResponse;

			const nextSelectedId = preferredReportId ?? selectedReportId ?? reports[0]?.id ?? '';
			if (nextSelectedId) {
				await selectReport(nextSelectedId, false);
			} else {
				selectedReportId = '';
				history = [];
				latestExecution = null;
				downloadPayload = null;
				draft = createEmptyReportDraft();
			}
		} catch (error) {
			uiError = error instanceof Error ? error.message : 'Unable to load reporting surfaces';
			notifications.error(uiError);
		} finally {
			loading = false;
		}
	}

	async function selectReport(reportId: string, notify = true) {
		selectedReportId = reportId;
		const report = reports.find((entry) => entry.id === reportId);
		draft = report ? reportToDraft(report) : createEmptyReportDraft();
		await loadHistory(reportId);
		if (notify) {
			notifications.info(`Loaded ${report?.name ?? 'report'} context`);
		}
	}

	async function loadHistory(reportId: string) {
		const response = await listHistory(reportId);
		history = response.items;
		if (history.length > 0) {
			latestExecution = history[0];
			downloadPayload = await getDownload(history[0].id);
		} else {
			latestExecution = null;
			downloadPayload = null;
		}
	}

	function updateDraft(patch: Partial<ReportDraft>) {
		draft = { ...draft, ...patch };
	}

	function resetDraft() {
		selectedReportId = '';
		draft = createEmptyReportDraft();
		history = [];
		latestExecution = null;
		downloadPayload = null;
	}

	async function saveReportDraft() {
		busyAction = 'save-report';
		uiError = '';
		try {
			const payload = {
				name: draft.name,
				description: draft.description,
				owner: draft.owner,
				generator_kind: draft.generator_kind,
				dataset_name: draft.dataset_name,
				active: draft.active,
				tags: parseCsv(draft.tags_text),
				schedule: parseJson<ReportSchedule>(draft.schedule_text),
				template: parseJson<ReportTemplate>(draft.template_text),
				recipients: parseJson<DistributionRecipient[]>(draft.recipients_text),
				parameters: {},
			};

			const report = draft.id
				? await updateReport(draft.id, payload)
				: await createReport(payload);

			notifications.success(`${report.name} saved`);
			await refreshAll(report.id);
		} catch (error) {
			uiError = error instanceof Error ? error.message : 'Unable to save report';
			notifications.error(uiError);
		} finally {
			busyAction = '';
		}
	}

	async function runSelectedReport() {
		if (!selectedReportId) {
			notifications.warning('Select or create a report before generating it');
			return;
		}

		busyAction = 'run-report';
		uiError = '';
		try {
			const execution = await generateReport(selectedReportId);
			latestExecution = execution;
			downloadPayload = await getDownload(execution.id);
			notifications.success(`${execution.report_name} generated`);
			await refreshAll(selectedReportId);
		} catch (error) {
			uiError = error instanceof Error ? error.message : 'Unable to generate report';
			notifications.error(uiError);
		} finally {
			busyAction = '';
		}
	}

	async function selectExecution(executionId: string) {
		busyAction = 'load-execution';
		uiError = '';
		try {
			latestExecution = await getExecution(executionId);
			downloadPayload = await getDownload(executionId);
		} catch (error) {
			uiError = error instanceof Error ? error.message : 'Unable to load execution';
			notifications.error(uiError);
		} finally {
			busyAction = '';
		}
	}
</script>

<svelte:head>
	<title>OpenFoundry Reports</title>
</svelte:head>

<div class="min-h-screen bg-[radial-gradient(circle_at_top_left,_rgba(217,119,6,0.16),_transparent_30%),linear-gradient(180deg,_#fffaf4_0%,_#f6f2ea_55%,_#eee5d8_100%)] px-6 py-8 text-stone-900 lg:px-10">
	<div class="mx-auto max-w-7xl space-y-6">
		<section class="grid gap-6 rounded-[2rem] border border-stone-200/80 bg-white/80 p-6 shadow-xl shadow-stone-200/60 backdrop-blur xl:grid-cols-[1.1fr_0.9fr]">
			<div>
				<p class="text-xs font-semibold uppercase tracking-[0.28em] text-amber-700">Milestone 4.3</p>
				<h1 class="mt-3 text-4xl font-semibold tracking-tight text-stone-950">Reports control plane</h1>
				<p class="mt-3 max-w-2xl text-base leading-7 text-stone-600">
					Design definitions, preview generated artifacts, manage cadence, and deliver reports into email, S3, Slack, Teams, and webhooks from a single workspace.
				</p>
			</div>
			<div class="grid gap-4 sm:grid-cols-2">
				<div class="rounded-3xl border border-stone-200 bg-stone-50 p-4">
					<p class="text-xs font-semibold uppercase tracking-[0.2em] text-stone-500">Definitions</p>
					<p class="mt-3 text-3xl font-semibold text-stone-950">{overview?.report_count ?? 0}</p>
				</div>
				<div class="rounded-3xl border border-stone-200 bg-stone-50 p-4">
					<p class="text-xs font-semibold uppercase tracking-[0.2em] text-stone-500">Active schedules</p>
					<p class="mt-3 text-3xl font-semibold text-stone-950">{overview?.active_schedules ?? 0}</p>
				</div>
				<div class="rounded-3xl border border-stone-200 bg-stone-50 p-4">
					<p class="text-xs font-semibold uppercase tracking-[0.2em] text-stone-500">Executions 24h</p>
					<p class="mt-3 text-3xl font-semibold text-stone-950">{overview?.executions_24h ?? 0}</p>
				</div>
				<div class="rounded-3xl border border-stone-200 bg-stone-50 p-4">
					<p class="text-xs font-semibold uppercase tracking-[0.2em] text-stone-500">Generators</p>
					<p class="mt-3 text-sm font-medium text-stone-900">{overview?.generator_mix.join(' • ') || 'No generators yet'}</p>
				</div>
			</div>
		</section>

		{#if uiError}
			<div class="rounded-2xl border border-rose-300 bg-rose-50 px-4 py-3 text-sm text-rose-700">{uiError}</div>
		{/if}

		<ReportDesigner
			reports={reports}
			selectedReportId={selectedReportId}
			draft={draft}
			busy={busy}
			onSelect={selectReport}
			onDraftChange={updateDraft}
			onSave={saveReportDraft}
			onReset={resetDraft}
		/>

		<div class="grid gap-6 xl:grid-cols-[0.95fr_1.05fr]">
			<TemplateLibrary {catalog} />
			<ScheduleManager
				board={scheduleBoard}
				selectedReportId={selectedReportId}
				busy={busy}
				onSelectReport={selectReport}
				onGenerate={runSelectedReport}
			/>
		</div>

		<div class="grid gap-6 xl:grid-cols-[1.1fr_0.9fr]">
			<ReportPreview execution={latestExecution} download={downloadPayload} />
			<ReportHistory {history} onSelectExecution={selectExecution} />
		</div>
	</div>
</div>
