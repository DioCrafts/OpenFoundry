import api from './client';

export interface LoginRequest {
  email: string;
  password: string;
}

export interface RegisterRequest {
  email: string;
  password: string;
  name: string;
}

export interface TokenResponse {
  access_token: string;
  refresh_token: string;
  token_type: string;
  expires_in: number;
}

export interface AuthenticatedResponse extends TokenResponse {
  status: 'authenticated';
}

export interface MfaRequiredResponse {
  status: 'mfa_required';
  challenge_token: string;
  methods: string[];
  expires_in: number;
}

export type LoginResponse = AuthenticatedResponse | MfaRequiredResponse;

export interface UserProfile {
  id: string;
  email: string;
  name: string;
  is_active: boolean;
  roles: string[];
  groups: string[];
  permissions: string[];
  organization_id: string | null;
  attributes: Record<string, unknown>;
  mfa_enabled: boolean;
  mfa_enforced: boolean;
  auth_source: string;
  created_at: string;
}

export interface PermissionRecord {
  id: string;
  resource: string;
  action: string;
  description: string | null;
  created_at: string;
}

export interface RoleRecord {
  id: string;
  name: string;
  description: string | null;
  created_at: string;
  permission_ids: string[];
  permissions: string[];
}

export interface GroupRecord {
  id: string;
  name: string;
  description: string | null;
  created_at: string;
  member_count: number;
  role_ids: string[];
  roles: string[];
}

export interface PolicyRecord {
  id: string;
  name: string;
  description: string | null;
  effect: string;
  resource: string;
  action: string;
  conditions: Record<string, unknown>;
  row_filter: string | null;
  enabled: boolean;
  created_by: string | null;
  created_at: string;
  updated_at: string;
}

export interface PolicyEvaluationResult {
  allowed: boolean;
  matched_policy_ids: string[];
  deny_policy_ids: string[];
  row_filter: string | null;
}

export interface ApiKeyRecord {
  id: string;
  user_id: string;
  name: string;
  prefix: string;
  scopes: string[];
  expires_at: string | null;
  last_used_at: string | null;
  revoked_at: string | null;
  created_at: string;
}

export interface ApiKeyWithSecret {
  id: string;
  name: string;
  prefix: string;
  token: string;
  scopes: string[];
  expires_at: string | null;
  created_at: string;
}

export interface MfaStatusResponse {
  configured: boolean;
  enabled: boolean;
  recovery_codes_remaining: number;
}

export interface MfaEnrollmentResponse {
  secret: string;
  recovery_codes: string[];
  otpauth_uri: string;
}

export interface PublicSsoProvider {
  id: string;
  slug: string;
  name: string;
  provider_type: string;
}

export interface SsoProviderRecord {
  id: string;
  slug: string;
  name: string;
  provider_type: string;
  enabled: boolean;
  client_id: string | null;
  client_secret_configured: boolean;
  issuer_url: string | null;
  authorization_url: string | null;
  token_url: string | null;
  userinfo_url: string | null;
  scopes: string[];
  saml_metadata_url: string | null;
  saml_entity_id: string | null;
  saml_sso_url: string | null;
  attribute_mapping: Record<string, unknown>;
  created_at: string;
  updated_at: string;
}

export interface StartSsoLoginResponse {
  authorization_url: string;
}

export interface SessionScope {
  allowed_methods: string[];
  allowed_path_prefixes: string[];
  allowed_subject_ids: string[];
  allowed_org_ids: string[];
  workspace: string | null;
  classification_clearance: string | null;
  guest_email: string | null;
  guest_display_name: string | null;
}

export interface ScopedSessionRecord {
  id: string;
  user_id: string;
  label: string;
  session_kind: 'scoped' | 'guest';
  scope: SessionScope;
  guest_email: string | null;
  guest_name: string | null;
  expires_at: string;
  revoked_at: string | null;
  created_at: string;
}

export interface ScopedSessionWithToken {
  id: string;
  label: string;
  session_kind: 'scoped' | 'guest';
  scope: SessionScope;
  token: string;
  expires_at: string;
  guest_email: string | null;
  guest_name: string | null;
  created_at: string;
}

