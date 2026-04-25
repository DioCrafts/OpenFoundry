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

export interface SharedPropertyType {
  id: string;
  name: string;
  display_name: string;
  description: string;
  property_type: string;
  required: boolean;
  unique_constraint: boolean;
  time_dependent: boolean;
  default_value: unknown;
  validation_rules: unknown;
  owner_id: string;
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

export interface GraphSummary {
  scope: string;
  node_kinds: Record<string, number>;
  edge_kinds: Record<string, number>;
  object_types: Record<string, number>;
  markings: Record<string, number>;
  root_neighbor_count: number;
  max_hops_reached: number;
  boundary_crossings: number;
  sensitive_objects: number;
  sensitive_markings: string[];
}

export interface GraphResponse {
  mode: string;
  root_object_id: string | null;
  root_type_id: string | null;
  depth: number;
  total_nodes: number;
  total_edges: number;
  summary: GraphSummary;
  nodes: GraphNode[];
  edges: GraphEdge[];
}

export interface ObjectSetPolicy {
  allowed_markings: string[];
  minimum_clearance: string | null;
  deny_guest_sessions: boolean;
  required_restricted_view_id: string | null;
}

export interface ObjectSetFilter {
  field: string;
  operator: string;
  value: unknown;
}

export interface ObjectSetTraversal {
  direction: 'outbound' | 'inbound' | 'both';
  link_type_id: string | null;
  target_object_type_id: string | null;
  max_hops: number;
}

export interface ObjectSetJoin {
  secondary_object_type_id: string;
  left_field: string;
  right_field: string;
  join_kind: 'inner' | 'left';
}

export interface ObjectSetDefinition {
  id: string;
  name: string;
  description: string;
  base_object_type_id: string;
  filters: ObjectSetFilter[];
  traversals: ObjectSetTraversal[];
  join: ObjectSetJoin | null;
  projections: string[];
  what_if_label: string | null;
  policy: ObjectSetPolicy;
  materialized_snapshot: unknown[] | null;
  materialized_at: string | null;
  materialized_row_count: number;
  owner_id: string;
  created_at: string;
  updated_at: string;
}

export interface ObjectSetEvaluationResponse {
  object_set: ObjectSetDefinition;
  total_base_matches: number;
  total_rows: number;
  traversal_neighbor_count: number;
  rows: Record<string, unknown>[];
  generated_at: string;
  materialized: boolean;
}

export type QuiverChartKind = 'line' | 'area' | 'bar' | 'point';

export interface QuiverVisualFunction {
  id: string;
  name: string;
  description: string;
  primary_type_id: string;
  secondary_type_id: string | null;
  join_field: string;
  secondary_join_field: string;
  date_field: string;
  metric_field: string;
  group_field: string;
  selected_group: string | null;
  chart_kind: QuiverChartKind;
  shared: boolean;
  vega_spec: Record<string, unknown>;
  owner_id: string;
  created_at: string;
  updated_at: string;
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

export interface ActionAuthorizationPolicy {
  required_permission_keys?: string[];
  any_role?: string[];
  all_roles?: string[];
  attribute_equals?: Record<string, unknown>;
  allowed_markings?: string[];
  minimum_clearance?: string | null;
  deny_guest_sessions?: boolean;
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
  authorization_policy: ActionAuthorizationPolicy;
  owner_id: string;
  created_at: string;
  updated_at: string;
}

export interface ActionWhatIfBranch {
  id: string;
  action_id: string;
  target_object_id: string | null;
  name: string;
  description: string;
  parameters: Record<string, unknown>;
  preview: Record<string, unknown>;
  before_object: Record<string, unknown> | null;
  after_object: Record<string, unknown> | null;
  deleted: boolean;
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
  priority_score?: number;
  estimated_duration_minutes?: number;
  required_capability?: string | null;
  constraint_tags?: string[];
  hard_deadline_hours?: number | null;
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
  overdue_schedules: number;
  avg_schedule_lead_hours: number | null;
  dynamic_pressure: string;
  last_matched_at: string | null;
  last_object_id: string | null;
}

export interface MachineryQueueItem {
  id: string;
  rule_id: string;
  rule_run_id: string;
  object_id: string;
  rule_name: string;
  rule_display_name: string;
  object_type_id: string;
  status: string;
  scheduled_for: string;
  priority_score: number;
  estimated_duration_minutes: number;
  required_capability: string | null;
  constraint_snapshot: Record<string, unknown>;
  created_by: string;
  created_at: string;
  updated_at: string;
  started_at: string | null;
  completed_at: string | null;
}

export interface MachineryCapabilityLoad {
  capability: string;
  pending_count: number;
  total_estimated_minutes: number;
}

export interface MachineryQueueRecommendation {
  generated_at: string;
  strategy: string;
  queue_depth: number;
  overdue_count: number;
  total_estimated_minutes: number;
  next_due_at: string | null;
  recommended_order: string[];
  capability_load: MachineryCapabilityLoad[];
}

export interface MachineryQueueResponse {
  object_type_id: string | null;
  data: MachineryQueueItem[];
  recommendation: MachineryQueueRecommendation;
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
  impact_summary: {
    scope: string;
    action_kind: string;
    predicted_delete: boolean;
    impacted_object_count: number;
    impacted_type_count: number;
    impacted_types: string[];
    direct_neighbors: number;
    max_hops_reached: number;
    boundary_crossings: number;
    sensitive_objects: number;
    sensitive_markings: string[];
    matching_rules: number;
    changed_properties: string[];
  };
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
  authorization_policy?: ActionAuthorizationPolicy;
}

export interface UpdateActionTypeBody {
  display_name?: string;
  description?: string;
  operation_kind?: ActionOperationKind;
  input_schema?: ActionInputField[];
  config?: unknown;
  confirmation_required?: boolean;
  permission_key?: string;
  authorization_policy?: ActionAuthorizationPolicy;
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

export function listQuiverVisualFunctions(params?: {
  page?: number;
  per_page?: number;
  search?: string;
  include_shared?: boolean;
}) {
  const qs = new URLSearchParams();
  if (params?.page) qs.set('page', String(params.page));
  if (params?.per_page) qs.set('per_page', String(params.per_page));
  if (params?.search) qs.set('search', params.search);
  if (typeof params?.include_shared === 'boolean') {
    qs.set('include_shared', params.include_shared ? 'true' : 'false');
  }
  return api.get<{ data: QuiverVisualFunction[]; total: number; page: number; per_page: number }>(
    `/ontology/quiver/visual-functions?${qs}`,
  );
}

export function getQuiverVisualFunction(id: string) {
  return api.get<QuiverVisualFunction>(`/ontology/quiver/visual-functions/${id}`);
}

export function createQuiverVisualFunction(body: {
  name: string;
  description?: string;
  primary_type_id: string;
  secondary_type_id?: string | null;
  join_field: string;
  secondary_join_field?: string;
  date_field: string;
  metric_field: string;
  group_field: string;
  selected_group?: string | null;
  chart_kind?: QuiverChartKind;
  shared?: boolean;
}) {
  return api.post<QuiverVisualFunction>('/ontology/quiver/visual-functions', body);
}

export function updateQuiverVisualFunction(
  id: string,
  body: Partial<{
    name: string;
    description: string;
    primary_type_id: string;
    secondary_type_id: string | null;
    join_field: string;
    secondary_join_field: string;
    date_field: string;
    metric_field: string;
    group_field: string;
    selected_group: string | null;
    chart_kind: QuiverChartKind;
    shared: boolean;
  }>,
) {
  return api.patch<QuiverVisualFunction>(`/ontology/quiver/visual-functions/${id}`, body);
}

export function deleteQuiverVisualFunction(id: string) {
  return api.delete(`/ontology/quiver/visual-functions/${id}`);
}

export function getQuiverVegaSpec(body: {
  name: string;
  description?: string;
  primary_type_id: string;
  secondary_type_id?: string | null;
  join_field: string;
  secondary_join_field?: string;
  date_field: string;
  metric_field: string;
  group_field: string;
  selected_group?: string | null;
  chart_kind?: QuiverChartKind;
  shared?: boolean;
}) {
  return api.post<{ spec: Record<string, unknown> }>('/ontology/quiver/vega-spec', body);
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
  justification?: string;
}) {
  return api.post<ExecuteActionResponse>(`/ontology/actions/${id}/execute`, body);
}

export function listActionWhatIfBranches(
  id: string,
  params?: { target_object_id?: string; page?: number; per_page?: number },
) {
  const qs = new URLSearchParams();
  if (params?.target_object_id) qs.set('target_object_id', params.target_object_id);
  if (params?.page) qs.set('page', String(params.page));
  if (params?.per_page) qs.set('per_page', String(params.per_page));
  return api.get<{ data: ActionWhatIfBranch[]; total: number; page: number; per_page: number }>(
    `/ontology/actions/${id}/what-if?${qs}`,
  );
}

export function createActionWhatIfBranch(
  id: string,
  body: {
    target_object_id?: string;
    parameters?: Record<string, unknown>;
    name?: string;
    description?: string;
  },
) {
  return api.post<ActionWhatIfBranch>(`/ontology/actions/${id}/what-if`, body);
}

export function deleteActionWhatIfBranch(id: string, branchId: string) {
  return api.delete(`/ontology/actions/${id}/what-if/${branchId}`);
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

export function getMachineryQueue(params?: { object_type_id?: string }) {
  const qs = new URLSearchParams();
  if (params?.object_type_id) qs.set('object_type_id', params.object_type_id);
  return api.get<MachineryQueueResponse>(`/ontology/rules/machinery/queue?${qs}`);
}

export function updateMachineryQueueItem(id: string, body: { status: string }) {
  return api.patch<MachineryQueueItem>(`/ontology/rules/machinery/queue/${id}`, body);
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

export function listSharedPropertyTypes(params?: {
  page?: number;
  per_page?: number;
  search?: string;
}) {
  const qs = new URLSearchParams();
  if (params?.page) qs.set('page', String(params.page));
  if (params?.per_page) qs.set('per_page', String(params.per_page));
  if (params?.search) qs.set('search', params.search);
  return api.get<{ data: SharedPropertyType[]; total: number; page: number; per_page: number }>(
    `/ontology/shared-property-types?${qs}`,
  );
}

export function createSharedPropertyType(body: {
  name: string;
  display_name?: string;
  description?: string;
  property_type: string;
  required?: boolean;
  unique_constraint?: boolean;
  time_dependent?: boolean;
  default_value?: unknown;
  validation_rules?: unknown;
}) {
  return api.post<SharedPropertyType>('/ontology/shared-property-types', body);
}

export function listTypeSharedPropertyTypes(typeId: string) {
  return api
    .get<{ data: SharedPropertyType[] }>(`/ontology/types/${typeId}/shared-property-types`)
    .then((response) => response.data);
}

export function attachSharedPropertyType(typeId: string, sharedPropertyTypeId: string) {
  return api.post<{ object_type_id: string; shared_property_type_id: string }>(
    `/ontology/types/${typeId}/shared-property-types/${sharedPropertyTypeId}`,
    {},
  );
}

export function detachSharedPropertyType(typeId: string, sharedPropertyTypeId: string) {
  return api.delete(`/ontology/types/${typeId}/shared-property-types/${sharedPropertyTypeId}`);
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

export function listObjectSets() {
  return api.get<{ data: ObjectSetDefinition[] }>('/ontology/object-sets');
}

export function createObjectSet(body: {
  name: string;
  description?: string;
  base_object_type_id: string;
  filters?: ObjectSetFilter[];
  traversals?: ObjectSetTraversal[];
  join?: ObjectSetJoin | null;
  projections?: string[];
  what_if_label?: string | null;
  policy?: Partial<ObjectSetPolicy>;
}) {
  return api.post<ObjectSetDefinition>('/ontology/object-sets', body);
}

export function updateObjectSet(
  id: string,
  body: Partial<{
    name: string;
    description: string;
    base_object_type_id: string;
    filters: ObjectSetFilter[];
    traversals: ObjectSetTraversal[];
    join: ObjectSetJoin | null;
    projections: string[];
    what_if_label: string | null;
    policy: ObjectSetPolicy;
  }>,
) {
  return api.patch<ObjectSetDefinition>(`/ontology/object-sets/${id}`, body);
}

export function deleteObjectSet(id: string) {
  return api.delete(`/ontology/object-sets/${id}`);
}

export function evaluateObjectSet(id: string, body?: { limit?: number }) {
  return api.post<ObjectSetEvaluationResponse>(`/ontology/object-sets/${id}/evaluate`, body ?? {});
}

export function materializeObjectSet(id: string, body?: { limit?: number }) {
  return api.post<ObjectSetEvaluationResponse>(`/ontology/object-sets/${id}/materialize`, body ?? {});
}
