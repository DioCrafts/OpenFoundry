import api from './client';

export interface ObjectType {
  id: string;
  name: string;
  display_name: string;
  description: string;
  primary_key_property: string | null;
  icon: string | null;
  color: string | null;
  owner_id: string;
  created_at: string;
  updated_at: string;
}

export interface Property {
  id: string;
  object_type_id: string;
  name: string;
  display_name: string;
  description: string;
  property_type: string;
  required: boolean;
  unique_constraint: boolean;
  time_dependent: boolean;
  default_value: unknown;
  validation_rules: unknown;
  created_at: string;
  updated_at: string;
}

export interface LinkType {
  id: string;
  name: string;
  display_name: string;
  description: string;
  source_type_id: string;
  target_type_id: string;
  cardinality: string;
  owner_id: string;
  created_at: string;
  updated_at: string;
}

export interface ObjectInstance {
  id: string;
  object_type_id: string;
  properties: Record<string, unknown>;
  created_by: string;
  organization_id?: string | null;
  marking?: string;
  created_at: string;
  updated_at: string;
}

export interface LinkInstance {
  id: string;
  link_type_id: string;
  source_object_id: string;
  target_object_id: string;
  properties: Record<string, unknown> | null;
  created_by: string;
  created_at: string;
}

export interface NeighborLink {
  direction: 'inbound' | 'outbound';
  link_id: string;
  link_type_id: string;
  link_name: string;
  object: ObjectInstance;
}

export interface SearchResult {
  kind: string;
  id: string;
  object_type_id: string | null;
  title: string;
  subtitle: string | null;
  snippet: string;
  score: number;
  route: string;
  metadata: Record<string, unknown>;
}

export interface GraphNode {
  id: string;
  kind: string;
  label: string;
  secondary_label: string | null;
  color: string | null;
  route: string | null;
  metadata: Record<string, unknown>;
}

export interface GraphEdge {
  id: string;
  kind: string;
  source: string;
  target: string;
  label: string;
  metadata: Record<string, unknown>;
}

export interface GraphResponse {
  mode: string;
  root_object_id: string | null;
  root_type_id: string | null;
  depth: number;
  total_nodes: number;
  total_edges: number;
  nodes: GraphNode[];
  edges: GraphEdge[];
}

export type ActionOperationKind =
  | 'update_object'
  | 'create_link'
  | 'delete_object'
  | 'invoke_function'
  | 'invoke_webhook';

export interface ActionInputField {
  name: string;
  display_name?: string | null;
  description?: string | null;
  property_type: string;
  required: boolean;
  default_value?: unknown;
}

export interface ActionType {
  id: string;
  name: string;
  display_name: string;
  description: string;
  object_type_id: string;
  operation_kind: ActionOperationKind;
  input_schema: ActionInputField[];
  config: unknown;
  confirmation_required: boolean;
  permission_key: string | null;
  owner_id: string;
  created_at: string;
  updated_at: string;
}

export interface ValidateActionResponse {
  valid: boolean;
  errors: string[];
  preview: unknown;
}

export interface ExecuteActionResponse {
  action: ActionType;
  target_object_id: string | null;
  deleted: boolean;
  preview: unknown;
  object: unknown | null;
  link: unknown | null;
  result: unknown | null;
}

export interface FunctionCapabilities {
  allow_ontology_read: boolean;
  allow_ontology_write: boolean;
  allow_ai: boolean;
  allow_network: boolean;
  timeout_seconds: number;
  max_source_bytes: number;
}

export interface FunctionPackageSummary {
  id: string;
  name: string;
  display_name: string;
  runtime: string;
  entrypoint: string;
  capabilities: FunctionCapabilities;
}

export interface FunctionPackage {
  id: string;
  name: string;
  display_name: string;
  description: string;
  runtime: string;
  source: string;
  entrypoint: string;
  capabilities: FunctionCapabilities;
  owner_id: string;
  created_at: string;
  updated_at: string;
}

export type RuleEvaluationMode = 'advisory' | 'automatic';

export interface RuleTriggerSpec {
  equals?: Record<string, unknown>;
  numeric_gte?: Record<string, number>;
  numeric_lte?: Record<string, number>;
  exists?: string[];
  changed_properties?: string[];
  markings?: string[];
}

export interface RuleAlertSpec {
  severity: 'low' | 'medium' | 'high' | 'critical';
  title: string;
  message?: string | null;
}

