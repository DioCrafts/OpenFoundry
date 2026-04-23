import api from './client';

export interface ListResponse<T> {
	data: T[];
}

export interface FusionOverview {
	rule_count: number;
	active_job_count: number;
	completed_job_count: number;
	cluster_count: number;
	pending_review_count: number;
	golden_record_count: number;
	auto_merged_cluster_count: number;
}

export interface BlockingStrategyConfig {
	strategy_type: string;
	key_fields: string[];
	window_size: number;
	bucket_count: number;
}

export interface MatchCondition {
	field: string;
	comparator: string;
	weight: number;
	threshold: number;
	required: boolean;
}

export interface MatchRule {
	id: string;
	name: string;
	description: string;
	status: string;
	entity_type: string;
	blocking_strategy: BlockingStrategyConfig;
	conditions: MatchCondition[];
	review_threshold: number;
	auto_merge_threshold: number;
	created_at: string;
	updated_at: string;
}

export interface SurvivorshipRule {
	field: string;
	strategy: string;
	source_priority: string[];
	fallback: string;
}

export interface MergeStrategy {
	id: string;
	name: string;
	description: string;
	status: string;
	entity_type: string;
	default_strategy: string;
	rules: SurvivorshipRule[];
	created_at: string;
	updated_at: string;
}

export interface ResolutionJobConfig {
	source_labels: string[];
	record_count: number;
	blocking_strategy_override: BlockingStrategyConfig | null;
	review_sampling_rate: number;
}

export interface FusionJobMetrics {
	candidate_pairs: number;
	matched_pairs: number;
	review_pairs: number;
	cluster_count: number;
	golden_record_count: number;
	precision_estimate: number;
	recall_estimate: number;
}

export interface FusionJob {
	id: string;
	name: string;
	description: string;
	status: string;
	entity_type: string;
	match_rule_id: string;
	merge_strategy_id: string;
	config: ResolutionJobConfig;
	metrics: FusionJobMetrics;
	last_run_summary: string;
	last_run_at: string | null;
	created_at: string;
	updated_at: string;
}

export interface RunResolutionJobResponse {
	job: FusionJob;
	cluster_ids: string[];
	golden_record_ids: string[];
	review_queue_item_ids: string[];
	executed_at: string;
}

export interface EntityRecord {
	record_id: string;
	source: string;
	external_id: string;
	display_name: string;
	confidence: number;
	attributes: Record<string, unknown>;
}

export interface MatchEvidence {
	left_record_id: string;
	right_record_id: string;
	blocking_key: string;
	rule_score: number;
	ml_score: number;
	final_score: number;
	comparators: string[];
	explanation: string;
	requires_review: boolean;
}

export interface ReviewQueueItem {
	id: string;
	cluster_id: string;
	status: string;
	severity: string;
	recommended_action: string;
	rationale: string[];
	assigned_to: string | null;
	reviewed_by: string | null;
	notes: string;
	created_at: string;
	updated_at: string;
}

export interface ResolvedCluster {
	id: string;
	job_id: string;
	cluster_key: string;
	status: string;
	records: EntityRecord[];
	evidence: MatchEvidence[];
	confidence_score: number;
	requires_review: boolean;
	suggested_golden_record_id: string | null;
	created_at: string;
	updated_at: string;
}

export interface GoldenRecordProvenance {
	field: string;
	source: string;
	external_id: string;
	strategy: string;
}

export interface GoldenRecord {
	id: string;
	cluster_id: string;
	title: string;
	canonical_values: Record<string, unknown>;
	provenance: GoldenRecordProvenance[];
	completeness_score: number;
	confidence_score: number;
	status: string;
	created_at: string;
	updated_at: string;
}

export interface ClusterDetail {
	cluster: ResolvedCluster;
	review_item: ReviewQueueItem | null;
	golden_record: GoldenRecord | null;
}

export function getOverview() {
	return api.get<FusionOverview>('/fusion/overview');
}

export function listRules() {
	return api.get<ListResponse<MatchRule>>('/fusion/rules');
}

export function createRule(body: {
	name: string;
	description?: string;
	status?: string;
	entity_type?: string;
	blocking_strategy?: BlockingStrategyConfig;
	conditions: MatchCondition[];
	review_threshold?: number;
	auto_merge_threshold?: number;
}) {
	return api.post<MatchRule>('/fusion/rules', body);
}

export function updateRule(id: string, body: {
	name?: string;
	description?: string;
	status?: string;
	entity_type?: string;
	blocking_strategy?: BlockingStrategyConfig;
	conditions?: MatchCondition[];
	review_threshold?: number;
	auto_merge_threshold?: number;
}) {
	return api.patch<MatchRule>(`/fusion/rules/${id}`, body);
}

export function listMergeStrategies() {
	return api.get<ListResponse<MergeStrategy>>('/fusion/merge-strategies');
}

export function createMergeStrategy(body: {
	name: string;
	description?: string;
	status?: string;
	entity_type?: string;
	default_strategy?: string;
	rules: SurvivorshipRule[];
}) {
	return api.post<MergeStrategy>('/fusion/merge-strategies', body);
}

export function updateMergeStrategy(id: string, body: {
	name?: string;
	description?: string;
	status?: string;
	entity_type?: string;
	default_strategy?: string;
	rules?: SurvivorshipRule[];
}) {
	return api.patch<MergeStrategy>(`/fusion/merge-strategies/${id}`, body);
}

export function listJobs() {
	return api.get<ListResponse<FusionJob>>('/fusion/jobs');
}

export function createJob(body: {
	name: string;
	description?: string;
	status?: string;
	entity_type?: string;
	match_rule_id: string;
	merge_strategy_id: string;
	config?: ResolutionJobConfig;
}) {
	return api.post<FusionJob>('/fusion/jobs', body);
}

export function runJob(id: string) {
	return api.post<RunResolutionJobResponse>(`/fusion/jobs/${id}/run`, {});
}

export function listClusters() {
	return api.get<ListResponse<ResolvedCluster>>('/fusion/clusters');
}

export function getCluster(id: string) {
	return api.get<ClusterDetail>(`/fusion/clusters/${id}`);
}

export function listReviewQueue() {
	return api.get<ListResponse<ReviewQueueItem>>('/fusion/review-queue');
}

export function submitReview(id: string, body: {
	decision: string;
	notes?: string;
	reviewed_by?: string;
}) {
	return api.post<ClusterDetail>(`/fusion/clusters/${id}/review`, body);
}

export function listGoldenRecords() {
	return api.get<ListResponse<GoldenRecord>>('/fusion/golden-records');
}