export interface HashContentResponse {
  algorithm: string;
  digest: string;
}

export interface SignContentResponse {
  algorithm: string;
  signature: string;
}

export interface VerifySignatureResponse {
  algorithm: string;
  valid: boolean;
}

export function login(data: LoginRequest) {
  return api.post<LoginResponse>('/auth/login', data);
}

export function register(data: RegisterRequest) {
  return api.post<{ id: string; email: string; name: string }>('/auth/register', data);
}

export function refreshToken(refresh_token: string) {
  return api.post<TokenResponse>('/auth/refresh', { refresh_token });
}

export function getMe() {
  return api.get<UserProfile>('/users/me');
}

export function completeMfaLogin(data: { challenge_token: string; code: string }) {
  return api.post<TokenResponse>('/auth/mfa/complete', data);
}

export function getMfaStatus() {
  return api.get<MfaStatusResponse>('/auth/mfa');
}

export function enrollMfa() {
  return api.post<MfaEnrollmentResponse>('/auth/mfa/enroll', {});
}

export function verifyMfaSetup(data: { code: string }) {
  return api.post<{ enabled: boolean }>('/auth/mfa/verify', data);
}

export function disableMfa(data: { code: string }) {
  return api.fetch<void>('/auth/mfa', { method: 'DELETE', body: data });
}

export function listPublicSsoProviders() {
  return api.get<PublicSsoProvider[]>('/auth/sso/providers/public');
}

export function startSsoLogin(slug: string) {
  return api.get<StartSsoLoginResponse>(`/auth/sso/providers/${slug}/start`);
}

export function completeSsoLogin(data: {
  code?: string;
  state?: string;
  saml_response?: string;
  relay_state?: string;
}) {
  return api.post<LoginResponse>('/auth/sso/callback', data);
}

export function listUsers() {
  return api.get<UserProfile[]>('/users');
}

export function listScopedSessions() {
  return api.get<ScopedSessionRecord[]>('/auth/sessions');
}

export function createScopedSession(data: {
  label: string;
  permissions?: string[];
  allowed_methods?: string[];
  allowed_path_prefixes?: string[];
  allowed_subject_ids?: string[];
  allowed_org_ids?: string[];
  workspace?: string | null;
  classification_clearance?: string | null;
  expires_at?: string | null;
}) {
  return api.post<ScopedSessionWithToken>('/auth/sessions/scoped', data);
}

export function createGuestSession(data: {
  label: string;
  guest_email: string;
  guest_name?: string | null;
  permissions?: string[];
  allowed_methods?: string[];
  allowed_path_prefixes?: string[];
  allowed_subject_ids?: string[];
  allowed_org_ids?: string[];
  workspace?: string | null;
  classification_clearance?: string | null;
  expires_at?: string | null;
}) {
  return api.post<ScopedSessionWithToken>('/auth/sessions/guest', data);
}

export function revokeScopedSession(id: string) {
  return api.fetch<void>(`/auth/sessions/${id}`, { method: 'DELETE' });
}

export function hashCipherContent(data: { content: string; salt?: string | null }) {
  return api.post<HashContentResponse>('/auth/cipher/hash', data);
}

export function signCipherContent(data: { content: string; key_material: string }) {
  return api.post<SignContentResponse>('/auth/cipher/sign', data);
}

export function verifyCipherSignature(data: { content: string; key_material: string; signature: string }) {
  return api.post<VerifySignatureResponse>('/auth/cipher/verify', data);
}

export function updateUser(userId: string, data: Partial<Pick<UserProfile, 'name' | 'organization_id' | 'attributes' | 'mfa_enforced' | 'is_active'>>) {
  return api.patch<UserProfile>(`/users/${userId}`, data);
}

export function deactivateUser(userId: string) {
  return api.delete<void>(`/users/${userId}`);
}

export function listPermissions() {
  return api.get<PermissionRecord[]>('/permissions');
}

export function createPermission(data: { resource: string; action: string; description?: string | null }) {
  return api.post<PermissionRecord>('/permissions', data);
}

export function listRoles() {
  return api.get<RoleRecord[]>('/roles');
}

