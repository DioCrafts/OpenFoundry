import api from './client';

export interface ListResponse<T> {
	items: T[];
}

export type RepositoryVisibility = 'public' | 'private';
export type PackageKind = 'connector' | 'transform' | 'widget' | 'app_template' | 'ml_model' | 'ai_agent';
export type MergeRequestStatus = 'open' | 'approved' | 'merged' | 'closed';

export interface RepositoryDefinition {
	id: string;
	name: string;
	slug: string;
	description: string;
	owner: string;
	default_branch: string;
	visibility: RepositoryVisibility;
	object_store_backend: string;
	package_kind: PackageKind;
	tags: string[];
	settings: Record<string, unknown>;
	created_at: string;
	updated_at: string;
}

export interface ReviewerState {
	reviewer: string;
	approved: boolean;
	state: string;
}

export interface MergeRequestDefinition {
	id: string;
	repository_id: string;
	title: string;
	description: string;
	source_branch: string;
	target_branch: string;
	status: MergeRequestStatus;
	author: string;
	labels: string[];
	reviewers: ReviewerState[];
	approvals_required: number;
	changed_files: number;
	created_at: string;
	updated_at: string;
	merged_at: string | null;
}

export interface RepositoryOverview {
	repository_count: number;
	private_repository_count: number;
	package_kind_mix: string[];
	open_merge_request_count: number;
	latest_merge_request: MergeRequestDefinition | null;
}

export interface BranchDefinition {
	id: string;
	repository_id: string;
	name: string;
	head_sha: string;
	base_branch: string | null;
	is_default: boolean;
	protected: boolean;
	ahead_by: number;
	pending_reviews: number;
	updated_at: string;
}

export interface CommitDefinition {
	id: string;
	repository_id: string;
	branch_name: string;
	sha: string;
	parent_sha: string | null;
	title: string;
	description: string;
	author_name: string;
	author_email: string;
	files_changed: number;
	additions: number;
	deletions: number;
	created_at: string;
}

export interface CiRun {
	id: string;
	repository_id: string;
	branch_name: string;
	commit_sha: string;
	pipeline_name: string;
	status: string;
	trigger: string;
	started_at: string;
	completed_at: string | null;
	checks: string[];
}

export interface RepositoryFile {
	id: string;
	repository_id: string;
	path: string;
	branch_name: string;
	language: string;
	size_bytes: number;
	content: string;
	last_commit_sha: string;
}

export interface SearchResult {
	path: string;
	branch_name: string;
	snippet: string;
	score: number;
}

export interface SearchResponse {
	query: string;
	results: SearchResult[];
}

export interface DiffResponse {
	branch_name: string;
	patch: string;
}

export interface ReviewComment {
	id: string;
	merge_request_id: string;
	author: string;
	body: string;
	file_path: string;
	line_number: number | null;
	resolved: boolean;
	created_at: string;
}

export interface MergeRequestDetail {
	merge_request: MergeRequestDefinition;
	comments: ReviewComment[];
	approval_count: number;
	thread_count: number;
}

export function getOverview() {
	return api.get<RepositoryOverview>('/code-repos/overview');
}

export function listRepositories() {
	return api.get<ListResponse<RepositoryDefinition>>('/code-repos/repositories');
}

export function createRepository(body: {
	name: string;
	slug: string;
	description?: string;
	owner: string;
	default_branch: string;
	visibility: RepositoryVisibility;
	object_store_backend: string;
	package_kind: PackageKind;
	tags?: string[];
	settings?: Record<string, unknown>;
}) {
	return api.post<RepositoryDefinition>('/code-repos/repositories', body);
}

export function updateRepository(
	id: string,
	body: Partial<{
		name: string;
		slug: string;
		description: string;
		owner: string;
		default_branch: string;
		visibility: RepositoryVisibility;
		object_store_backend: string;
		package_kind: PackageKind;
		tags: string[];
		settings: Record<string, unknown>;
	}>,
) {
	return api.patch<RepositoryDefinition>(`/code-repos/repositories/${id}`, body);
}

export function listBranches(repositoryId: string) {
	return api.get<ListResponse<BranchDefinition>>(`/code-repos/repositories/${repositoryId}/branches`);
}

export function createBranch(repositoryId: string, body: { name: string; base_branch: string; protected: boolean }) {
	return api.post<BranchDefinition>(`/code-repos/repositories/${repositoryId}/branches`, body);
}

export function listCommits(repositoryId: string) {
	return api.get<ListResponse<CommitDefinition>>(`/code-repos/repositories/${repositoryId}/commits`);
}

export function createCommit(
	repositoryId: string,
	body: {
		branch_name: string;
		title: string;
		description?: string;
		author_name: string;
		additions?: number;
		deletions?: number;
	},
) {
	return api.post<CommitDefinition>(`/code-repos/repositories/${repositoryId}/commits`, body);
}

export function listFiles(repositoryId: string) {
	return api.get<ListResponse<RepositoryFile>>(`/code-repos/repositories/${repositoryId}/files`);
}

export function getDiff(repositoryId: string, branch?: string) {
	const search = branch ? `?branch=${encodeURIComponent(branch)}` : '';
	return api.get<DiffResponse>(`/code-repos/repositories/${repositoryId}/diff${search}`);
}

export function searchFiles(repositoryId: string, query: string) {
	return api.get<SearchResponse>(`/code-repos/repositories/${repositoryId}/search?q=${encodeURIComponent(query)}`);
}

export function listCiRuns(repositoryId: string) {
	return api.get<ListResponse<CiRun>>(`/code-repos/repositories/${repositoryId}/ci`);
}

export function triggerCiRun(repositoryId: string, body: { branch_name: string }) {
	return api.post<CiRun>(`/code-repos/repositories/${repositoryId}/ci`, body);
}

export function listMergeRequests(repositoryId?: string) {
	const search = repositoryId ? `?repository_id=${encodeURIComponent(repositoryId)}` : '';
	return api.get<ListResponse<MergeRequestDefinition>>(`/code-repos/merge-requests${search}`);
}

export function createMergeRequest(body: {
	repository_id: string;
	title: string;
	description?: string;
	source_branch: string;
	target_branch: string;
	author: string;
	labels?: string[];
	reviewers?: ReviewerState[];
	approvals_required?: number;
	changed_files?: number;
}) {
	return api.post<MergeRequestDefinition>('/code-repos/merge-requests', body);
}

export function getMergeRequest(id: string) {
	return api.get<MergeRequestDetail>(`/code-repos/merge-requests/${id}`);
}

export function updateMergeRequest(
	id: string,
	body: Partial<{
		title: string;
		description: string;
		status: MergeRequestStatus;
		labels: string[];
		reviewers: ReviewerState[];
		approvals_required: number;
		changed_files: number;
	}>,
) {
	return api.patch<MergeRequestDefinition>(`/code-repos/merge-requests/${id}`, body);
}

export function listComments(id: string) {
	return api.get<ListResponse<ReviewComment>>(`/code-repos/merge-requests/${id}/comments`);
}

export function createComment(
	id: string,
	body: {
		author: string;
		body: string;
		file_path?: string;
		line_number?: number;
		resolved?: boolean;
	},
) {
	return api.post<ReviewComment>(`/code-repos/merge-requests/${id}/comments`, body);
}
