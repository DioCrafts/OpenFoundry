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
  inline_edit_config?: PropertyInlineEditConfig | null;
  created_at: string;
  updated_at: string;
}

export interface PropertyInlineEditConfig {
  action_type_id: string;
  input_name?: string | null;
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
  score_breakdown?: {
    fusion_strategy: string;
    lexical_rank: number | null;
    semantic_rank: number | null;
    lexical_score: number;
    semantic_score: number;
    title_bonus: number;
  } | null;
}

export type KnnMetric = 'cosine' | 'dot_product' | 'euclidean';

export interface KnnObjectResult {
  object: ObjectInstance;
  score: number;
  distance?: number | null;
}

export interface KnnObjectsResponse {
  property_name: string;
  metric: KnnMetric | string;
  total: number;
  data: KnnObjectResult[];
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

export interface ActionFormCondition {
  left: string;
  operator: string;
  right?: unknown;
}

export interface ActionFormSectionOverride {
  conditions?: ActionFormCondition[];
  hidden?: boolean;
  columns?: number;
  title?: string | null;
  description?: string | null;
}

export interface ActionFormSection {
  id: string;
  title?: string | null;
  description?: string | null;
  columns?: number;
  collapsible?: boolean;
  visible?: boolean;
  parameter_names?: string[];
  overrides?: ActionFormSectionOverride[];
}

export interface ActionFormParameterOverride {
  parameter_name: string;
  conditions?: ActionFormCondition[];
  hidden?: boolean;
  required?: boolean;
  default_value?: unknown;
  display_name?: string | null;
  description?: string | null;
}

export interface ActionFormSchema {
  sections?: ActionFormSection[];
  parameter_overrides?: ActionFormParameterOverride[];
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
  form_schema: ActionFormSchema;
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
  version: string;
  display_name: string;
  runtime: string;
  entrypoint: string;
  capabilities: FunctionCapabilities;
}

export interface FunctionPackage {
  id: string;
  name: string;
  version: string;
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

export interface FunctionPackageRun {
  id: string;
  function_package_id: string;
  function_package_name: string;
  function_package_version: string;
  runtime: string;
  status: 'success' | 'failure' | string;
  invocation_kind: 'simulation' | 'action' | string;
  action_id: string | null;
  action_name: string | null;
  object_type_id: string | null;
  target_object_id: string | null;
  actor_id: string;
  duration_ms: number;
  error_message: string | null;
  started_at: string;
  completed_at: string;
}

export interface FunctionPackageMetrics {
  package: FunctionPackageSummary;
  total_runs: number;
  successful_runs: number;
  failed_runs: number;
  simulation_runs: number;
  action_runs: number;
  success_rate: number;
  avg_duration_ms: number | null;
  p95_duration_ms: number | null;
  max_duration_ms: number | null;
  last_run_at: string | null;
  last_success_at: string | null;
  last_failure_at: string | null;
}

export interface OntologyFunnelPropertyMapping {
  source_field: string;
  target_property: string;
}

export interface OntologyFunnelSource {
  id: string;
  name: string;
  description: string;
  object_type_id: string;
  dataset_id: string;
  pipeline_id: string | null;
  dataset_branch: string | null;
  dataset_version: number | null;
  preview_limit: number;
  default_marking: string;
  status: string;
  property_mappings: OntologyFunnelPropertyMapping[];
  trigger_context: Record<string, unknown>;
  owner_id: string;
  last_run_at: string | null;
  created_at: string;
  updated_at: string;
}

export interface OntologyFunnelRun {
  id: string;
  source_id: string;
  object_type_id: string;
  dataset_id: string;
  pipeline_id: string | null;
  pipeline_run_id: string | null;
  status: string;
  trigger_type: string;
  started_by: string | null;
  rows_read: number;
  inserted_count: number;
  updated_count: number;
  skipped_count: number;
  error_count: number;
  details: Record<string, unknown>;
  error_message: string | null;
  started_at: string;
  finished_at: string | null;
}

export interface OntologyFunnelSourceHealth {
  source: OntologyFunnelSource;
  health_status: string;
  health_reason: string;
  total_runs: number;
  successful_runs: number;
  failed_runs: number;
  warning_runs: number;
  success_rate: number;
  avg_duration_ms: number | null;
  p95_duration_ms: number | null;
  max_duration_ms: number | null;
  latest_run_status: string | null;
  last_run_at: string | null;
  last_success_at: string | null;
  last_failure_at: string | null;
  last_warning_at: string | null;
  rows_read: number;
  inserted_count: number;
  updated_count: number;
  skipped_count: number;
  error_count: number;
}

export interface OntologyFunnelHealthSummary {
  stale_after_hours: number;
  total_sources: number;
  active_sources: number;
  paused_sources: number;
  healthy_sources: number;
  degraded_sources: number;
  failing_sources: number;
  stale_sources: number;
  never_run_sources: number;
  total_runs: number;
  successful_runs: number;
  failed_runs: number;
  warning_runs: number;
  success_rate: number;
  rows_read: number;
  inserted_count: number;
  updated_count: number;
  skipped_count: number;
  error_count: number;
  last_run_at: string | null;
  sources: OntologyFunnelSourceHealth[];
}

export interface FunctionAuthoringTemplate {
  id: string;
  runtime: string;
  display_name: string;
  description: string;
  entrypoint: string;
  starter_source: string;
  default_capabilities: FunctionCapabilities;
  recommended_use_cases: string[];
  cli_scaffold_template: string | null;
  sdk_packages: string[];
}

export interface FunctionSdkPackageReference {
  language: string;
  path: string;
  package_name: string;
  generated_by: string;
}

export interface FunctionAuthoringSurface {
  templates: FunctionAuthoringTemplate[];
  sdk_packages: FunctionSdkPackageReference[];
  cli_commands: string[];
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

export interface ScenarioSimulationOperation {
  label?: string | null;
  target_object_id?: string | null;
  action_id?: string | null;
  action_parameters?: Record<string, unknown>;
  properties_patch?: Record<string, unknown>;
}

export interface ScenarioSimulationCandidate {
  name: string;
  description?: string | null;
  operations: ScenarioSimulationOperation[];
}

export interface ScenarioMetricSpec {
  name: string;
  metric: string;
  comparator: string;
  target?: unknown;
  config?: Record<string, unknown>;
}

export interface ScenarioGoalSpec extends ScenarioMetricSpec {
  weight?: number;
}

export interface ScenarioObjectChange {
  object_id: string;
  object_type_id: string;
  object_type_label: string;
  deleted: boolean;
  changed_properties: string[];
  sources: string[];
  before: Record<string, unknown>;
  after: Record<string, unknown> | null;
}

export interface ScenarioRuleOutcome {
  object_id: string;
  rule_id: string;
  rule_name: string;
  rule_display_name: string;
  evaluation_mode: string;
  matched: boolean;
  auto_applied: boolean;
  trigger_payload: Record<string, unknown>;
  effect_preview: Record<string, unknown>;
}

export interface ScenarioLinkPreview {
  source_object_id: string | null;
  target_object_id: string | null;
  link_type_id: string | null;
  preview: Record<string, unknown>;
}

export interface ScenarioMetricEvaluation {
  name: string;
  metric: string;
  comparator: string;
  target: unknown;
  observed: unknown;
  passed: boolean;
  score: number | null;
  message: string;
}

export interface ScenarioSummary {
  impacted_object_count: number;
  changed_object_count: number;
  deleted_object_count: number;
  automatic_rule_matches: number;
  automatic_rule_applications: number;
  advisory_rule_matches: number;
  schedule_count: number;
  impacted_types: string[];
  changed_properties: string[];
  boundary_crossings: number;
  sensitive_objects: number;
  failed_constraints: number;
  achieved_goals: number;
  total_goals: number;
  goal_score: number;
}

export interface ScenarioSummaryDelta {
  impacted_object_count: number;
  changed_object_count: number;
  deleted_object_count: number;
  automatic_rule_matches: number;
  automatic_rule_applications: number;
  advisory_rule_matches: number;
  schedule_count: number;
  failed_constraints: number;
  goal_score: number;
}

export interface ScenarioSimulationResult {
  scenario_id: string;
  name: string;
  description: string | null;
  graph: GraphResponse;
  object_changes: ScenarioObjectChange[];
  rule_outcomes: ScenarioRuleOutcome[];
  link_previews: ScenarioLinkPreview[];
  constraints: ScenarioMetricEvaluation[];
  goals: ScenarioMetricEvaluation[];
  summary: ScenarioSummary;
  delta_from_baseline: ScenarioSummaryDelta | null;
}

export interface ObjectScenarioSimulationResponse {
  root_object_id: string;
  root_type_id: string;
  compared_at: string;
  baseline: ScenarioSimulationResult | null;
  scenarios: ScenarioSimulationResult[];
}

export interface CreateActionTypeBody {
  name: string;
  display_name?: string;
  description?: string;
  object_type_id: string;
  operation_kind: ActionOperationKind;
  input_schema?: ActionInputField[];
  form_schema?: ActionFormSchema;
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
  form_schema?: ActionFormSchema;
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
  hybrid_strategy?: 'rrf' | 'weighted';
  embedding_provider?: string;
  semantic_candidate_limit?: number;
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
  version?: string;
  display_name?: string;
  description?: string;
  runtime: string;
  source: string;
  entrypoint?: string;
  capabilities?: Partial<FunctionCapabilities>;
}) {
  return api.post<FunctionPackage>('/ontology/functions', body);
}

export function getFunctionAuthoringSurface() {
  return api.get<FunctionAuthoringSurface>('/ontology/functions/authoring-surface');
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

export function getFunctionPackageMetrics(id: string) {
  return api.get<FunctionPackageMetrics>(`/ontology/functions/${id}/metrics`);
}

export function listFunctionPackageRuns(id: string, params?: {
  page?: number;
  per_page?: number;
  status?: string;
  invocation_kind?: string;
}) {
  const qs = new URLSearchParams();
  if (params?.page) qs.set('page', String(params.page));
  if (params?.per_page) qs.set('per_page', String(params.per_page));
  if (params?.status) qs.set('status', params.status);
  if (params?.invocation_kind) qs.set('invocation_kind', params.invocation_kind);
  return api.get<{ data: FunctionPackageRun[]; total: number; page: number; per_page: number }>(
    `/ontology/functions/${id}/runs?${qs}`,
  );
}

export function getOntologyFunnelHealth(params?: {
  object_type_id?: string;
  stale_after_hours?: number;
}) {
  const qs = new URLSearchParams();
  if (params?.object_type_id) qs.set('object_type_id', params.object_type_id);
  if (params?.stale_after_hours) qs.set('stale_after_hours', String(params.stale_after_hours));
  return api.get<OntologyFunnelHealthSummary>(`/ontology/funnel/health?${qs}`);
}

export function getOntologyFunnelSourceHealth(id: string, params?: { stale_after_hours?: number }) {
  const qs = new URLSearchParams();
  if (params?.stale_after_hours) qs.set('stale_after_hours', String(params.stale_after_hours));
  return api.get<{ stale_after_hours: number; source_health: OntologyFunnelSourceHealth }>(
    `/ontology/funnel/sources/${id}/health?${qs}`,
  );
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

export function simulateObjectScenarios(
  typeId: string,
  objectId: string,
  body: {
    scenarios: ScenarioSimulationCandidate[];
    constraints?: ScenarioMetricSpec[];
    goals?: ScenarioGoalSpec[];
    depth?: number;
    max_iterations?: number;
    include_baseline?: boolean;
  },
) {
  return api.post<ObjectScenarioSimulationResponse>(
    `/ontology/types/${typeId}/objects/${objectId}/scenarios/simulate`,
    body,
  );
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
  time_dependent?: boolean;
  default_value?: unknown;
  validation_rules?: unknown;
  inline_edit_config?: PropertyInlineEditConfig | null;
}) {
  return api.post<Property>(`/ontology/types/${typeId}/properties`, body);
}

export function updateProperty(typeId: string, propertyId: string, body: {
  display_name?: string;
  description?: string;
  required?: boolean;
  unique_constraint?: boolean;
  time_dependent?: boolean;
  default_value?: unknown;
  validation_rules?: unknown;
  inline_edit_config?: PropertyInlineEditConfig | null;
}) {
  return api.patch<Property>(`/ontology/types/${typeId}/properties/${propertyId}`, body);
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

export function executeInlineEdit(
  typeId: string,
  objectId: string,
  propertyId: string,
  body: { value: unknown; justification?: string },
) {
  return api.post<ExecuteActionResponse>(
    `/ontology/types/${typeId}/objects/${objectId}/inline-edit/${propertyId}`,
    body,
  );
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

export function knnObjects(
  typeId: string,
  body: {
    property_name: string;
    anchor_object_id?: string;
    query_vector?: number[];
    limit?: number;
    metric?: KnnMetric;
    exclude_anchor?: boolean;
  },
) {
  return api.post<KnnObjectsResponse>(`/ontology/types/${typeId}/objects/knn`, body);
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
