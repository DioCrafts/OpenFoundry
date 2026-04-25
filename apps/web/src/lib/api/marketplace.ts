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

export interface PackageVersion {
	id: string;
	listing_id: string;
	version: string;
	changelog: string;
	dependency_mode: string;
	dependencies: DependencyRequirement[];
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
	installed_at: string;
	ready_at: string | null;
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
		changelog: string;
		dependency_mode?: string;
		dependencies?: DependencyRequirement[];
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

export function createInstall(body: { listing_id: string; version: string; workspace_name: string }) {
	return api.post<InstallRecord>('/marketplace/installs', body);
}
