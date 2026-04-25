<script lang="ts">
	import { onMount } from 'svelte';

	import AuditLogViewer from '$components/audit/AuditLogViewer.svelte';
	import AuditTimeline from '$components/audit/AuditTimeline.svelte';
	import ComplianceDashboard from '$components/audit/ComplianceDashboard.svelte';
	import ExportWizard from '$components/audit/ExportWizard.svelte';
	import GovernanceStudio from '$components/audit/GovernanceStudio.svelte';
	import PolicyManager from '$components/audit/PolicyManager.svelte';
	import {
		applyGovernanceTemplate,
		appendEvent,
		createPolicy,
		eraseSubjectData,
		exportSubjectData,
		generateReport,
		getCompliancePosture,
		getOverview,
		listAnomalies,
		listClassifications,
		listCollectors,
		listEvents,
		listGovernanceApplications,
		listGovernanceTemplates,
		listPolicies,
		listReports,
		scanSensitiveData,
		updatePolicy,
		type AnomalyAlert,
		type AuditEvent,
		type AuditEventStatus,
		type AuditOverview,
		type AuditPolicy,
		type AuditSeverity,
		type CompliancePostureOverview,
		type ClassificationCatalogEntry,
		type ClassificationLevel,
		type CollectorStatus,
		type ComplianceReport,
		type ComplianceStandard,
		type GdprEraseResponse,
		type GdprExportPayload,
		type GovernanceTemplate,
		type GovernanceTemplateApplication,
		type SensitiveDataScanResponse,
	} from '$lib/api/audit';
	import { notifications } from '$lib/stores/notifications';

	type EventFilterDraft = {
		source_service: string;
		subject_id: string;
		classification: string;
	};

	type EventDraft = {
		source_service: string;
		channel: string;
		actor: string;
		action: string;
		resource_type: string;
		resource_id: string;
		status: AuditEventStatus;
		severity: AuditSeverity;
		classification: ClassificationLevel;
		subject_id: string;
		ip_address: string;
		location: string;
		labels_text: string;
		metadata_text: string;
		retention_days: string;
	};

	type PolicyDraft = {
		id?: string;
		name: string;
		description: string;
		scope: string;
		classification: ClassificationLevel;
		retention_days: string;
		legal_hold: boolean;
		purge_mode: string;
		active: boolean;
		rules_text: string;
		updated_by: string;
	};

	type ReportDraft = {
		standard: ComplianceStandard;
		title: string;
		scope: string;
		window_start: string;
		window_end: string;
	};

	type GdprDraft = {
		subject_id: string;
		portable_format: string;
		hard_delete: boolean;
		legal_hold: boolean;
	};

	type GovernanceTemplateDraft = {
		scope: string;
		updated_by: string;
		scan_text: string;
	};

	let overview = $state<AuditOverview | null>(null);
	let events = $state<AuditEvent[]>([]);
	let collectors = $state<CollectorStatus[]>([]);
	let anomalies = $state<AnomalyAlert[]>([]);
	let policies = $state<AuditPolicy[]>([]);
	let reports = $state<ComplianceReport[]>([]);
	let classifications = $state<ClassificationCatalogEntry[]>([]);
	let governanceTemplates = $state<GovernanceTemplate[]>([]);
	let governanceApplications = $state<GovernanceTemplateApplication[]>([]);
	let compliancePosture = $state<CompliancePostureOverview | null>(null);
	let exportPayload = $state<GdprExportPayload | null>(null);
	let eraseResponse = $state<GdprEraseResponse | null>(null);
	let scanResult = $state<SensitiveDataScanResponse | null>(null);
	let selectedPolicyId = $state('');
	let loading = $state(true);
	let busyAction = $state('');
	let uiError = $state('');
	let filters = $state<EventFilterDraft>(createEmptyFilters());
	let eventDraft = $state<EventDraft>(createEmptyEventDraft());
	let policyDraft = $state<PolicyDraft>(createEmptyPolicyDraft());
	let reportDraft = $state<ReportDraft>(createEmptyReportDraft());
	let gdprDraft = $state<GdprDraft>(createEmptyGdprDraft());
	let governanceTemplateDraft = $state<GovernanceTemplateDraft>(createEmptyGovernanceTemplateDraft());

	const busy = $derived(loading || busyAction.length > 0);

	onMount(() => {
		void refreshAll();
	});

	function createEmptyFilters(): EventFilterDraft {
		return {
			source_service: '',
			subject_id: '',
			classification: '',
		};
	}

	function createEmptyEventDraft(): EventDraft {
		return {
			source_service: 'gateway',
			channel: 'http',
			actor: 'system:gateway',
			action: 'request.forwarded',
			resource_type: 'http_request',
			resource_id: '/api/v1/apps',
			status: 'success',
			severity: 'low',
			classification: 'confidential',
			subject_id: 'subject-demo-3',
			ip_address: '10.0.0.14',
			location: 'Madrid',
			labels_text: 'manual-probe, phase4',
			metadata_text: JSON.stringify({ method: 'GET', route: '/api/v1/apps', origin: 'audit-console' }, null, 2),
			retention_days: '365',
		};
	}

	function createEmptyPolicyDraft(): PolicyDraft {
		return {
			name: 'Access and export retention',
			description: 'Retain sensitive access and export audit events with legal hold support.',
			scope: 'production-platform',
			classification: 'pii',
			retention_days: '730',
			legal_hold: true,
			purge_mode: 'redact-then-retain-hash',
			active: true,
			rules_text: 'mask subject payloads on erasure\npreserve hash chain\nweekly legal hold review',
			updated_by: 'Security Governance',
		};
	}

	function createEmptyReportDraft(): ReportDraft {
		const now = new Date();
		const start = new Date(now.getTime() - 1000 * 60 * 60 * 24 * 30);
		return {
			standard: 'soc2',
			title: 'SOC2 Monthly Evidence Pack',
			scope: 'production-platform',
			window_start: toLocalDateTime(start),
			window_end: toLocalDateTime(now),
		};
	}

	function createEmptyGdprDraft(): GdprDraft {
		return {
			subject_id: 'subject-demo-1',
			portable_format: 'json',
			hard_delete: false,
			legal_hold: false,
		};
	}

	function createEmptyGovernanceTemplateDraft(): GovernanceTemplateDraft {
		return {
			scope: 'production-platform',
			updated_by: 'Security Governance',
			scan_text: 'Customer SSN 123-45-6789 exported to analytics bucket',
		};
	}

	function toLocalDateTime(date: Date) {
		const pad = (value: number) => String(value).padStart(2, '0');
		return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}T${pad(date.getHours())}:${pad(date.getMinutes())}`;
	}

	function parseCsv(value: string) {
		return value.split(',').map((entry) => entry.trim()).filter(Boolean);
	}

	function parseLines(value: string) {
		return value.split('\n').map((entry) => entry.trim()).filter(Boolean);
	}

	function parseJson<T>(value: string): T {
		return JSON.parse(value) as T;
	}

	function policyToDraft(policy: AuditPolicy): PolicyDraft {
		return {
			id: policy.id,
			name: policy.name,
			description: policy.description,
			scope: policy.scope,
			classification: policy.classification,
			retention_days: String(policy.retention_days),
			legal_hold: policy.legal_hold,
			purge_mode: policy.purge_mode,
			active: policy.active,
			rules_text: policy.rules.join('\n'),
			updated_by: policy.updated_by,
		};
	}

	function updateFilters(patch: Partial<EventFilterDraft>) {
		filters = { ...filters, ...patch };
	}

	function updateEventDraft(patch: Partial<EventDraft>) {
		eventDraft = { ...eventDraft, ...patch };
	}

	function updatePolicyDraft(patch: Partial<PolicyDraft>) {
		policyDraft = { ...policyDraft, ...patch };
	}

	function updateReportDraft(patch: Partial<ReportDraft>) {
		reportDraft = { ...reportDraft, ...patch };
	}

	function updateGdprDraft(patch: Partial<GdprDraft>) {
		gdprDraft = { ...gdprDraft, ...patch };
	}

	function updateGovernanceTemplateDraft(patch: Partial<GovernanceTemplateDraft>) {
		governanceTemplateDraft = { ...governanceTemplateDraft, ...patch };
	}

	async function refreshAll() {
		loading = true;
		uiError = '';
		try {
			const [
				overviewResponse,
				eventResponse,
				collectorsResponse,
				anomaliesResponse,
				policiesResponse,
				reportsResponse,
				classificationsResponse,
				governanceTemplateResponse,
				governanceApplicationResponse,
				compliancePostureResponse
			] = await Promise.all([
				getOverview(),
				listEvents(filters),
				listCollectors(),
				listAnomalies(),
				listPolicies(),
				listReports(),
				listClassifications(),
				listGovernanceTemplates(),
				listGovernanceApplications(),
				getCompliancePosture(),
			]);

			overview = overviewResponse;
			events = eventResponse.items;
			collectors = collectorsResponse;
			anomalies = anomaliesResponse;
			policies = policiesResponse.items;
			reports = reportsResponse.items;
			classifications = classificationsResponse;
			governanceTemplates = governanceTemplateResponse;
			governanceApplications = governanceApplicationResponse.items;
			compliancePosture = compliancePostureResponse;

			if (selectedPolicyId) {
				const selected = policies.find((policy) => policy.id === selectedPolicyId);
				if (selected) policyDraft = policyToDraft(selected);
			}
		} catch (error) {
			uiError = error instanceof Error ? error.message : 'Unable to load audit surfaces';
			notifications.error(uiError);
		} finally {
			loading = false;
		}
	}

	async function applyFilters() {
		busyAction = 'filters';
		try {
			const response = await listEvents(filters);
			events = response.items;
			notifications.success(`Loaded ${response.items.length} audit events`);
		} catch (error) {
			uiError = error instanceof Error ? error.message : 'Unable to apply audit filters';
			notifications.error(uiError);
		} finally {
			busyAction = '';
		}
	}

	async function appendEventAction() {
		busyAction = 'append-event';
		try {
			await appendEvent({
				source_service: eventDraft.source_service,
				channel: eventDraft.channel,
				actor: eventDraft.actor,
				action: eventDraft.action,
				resource_type: eventDraft.resource_type,
				resource_id: eventDraft.resource_id,
				status: eventDraft.status,
				severity: eventDraft.severity,
				classification: eventDraft.classification,
				subject_id: eventDraft.subject_id || null,
				ip_address: eventDraft.ip_address || null,
				location: eventDraft.location || null,
				metadata: parseJson<Record<string, unknown>>(eventDraft.metadata_text),
				labels: parseCsv(eventDraft.labels_text),
				retention_days: Number(eventDraft.retention_days),
			});
			await refreshAll();
			notifications.success('Appended audit event to immutable log');
		} catch (error) {
			uiError = error instanceof Error ? error.message : 'Unable to append audit event';
			notifications.error(uiError);
		} finally {
			busyAction = '';
		}
	}

	function selectPolicy(policyId: string) {
		selectedPolicyId = policyId;
		const policy = policies.find((entry) => entry.id === policyId);
		policyDraft = policy ? policyToDraft(policy) : createEmptyPolicyDraft();
	}

	async function savePolicy() {
		busyAction = 'policy';
		try {
			const payload = {
				name: policyDraft.name,
				description: policyDraft.description,
				scope: policyDraft.scope,
				classification: policyDraft.classification,
				retention_days: Number(policyDraft.retention_days),
				legal_hold: policyDraft.legal_hold,
				purge_mode: policyDraft.purge_mode,
				active: policyDraft.active,
				rules: parseLines(policyDraft.rules_text),
				updated_by: policyDraft.updated_by,
			};
			const policy = policyDraft.id ? await updatePolicy(policyDraft.id, payload) : await createPolicy(payload);
			selectedPolicyId = policy.id;
			await refreshAll();
			notifications.success(`${policyDraft.id ? 'Updated' : 'Created'} ${policy.name}`);
		} catch (error) {
			uiError = error instanceof Error ? error.message : 'Unable to save audit policy';
			notifications.error(uiError);
		} finally {
			busyAction = '';
		}
	}

	async function generateReportAction() {
		busyAction = 'report';
		try {
			await generateReport({
				standard: reportDraft.standard,
				title: reportDraft.title,
				scope: reportDraft.scope,
				window_start: new Date(reportDraft.window_start).toISOString(),
				window_end: new Date(reportDraft.window_end).toISOString(),
			});
			await refreshAll();
			notifications.success(`Generated ${reportDraft.standard} evidence pack`);
		} catch (error) {
			uiError = error instanceof Error ? error.message : 'Unable to generate compliance report';
			notifications.error(uiError);
		} finally {
			busyAction = '';
		}
	}

	async function exportSubjectAction() {
		busyAction = 'gdpr-export';
		try {
			exportPayload = await exportSubjectData({
				subject_id: gdprDraft.subject_id,
				portable_format: gdprDraft.portable_format,
			});
			notifications.success(`Exported data for ${gdprDraft.subject_id}`);
		} catch (error) {
			uiError = error instanceof Error ? error.message : 'Unable to export subject data';
			notifications.error(uiError);
		} finally {
			busyAction = '';
		}
	}

	async function eraseSubjectAction() {
		busyAction = 'gdpr-erase';
		try {
			eraseResponse = await eraseSubjectData({
				subject_id: gdprDraft.subject_id,
				hard_delete: gdprDraft.hard_delete,
				legal_hold: gdprDraft.legal_hold,
			});
			await refreshAll();
			notifications.success(`Processed erasure workflow for ${gdprDraft.subject_id}`);
		} catch (error) {
			uiError = error instanceof Error ? error.message : 'Unable to process subject erasure';
			notifications.error(uiError);
		} finally {
			busyAction = '';
		}
	}

	async function applyGovernanceTemplateAction(slug: string) {
		busyAction = `template-${slug}`;
		try {
			await applyGovernanceTemplate(slug, {
				scope: governanceTemplateDraft.scope,
				updated_by: governanceTemplateDraft.updated_by,
			});
			await refreshAll();
			notifications.success(`Applied governance template ${slug}`);
		} catch (error) {
			uiError = error instanceof Error ? error.message : 'Unable to apply governance template';
			notifications.error(uiError);
		} finally {
			busyAction = '';
		}
	}

	async function runSensitiveDataScanAction() {
		busyAction = 'sds-scan';
		try {
			scanResult = await scanSensitiveData({
				content: governanceTemplateDraft.scan_text,
				redact: true,
			});
			notifications.success('Sensitive data scan completed');
		} catch (error) {
			uiError = error instanceof Error ? error.message : 'Unable to run sensitive data scan';
			notifications.error(uiError);
		} finally {
			busyAction = '';
		}
	}
</script>

<div class="space-y-6">
	<section class="overflow-hidden rounded-[2rem] bg-gradient-to-br from-teal-950 via-stone-950 to-fuchsia-950 px-6 py-6 text-stone-50 shadow-xl shadow-teal-950/20">
		<div class="flex flex-col gap-6 lg:flex-row lg:items-end lg:justify-between">
			<div class="max-w-3xl">
				<p class="text-xs font-semibold uppercase tracking-[0.28em] text-teal-300">Milestone 4.5</p>
				<h1 class="mt-3 text-3xl font-semibold tracking-tight">Immutable audit, compliance evidence, GDPR workflows, and anomaly response</h1>
				<p class="mt-3 text-sm leading-6 text-stone-300">Operate the audit chain, collector health, retention policies, evidence pack generation, and subject-centric export/erasure from one workspace.</p>
			</div>
			<div class="rounded-2xl bg-white/10 px-4 py-4 backdrop-blur">
				<p class="text-xs uppercase tracking-[0.18em] text-teal-200">Latest event</p>
				<p class="mt-2 text-sm font-semibold">{overview?.latest_event?.action ?? 'No events yet'}</p>
				<p class="mt-1 text-xs text-stone-300">{overview?.latest_event?.source_service ?? 'n/a'}</p>
			</div>
		</div>
	</section>

	{#if uiError}
		<div class="rounded-2xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-700">{uiError}</div>
	{/if}

	<ComplianceDashboard {overview} {collectors} {anomalies} {reports} />

	<GovernanceStudio
		templates={governanceTemplates}
		applications={governanceApplications}
		posture={compliancePosture}
		{scanResult}
		draft={governanceTemplateDraft}
		{busy}
		onDraftChange={updateGovernanceTemplateDraft}
		onApplyTemplate={applyGovernanceTemplateAction}
		onScan={runSensitiveDataScanAction}
	/>

	<div class="grid gap-6 xl:grid-cols-[1.05fr_0.95fr]">
		<AuditLogViewer {events} {classifications} {filters} draft={eventDraft} {busy} onFilterChange={updateFilters} onApplyFilters={applyFilters} onDraftChange={updateEventDraft} onAppendEvent={appendEventAction} />
		<AuditTimeline {events} />
	</div>

	<div class="grid gap-6 xl:grid-cols-[0.98fr_1.02fr]">
		<PolicyManager {policies} {classifications} {selectedPolicyId} draft={policyDraft} {busy} onSelectPolicy={selectPolicy} onDraftChange={updatePolicyDraft} onSave={savePolicy} onReset={() => {
			selectedPolicyId = '';
			policyDraft = createEmptyPolicyDraft();
		}} />
		<ExportWizard {reports} {busy} reportDraft={reportDraft} gdprDraft={gdprDraft} {exportPayload} {eraseResponse} onReportDraftChange={updateReportDraft} onGdprDraftChange={updateGdprDraft} onGenerateReport={generateReportAction} onExportSubject={exportSubjectAction} onEraseSubject={eraseSubjectAction} />
	</div>
</div>
