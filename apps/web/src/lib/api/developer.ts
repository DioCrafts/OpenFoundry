import api from './client';

export interface ListResponse<T> {
	items: T[];
}

export type IntegrationProvider = 'github' | 'gitlab';

export interface RepositoryIntegration {
	id: string;
	repository_id: string;
	provider: IntegrationProvider;
	external_namespace: string;
	external_project: string;
	external_url: string;
	sync_mode: string;
	ci_trigger_strategy: string;
	status: string;
	default_branch: string;
	branch_mapping: string[];
	webhook_url: string;
	last_synced_at: string | null;
	created_at: string;
	updated_at: string;
}

export interface ExternalSyncRun {
	id: string;
	integration_id: string;
	repository_id: string;
	trigger: string;
	status: string;
	commit_sha: string;
	branch_name: string;
	summary: string;
	checks: string[];
	started_at: string;
	completed_at: string | null;
}

export interface IntegrationDetail {
	integration: RepositoryIntegration;
	sync_runs: ExternalSyncRun[];
}

export interface CreateIntegrationRequest {
	repository_id: string;
	provider: IntegrationProvider;
	external_namespace: string;
	external_project: string;
	external_url: string;
	sync_mode: string;
	ci_trigger_strategy: string;
	default_branch: string;
	branch_mapping?: string[];
	webhook_url: string;
}

export interface UpdateIntegrationRequest {
	external_namespace?: string;
	external_project?: string;
	external_url?: string;
	sync_mode?: string;
	ci_trigger_strategy?: string;
	status?: string;
	default_branch?: string;
	branch_mapping?: string[];
	webhook_url?: string;
}

export interface TriggerSyncRequest {
	trigger: string;
	commit_sha: string;
	branch_name: string;
}

export interface OpenApiSpec {
	openapi: string;
	info: {
		title: string;
		version: string;
		description: string;
	};
	paths: Record<string, Record<string, OpenApiOperation>>;
	components: {
		schemas: Record<string, Record<string, unknown>>;
	};
}

export interface OpenApiOperation {
	summary: string;
	operationId: string;
	tags: string[];
	requestBody?: {
		required: boolean;
		content: Record<string, OpenApiMediaType>;
	};
	responses: Record<string, OpenApiResponse>;
}

export interface OpenApiMediaType {
	schema: Record<string, unknown>;
}

export interface OpenApiResponse {
	description: string;
	content: Record<string, OpenApiMediaType>;
}

export interface TerraformProviderSchema {
	provider: {
		name: string;
		version: string;
		configuration: Record<string, string>;
	};
	resources: TerraformSchemaEntry[];
	data_sources: TerraformSchemaEntry[];
}

export interface TerraformSchemaEntry {
	name: string;
	description: string;
	attributes: Record<string, string>;
}

export function listIntegrations(repositoryId?: string) {
	const search = repositoryId ? `?repository_id=${encodeURIComponent(repositoryId)}` : '';
	return api.get<ListResponse<RepositoryIntegration>>(`/code-repos/integrations${search}`);
}

export function getIntegration(id: string) {
	return api.get<IntegrationDetail>(`/code-repos/integrations/${id}`);
}

export function createIntegration(body: CreateIntegrationRequest) {
	return api.post<RepositoryIntegration>('/code-repos/integrations', body);
}

export function updateIntegration(id: string, body: UpdateIntegrationRequest) {
	return api.patch<RepositoryIntegration>(`/code-repos/integrations/${id}`, body);
}

export function triggerIntegrationSync(id: string, body: TriggerSyncRequest) {
	return api.post<ExternalSyncRun>(`/code-repos/integrations/${id}/sync`, body);
}

export function loadOpenApiSpec() {
	return loadStaticJson<OpenApiSpec>('/generated/openapi/openfoundry.json');
}

export function loadTerraformProviderSchema() {
	return loadStaticJson<TerraformProviderSchema>('/generated/terraform/openfoundry-provider.json');
}

async function loadStaticJson<T>(path: string): Promise<T> {
	const response = await fetch(path);
	if (!response.ok) {
		throw new Error(`Unable to load static asset: ${path}`);
	}
	return response.json() as Promise<T>;
}