import api from './client';

export interface AppTheme {
	name: string;
	primary_color: string;
	accent_color: string;
	background_color: string;
	surface_color: string;
	text_color: string;
	heading_font: string;
	body_font: string;
	border_radius: number;
	logo_url: string | null;
}

export interface AppSettings {
	home_page_id: string | null;
	navigation_style: string;
	max_width: string;
	show_branding: boolean;
	custom_css: string | null;
	builder_experience: string;
	consumer_mode: ConsumerModeSettings;
	interactive_workshop: WorkshopInteractiveSettings;
	slate: SlateSettings;
}

export interface ConsumerModeSettings {
	enabled: boolean;
	allow_guest_access: boolean;
	portal_title: string | null;
	portal_subtitle: string | null;
	primary_cta_label: string | null;
	primary_cta_url: string | null;
}

export interface WorkshopScenarioPreset {
	id: string;
	label: string;
	description: string | null;
	parameters: Record<string, string>;
	prompt_template: string | null;
}

export interface WorkshopInteractiveSettings {
	enabled: boolean;
	title: string | null;
	subtitle: string | null;
	briefing_template: string | null;
	primary_scenario_widget_id: string | null;
	primary_agent_widget_id: string | null;
	suggested_questions: string[];
	scenario_presets: WorkshopScenarioPreset[];
}

export interface SlateSettings {
	enabled: boolean;
	framework: string;
	package_name: string;
	entry_file: string;
	sdk_import: string;
	workspace: SlateWorkspaceSettings;
	quiver_embed: QuiverEmbedSettings;
}

export interface SlateWorkspaceSettings {
	enabled: boolean;
	repository_id: string | null;
	layout: string;
	runtime: string;
	dev_command: string;
	preview_command: string;
	files: SlatePackageFile[];
}

export interface QuiverEmbedSettings {
	enabled: boolean;
	primary_type_id: string | null;
	secondary_type_id: string | null;
	join_field: string | null;
	secondary_join_field: string | null;
	date_field: string | null;
	metric_field: string | null;
	group_field: string | null;
	selected_group: string | null;
}

export interface AppWidgetPosition {
	x: number;
	y: number;
	width: number;
	height: number;
}

export interface WidgetBinding {
	source_type: string;
	source_id?: string | null;
	query_text?: string | null;
	path?: string | null;
	fields: string[];
	parameters: Record<string, unknown>;
	limit?: number | null;
}

export interface WidgetEvent {
	id: string;
	trigger: string;
	action: string;
	label?: string | null;
	config: Record<string, unknown>;
}

export interface AppWidget {
	id: string;
	widget_type: string;
	title: string;
	description: string;
	position: AppWidgetPosition;
	props: Record<string, unknown>;
	binding?: WidgetBinding | null;
	events: WidgetEvent[];
	children: AppWidget[];
}

export interface PageLayout {
	kind: string;
	columns: number;
	gap: string;
	max_width: string;
}

export interface AppPage {
	id: string;
	name: string;
	path: string;
	description: string;
	layout: PageLayout;
	widgets: AppWidget[];
	visible: boolean;
}

export interface AppDefinition {
	id: string;
	name: string;
	slug: string;
	description: string;
	status: string;
	pages: AppPage[];
	theme: AppTheme;
	settings: AppSettings;
	template_key: string | null;
	created_by: string | null;
	published_version_id: string | null;
	created_at: string;
	updated_at: string;
}

export interface AppSummary {
	id: string;
	name: string;
	slug: string;
	description: string;
	status: string;
	page_count: number;
	widget_count: number;
	template_key: string | null;
	published_version_id: string | null;
	created_at: string;
	updated_at: string;
}

export interface AppListResponse {
	data: AppSummary[];
	total: number;
}

export interface AppTemplateDefinition {
	pages: AppPage[];
	theme: AppTheme;
	settings: AppSettings;
}

export interface AppTemplate {
	id: string;
	key: string;
	name: string;
	description: string;
	category: string;
	preview_image_url: string | null;
	definition: AppTemplateDefinition;
	created_at: string;
}

export interface AppTemplateListResponse {
	data: AppTemplate[];
}

export interface AppEmbedInfo {
	url: string;
	iframe_html: string;
}

export interface WidgetDefaultSize {
	width: number;
	height: number;
}

export interface WidgetCatalogItem {
	widget_type: string;
	label: string;
	description: string;
	category: string;
	default_props: Record<string, unknown>;
	default_size: WidgetDefaultSize;
	supported_bindings: string[];
	supports_children: boolean;
}

export interface AppPreviewResponse {
	app: AppDefinition;
	widget_catalog: WidgetCatalogItem[];
	embed: AppEmbedInfo;
}

