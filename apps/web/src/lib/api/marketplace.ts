import api from './client';

export interface ListResponse<T> {
	items: T[];
}

export type PackageType = 'connector' | 'transform' | 'widget' | 'app_template' | 'ml_model' | 'ai_agent';

export interface CategoryDefinition {
	slug: string;
	name: string;
	description: string;
	listing_count: number;
}

export interface ListingDefinition {
	id: string;
	name: string;
	slug: string;
	summary: string;
	description: string;
	publisher: string;
	category_slug: string;
	package_kind: PackageType;
	repository_slug: string;
	visibility: string;
	tags: string[];
	capabilities: string[];
	install_count: number;
	average_rating: number;
	created_at: string;
	updated_at: string;
}

export interface DependencyRequirement {
	package_slug: string;
	version_req: string;
	required: boolean;
}

export interface PackagedResource {
	kind: string;
	name: string;
	resource_ref: string;
	source_branch?: string | null;
	required: boolean;
}

export interface MaintenanceWindow {
	timezone: string;
	days: string[];
	start_hour_utc: number;
	duration_minutes: number;
}

export interface DeploymentCell {
	name: string;
	cloud: string;
	region: string;
	workspace_targets: string[];
	traffic_weight: number;
	status: string;
	sovereign_boundary?: string | null;
}

export interface ResidencyPolicy {
	mode: string;
	allowed_regions: string[];
	failover_regions: string[];
	require_same_sovereign_boundary: boolean;
}

export interface PromotionGateSummary {
	total: number;
	passed: number;
	blocking: number;
}

export interface PackageVersion {
	id: string;
	listing_id: string;
	version: string;
	release_channel: string;
	changelog: string;
	dependency_mode: string;
	dependencies: DependencyRequirement[];
	packaged_resources: PackagedResource[];
	manifest: Record<string, unknown>;
	published_at: string;
}

export interface ListingReview {
	id: string;
	listing_id: string;
	author: string;
	rating: number;
	headline: string;
	body: string;
	recommended: boolean;
	created_at: string;
}

export interface InstallRecord {
	id: string;
	listing_id: string;
	listing_name: string;
	version: string;
	release_channel: string;
	workspace_name: string;
	status: string;
	dependency_plan: DependencyRequirement[];
	activation: {
		kind: string;
		status: string;
		resource_id: string | null;
		resource_slug: string | null;
		public_url: string | null;
		notes: string | null;
	};
	fleet_id: string | null;
	fleet_name: string | null;
	auto_upgrade_enabled: boolean;
	maintenance_window: MaintenanceWindow | null;
	enrollment_branch: string | null;
	installed_at: string;
	ready_at: string | null;
}

export interface ProductFleetRecord {
	id: string;
	listing_id: string;
	listing_name: string;
	name: string;
	environment: string;
	workspace_targets: string[];
	release_channel: string;
	auto_upgrade_enabled: boolean;
	maintenance_window: MaintenanceWindow;
	branch_strategy: string;
	rollout_strategy: string;
	deployment_cells: DeploymentCell[];
	residency_policy: ResidencyPolicy;
	promotion_gate_summary: PromotionGateSummary;
	status: string;
	install_count: number;
	current_version: string | null;
	target_version: string | null;
	pending_upgrade_count: number;
	last_synced_at: string | null;
	created_at: string;
	updated_at: string;
}

export interface FleetSyncResult {
	fleet: ProductFleetRecord;
	target_version: string | null;
	upgraded_workspaces: string[];
	skipped_workspaces: string[];
	blocked_workspaces: string[];
	workspace_cell_assignments: Record<string, string>;
	blocking_gates: string[];
	blocked_reason: string | null;
	generated_at: string;
}

export interface PromotionGateRecord {
	id: string;
	fleet_id: string;
	fleet_name: string;
	name: string;
	gate_kind: string;
	required: boolean;
	status: string;
	evidence: Record<string, unknown>;
	notes: string;
	last_evaluated_at: string | null;
	created_at: string;
	updated_at: string;
}

export interface EnrollmentBranchRecord {
	id: string;
	fleet_id: string;
	fleet_name: string;
	listing_id: string;
	listing_name: string;
	name: string;
	repository_branch: string;
	source_release_channel: string;
	source_version: string | null;
	workspace_targets: string[];
	status: string;
	notes: string;
	created_at: string;
	updated_at: string;
}

export interface ListingDetail {
	listing: ListingDefinition;
	latest_version: PackageVersion | null;
	versions: PackageVersion[];
	reviews: ListingReview[];
}

export interface MarketplaceOverview {
	listing_count: number;
	category_count: number;
	featured: ListingDefinition[];
	total_installs: number;
}

export type SearchHit = [ListingDefinition, number];