export interface RuleScheduleSpec {
  property_name: string;
  offset_hours: number;
}

export interface RuleEffectSpec {
  object_patch?: Record<string, unknown> | null;
  schedule?: RuleScheduleSpec | null;
  alert?: RuleAlertSpec | null;
}

export interface OntologyRule {
  id: string;
  name: string;
  display_name: string;
  description: string;
  object_type_id: string;
  evaluation_mode: RuleEvaluationMode;
  trigger_spec: RuleTriggerSpec;
  effect_spec: RuleEffectSpec;
  owner_id: string;
  created_at: string;
  updated_at: string;
}

export interface RuleMatchResponse {
  rule_id: string;
  matched: boolean;
  trigger_payload: Record<string, unknown>;
  effect_preview: Record<string, unknown> | null;
}

export interface OntologyRuleRun {
  id: string;
  rule_id: string;
  object_id: string;
  matched: boolean;
  simulated: boolean;
  trigger_payload: Record<string, unknown>;
  effect_preview: Record<string, unknown> | null;
  created_by: string;
  created_at: string;
}

export interface MachineryInsight {
  rule_id: string;
  name: string;
  display_name: string;
  evaluation_mode: RuleEvaluationMode;
  matched_runs: number;
  total_runs: number;
  pending_schedules: number;
  last_matched_at: string | null;
  last_object_id: string | null;
}

export interface ObjectViewResponse {
  object: ObjectInstance;
  summary: Record<string, unknown>;
  neighbors: NeighborLink[];
  graph: GraphResponse;
  applicable_actions: ActionType[];
  matching_rules: RuleMatchResponse[];
  recent_rule_runs: OntologyRuleRun[];
  timeline: Array<Record<string, unknown>>;
}

export interface ObjectSimulationResponse {
  before: ObjectInstance;
  after: ObjectInstance | null;
  deleted: boolean;
  action_preview: Record<string, unknown>;
  matching_rules: RuleMatchResponse[];
  graph: GraphResponse;
  impacted_objects: string[];
  timeline: Array<Record<string, unknown>>;
}

export interface CreateActionTypeBody {
  name: string;
  display_name?: string;
  description?: string;
  object_type_id: string;
  operation_kind: ActionOperationKind;
  input_schema?: ActionInputField[];
  config?: unknown;
  confirmation_required?: boolean;
  permission_key?: string;
}

export interface UpdateActionTypeBody {
  display_name?: string;
  description?: string;
  operation_kind?: ActionOperationKind;
  input_schema?: ActionInputField[];
  config?: unknown;
  confirmation_required?: boolean;
  permission_key?: string;
}

// Object Types
export function listObjectTypes(params?: { page?: number; per_page?: number; search?: string }) {
  const qs = new URLSearchParams();
  if (params?.page) qs.set('page', String(params.page));
  if (params?.per_page) qs.set('per_page', String(params.per_page));
  if (params?.search) qs.set('search', params.search);
  return api.get<{ data: ObjectType[]; total: number; page: number; per_page: number }>(
    `/ontology/types?${qs}`,
  );
}

export function getObjectType(id: string) {
  return api.get<ObjectType>(`/ontology/types/${id}`);
}

export function createObjectType(body: {
  name: string;
  display_name?: string;
  description?: string;
  icon?: string;
  color?: string;
}) {
  return api.post<ObjectType>('/ontology/types', body);
}

export function updateObjectType(id: string, body: {
  display_name?: string;
  description?: string;
  icon?: string;
  color?: string;
}) {
  return api.put<ObjectType>(`/ontology/types/${id}`, body);
}

export function deleteObjectType(id: string) {
  return api.delete(`/ontology/types/${id}`);
}

export function searchOntology(body: {
  query: string;
  kind?: string;
  object_type_id?: string;
  limit?: number;
  semantic?: boolean;
}) {
  return api.post<{ query: string; total: number; data: SearchResult[] }>('/ontology/search', body);
}

export function getOntologyGraph(params?: {
  root_object_id?: string;
  root_type_id?: string;
  depth?: number;
  limit?: number;
}) {
  const qs = new URLSearchParams();
  if (params?.root_object_id) qs.set('root_object_id', params.root_object_id);
  if (params?.root_type_id) qs.set('root_type_id', params.root_type_id);
  if (params?.depth) qs.set('depth', String(params.depth));
  if (params?.limit) qs.set('limit', String(params.limit));
  return api.get<GraphResponse>(`/ontology/graph?${qs}`);
}