export interface PublishedAppResponse {
	app: AppDefinition;
	embed: AppEmbedInfo;
	published_version_number: number;
	published_at: string;
}

export interface SlatePackageFile {
	path: string;
	language: string;
	content: string;
}

export interface SlatePackageResponse {
	app_id: string;
	app_slug: string;
	framework: string;
	package_name: string;
	entry_file: string;
	sdk_import: string;
	files: SlatePackageFile[];
}

export interface ImportSlatePackageParams {
	package_name?: string | null;
	entry_file?: string | null;
	sdk_import?: string | null;
	framework?: string | null;
	repository_id?: string | null;
	layout?: string | null;
	runtime?: string | null;
	dev_command?: string | null;
	preview_command?: string | null;
	files: SlatePackageFile[];
}

export interface SlateRoundTripResponse {
	app: AppDefinition;
	slate_package: SlatePackageResponse;
}

export interface AppVersionSnapshot {
	name: string;
	slug: string;
	description: string;
	status: string;
	pages: AppPage[];
	theme: AppTheme;
	settings: AppSettings;
	template_key: string | null;
}

export interface AppVersion {
	id: string;
	app_id: string;
	version_number: number;
	status: string;
	app_snapshot: AppVersionSnapshot;
	notes: string;
	created_by: string | null;
	created_at: string;
	published_at: string | null;
}

export interface AppVersionListResponse {
	data: AppVersion[];
}

export interface CreateAppParams {
	name: string;
	slug?: string;
	description?: string;
	status?: string;
	pages?: AppPage[];
	theme?: AppTheme;
	settings?: AppSettings;
	template_key?: string;
}

export interface UpdateAppParams {
	name?: string;
	slug?: string;
	description?: string;
	status?: string;
	pages?: AppPage[];
	theme?: AppTheme;
	settings?: AppSettings;
	template_key?: string;
}

export interface PublishAppParams {
	notes?: string;
}

export function listApps(params?: { page?: number; per_page?: number; search?: string; status?: string }) {
	const query = new URLSearchParams();
	if (params?.page) query.set('page', String(params.page));
	if (params?.per_page) query.set('per_page', String(params.per_page));
	if (params?.search) query.set('search', params.search);
	if (params?.status) query.set('status', params.status);
	const qs = query.toString();
	return api.get<AppListResponse>(`/apps${qs ? `?${qs}` : ''}`);
}

export function listAppTemplates() {
	return api.get<AppTemplateListResponse>('/apps/templates');
}

export function listWidgetCatalog() {
	return api.get<WidgetCatalogItem[]>('/widgets/catalog');
}

export function getApp(id: string) {
	return api.get<AppDefinition>(`/apps/${id}`);
}

export function createApp(body: CreateAppParams) {
	return api.post<AppDefinition>('/apps', body);
}

export function createAppFromTemplate(body: CreateAppParams) {
	return api.post<AppDefinition>('/apps/from-template', body);
}

export function updateApp(id: string, body: UpdateAppParams) {
	return api.patch<AppDefinition>(`/apps/${id}`, body);
}

export function deleteApp(id: string) {
	return api.delete(`/apps/${id}`);
}

export function addPage(appId: string, page: AppPage) {
	return api.post<AppDefinition>(`/apps/${appId}/pages`, page);
}

export function updatePage(appId: string, pageId: string, page: AppPage) {
	return api.patch<AppDefinition>(`/apps/${appId}/pages/${pageId}`, page);
}

export function deletePage(appId: string, pageId: string) {
	return api.delete<AppDefinition>(`/apps/${appId}/pages/${pageId}`);
}

export function previewApp(id: string) {
	return api.get<AppPreviewResponse>(`/apps/${id}/preview`);
}

export function getSlatePackage(id: string) {
	return api.get<SlatePackageResponse>(`/apps/${id}/slate-package`);
}

export function importSlatePackage(id: string, body: ImportSlatePackageParams) {
	return api.post<SlateRoundTripResponse>(`/apps/${id}/slate-package`, body);
}

export function listAppVersions(appId: string) {
	return api.get<AppVersionListResponse>(`/apps/${appId}/versions`);
}

export function publishApp(appId: string, body: PublishAppParams = {}) {
	return api.post<AppVersion>(`/apps/${appId}/publish`, body);
}

export function getPublishedApp(slug: string) {
	return api.get<PublishedAppResponse>(`/apps/public/${encodeURIComponent(slug)}`);
}

export function getAppEmbedInfo(slug: string) {
	return api.get<AppEmbedInfo>(`/apps/public/${encodeURIComponent(slug)}/embed`);
}
