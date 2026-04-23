import api from './client';

export interface ListResponse<T> {
	items: T[];
}

export interface NexusOverview {
	peer_count: number;
	active_peer_count: number;
	contract_count: number;
	active_contract_count: number;
	share_count: number;
	federated_access_count: number;
	encrypted_share_count: number;
	replication_ready_count: number;
	pending_schema_reviews: number;
	audit_bridge_status: string;
	latest_sync_at: string | null;
}

export interface PeerOrganization {
	id: string;
	slug: string;
	display_name: string;
	region: string;
	endpoint_url: string;
	auth_mode: string;
	trust_level: string;
	public_key_fingerprint: string;
	shared_scopes: string[];
	status: string;
	last_handshake_at: string | null;
	created_at: string;
	updated_at: string;
}

export interface SharingContract {
	id: string;
	peer_id: string;
	name: string;
	description: string;
	dataset_locator: string;
	allowed_purposes: string[];
	data_classes: string[];
	residency_region: string;
	query_template: string;
	max_rows_per_query: number;
	replication_mode: string;
	encryption_profile: string;
	retention_days: number;
	status: string;
	signed_at: string | null;
	expires_at: string;
	created_at: string;
	updated_at: string;
}

export interface AccessGrant {
	id: string;
	share_id: string;
	peer_id: string;
	query_template: string;
	max_rows_per_query: number;
	can_replicate: boolean;
	allowed_purposes: string[];
	expires_at: string;
	issued_at: string;
}

export interface SyncStatus {
	id: string;
	share_id: string;
	mode: string;
	status: string;
	rows_replicated: number;
	backlog_rows: number;
	encrypted_in_transit: boolean;
	encrypted_at_rest: boolean;
	key_version: string;
	last_sync_at: string | null;
	next_sync_at: string | null;
	audit_cursor: string;
	updated_at: string;
}

export interface EncryptionPosture {
	share_id: string;
	transport_cipher: string;
	at_rest_cipher: string;
	key_version: string;
	profile: string;
	encrypted_in_transit: boolean;
	encrypted_at_rest: boolean;
	recommendation: string;
}

export interface SchemaCompatibilityReport {
	share_id: string;
	compatible: boolean;
	missing_fields: string[];
	type_mismatches: string[];
	reviewed_at: string;
	summary: string;
}

export interface SharedDataset {
	id: string;
	contract_id: string;
	provider_peer_id: string;
	consumer_peer_id: string;
	dataset_name: string;
	selector: Record<string, unknown>;
	provider_schema: Record<string, unknown>;
	consumer_schema: Record<string, unknown>;
	sample_rows: Record<string, unknown>[];
	replication_mode: string;
	status: string;
	last_sync_at: string | null;
	created_at: string;
	updated_at: string;
}

export interface ShareDetail {
	share: SharedDataset;
	access_grant: AccessGrant | null;
	sync_status: SyncStatus | null;
	encryption: EncryptionPosture;
	compatibility: SchemaCompatibilityReport;
}

export interface ReplicationPlan {
	share_id: string;
	dataset_name: string;
	mode: string;
	status: string;
	rows_replicated: number;
	backlog_rows: number;
	next_sync_at: string | null;
	selective_filter: Record<string, unknown>;
	encrypted: boolean;
}

export interface AuditBridgeEntry {
	share_id: string;
	dataset_name: string;
	peer_name: string;
	contract_name: string;
	audit_cursor: string;
	last_sync_at: string | null;
	status: string;
	evidence: string[];
}

export interface AuditBridgeSummary {
	bridge_status: string;
	entry_count: number;
	latest_cursor: string;
	entries: AuditBridgeEntry[];
}

export interface FederatedQueryResult {
	share_id: string;
	dataset_name: string;
	source_peer: string;
	executed_sql: string;
	query_mode: string;
	limit: number;
	columns: string[];
	rows: Record<string, unknown>[];
}

export function getOverview() {
	return api.get<NexusOverview>('/nexus/overview');
}

export function listPeers() {
	return api.get<ListResponse<PeerOrganization>>('/nexus/peers');
}

export function createPeer(body: {
	slug: string;
	display_name: string;
	region: string;
	endpoint_url: string;
	auth_mode: string;
	trust_level: string;
	public_key_fingerprint: string;
	shared_scopes?: string[];
}) {
	return api.post<PeerOrganization>('/nexus/peers', body);
}

export function authenticatePeer(id: string) {
	return api.post<PeerOrganization>(`/nexus/peers/${id}/authenticate`, {});
}

export function listContracts() {
	return api.get<ListResponse<SharingContract>>('/nexus/contracts');
}

export function createContract(body: {
	peer_id: string;
	name: string;
	description: string;
	dataset_locator: string;
	allowed_purposes?: string[];
	data_classes?: string[];
	residency_region: string;
	query_template: string;
	max_rows_per_query: number;
	replication_mode: string;
	encryption_profile: string;
	retention_days: number;
	status: string;
	expires_at: string;
}) {
	return api.post<SharingContract>('/nexus/contracts', body);
}

export function updateContract(id: string, body: Partial<{
	name: string;
	description: string;
	dataset_locator: string;
	allowed_purposes: string[];
	data_classes: string[];
	residency_region: string;
	query_template: string;
	max_rows_per_query: number;
	replication_mode: string;
	encryption_profile: string;
	retention_days: number;
	status: string;
	expires_at: string;
}>) {
	return api.patch<SharingContract>(`/nexus/contracts/${id}`, body);
}

export function listShares() {
	return api.get<ListResponse<ShareDetail>>('/nexus/shares');
}

export function createShare(body: {
	contract_id: string;
	provider_peer_id: string;
	consumer_peer_id: string;
	dataset_name: string;
	selector?: Record<string, unknown>;
	provider_schema: Record<string, unknown>;
	consumer_schema: Record<string, unknown>;
	sample_rows?: Record<string, unknown>[];
	replication_mode: string;
}) {
	return api.post<ShareDetail>('/nexus/shares', body);
}

export function updateShare(id: string, body: Partial<{
	dataset_name: string;
	selector: Record<string, unknown>;
	consumer_schema: Record<string, unknown>;
	sample_rows: Record<string, unknown>[];
	replication_mode: string;
	status: string;
}>) {
	return api.patch<ShareDetail>(`/nexus/shares/${id}`, body);
}

export function runFederatedQuery(body: {
	share_id: string;
	sql: string;
	purpose: string;
	limit?: number;
}) {
	return api.post<FederatedQueryResult>('/nexus/federation/query', body);
}

export function listReplicationPlans() {
	return api.get<ListResponse<ReplicationPlan>>('/nexus/replication/plans');
}

export function listSchemaCompatibility() {
	return api.get<ListResponse<SchemaCompatibilityReport>>('/nexus/schema-compatibility');
}

export function getAuditBridge() {
	return api.get<AuditBridgeSummary>('/nexus/audit-bridge');
}