export function createRole(data: { name: string; description?: string | null; permission_ids: string[] }) {
  return api.post<RoleRecord>('/roles', data);
}

export function updateRole(roleId: string, data: { description?: string | null; permission_ids: string[] }) {
  return api.put<RoleRecord>(`/roles/${roleId}`, data);
}

export function assignUserRole(userId: string, role_id: string) {
  return api.post<void>(`/users/${userId}/roles`, { role_id });
}

export function removeUserRole(userId: string, roleId: string) {
  return api.delete<void>(`/users/${userId}/roles/${roleId}`);
}

export function listGroups() {
  return api.get<GroupRecord[]>('/groups');
}

export function createGroup(data: { name: string; description?: string | null; role_ids: string[] }) {
  return api.post<GroupRecord>('/groups', data);
}

export function updateGroup(groupId: string, data: { description?: string | null; role_ids: string[] }) {
  return api.put<GroupRecord>(`/groups/${groupId}`, data);
}

export function addUserToGroup(userId: string, group_id: string) {
  return api.post<void>(`/users/${userId}/groups`, { group_id });
}

export function removeUserFromGroup(userId: string, groupId: string) {
  return api.delete<void>(`/users/${userId}/groups/${groupId}`);
}

export function listPolicies() {
  return api.get<PolicyRecord[]>('/policies');
}

export function createPolicy(data: {
  name: string;
  description?: string | null;
  effect: string;
  resource: string;
  action: string;
  conditions: Record<string, unknown>;
  row_filter?: string | null;
  enabled: boolean;
}) {
  return api.post<PolicyRecord>('/policies', data);
}

export function updatePolicy(
  policyId: string,
  data: {
    name: string;
    description?: string | null;
    effect: string;
    resource: string;
    action: string;
    conditions: Record<string, unknown>;
    row_filter?: string | null;
    enabled: boolean;
  },
) {
  return api.put<PolicyRecord>(`/policies/${policyId}`, data);
}

export function deletePolicy(policyId: string) {
  return api.delete<void>(`/policies/${policyId}`);
}

export function evaluatePolicy(data: {
  resource: string;
  action: string;
  resource_attributes: Record<string, unknown>;
}) {
  return api.post<PolicyEvaluationResult>('/policies/evaluate', data);
}

export function listApiKeys() {
  return api.get<ApiKeyRecord[]>('/api-keys');
}

export function createApiKey(data: { name: string; scopes: string[]; expires_at?: string | null }) {
  return api.post<ApiKeyWithSecret>('/api-keys', data);
}

export function revokeApiKey(apiKeyId: string) {
  return api.delete<void>(`/api-keys/${apiKeyId}`);
}

export function listSsoProviders() {
  return api.get<SsoProviderRecord[]>('/auth/sso/providers');
}

export function createSsoProvider(data: {
  slug: string;
  name: string;
  provider_type: string;
  enabled: boolean;
  client_id?: string | null;
  client_secret?: string | null;
  issuer_url?: string | null;
  authorization_url?: string | null;
  token_url?: string | null;
  userinfo_url?: string | null;
  scopes: string[];
  saml_metadata_url?: string | null;
  saml_entity_id?: string | null;
  saml_sso_url?: string | null;
  saml_certificate?: string | null;
  attribute_mapping: Record<string, unknown>;
}) {
  return api.post<SsoProviderRecord>('/auth/sso/providers', data);
}

export function updateSsoProvider(
  providerId: string,
  data: {
    slug: string;
    name: string;
    provider_type: string;
    enabled: boolean;
    client_id?: string | null;
    client_secret?: string | null;
    issuer_url?: string | null;
    authorization_url?: string | null;
    token_url?: string | null;
    userinfo_url?: string | null;
    scopes: string[];
    saml_metadata_url?: string | null;
    saml_entity_id?: string | null;
    saml_sso_url?: string | null;
    saml_certificate?: string | null;
    attribute_mapping: Record<string, unknown>;
  },
) {
  return api.put<SsoProviderRecord>(`/auth/sso/providers/${providerId}`, data);
}

export function deleteSsoProvider(providerId: string) {
  return api.delete<void>(`/auth/sso/providers/${providerId}`);
}
