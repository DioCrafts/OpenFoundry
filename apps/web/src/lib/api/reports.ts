import api from './client';

export interface ListResponse<T> {
	items: T[];
}

export type GeneratorKind = 'pdf' | 'excel' | 'csv' | 'html' | 'pptx';
export type ScheduleCadence = 'manual' | 'cron' | 'daily' | 'weekly' | 'monthly';
export type SectionKind = 'kpi' | 'table' | 'chart' | 'narrative' | 'map';
export type DistributionChannel = 'email' | 's3' | 'slack' | 'teams' | 'webhook';

export interface ReportSection {
	id: string;
	title: string;
	kind: SectionKind;
	query: string;
	description: string;
	config: Record<string, unknown>;
}

export interface ReportTemplate {
	title: string;
	subtitle: string;
	theme: string;
	layout: string;
	sections: ReportSection[];
}

export interface ReportSchedule {
	cadence: ScheduleCadence;
	expression: string | null;
	timezone: string;
	anchor_time: string;
	interval_minutes: number | null;
	enabled: boolean;
	next_run_at: string | null;
}

export interface DistributionRecipient {
	id: string;
	channel: DistributionChannel;
	target: string;
	label: string | null;
	config: Record<string, unknown>;
}

export interface DistributionResult {
	channel: DistributionChannel;
	target: string;
	status: string;
	delivered_at: string;
	detail: string;
}

export interface DistributionChannelCatalogEntry {
	channel: DistributionChannel;
	display_name: string;
	description: string;
	configuration_fields: string[];
}

export interface ReportDefinition {
	id: string;
	name: string;
	description: string;
	owner: string;
	generator_kind: GeneratorKind;
	dataset_name: string;
	template: ReportTemplate;
	schedule: ReportSchedule;
	recipients: DistributionRecipient[];
	tags: string[];
	parameters: Record<string, unknown>;
	active: boolean;
	last_generated_at: string | null;
	created_at: string;
	updated_at: string;
}

export interface ReportPreviewHighlight {
	label: string;
	value: string;
	delta: string;
}

export interface ReportPreviewSection {
	section_id: string;
	title: string;
	kind: SectionKind;
	summary: string;
	rows: Array<Record<string, unknown>>;
}

export interface ReportExecutionPreview {
	headline: string;
	generated_for: string;
	engine: string;
	highlights: ReportPreviewHighlight[];
	sections: ReportPreviewSection[];
}

export interface ReportArtifact {
	file_name: string;
	mime_type: string;
	size_bytes: number;
	storage_url: string;
	checksum: string;
}

export interface ReportExecutionMetrics {
	duration_ms: number;
	row_count: number;
	section_count: number;
	recipient_count: number;
}

export interface ReportExecution {
	id: string;
	report_id: string;
	report_name: string;
	status: string;
	generator_kind: GeneratorKind;
	triggered_by: string;
	generated_at: string;
	completed_at: string | null;
	preview: ReportExecutionPreview;
	artifact: ReportArtifact;
	distributions: DistributionResult[];
	metrics: ReportExecutionMetrics;
}

export interface ReportOverview {
	report_count: number;
	active_schedules: number;
	executions_24h: number;
	generator_mix: string[];
	latest_execution: ReportExecution | null;
}

export interface GeneratorCatalogEntry {
	kind: GeneratorKind;
	display_name: string;
	engine: string;
	extensions: string[];
	capabilities: string[];
}

export interface ReportCatalog {
	generators: GeneratorCatalogEntry[];
	delivery_channels: DistributionChannelCatalogEntry[];
}

export interface ScheduledRun {
	report_id: string;
	report_name: string;
	generator_kind: GeneratorKind;
	next_run_at: string;
	recipient_count: number;
	cadence: ScheduleCadence;
}

export interface ScheduleBoard {
	active_schedules: number;
	paused_reports: number;
	upcoming: ScheduledRun[];
	recent_executions: ReportExecution[];
}

export interface DownloadPayload {
	file_name: string;
	mime_type: string;
	storage_url: string;
	preview_excerpt: string;
	report_name: string;
}

export function getOverview() {
	return api.get<ReportOverview>('/reports/overview');
}

export function getCatalog() {
	return api.get<ReportCatalog>('/reports/catalog');
}

export function listReports() {
	return api.get<ListResponse<ReportDefinition>>('/reports/definitions');
}

export function createReport(body: {
	name: string;
	description?: string;
	owner: string;
	generator_kind: GeneratorKind;
	dataset_name: string;
	template: ReportTemplate;
	schedule: ReportSchedule;
	recipients: DistributionRecipient[];
	tags?: string[];
	parameters?: Record<string, unknown>;
	active?: boolean;
}) {
	return api.post<ReportDefinition>('/reports/definitions', body);
}

export function updateReport(
	id: string,
	body: Partial<{
		name: string;
		description: string;
		owner: string;
		generator_kind: GeneratorKind;
		dataset_name: string;
		template: ReportTemplate;
		schedule: ReportSchedule;
		recipients: DistributionRecipient[];
		tags: string[];
		parameters: Record<string, unknown>;
		active: boolean;
	}>,
) {
	return api.patch<ReportDefinition>(`/reports/definitions/${id}`, body);
}

export function generateReport(id: string) {
	return api.post<ReportExecution>(`/reports/definitions/${id}/generate`, {});
}

export function listHistory(id: string) {
	return api.get<ListResponse<ReportExecution>>(`/reports/definitions/${id}/history`);
}

export function getScheduleBoard() {
	return api.get<ScheduleBoard>('/reports/schedules');
}

export function getExecution(id: string) {
	return api.get<ReportExecution>(`/reports/executions/${id}`);
}

export function getDownload(id: string) {
	return api.get<DownloadPayload>(`/reports/executions/${id}/download`);
}