// Action Types
export function listActionTypes(params?: {
  object_type_id?: string;
  page?: number;
  per_page?: number;
  search?: string;
}) {
  const qs = new URLSearchParams();
  if (params?.object_type_id) qs.set('object_type_id', params.object_type_id);
  if (params?.page) qs.set('page', String(params.page));
  if (params?.per_page) qs.set('per_page', String(params.per_page));
  if (params?.search) qs.set('search', params.search);
  return api.get<{ data: ActionType[]; total: number; page: number; per_page: number }>(
    `/ontology/actions?${qs}`,
  );
}

export function getActionType(id: string) {
  return api.get<ActionType>(`/ontology/actions/${id}`);
}

export function createActionType(body: CreateActionTypeBody) {
  return api.post<ActionType>('/ontology/actions', body);
}

export function updateActionType(id: string, body: UpdateActionTypeBody) {
  return api.put<ActionType>(`/ontology/actions/${id}`, body);
}

export function deleteActionType(id: string) {
  return api.delete(`/ontology/actions/${id}`);
}

export function validateAction(id: string, body: {
  target_object_id?: string;
  parameters?: Record<string, unknown>;
}) {
  return api.post<ValidateActionResponse>(`/ontology/actions/${id}/validate`, body);
}

export function executeAction(id: string, body: {
  target_object_id?: string;
  parameters?: Record<string, unknown>;
}) {
  return api.post<ExecuteActionResponse>(`/ontology/actions/${id}/execute`, body);
}

export function listFunctionPackages(params?: {
  runtime?: string;
  search?: string;
  page?: number;
  per_page?: number;
}) {
  const qs = new URLSearchParams();
  if (params?.runtime) qs.set('runtime', params.runtime);
  if (params?.search) qs.set('search', params.search);
  if (params?.page) qs.set('page', String(params.page));
  if (params?.per_page) qs.set('per_page', String(params.per_page));
  return api.get<{ data: FunctionPackage[]; total: number; page: number; per_page: number }>(
    `/ontology/functions?${qs}`,
  );
}

export function createFunctionPackage(body: {
  name: string;
  display_name?: string;
  description?: string;
  runtime: string;
  source: string;
  entrypoint?: string;
  capabilities?: Partial<FunctionCapabilities>;
}) {
  return api.post<FunctionPackage>('/ontology/functions', body);
}

export function validateFunctionPackage(id: string, body: {
  object_type_id?: string;
  target_object_id?: string;
  parameters?: Record<string, unknown>;
  justification?: string;
}) {
  return api.post<{
    valid: boolean;
    package: FunctionPackageSummary;
    preview: Record<string, unknown>;
    errors: string[];
  }>(`/ontology/functions/${id}/validate`, body);
}

export function simulateFunctionPackage(id: string, body: {
  object_type_id: string;
  target_object_id?: string;
  parameters?: Record<string, unknown>;
  justification?: string;
}) {
  return api.post<{
    package: FunctionPackageSummary;
    preview: Record<string, unknown>;
    result: Record<string, unknown>;
  }>(`/ontology/functions/${id}/simulate`, body);
}

export function listRules(params?: {
  object_type_id?: string;
  search?: string;
  page?: number;
  per_page?: number;
}) {
  const qs = new URLSearchParams();
  if (params?.object_type_id) qs.set('object_type_id', params.object_type_id);
  if (params?.search) qs.set('search', params.search);
  if (params?.page) qs.set('page', String(params.page));
  if (params?.per_page) qs.set('per_page', String(params.per_page));
  return api.get<{ data: OntologyRule[]; total: number; page: number; per_page: number }>(
    `/ontology/rules?${qs}`,
  );
}

export function createRule(body: {
  name: string;
  display_name?: string;
  description?: string;
  object_type_id: string;
  evaluation_mode?: RuleEvaluationMode;
  trigger_spec?: RuleTriggerSpec;
  effect_spec?: RuleEffectSpec;
}) {
  return api.post<OntologyRule>('/ontology/rules', body);
}

export function simulateRule(id: string, body: {
  object_id: string;
  properties_patch?: Record<string, unknown>;
}) {
  return api.post<{
    rule: OntologyRule;
    matched: boolean;
    trigger_payload: Record<string, unknown>;
    effect_preview: Record<string, unknown> | null;
    object: ObjectInstance;
  }>(`/ontology/rules/${id}/simulate`, body);
}