export interface SearchResponse {
	query: string;
	results: SearchHit[];
}

export function getOverview() {
	return api.get<MarketplaceOverview>('/marketplace/overview');
}

export function listCategories() {
	return api.get<ListResponse<CategoryDefinition>>('/marketplace/categories');
}

export function listListings() {
	return api.get<ListResponse<ListingDefinition>>('/marketplace/listings');
}

export function getListing(id: string) {
	return api.get<ListingDetail>(`/marketplace/listings/${id}`);
}

export function searchListings(query: string, category?: string) {
	const params = new URLSearchParams();
	if (query) params.set('q', query);
	if (category) params.set('category', category);
	const search = params.toString();
	return api.get<SearchResponse>(`/marketplace/search${search ? `?${search}` : ''}`);
}

export function createListing(body: {
	name: string;
	slug: string;
	summary: string;
	description?: string;
	publisher: string;
	category_slug: string;
	package_kind: PackageType;
	repository_slug: string;
	visibility?: string;
	tags?: string[];
	capabilities?: string[];
}) {
	return api.post<ListingDefinition>('/marketplace/listings', body);
}

export function updateListing(
	id: string,
	body: Partial<{
		name: string;
		summary: string;
		description: string;
		category_slug: string;
		repository_slug: string;
		visibility: string;
		tags: string[];
		capabilities: string[];
	}>,
) {
	return api.patch<ListingDefinition>(`/marketplace/listings/${id}`, body);
}

export function listVersions(id: string) {
	return api.get<ListResponse<PackageVersion>>(`/marketplace/listings/${id}/versions`);
}

export function publishVersion(
	id: string,
	body: {
		version: string;
		release_channel?: string;
		changelog: string;
		dependency_mode?: string;
		dependencies?: DependencyRequirement[];
		packaged_resources?: PackagedResource[];
		manifest?: Record<string, unknown>;
	},
) {
	return api.post<PackageVersion>(`/marketplace/listings/${id}/versions`, body);
}

export function listReviews(id: string) {
	return api.get<ListResponse<ListingReview>>(`/marketplace/listings/${id}/reviews`);
}

export function createReview(
	id: string,
	body: {
		author: string;
		rating: number;
		headline: string;
		body: string;
		recommended?: boolean;
	},
) {
	return api.post<ListingReview>(`/marketplace/listings/${id}/reviews`, body);
}

export function listInstalls() {
	return api.get<ListResponse<InstallRecord>>('/marketplace/installs');
}

export function createInstall(body: {
	listing_id: string;
	version?: string;
	workspace_name: string;
	release_channel?: string;
	fleet_id?: string | null;
	enrollment_branch?: string | null;
}) {
	return api.post<InstallRecord>('/marketplace/installs', body);
}

export function listFleets() {
	return api.get<ListResponse<ProductFleetRecord>>('/marketplace/devops/fleets');
}

export function createFleet(body: {
	listing_id: string;
	name: string;
	environment?: string;
	workspace_targets: string[];
	release_channel?: string;
	auto_upgrade_enabled?: boolean;
	maintenance_window?: MaintenanceWindow;
	branch_strategy?: string;
	rollout_strategy?: string;
	deployment_cells?: DeploymentCell[];
	residency_policy?: ResidencyPolicy;
}) {
	return api.post<ProductFleetRecord>('/marketplace/devops/fleets', body);
}

export function syncFleet(id: string, body?: { force?: boolean }) {
	return api.post<FleetSyncResult>(`/marketplace/devops/fleets/${id}/sync`, body ?? {});
}

export function listPromotionGates(fleetId: string) {
	return api.get<ListResponse<PromotionGateRecord>>(`/marketplace/devops/fleets/${fleetId}/promotion-gates`);
}

export function createPromotionGate(
	fleetId: string,
	body: {
		name: string;
		gate_kind: string;
		required?: boolean;
		status?: string;
		evidence?: Record<string, unknown>;
		notes?: string;
	},
) {
	return api.post<PromotionGateRecord>(`/marketplace/devops/fleets/${fleetId}/promotion-gates`, body);
}

export function updatePromotionGate(
	id: string,
	body: Partial<{
		required: boolean;
		status: string;
		evidence: Record<string, unknown>;
		notes: string;
	}>,
) {
	return api.patch<PromotionGateRecord>(`/marketplace/devops/promotion-gates/${id}`, body);
}

export function listEnrollmentBranches() {
	return api.get<ListResponse<EnrollmentBranchRecord>>('/marketplace/devops/branches');
}

export function createEnrollmentBranch(body: {
	fleet_id: string;
	name: string;
	repository_branch?: string | null;
	notes?: string;
}) {
	return api.post<EnrollmentBranchRecord>('/marketplace/devops/branches', body);
}