export function applyRule(id: string, body: {
  object_id: string;
  properties_patch?: Record<string, unknown>;
}) {
  return api.post<{
    rule: OntologyRule;
    matched: boolean;
    trigger_payload: Record<string, unknown>;
    effect_preview: Record<string, unknown> | null;
    object: ObjectInstance;
  }>(`/ontology/rules/${id}/apply`, body);
}

export function getMachineryInsights(params?: { object_type_id?: string }) {
  const qs = new URLSearchParams();
  if (params?.object_type_id) qs.set('object_type_id', params.object_type_id);
  return api.get<{ object_type_id: string | null; data: MachineryInsight[] }>(
    `/ontology/rules/insights?${qs}`,
  );
}

export function getObjectView(typeId: string, objectId: string) {
  return api.get<ObjectViewResponse>(`/ontology/types/${typeId}/objects/${objectId}/view`);
}

export function simulateObject(
  typeId: string,
  objectId: string,
  body: {
    action_id?: string;
    action_parameters?: Record<string, unknown>;
    properties_patch?: Record<string, unknown>;
    depth?: number;
  },
) {
  return api.post<ObjectSimulationResponse>(`/ontology/types/${typeId}/objects/${objectId}/simulate`, body);
}

// Properties
export function listProperties(typeId: string) {
  return api
    .get<{ data: Property[] }>(`/ontology/types/${typeId}/properties`)
    .then((response) => response.data);
}

export function createProperty(typeId: string, body: {
  name: string;
  display_name?: string;
  description?: string;
  property_type: string;
  required?: boolean;
  unique_constraint?: boolean;
}) {
  return api.post<Property>(`/ontology/types/${typeId}/properties`, body);
}

// Link Types
export function listLinkTypes(params?: { object_type_id?: string; page?: number; per_page?: number }) {
  const qs = new URLSearchParams();
  if (params?.object_type_id) qs.set('object_type_id', params.object_type_id);
  if (params?.page) qs.set('page', String(params.page));
  if (params?.per_page) qs.set('per_page', String(params.per_page));
  return api.get<{ data: LinkType[]; total: number }>(`/ontology/links?${qs}`);
}

export function createLinkType(body: {
  name: string;
  display_name?: string;
  description?: string;
  source_type_id: string;
  target_type_id: string;
  cardinality?: string;
}) {
  return api.post<LinkType>('/ontology/links', body);
}

export function deleteLinkType(id: string) {
  return api.delete(`/ontology/links/${id}`);
}

// Object Instances
export function listObjects(typeId: string, params?: { page?: number; per_page?: number }) {
  const qs = new URLSearchParams();
  if (params?.page) qs.set('page', String(params.page));
  if (params?.per_page) qs.set('per_page', String(params.per_page));
  return api.get<{ data: ObjectInstance[]; total: number; page?: number; per_page?: number }>(
    `/ontology/types/${typeId}/objects?${qs}`,
  );
}

export function getObject(typeId: string, objectId: string) {
  return api.get<ObjectInstance>(`/ontology/types/${typeId}/objects/${objectId}`);
}

export function updateObject(
  typeId: string,
  objectId: string,
  body: { properties: Record<string, unknown>; replace?: boolean; marking?: string },
) {
  return api.patch<ObjectInstance>(`/ontology/types/${typeId}/objects/${objectId}`, body);
}

export function queryObjects(
  typeId: string,
  body: { equals?: Record<string, unknown>; limit?: number },
) {
  return api.post<{ data: ObjectInstance[]; total: number }>(
    `/ontology/types/${typeId}/objects/query`,
    body,
  );
}

export function listNeighbors(typeId: string, objectId: string) {
  return api
    .get<{ data: NeighborLink[] }>(`/ontology/types/${typeId}/objects/${objectId}/neighbors`)
    .then((response) => response.data);
}

export function createLinkInstance(
  linkTypeId: string,
  body: { source_object_id: string; target_object_id: string; properties?: Record<string, unknown> },
) {
  return api.post<LinkInstance>(`/ontology/links/${linkTypeId}/instances`, body);
}

export function createObject(typeId: string, properties: Record<string, unknown>) {
  return api.post<ObjectInstance>(`/ontology/types/${typeId}/objects`, { properties });
}

export function deleteObject(typeId: string, objectId: string) {
  return api.delete(`/ontology/types/${typeId}/objects/${objectId}`);
}
