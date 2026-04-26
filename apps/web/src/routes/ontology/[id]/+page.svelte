<script lang="ts">
  import { onMount } from 'svelte';
  import { page as pageStore } from '$app/stores';
  import ActionExecutor from '$lib/components/ontology/ActionExecutor.svelte';
  import {
    applyRule,
    attachSharedPropertyType,
    createActionType,
    createActionWhatIfBranch,
    createFunctionPackage,
    createObject,
    createRule,
    createSharedPropertyType,
    deleteActionType,
    deleteActionWhatIfBranch,
    deleteObject,
    detachSharedPropertyType,
    executeInlineEdit,
    executeAction,
    getFunctionAuthoringSurface,
    getFunctionPackageMetrics,
    getMachineryInsights,
    getMachineryQueue,
    getObjectView,
    getObjectType,
    listActionTypes,
    listFunctionPackages,
    listFunctionPackageRuns,
    listLinkTypes,
    listObjects,
    listProperties,
    listRules,
    listActionWhatIfBranches,
    listSharedPropertyTypes,
    listTypeSharedPropertyTypes,
    simulateFunctionPackage,
    simulateObject,
    simulateObjectScenarios,
    simulateRule,
    updateProperty,
    updateMachineryQueueItem,
    validateAction,
    type ActionAuthorizationPolicy,
    type ActionFormSchema,
    type ActionInputField,
    type ActionOperationKind,
    type ActionType,
    type ActionWhatIfBranch,
    type ExecuteActionResponse,
    type FunctionCapabilities,
    type FunctionAuthoringSurface,
    type FunctionAuthoringTemplate,
    type FunctionPackageMetrics,
    type FunctionPackage,
    type FunctionPackageRun,
    type LinkType,
    type MachineryInsight,
    type MachineryQueueResponse,
    type ObjectInstance,
    type ObjectScenarioSimulationResponse,
    type ObjectSimulationResponse,
    type ObjectViewResponse,
    type ObjectType,
    type OntologyRule,
    type Property,
    type PropertyInlineEditConfig,
    type ScenarioGoalSpec,
    type ScenarioMetricSpec,
    type ScenarioSimulationCandidate,
    type ScenarioSimulationResult,
    type SharedPropertyType,
    type ValidateActionResponse,
  } from '$lib/api/ontology';

  const objectTypeId = $derived($pageStore.params.id ?? '');

  const actionTemplates: Record<
    ActionOperationKind,
    { inputSchema: ActionInputField[]; config: Record<string, unknown>; notes: string }
  > = {
    update_object: {
      inputSchema: [
        {
          name: 'status',
          display_name: 'Status',
          description: 'New property value to write onto the selected object.',
          property_type: 'string',
          required: true,
        },
      ],
      config: {
        property_mappings: [{ property_name: 'status', input_name: 'status' }],
      },
      notes: 'Maps validated inputs onto object properties. Optional static_patch can add fixed values.',
    },
    create_link: {
      inputSchema: [
        {
          name: 'related_object_id',
          display_name: 'Related Object ID',
          description: 'UUID of the object that should be linked.',
          property_type: 'reference',
          required: true,
        },
        {
          name: 'link_properties',
          display_name: 'Link Properties',
          description: 'Optional metadata stored on the created link instance.',
          property_type: 'json',
          required: false,
          default_value: {},
        },
      ],
      config: {
        link_type_id: '00000000-0000-0000-0000-000000000000',
        target_input_name: 'related_object_id',
        source_role: 'source',
        properties_input_name: 'link_properties',
      },
      notes: 'Replace link_type_id with one of the link types listed below before saving.',
    },
    delete_object: {
      inputSchema: [],
      config: {},
      notes: 'Deletes the selected object instance immediately after validation succeeds.',
    },
    invoke_function: {
      inputSchema: [
        {
          name: 'payload',
          display_name: 'Payload',
          description: 'Function input payload. Any JSON shape is accepted.',
          property_type: 'json',
          required: false,
          default_value: {},
        },
      ],
      config: {
        runtime: 'typescript',
        source: `export default async function handler(context) {
  const targetId = context.targetObject?.id;
  const summary = targetId
    ? await context.llm.complete({
        userMessage: \`Summarize the current state of object \${targetId} in one sentence.\`,
        maxTokens: 128,
      })
    : null;

  const related = await context.sdk.ontology.search({
    query: context.parameters.payload?.query ?? 'customer health',
    kind: 'object_instance',
    limit: 5,
  });

  return {
    output: {
      related,
      summary: summary?.reply ?? null,
    },
    object_patch: targetId
      ? {
          status: 'reviewed',
        }
      : null,
  };
}`,
      },
      notes: 'Use either inline TypeScript/Python source or a reusable function package via {"function_package_id":"..."}. Functions can call context.sdk.ontology.* and context.llm.complete(...) based on package capabilities.',
    },
    invoke_webhook: {
      inputSchema: [
        {
          name: 'event',
          display_name: 'Event Body',
          description: 'JSON event fragment sent to the external webhook.',
          property_type: 'json',
          required: false,
          default_value: {},
        },
      ],
      config: {
        url: 'https://example.com/webhooks/action',
        method: 'POST',
        headers: {},
      },
      notes: 'Webhook actions only return the external response payload; they do not mutate ontology objects directly.',
    },
  };

  const propertyTypeOptions = [
    'string',
    'integer',
    'float',
    'boolean',
    'date',
    'timestamp',
    'json',
    'array',
    'vector',
    'reference',
    'geo_point',
    'media_reference',
  ];

  function buildDefaultActionFormSchema(inputSchema: ActionInputField[]): ActionFormSchema {
    const requiredFields = inputSchema.filter((field) => field.required).map((field) => field.name);
    const optionalFields = inputSchema.filter((field) => !field.required).map((field) => field.name);
    const sections = [];

    if (requiredFields.length) {
      sections.push({
        id: 'core',
        title: 'Core inputs',
        description: 'The minimum information required to run this action.',
        columns: requiredFields.length > 1 ? 2 : 1,
        parameter_names: requiredFields,
      });
    }

    if (optionalFields.length) {
      sections.push({
        id: sections.length ? 'optional' : 'main',
        title: sections.length ? 'Optional context' : 'Parameters',
        description: sections.length
          ? 'Additional context that can refine the action execution.'
          : 'Parameters required for this action.',
        columns: optionalFields.length > 1 ? 2 : 1,
        parameter_names: optionalFields,
      });
    }

    return {
      sections,
      parameter_overrides: [],
    };
  }

  const functionSourceTemplates = {
    typescript: `export default async function handler(context) {
  const target = context.targetObject;
  const related = await context.sdk.ontology.search({
    query: target?.properties?.name ?? 'high risk case',
    kind: 'object_instance',
    limit: 5,
  });

  return {
    output: {
      inspectedObjectId: target?.id ?? null,
      related,
      capabilities: context.capabilities,
    },
  };
}`,
    python: `def handler(context):
    target = context.get("target_object")
    related = context["sdk"].ontology.search(
        query=(target or {}).get("properties", {}).get("name", "high risk case"),
        kind="object_instance",
        limit=5,
    )

    summary = None
    if context["capabilities"].get("allow_ai"):
        summary = context["llm"].complete(
            user_message=f"Summarize object {(target or {}).get('id', 'n/a')} in one sentence.",
            max_tokens=128,
        )

    return {
        "output": {
            "inspectedObjectId": (target or {}).get("id"),
            "related": related,
            "summary": summary,
            "capabilities": context["capabilities"],
        }
    }
}`,
  } as const;

  let objectType = $state<ObjectType | null>(null);
  let properties = $state<Property[]>([]);
  let sharedPropertyCatalog = $state<SharedPropertyType[]>([]);
  let attachedSharedPropertyTypes = $state<SharedPropertyType[]>([]);
  let linkTypes = $state<LinkType[]>([]);
  let objects = $state<ObjectInstance[]>([]);
  let actions = $state<ActionType[]>([]);
  const inlineEditActionOptions = $derived(
    actions.filter((action) => action.operation_kind === 'update_object'),
  );
  let loading = $state(true);
  let error = $state('');
  const attachableSharedPropertyTypes = $derived(
    sharedPropertyCatalog.filter(
      (candidate) => !attachedSharedPropertyTypes.some((attached) => attached.id === candidate.id),
    ),
  );

  let actionFormError = $state('');
  let actionFormSuccess = $state('');
  let objectError = $state('');
  let objectSuccess = $state('');
  let runtimeError = $state('');
  let sharedPropertyFormError = $state('');
  let sharedPropertyFormSuccess = $state('');
  let propertyConfigError = $state('');
  let propertyConfigSuccess = $state('');

  let creatingAction = $state(false);
  let creatingObject = $state(false);
  let validatingAction = $state(false);
  let executingAction = $state(false);
  let creatingSharedPropertyType = $state(false);
  let creatingWhatIfBranch = $state(false);
  let attachingSharedPropertyType = $state(false);
  let detachingSharedPropertyTypeId = $state('');
  let deletingWhatIfBranchId = $state('');
  let savingInlineEditPropertyId = $state('');
  let executingInlineEditKey = $state('');

  let selectedActionId = $state('');
  let selectedTargetObjectId = $state('');
  let selectedSharedPropertyTypeId = $state('');
  let validation = $state<ValidateActionResponse | null>(null);
  let execution = $state<ExecuteActionResponse | null>(null);
  let actionWhatIfBranches = $state<ActionWhatIfBranch[]>([]);
  let functionAuthoringSurface = $state<FunctionAuthoringSurface | null>(null);
  let functionPackages = $state<FunctionPackage[]>([]);
  let functionMetrics = $state<FunctionPackageMetrics | null>(null);
  let functionRuns = $state<FunctionPackageRun[]>([]);
  let rules = $state<OntologyRule[]>([]);
  let machineryInsights = $state<MachineryInsight[]>([]);
  let machineryQueue = $state<MachineryQueueResponse | null>(null);
  let objectView = $state<ObjectViewResponse | null>(null);
  let simulation = $state<ObjectSimulationResponse | null>(null);
  let scenarioComparison = $state<ObjectScenarioSimulationResponse | null>(null);

  let objectViewLoading = $state(false);
  let simulationLoading = $state(false);
  let scenarioLoading = $state(false);
  let creatingFunctionPackage = $state(false);
  let creatingRule = $state(false);
  let functionRuntimeLoading = $state(false);
  let functionMonitoringLoading = $state(false);
  let ruleRuntimeLoading = $state(false);

  let functionFormError = $state('');
  let functionFormSuccess = $state('');
  let functionRuntimeError = $state('');
  let functionRuntimeResult = $state<Record<string, unknown> | null>(null);
  let functionMonitoringError = $state('');
  let ruleFormError = $state('');
  let ruleFormSuccess = $state('');
  let ruleRuntimeError = $state('');
  let ruleRuntimeResult = $state<Record<string, unknown> | null>(null);
  let updatingMachineryQueueItemId = $state('');

  let functionName = $state('');
  let functionVersion = $state('0.1.0');
  let functionDisplayName = $state('');
  let functionDescription = $state('');
  let functionRuntime = $state<'typescript' | 'python'>('typescript');
  let functionEntrypoint = $state<'default' | 'handler'>('default');
  let functionCapabilitiesText = $state(
    JSON.stringify(
      {
        allow_ontology_read: true,
        allow_ontology_write: true,
        allow_ai: true,
        allow_network: false,
        timeout_seconds: 15,
        max_source_bytes: 65536,
      } satisfies FunctionCapabilities,
      null,
      2,
    ),
  );
  let functionSourceText = $state<string>(functionSourceTemplates.typescript);
  let selectedFunctionPackageId = $state('');

  let ruleName = $state('');
  let ruleDisplayName = $state('');
  let ruleDescription = $state('');
  let ruleEvaluationMode = $state<'advisory' | 'automatic'>('advisory');
  let ruleTriggerSpecText = $state(
    JSON.stringify(
      {
        equals: { status: 'pending' },
        numeric_gte: { risk_score: 0.8 },
        changed_properties: ['status', 'risk_score'],
      },
      null,
      2,
    ),
  );
  let ruleEffectSpecText = $state(
    JSON.stringify(
      {
        object_patch: { priority: 'high' },
        schedule: {
          property_name: 'next_review_at',
          offset_hours: 24,
          priority_score: 70,
          estimated_duration_minutes: 30,
          required_capability: 'case_manager',
          constraint_tags: ['renewal', 'ops'],
        },
        alert: { severity: 'high', title: 'Escalate review' },
      },
      null,
      2,
    ),
  );

  let simulationPatchText = $state('{}');
  let scenarioCandidatesText = $state(
    JSON.stringify(
      [
        {
          name: 'Candidate scenario',
          description: 'Compare a candidate operating state against the baseline graph neighborhood.',
          operations: [
            {
              label: 'root_patch',
              target_object_id: null,
              action_id: null,
              action_parameters: {},
              properties_patch: {},
            },
          ],
        },
      ] satisfies ScenarioSimulationCandidate[],
      null,
      2,
    ),
  );
  let scenarioConstraintsText = $state(
    JSON.stringify(
      [
        {
          name: 'Keep blast radius contained',
          metric: 'changed_object_count',
          comparator: 'lte',
          target: 5,
          config: {},
        },
        {
          name: 'No overloaded schedule queue',
          metric: 'schedule_count',
          comparator: 'lte',
          target: 4,
          config: {},
        },
      ] satisfies ScenarioMetricSpec[],
      null,
      2,
    ),
  );
  let scenarioGoalsText = $state(
    JSON.stringify(
      [
        {
          name: 'Reach active state on the selected root',
          metric: 'property_equals_count',
          comparator: 'gte',
          target: 1,
          config: {
            property: 'status',
            value: 'active',
            only_changed: true,
          },
          weight: 1.5,
        },
        {
          name: 'Minimize automatic rule churn',
          metric: 'automatic_rule_applications',
          comparator: 'lte',
          target: 2,
          config: {},
          weight: 1,
        },
      ] satisfies ScenarioGoalSpec[],
      null,
      2,
    ),
  );

  let actionName = $state('');
  let actionDisplayName = $state('');
  let actionDescription = $state('');
  let actionOperationKind = $state<ActionOperationKind>('update_object');
  let actionConfirmationRequired = $state(false);
  let actionPermissionKey = $state('');
  let actionAuthorizationPolicyText = $state(JSON.stringify({}, null, 2));
  let actionInputSchemaText = $state(JSON.stringify(actionTemplates.update_object.inputSchema, null, 2));
  let actionFormSchemaText = $state(
    JSON.stringify(buildDefaultActionFormSchema(actionTemplates.update_object.inputSchema), null, 2),
  );
  let actionConfigText = $state(JSON.stringify(actionTemplates.update_object.config, null, 2));

  let objectPropertiesText = $state('{}');
  let propertyInlineEditActionSelections = $state<Record<string, string>>({});
  let propertyInlineEditInputNames = $state<Record<string, string>>({});
  let objectInlineEditDrafts = $state<Record<string, string>>({});
  let actionParametersText = $state('{}');
  let actionParametersDraft = $state<Record<string, unknown>>({});
  let actionParametersInputError = $state('');
  let actionJustification = $state('');
  let whatIfBranchName = $state('');
  let whatIfBranchDescription = $state('');
  let sharedPropertyName = $state('');
  let sharedPropertyDisplayName = $state('');
  let sharedPropertyDescription = $state('');
  let sharedPropertyType = $state('string');
  let sharedPropertyRequired = $state(false);
  let sharedPropertyUniqueConstraint = $state(false);
  let sharedPropertyTimeDependent = $state(false);

  function formatJson(value: unknown): string {
    return JSON.stringify(value ?? null, null, 2);
  }

  function formatTimestamp(value: string | null | undefined) {
    return value ? new Date(value).toLocaleString() : 'n/a';
  }

  function formatDuration(value: number | null | undefined) {
    if (value === null || value === undefined || Number.isNaN(value)) return 'n/a';
    return `${Math.round(value)} ms`;
  }

  function formatPercent(value: number | null | undefined) {
    if (value === null || value === undefined || Number.isNaN(value)) return 'n/a';
    return `${(value * 100).toFixed(1)}%`;
  }

  function applyFunctionAuthoringTemplate(template: FunctionAuthoringTemplate) {
    functionRuntime = template.runtime === 'python' ? 'python' : 'typescript';
    functionEntrypoint = template.entrypoint === 'handler' ? 'handler' : 'default';
    functionCapabilitiesText = formatJson(template.default_capabilities);
    functionSourceText = template.starter_source;
  }

  function countEntries(entries: Record<string, number> | undefined) {
    return Object.entries(entries ?? {}).sort((left, right) => right[1] - left[1]);
  }

  function formatScope(scope: string | undefined) {
    return (scope ?? 'local').replaceAll('_', ' ');
  }

  function pressureToneClass(pressure: string | undefined) {
    switch (pressure) {
      case 'high':
        return 'bg-rose-50 text-rose-700 dark:bg-rose-950/40 dark:text-rose-300';
      case 'medium':
        return 'bg-amber-50 text-amber-700 dark:bg-amber-950/40 dark:text-amber-300';
      default:
        return 'bg-emerald-50 text-emerald-700 dark:bg-emerald-950/40 dark:text-emerald-300';
    }
  }

  async function handleMachineryQueueTransition(id: string, status: string) {
    updatingMachineryQueueItemId = id;
    runtimeError = '';

    try {
      await updateMachineryQueueItem(id, { status });
      if (objectTypeId) {
        machineryQueue = await getMachineryQueue({ object_type_id: objectTypeId });
        const response = await getMachineryInsights({ object_type_id: objectTypeId });
        machineryInsights = response.data;
      }
    } catch (cause) {
      runtimeError = cause instanceof Error ? cause.message : 'Failed to update machinery queue item';
    } finally {
      updatingMachineryQueueItemId = '';
    }
  }

  function parseJsonValue(source: string, label: string, fallback: unknown): unknown {
    try {
      return source.trim() ? JSON.parse(source) : fallback;
    } catch (cause) {
      throw new Error(`${label} must be valid JSON`, { cause });
    }
  }

  function parseJsonArray<T>(source: string, label: string): T[] {
    const parsed = parseJsonValue(source, label, []);
    if (!Array.isArray(parsed)) {
      throw new Error(`${label} must be a JSON array`);
    }
    return parsed as T[];
  }

  function parseJsonObject(source: string, label: string): Record<string, unknown> {
    const parsed = parseJsonValue(source, label, {});
    if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) {
      throw new Error(`${label} must be a JSON object`);
    }
    return parsed as Record<string, unknown>;
  }

  function resetActionParameters() {
    actionParametersDraft = {};
    actionParametersText = '{}';
    actionParametersInputError = '';
  }

  function handleStructuredActionParametersChange(
    event: CustomEvent<{ parameters: Record<string, unknown> }>,
  ) {
    actionParametersDraft = event.detail.parameters;
    actionParametersText = formatJson(event.detail.parameters);
    actionParametersInputError = '';
  }

  function handleActionParametersTextInput(event: Event) {
    const next = (event.currentTarget as HTMLTextAreaElement).value;
    actionParametersText = next;
    try {
      actionParametersDraft = parseJsonObject(next, 'Action parameters');
      actionParametersInputError = '';
    } catch (cause) {
      actionParametersInputError =
        cause instanceof Error ? cause.message : 'Action parameters must be valid JSON';
    }
  }

  function getSelectedTargetObject(): ObjectInstance | null {
    return objects.find((object) => object.id === selectedTargetObjectId) ?? null;
  }

  function inlineEditFieldKey(objectId: string, propertyId: string) {
    return `${objectId}:${propertyId}`;
  }

  function formatInlineEditDraft(value: unknown): string {
    if (value === null || value === undefined) return '';
    if (typeof value === 'string') return value;
    return JSON.stringify(value);
  }

  function parseInlineEditSubmissionValue(property: Property, raw: string): unknown {
    const trimmed = raw.trim();

    switch (property.property_type) {
      case 'integer': {
        const parsed = Number.parseInt(trimmed, 10);
        if (Number.isNaN(parsed)) throw new Error(`${property.display_name} must be an integer`);
        return parsed;
      }
      case 'float': {
        const parsed = Number.parseFloat(trimmed);
        if (Number.isNaN(parsed)) throw new Error(`${property.display_name} must be a number`);
        return parsed;
      }
      case 'boolean': {
        if (trimmed === 'true') return true;
        if (trimmed === 'false') return false;
        throw new Error(`${property.display_name} must be true or false`);
      }
      case 'json':
      case 'array':
        return parseJsonValue(trimmed, `${property.display_name} inline edit value`, null);
      case 'vector': {
        const parsed = parseJsonArray<number>(trimmed, `${property.display_name} inline edit value`);
        if (!parsed.length || !parsed.every((entry) => typeof entry === 'number' && Number.isFinite(entry))) {
          throw new Error(`${property.display_name} must be a non-empty JSON array of numeric values`);
        }
        return parsed;
      }
      default:
        return raw;
    }
  }

  function actionLabelById(actionId: string | null | undefined) {
    if (!actionId) return 'No action configured';
    const action = actions.find((candidate) => candidate.id === actionId);
    return action?.display_name || action?.name || actionId;
  }

  function syncPropertyInlineEditConfigDrafts(nextProperties: Property[]) {
    propertyInlineEditActionSelections = Object.fromEntries(
      nextProperties.map((property) => [
        property.id,
        property.inline_edit_config?.action_type_id ?? '',
      ]),
    );
    propertyInlineEditInputNames = Object.fromEntries(
      nextProperties.map((property) => [
        property.id,
        property.inline_edit_config?.input_name ?? '',
      ]),
    );
  }

  function syncObjectInlineEditDrafts(nextProperties: Property[], nextObjects: ObjectInstance[]) {
    const inlineEditableProperties = nextProperties.filter((property) => property.inline_edit_config);
    const nextDrafts: Record<string, string> = {};

    for (const object of nextObjects) {
      for (const property of inlineEditableProperties) {
        nextDrafts[inlineEditFieldKey(object.id, property.id)] = formatInlineEditDraft(
          object.properties?.[property.name],
        );
      }
    }

    objectInlineEditDrafts = nextDrafts;
  }

  function scenarioDeltaClass(
    value: number | undefined,
    preference: 'higher' | 'lower' | 'neutral' = 'neutral',
  ) {
    if (!value || value === 0) return 'text-slate-500 dark:text-slate-400';
    if (preference === 'higher') {
      return value > 0
        ? 'text-emerald-700 dark:text-emerald-300'
        : 'text-rose-700 dark:text-rose-300';
    }
    if (preference === 'lower') {
      return value < 0
        ? 'text-emerald-700 dark:text-emerald-300'
        : 'text-rose-700 dark:text-rose-300';
    }
    return value > 0
      ? 'text-sky-700 dark:text-sky-300'
      : 'text-amber-700 dark:text-amber-300';
  }

  function scenarioDeltaPrefix(value: number | undefined) {
    return value && value > 0 ? '+' : '';
  }

  function scenarioGoalScore(result: ScenarioSimulationResult) {
    return Number(result.summary.goal_score ?? 0).toFixed(2);
  }

  function formatScenarioMetricValue(value: unknown): string {
    if (value === null || value === undefined) return 'n/a';
    if (typeof value === 'object') return JSON.stringify(value);
    return String(value);
  }

  function scenarioMetricToneClass(passed: boolean) {
    return passed
      ? 'bg-emerald-50 text-emerald-700 dark:bg-emerald-950/40 dark:text-emerald-300'
      : 'bg-rose-50 text-rose-700 dark:bg-rose-950/40 dark:text-rose-300';
  }

  function scenarioObjectLabel(change: ScenarioSimulationResult['object_changes'][number]) {
    const candidate = change.after ?? change.before;
    for (const key of ['display_name', 'name', 'title', 'label']) {
      const value = candidate?.[key];
      if (typeof value === 'string' && value.trim()) {
        return value;
      }
    }
    return change.object_id;
  }

  function seedScenarioDraftFromSelection() {
    runtimeError = '';
    try {
      scenarioCandidatesText = formatJson([
        {
          name: 'Seeded candidate',
          description: 'Generated from the current object/action selection in Object View.',
          operations: [
            {
              label: 'selected_operation',
              target_object_id: selectedTargetObjectId || null,
              action_id: selectedActionId || null,
              action_parameters: parseJsonObject(actionParametersText, 'Scenario seed action parameters'),
              properties_patch: parseJsonObject(simulationPatchText, 'Scenario seed patch'),
            },
          ],
        },
      ] satisfies ScenarioSimulationCandidate[]);
    } catch (cause) {
      runtimeError = cause instanceof Error ? cause.message : 'Failed to seed scenario draft';
    }
  }

  function getSelectedAction(): ActionType | null {
    return actions.find((action) => action.id === selectedActionId) ?? null;
  }

  function operationRequiresTarget(kind: ActionOperationKind | undefined): boolean {
    return kind === 'update_object' || kind === 'create_link' || kind === 'delete_object';
  }

  function applyTemplate(kind: ActionOperationKind) {
    const nextInputSchema = actionTemplates[kind].inputSchema;
    actionInputSchemaText = formatJson(nextInputSchema);
    actionFormSchemaText = formatJson(buildDefaultActionFormSchema(nextInputSchema));
    actionConfigText = formatJson(actionTemplates[kind].config);
  }

  function applyFunctionRuntimeTemplate(runtime: string) {
    const nextRuntime = runtime === 'python' ? 'python' : 'typescript';
    functionRuntime = nextRuntime;
    functionEntrypoint = nextRuntime === 'typescript' ? 'default' : 'handler';
    functionSourceText = functionSourceTemplates[nextRuntime];
  }

  function hasAuthorizationPolicy(policy: ActionAuthorizationPolicy | undefined | null) {
    return Object.entries(policy ?? {}).some(([, value]) => {
      if (Array.isArray(value)) return value.length > 0;
      if (value && typeof value === 'object') return Object.keys(value as Record<string, unknown>).length > 0;
      return value !== undefined && value !== null && value !== false && value !== '';
    });
  }

  function syncSelections(nextActions: ActionType[], nextObjects: ObjectInstance[]) {
    const previousSelectedActionId = selectedActionId;
    if (!nextActions.some((action) => action.id === selectedActionId)) {
      selectedActionId = nextActions[0]?.id ?? '';
    }
    if (selectedActionId !== previousSelectedActionId) {
      resetActionParameters();
    }

    if (!nextObjects.some((object) => object.id === selectedTargetObjectId)) {
      selectedTargetObjectId = '';
    }

    if (!selectedTargetObjectId && nextObjects[0]) {
      selectedTargetObjectId = nextObjects[0].id;
    }
  }

  function syncFunctionPackageSelection(nextFunctionPackages: FunctionPackage[]) {
    if (
      selectedFunctionPackageId &&
      nextFunctionPackages.some((functionPackage) => functionPackage.id === selectedFunctionPackageId)
    ) {
      return;
    }

    selectedFunctionPackageId = nextFunctionPackages[0]?.id ?? '';
  }

  async function loadFunctionPackageMonitoring(functionPackageId = selectedFunctionPackageId) {
    if (!functionPackageId) {
      functionMetrics = null;
      functionRuns = [];
      functionMonitoringError = '';
      return;
    }

    functionMonitoringLoading = true;
    functionMonitoringError = '';

    try {
      const [nextMetrics, nextRuns] = await Promise.all([
        getFunctionPackageMetrics(functionPackageId),
        listFunctionPackageRuns(functionPackageId, { page: 1, per_page: 8 }),
      ]);
      functionMetrics = nextMetrics;
      functionRuns = nextRuns.data;
    } catch (cause) {
      functionMonitoringError =
        cause instanceof Error ? cause.message : 'Failed to load function monitoring';
      functionMetrics = null;
      functionRuns = [];
    } finally {
      functionMonitoringLoading = false;
    }
  }

  async function handleSelectFunctionPackage(functionPackageId: string) {
    selectedFunctionPackageId = functionPackageId;
    await loadFunctionPackageMonitoring(functionPackageId);
  }

  async function loadActionWhatIfHistory(actionId = selectedActionId) {
    if (!actionId) {
      actionWhatIfBranches = [];
      return;
    }

    try {
      const response = await listActionWhatIfBranches(actionId, { page: 1, per_page: 20 });
      actionWhatIfBranches = response.data;
    } catch (cause) {
      runtimeError = cause instanceof Error ? cause.message : 'Failed to load what-if branches';
      actionWhatIfBranches = [];
    }
  }

  async function handleSelectAction(id: string) {
    selectedActionId = id;
    runtimeError = '';
    validation = null;
    execution = null;
    resetActionParameters();
    await loadActionWhatIfHistory(id);
  }

  async function loadObjectInspector(objectId: string) {
    if (!objectTypeId || !objectId) {
      objectView = null;
      scenarioComparison = null;
      return;
    }

    objectViewLoading = true;
    scenarioComparison = null;
    try {
      objectView = await getObjectView(objectTypeId, objectId);
    } catch (cause) {
      runtimeError = cause instanceof Error ? cause.message : 'Failed to load object view';
      objectView = null;
    } finally {
      objectViewLoading = false;
    }
  }

  async function load() {
    if (!objectTypeId) {
      error = 'Missing object type id';
      loading = false;
      return;
    }

    loading = true;
    error = '';

    try {
      const [
        nextType,
        nextProperties,
        nextSharedPropertyCatalog,
        nextAttachedSharedPropertyTypes,
        nextLinkTypes,
        nextObjects,
        nextActions,
        nextFunctionAuthoringSurface,
        nextFunctionPackages,
        nextRules,
        nextMachineryInsights,
        nextMachineryQueue,
      ] = await Promise.all([
        getObjectType(objectTypeId),
        listProperties(objectTypeId),
        listSharedPropertyTypes({ page: 1, per_page: 100 }),
        listTypeSharedPropertyTypes(objectTypeId),
        listLinkTypes({ object_type_id: objectTypeId, page: 1, per_page: 100 }),
        listObjects(objectTypeId, { page: 1, per_page: 50 }),
        listActionTypes({ object_type_id: objectTypeId, page: 1, per_page: 100 }),
        getFunctionAuthoringSurface(),
        listFunctionPackages({ page: 1, per_page: 100 }),
        listRules({ object_type_id: objectTypeId, page: 1, per_page: 100 }),
        getMachineryInsights({ object_type_id: objectTypeId }),
        getMachineryQueue({ object_type_id: objectTypeId }),
      ]);

      objectType = nextType;
      properties = nextProperties;
      sharedPropertyCatalog = nextSharedPropertyCatalog.data;
      attachedSharedPropertyTypes = nextAttachedSharedPropertyTypes;
      linkTypes = nextLinkTypes.data;
      objects = nextObjects.data;
      actions = nextActions.data;
      functionAuthoringSurface = nextFunctionAuthoringSurface;
      syncPropertyInlineEditConfigDrafts(nextProperties);
      syncObjectInlineEditDrafts(nextProperties, nextObjects.data);
      functionPackages = nextFunctionPackages.data;
      rules = nextRules.data;
      machineryInsights = nextMachineryInsights.data;
      machineryQueue = nextMachineryQueue;
      const nextAttachableSharedPropertyTypes = nextSharedPropertyCatalog.data.filter(
        (candidate) =>
          !nextAttachedSharedPropertyTypes.some((attached) => attached.id === candidate.id),
      );
      if (
        !nextAttachableSharedPropertyTypes.some(
          (candidate) => candidate.id === selectedSharedPropertyTypeId,
        )
      ) {
        selectedSharedPropertyTypeId = nextAttachableSharedPropertyTypes[0]?.id ?? '';
      }
      syncSelections(nextActions.data, nextObjects.data);
      syncFunctionPackageSelection(nextFunctionPackages.data);
      await loadActionWhatIfHistory(selectedActionId);
      await loadFunctionPackageMonitoring(selectedFunctionPackageId);
      if (selectedTargetObjectId) {
        await loadObjectInspector(selectedTargetObjectId);
      } else {
        objectView = null;
      }
    } catch (cause) {
      error = cause instanceof Error ? cause.message : 'Failed to load ontology details';
    } finally {
      loading = false;
    }
  }

  async function handleCreateSharedPropertyType(event: Event) {
    event.preventDefault();
    if (!objectTypeId) {
      return;
    }

    creatingSharedPropertyType = true;
    sharedPropertyFormError = '';
    sharedPropertyFormSuccess = '';

    try {
      if (!sharedPropertyName.trim()) {
        throw new Error('Shared property type name is required');
      }

      const created = await createSharedPropertyType({
        name: sharedPropertyName.trim(),
        display_name: sharedPropertyDisplayName.trim() || undefined,
        description: sharedPropertyDescription.trim() || undefined,
        property_type: sharedPropertyType,
        required: sharedPropertyRequired,
        unique_constraint: sharedPropertyUniqueConstraint,
        time_dependent: sharedPropertyTimeDependent,
      });

      await attachSharedPropertyType(objectTypeId, created.id);
      sharedPropertyFormSuccess = `Created and attached ${created.display_name}.`;
      sharedPropertyName = '';
      sharedPropertyDisplayName = '';
      sharedPropertyDescription = '';
      sharedPropertyType = 'string';
      sharedPropertyRequired = false;
      sharedPropertyUniqueConstraint = false;
      sharedPropertyTimeDependent = false;
      await load();
    } catch (cause) {
      sharedPropertyFormError =
        cause instanceof Error ? cause.message : 'Failed to create shared property type';
    } finally {
      creatingSharedPropertyType = false;
    }
  }

  async function handleAttachSharedPropertyType() {
    if (!objectTypeId) {
      return;
    }
    if (!selectedSharedPropertyTypeId) {
      sharedPropertyFormError = 'Select a shared property type to attach';
      return;
    }

    attachingSharedPropertyType = true;
    sharedPropertyFormError = '';
    sharedPropertyFormSuccess = '';

    try {
      await attachSharedPropertyType(objectTypeId, selectedSharedPropertyTypeId);
      sharedPropertyFormSuccess = 'Attached shared property type to this object type.';
      await load();
    } catch (cause) {
      sharedPropertyFormError =
        cause instanceof Error ? cause.message : 'Failed to attach shared property type';
    } finally {
      attachingSharedPropertyType = false;
    }
  }

  async function handleDetachSharedPropertyType(id: string) {
    if (!objectTypeId || !confirm('Detach this shared property type from the object type?')) {
      return;
    }

    detachingSharedPropertyTypeId = id;
    sharedPropertyFormError = '';
    sharedPropertyFormSuccess = '';

    try {
      await detachSharedPropertyType(objectTypeId, id);
      sharedPropertyFormSuccess = 'Detached shared property type.';
      await load();
    } catch (cause) {
      sharedPropertyFormError =
        cause instanceof Error ? cause.message : 'Failed to detach shared property type';
    } finally {
      detachingSharedPropertyTypeId = '';
    }
  }

  async function handleCreateObject(event: Event) {
    event.preventDefault();
    if (!objectTypeId) {
      return;
    }

    creatingObject = true;
    objectError = '';
    objectSuccess = '';

    try {
      const propertiesPayload = parseJsonObject(objectPropertiesText, 'Object properties');
      const created = await createObject(objectTypeId, propertiesPayload);
      objectPropertiesText = '{}';
      selectedTargetObjectId = created.id;
      objectSuccess = 'Object created and selected for action testing.';
      await load();
    } catch (cause) {
      objectError = cause instanceof Error ? cause.message : 'Failed to create object';
    } finally {
      creatingObject = false;
    }
  }

  async function handleDeleteObject(id: string) {
    if (!objectTypeId || !confirm('Delete this object instance?')) {
      return;
    }

    objectError = '';
    objectSuccess = '';

    try {
      await deleteObject(objectTypeId, id);
      if (selectedTargetObjectId === id) {
        selectedTargetObjectId = '';
        objectView = null;
        simulation = null;
      }
      await load();
    } catch (cause) {
      objectError = cause instanceof Error ? cause.message : 'Failed to delete object';
    }
  }

  async function handleSaveInlineEditConfig(property: Property) {
    if (!objectTypeId) {
      return;
    }

    const selectedActionId = propertyInlineEditActionSelections[property.id]?.trim();
    const inputName = propertyInlineEditInputNames[property.id]?.trim();

    savingInlineEditPropertyId = property.id;
    propertyConfigError = '';
    propertyConfigSuccess = '';

    try {
      if (!selectedActionId) {
        throw new Error('Select an update_object action or clear the configuration');
      }

      const inlineEditConfig: PropertyInlineEditConfig = {
        action_type_id: selectedActionId,
        input_name: inputName || undefined,
      };

      await updateProperty(objectTypeId, property.id, {
        inline_edit_config: inlineEditConfig,
      });
      propertyConfigSuccess = `Inline edit configured for ${property.display_name}.`;
      await load();
    } catch (cause) {
      propertyConfigError =
        cause instanceof Error ? cause.message : 'Failed to save inline edit configuration';
    } finally {
      savingInlineEditPropertyId = '';
    }
  }

  async function handleClearInlineEditConfig(property: Property) {
    if (!objectTypeId) {
      return;
    }

    savingInlineEditPropertyId = property.id;
    propertyConfigError = '';
    propertyConfigSuccess = '';

    try {
      await updateProperty(objectTypeId, property.id, {
        inline_edit_config: null,
      });
      propertyConfigSuccess = `Inline edit cleared for ${property.display_name}.`;
      await load();
    } catch (cause) {
      propertyConfigError =
        cause instanceof Error ? cause.message : 'Failed to clear inline edit configuration';
    } finally {
      savingInlineEditPropertyId = '';
    }
  }

  async function handleExecuteInlineEdit(objectId: string, property: Property) {
    if (!objectTypeId) {
      return;
    }

    const draftKey = inlineEditFieldKey(objectId, property.id);
    const rawValue = objectInlineEditDrafts[draftKey] ?? '';

    executingInlineEditKey = draftKey;
    objectError = '';
    objectSuccess = '';
    runtimeError = '';
    execution = null;

    try {
      const value = parseInlineEditSubmissionValue(property, rawValue);
      selectedTargetObjectId = objectId;
      execution = await executeInlineEdit(objectTypeId, objectId, property.id, { value });
      objectSuccess = `Inline edit applied through ${actionLabelById(property.inline_edit_config?.action_type_id)}.`;
      await load();
    } catch (cause) {
      objectError = cause instanceof Error ? cause.message : 'Failed to apply inline edit';
    } finally {
      executingInlineEditKey = '';
    }
  }

  async function handleCreateAction(event: Event) {
    event.preventDefault();
    if (!objectTypeId) {
      return;
    }

    creatingAction = true;
    actionFormError = '';
    actionFormSuccess = '';

    try {
      if (!actionName.trim()) {
        throw new Error('Action name is required');
      }

      const inputSchema = parseJsonArray<ActionInputField>(actionInputSchemaText, 'Action input schema');
      const formSchema = parseJsonObject(actionFormSchemaText, 'Action form schema') as ActionFormSchema;
      const config = parseJsonValue(actionConfigText, 'Action config', {});
      const authorizationPolicy = parseJsonObject(
        actionAuthorizationPolicyText,
        'Action authorization policy',
      ) as ActionAuthorizationPolicy;
      const created = await createActionType({
        name: actionName.trim(),
        display_name: actionDisplayName.trim() || undefined,
        description: actionDescription.trim() || undefined,
        object_type_id: objectTypeId,
        operation_kind: actionOperationKind,
        input_schema: inputSchema,
        form_schema: formSchema,
        config,
        confirmation_required: actionConfirmationRequired,
        permission_key: actionPermissionKey.trim() || undefined,
        authorization_policy: authorizationPolicy,
      });

      selectedActionId = created.id;
      actionFormSuccess = `Created action ${created.display_name}.`;
      validation = null;
      execution = null;
      await load();
    } catch (cause) {
      actionFormError = cause instanceof Error ? cause.message : 'Failed to create action type';
    } finally {
      creatingAction = false;
    }
  }

  async function handleDeleteAction(id: string) {
    if (!confirm('Delete this action type?')) {
      return;
    }

    actionFormError = '';
    actionFormSuccess = '';

    try {
      await deleteActionType(id);
      if (selectedActionId === id) {
        selectedActionId = '';
        validation = null;
        execution = null;
        actionWhatIfBranches = [];
      }
      await load();
    } catch (cause) {
      actionFormError = cause instanceof Error ? cause.message : 'Failed to delete action type';
    }
  }

  async function handleCreateWhatIfBranch() {
    const action = getSelectedAction();
    if (!action) {
      runtimeError = 'Select an action first';
      return;
    }

    creatingWhatIfBranch = true;
    runtimeError = '';

    try {
      const branch = await createActionWhatIfBranch(action.id, {
        ...buildInvocationBody(action),
        name: whatIfBranchName.trim() || undefined,
        description: whatIfBranchDescription.trim() || undefined,
      });
      whatIfBranchName = '';
      whatIfBranchDescription = '';
      validation = {
        valid: true,
        errors: [],
        preview: branch.preview,
      };
      await loadActionWhatIfHistory(action.id);
    } catch (cause) {
      runtimeError = cause instanceof Error ? cause.message : 'Failed to create what-if branch';
    } finally {
      creatingWhatIfBranch = false;
    }
  }

  async function handleDeleteWhatIfBranch(branchId: string) {
    const action = getSelectedAction();
    if (!action || !confirm('Delete this what-if branch?')) {
      return;
    }

    deletingWhatIfBranchId = branchId;
    runtimeError = '';

    try {
      await deleteActionWhatIfBranch(action.id, branchId);
      await loadActionWhatIfHistory(action.id);
    } catch (cause) {
      runtimeError = cause instanceof Error ? cause.message : 'Failed to delete what-if branch';
    } finally {
      deletingWhatIfBranchId = '';
    }
  }

  function buildInvocationBody(action: ActionType) {
    if (operationRequiresTarget(action.operation_kind) && !selectedTargetObjectId) {
      throw new Error('This action requires a target object. Create or select one first.');
    }

    return {
      target_object_id: selectedTargetObjectId || undefined,
      parameters: parseJsonObject(actionParametersText, 'Action parameters'),
    };
  }

  async function handleValidateAction() {
    const action = getSelectedAction();
    if (!action) {
      runtimeError = 'Select an action first';
      return;
    }

    validatingAction = true;
    runtimeError = '';
    execution = null;

    try {
      validation = await validateAction(action.id, buildInvocationBody(action));
    } catch (cause) {
      runtimeError = cause instanceof Error ? cause.message : 'Failed to validate action';
    } finally {
      validatingAction = false;
    }
  }

  async function handleExecuteAction() {
    const action = getSelectedAction();
    if (!action) {
      runtimeError = 'Select an action first';
      return;
    }

    executingAction = true;
    runtimeError = '';

    try {
      execution = await executeAction(action.id, {
        ...buildInvocationBody(action),
        justification: actionJustification.trim() || undefined,
      });
      await load();
    } catch (cause) {
      runtimeError = cause instanceof Error ? cause.message : 'Failed to execute action';
    } finally {
      executingAction = false;
    }
  }

  async function handleCreateFunctionPackage(event: Event) {
    event.preventDefault();
    if (!objectTypeId) return;

    creatingFunctionPackage = true;
    functionFormError = '';
    functionFormSuccess = '';

    try {
      const capabilities = parseJsonObject(
        functionCapabilitiesText,
        'Function capabilities',
      ) as Partial<FunctionCapabilities>;
      const created = await createFunctionPackage({
        name: functionName.trim(),
        version: functionVersion.trim() || undefined,
        display_name: functionDisplayName.trim() || undefined,
        description: functionDescription.trim() || undefined,
        runtime: functionRuntime,
        source: functionSourceText,
        entrypoint: functionEntrypoint,
        capabilities,
      });
      selectedFunctionPackageId = created.id;
      functionFormSuccess = `Created function package ${created.display_name}.`;
      functionName = '';
      functionVersion = '0.1.0';
      functionDisplayName = '';
      functionDescription = '';
      await load();
    } catch (cause) {
      functionFormError = cause instanceof Error ? cause.message : 'Failed to create function package';
    } finally {
      creatingFunctionPackage = false;
    }
  }

  function usePinnedFunctionPackageInActionConfig(packageId: string) {
    actionOperationKind = 'invoke_function';
    actionConfigText = formatJson({ function_package_id: packageId });
  }

  function useVersionedFunctionPackageInActionConfig(
    functionPackage: FunctionPackage,
    autoUpgrade = false,
  ) {
    actionOperationKind = 'invoke_function';
    actionConfigText = formatJson({
      function_package_name: functionPackage.name,
      function_package_version: functionPackage.version,
      ...(autoUpgrade ? { function_package_auto_upgrade: true } : {}),
    });
  }

  async function handleSimulateFunctionPackage(packageId: string) {
    if (!objectTypeId) return;

    functionRuntimeLoading = true;
    functionRuntimeError = '';
    functionRuntimeResult = null;
    selectedFunctionPackageId = packageId;

    try {
      const result = await simulateFunctionPackage(packageId, {
        object_type_id: objectTypeId,
        target_object_id: selectedTargetObjectId || undefined,
        parameters: parseJsonObject(actionParametersText, 'Function package parameters'),
      });
      functionRuntimeResult = result as unknown as Record<string, unknown>;
      await loadFunctionPackageMonitoring(packageId);
    } catch (cause) {
      functionRuntimeError = cause instanceof Error ? cause.message : 'Failed to simulate function package';
      await loadFunctionPackageMonitoring(packageId);
    } finally {
      functionRuntimeLoading = false;
    }
  }

  async function handleCreateRule(event: Event) {
    event.preventDefault();
    if (!objectTypeId) return;

    creatingRule = true;
    ruleFormError = '';
    ruleFormSuccess = '';

    try {
      const triggerSpec = parseJsonObject(ruleTriggerSpecText, 'Rule trigger spec');
      const effectSpec = parseJsonObject(ruleEffectSpecText, 'Rule effect spec');
      const created = await createRule({
        name: ruleName.trim(),
        display_name: ruleDisplayName.trim() || undefined,
        description: ruleDescription.trim() || undefined,
        object_type_id: objectTypeId,
        evaluation_mode: ruleEvaluationMode,
        trigger_spec: triggerSpec,
        effect_spec: effectSpec,
      });
      ruleFormSuccess = `Created rule ${created.display_name}.`;
      ruleName = '';
      ruleDisplayName = '';
      ruleDescription = '';
      await load();
    } catch (cause) {
      ruleFormError = cause instanceof Error ? cause.message : 'Failed to create rule';
    } finally {
      creatingRule = false;
    }
  }

  async function handleRuleRuntime(ruleId: string, mode: 'simulate' | 'apply') {
    if (!selectedTargetObjectId) {
      ruleRuntimeError = 'Select a target object first';
      return;
    }

    ruleRuntimeLoading = true;
    ruleRuntimeError = '';
    ruleRuntimeResult = null;

    try {
      const body = {
        object_id: selectedTargetObjectId,
        properties_patch: parseJsonObject(simulationPatchText, 'Rule simulation patch'),
      };
      const result =
        mode === 'apply' ? await applyRule(ruleId, body) : await simulateRule(ruleId, body);
      ruleRuntimeResult = result as unknown as Record<string, unknown>;
      if (mode === 'apply') {
        await load();
      }
    } catch (cause) {
      ruleRuntimeError = cause instanceof Error ? cause.message : `Failed to ${mode} rule`;
    } finally {
      ruleRuntimeLoading = false;
    }
  }

  async function handleSimulateObject() {
    if (!objectTypeId || !selectedTargetObjectId) {
      runtimeError = 'Select a target object first';
      return;
    }

    simulationLoading = true;
    runtimeError = '';
    simulation = null;

    try {
      simulation = await simulateObject(objectTypeId, selectedTargetObjectId, {
        action_id: selectedActionId || undefined,
        action_parameters: parseJsonObject(actionParametersText, 'Simulation action parameters'),
        properties_patch: parseJsonObject(simulationPatchText, 'Simulation patch'),
        depth: 2,
      });
    } catch (cause) {
      runtimeError = cause instanceof Error ? cause.message : 'Failed to simulate object';
    } finally {
      simulationLoading = false;
    }
  }

  async function handleSimulateScenarios() {
    if (!objectTypeId || !selectedTargetObjectId) {
      runtimeError = 'Select a target object first';
      return;
    }

    scenarioLoading = true;
    runtimeError = '';
    scenarioComparison = null;

    try {
      scenarioComparison = await simulateObjectScenarios(objectTypeId, selectedTargetObjectId, {
        scenarios: parseJsonArray<ScenarioSimulationCandidate>(scenarioCandidatesText, 'Scenario candidates'),
        constraints: parseJsonArray<ScenarioMetricSpec>(scenarioConstraintsText, 'Scenario constraints'),
        goals: parseJsonArray<ScenarioGoalSpec>(scenarioGoalsText, 'Scenario goals'),
        depth: 2,
        max_iterations: 6,
        include_baseline: true,
      });
    } catch (cause) {
      runtimeError = cause instanceof Error ? cause.message : 'Failed to compare scenarios';
    } finally {
      scenarioLoading = false;
    }
  }

  onMount(() => {
    void load();
  });
</script>

<svelte:head>
  <title>OpenFoundry - Ontology Type Details</title>
</svelte:head>

{#if loading}
  <div class="rounded-[1.75rem] border border-dashed border-slate-300 px-6 py-20 text-center text-sm text-slate-500 dark:border-slate-700">
    Loading ontology detail page...
  </div>
{:else if error || !objectType}
  <div class="rounded-[1.75rem] border border-rose-200 bg-rose-50 px-6 py-12 text-sm text-rose-700 dark:border-rose-900/40 dark:bg-rose-950/20 dark:text-rose-300">
    {error || 'Object type not found.'}
  </div>
{:else}
  <div class="space-y-6">
    <div class="flex flex-wrap items-start justify-between gap-4 rounded-[2rem] border border-slate-200 bg-white p-6 shadow-sm dark:border-slate-800 dark:bg-slate-900">
      <div class="space-y-3">
        <div class="flex items-center gap-3">
          {#if objectType.icon}
            <span class="text-3xl">{objectType.icon}</span>
          {:else}
            <span
              class="flex h-12 w-12 items-center justify-center rounded-2xl text-lg font-semibold text-white"
              style={`background-color: ${objectType.color || '#0f766e'}`}
            >
              {objectType.name.slice(0, 1).toUpperCase()}
            </span>
          {/if}
          <div>
            <p class="text-xs uppercase tracking-[0.2em] text-slate-500">Object Type</p>
            <h1 class="text-3xl font-semibold tracking-tight text-slate-950 dark:text-slate-50">{objectType.display_name}</h1>
            <p class="mt-1 font-mono text-xs text-slate-500">{objectType.name}</p>
          </div>
        </div>
        <p class="max-w-3xl text-sm text-slate-600 dark:text-slate-300">
          {objectType.description || 'No description has been set for this object type yet.'}
        </p>
      </div>

      <div class="grid min-w-[220px] gap-3 text-sm text-slate-600 dark:text-slate-300">
        <div class="rounded-2xl bg-slate-100 px-4 py-3 dark:bg-slate-800/70">
          <div class="text-xs uppercase tracking-[0.2em] text-slate-500">Properties</div>
          <div class="mt-1 text-2xl font-semibold text-slate-900 dark:text-slate-100">{properties.length}</div>
        </div>
        <div class="rounded-2xl bg-slate-100 px-4 py-3 dark:bg-slate-800/70">
          <div class="text-xs uppercase tracking-[0.2em] text-slate-500">Shared Property Types</div>
          <div class="mt-1 text-2xl font-semibold text-slate-900 dark:text-slate-100">{attachedSharedPropertyTypes.length}</div>
        </div>
        <div class="rounded-2xl bg-slate-100 px-4 py-3 dark:bg-slate-800/70">
          <div class="text-xs uppercase tracking-[0.2em] text-slate-500">Objects</div>
          <div class="mt-1 text-2xl font-semibold text-slate-900 dark:text-slate-100">{objects.length}</div>
        </div>
        <div class="rounded-2xl bg-slate-100 px-4 py-3 dark:bg-slate-800/70">
          <div class="text-xs uppercase tracking-[0.2em] text-slate-500">Action Types</div>
          <div class="mt-1 text-2xl font-semibold text-slate-900 dark:text-slate-100">{actions.length}</div>
        </div>
      </div>
    </div>

    <div class="grid gap-6 lg:grid-cols-2">
      <section class="rounded-[1.75rem] border border-slate-200 bg-white p-6 shadow-sm dark:border-slate-800 dark:bg-slate-900">
        <div class="flex items-center justify-between gap-3">
          <div>
            <h2 class="text-lg font-semibold text-slate-950 dark:text-slate-50">Properties</h2>
            <p class="mt-1 text-sm text-slate-500">Direct definitions and reusable shared property types both flow into the effective schema used for object validation.</p>
          </div>
          <a href="/ontology/graph" class="rounded-full border border-slate-300 px-3 py-1.5 text-xs font-medium text-slate-700 hover:bg-slate-100 dark:border-slate-700 dark:text-slate-200 dark:hover:bg-slate-800">
            Open graph view
          </a>
        </div>

        {#if propertyConfigError}
          <div class="mt-4 rounded-2xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-700 dark:border-rose-900/40 dark:bg-rose-950/20 dark:text-rose-300">
            {propertyConfigError}
          </div>
        {/if}

        {#if propertyConfigSuccess}
          <div class="mt-4 rounded-2xl border border-emerald-200 bg-emerald-50 px-4 py-3 text-sm text-emerald-700 dark:border-emerald-900/40 dark:bg-emerald-950/20 dark:text-emerald-300">
            {propertyConfigSuccess}
          </div>
        {/if}

        {#if properties.length === 0}
          <div class="mt-4 rounded-2xl border border-dashed border-slate-300 px-4 py-6 text-sm text-slate-500 dark:border-slate-700">
            No properties have been defined yet.
          </div>
        {:else}
          <div class="mt-4 space-y-3">
            {#each properties as property (property.id)}
              <div class="rounded-2xl border border-slate-200 px-4 py-3 dark:border-slate-800">
                <div class="flex flex-wrap items-center gap-2">
                  <h3 class="font-medium text-slate-900 dark:text-slate-100">{property.display_name}</h3>
                  <span class="rounded-full bg-slate-100 px-2 py-0.5 font-mono text-xs text-slate-600 dark:bg-slate-800 dark:text-slate-300">{property.name}</span>
                  <span class="rounded-full bg-teal-50 px-2 py-0.5 text-xs text-teal-700 dark:bg-teal-950/40 dark:text-teal-300">{property.property_type}</span>
                  {#if property.required}
                    <span class="rounded-full bg-amber-50 px-2 py-0.5 text-xs text-amber-700 dark:bg-amber-950/40 dark:text-amber-300">required</span>
                  {/if}
                  {#if property.time_dependent}
                    <span class="rounded-full bg-sky-50 px-2 py-0.5 text-xs text-sky-700 dark:bg-sky-950/40 dark:text-sky-300">time-dependent</span>
                  {/if}
                </div>
                {#if property.description}
                  <p class="mt-2 text-sm text-slate-500">{property.description}</p>
                {/if}
                <div class="mt-3 rounded-2xl bg-slate-50 p-4 dark:bg-slate-950/40">
                  <div class="flex flex-wrap items-center justify-between gap-2">
                    <div>
                      <p class="text-sm font-medium text-slate-900 dark:text-slate-100">Action-backed inline edit</p>
                      <p class="mt-1 text-xs text-slate-500">
                        Back this property with an `update_object` action. At execution time the edited value is sent through the action and other mapped inputs are prefilled from the current object when possible.
                      </p>
                    </div>
                    <span class={`rounded-full px-2.5 py-1 text-xs font-medium ${property.inline_edit_config ? 'bg-emerald-50 text-emerald-700 dark:bg-emerald-950/40 dark:text-emerald-300' : 'bg-slate-200 text-slate-700 dark:bg-slate-800 dark:text-slate-300'}`}>
                      {property.inline_edit_config ? 'Configured' : 'Disabled'}
                    </span>
                  </div>
                  <div class="mt-4 grid gap-3 xl:grid-cols-[minmax(0,1fr)_minmax(0,0.75fr)_auto_auto]">
                    <div>
                      <label
                        class="mb-1 block text-xs font-medium uppercase tracking-[0.2em] text-slate-500"
                        for={`inline-action-${property.id}`}
                      >
                        Update action
                      </label>
                      <select
                        id={`inline-action-${property.id}`}
                        bind:value={propertyInlineEditActionSelections[property.id]}
                        class="w-full rounded-2xl border border-slate-300 px-4 py-2.5 text-sm dark:border-slate-700 dark:bg-slate-950 dark:text-slate-100"
                      >
                        <option value="">Select action</option>
                        {#each inlineEditActionOptions as action (action.id)}
                          <option value={action.id}>{action.display_name}</option>
                        {/each}
                      </select>
                    </div>
                    <div>
                      <label
                        class="mb-1 block text-xs font-medium uppercase tracking-[0.2em] text-slate-500"
                        for={`inline-input-${property.id}`}
                      >
                        Input name
                      </label>
                      <input
                        id={`inline-input-${property.id}`}
                        bind:value={propertyInlineEditInputNames[property.id]}
                        class="w-full rounded-2xl border border-slate-300 px-4 py-2.5 font-mono text-sm dark:border-slate-700 dark:bg-slate-950 dark:text-slate-100"
                        placeholder="Auto-detect"
                      />
                    </div>
                    <button
                      type="button"
                      class="rounded-full bg-teal-600 px-4 py-2 text-sm font-medium text-white hover:bg-teal-700 disabled:opacity-50"
                      disabled={savingInlineEditPropertyId === property.id}
                      onclick={() => void handleSaveInlineEditConfig(property)}
                    >
                      {savingInlineEditPropertyId === property.id ? 'Saving...' : 'Save'}
                    </button>
                    <button
                      type="button"
                      class="rounded-full border border-slate-300 px-4 py-2 text-sm font-medium text-slate-700 hover:bg-slate-100 disabled:opacity-50 dark:border-slate-700 dark:text-slate-200 dark:hover:bg-slate-800"
                      disabled={savingInlineEditPropertyId === property.id}
                      onclick={() => void handleClearInlineEditConfig(property)}
                    >
                      Clear
                    </button>
                  </div>
                  {#if property.inline_edit_config}
                    <p class="mt-3 text-xs text-slate-500">
                      Current action: <span class="font-medium text-slate-700 dark:text-slate-200">{actionLabelById(property.inline_edit_config.action_type_id)}</span>
                    </p>
                  {/if}
                </div>
              </div>
            {/each}
          </div>
        {/if}

        <div class="mt-6 border-t border-slate-200 pt-6 dark:border-slate-800">
          <div class="flex items-center justify-between gap-3">
            <div>
              <h3 class="text-base font-semibold text-slate-950 dark:text-slate-50">Shared Property Types</h3>
              <p class="mt-1 text-sm text-slate-500">Reusable property contracts that can be attached across multiple object types without duplicating schema definitions.</p>
            </div>
            <span class="rounded-full bg-slate-100 px-3 py-1 text-xs text-slate-600 dark:bg-slate-800 dark:text-slate-300">{attachedSharedPropertyTypes.length} attached</span>
          </div>

          {#if sharedPropertyFormError}
            <div class="mt-4 rounded-2xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-700 dark:border-rose-900/40 dark:bg-rose-950/20 dark:text-rose-300">
              {sharedPropertyFormError}
            </div>
          {/if}

          {#if sharedPropertyFormSuccess}
            <div class="mt-4 rounded-2xl border border-emerald-200 bg-emerald-50 px-4 py-3 text-sm text-emerald-700 dark:border-emerald-900/40 dark:bg-emerald-950/20 dark:text-emerald-300">
              {sharedPropertyFormSuccess}
            </div>
          {/if}

          {#if attachedSharedPropertyTypes.length === 0}
            <div class="mt-4 rounded-2xl border border-dashed border-slate-300 px-4 py-6 text-sm text-slate-500 dark:border-slate-700">
              No shared property types are attached yet.
            </div>
          {:else}
            <div class="mt-4 space-y-3">
              {#each attachedSharedPropertyTypes as sharedProperty (sharedProperty.id)}
                <div class="rounded-2xl border border-slate-200 px-4 py-3 dark:border-slate-800">
                  <div class="flex flex-wrap items-start justify-between gap-3">
                    <div>
                      <div class="flex flex-wrap items-center gap-2">
                        <h4 class="font-medium text-slate-900 dark:text-slate-100">{sharedProperty.display_name}</h4>
                        <span class="rounded-full bg-slate-100 px-2 py-0.5 font-mono text-xs text-slate-600 dark:bg-slate-800 dark:text-slate-300">{sharedProperty.name}</span>
                        <span class="rounded-full bg-indigo-50 px-2 py-0.5 text-xs text-indigo-700 dark:bg-indigo-950/40 dark:text-indigo-300">{sharedProperty.property_type}</span>
                        {#if sharedProperty.required}
                          <span class="rounded-full bg-amber-50 px-2 py-0.5 text-xs text-amber-700 dark:bg-amber-950/40 dark:text-amber-300">required</span>
                        {/if}
                        {#if sharedProperty.unique_constraint}
                          <span class="rounded-full bg-fuchsia-50 px-2 py-0.5 text-xs text-fuchsia-700 dark:bg-fuchsia-950/40 dark:text-fuchsia-300">unique</span>
                        {/if}
                        {#if sharedProperty.time_dependent}
                          <span class="rounded-full bg-sky-50 px-2 py-0.5 text-xs text-sky-700 dark:bg-sky-950/40 dark:text-sky-300">time-dependent</span>
                        {/if}
                      </div>
                      {#if sharedProperty.description}
                        <p class="mt-2 text-sm text-slate-500">{sharedProperty.description}</p>
                      {/if}
                    </div>
                    <button
                      type="button"
                      class="rounded-full border border-slate-300 px-3 py-1 text-xs font-medium text-slate-700 hover:bg-slate-100 disabled:opacity-50 dark:border-slate-700 dark:text-slate-200 dark:hover:bg-slate-800"
                      disabled={detachingSharedPropertyTypeId === sharedProperty.id}
                      onclick={() => void handleDetachSharedPropertyType(sharedProperty.id)}
                    >
                      {detachingSharedPropertyTypeId === sharedProperty.id ? 'Detaching...' : 'Detach'}
                    </button>
                  </div>
                </div>
              {/each}
            </div>
          {/if}

          <div class="mt-4 grid gap-3 rounded-2xl border border-slate-200 p-4 dark:border-slate-800 lg:grid-cols-[1fr_auto]">
            <div>
              <label for="attach-shared-property-type" class="mb-1 block text-sm font-medium text-slate-700 dark:text-slate-200">Attach existing shared property type</label>
              <select
                id="attach-shared-property-type"
                bind:value={selectedSharedPropertyTypeId}
                class="w-full rounded-2xl border border-slate-300 px-4 py-2.5 text-sm dark:border-slate-700 dark:bg-slate-950 dark:text-slate-100"
                disabled={attachableSharedPropertyTypes.length === 0}
              >
                {#if attachableSharedPropertyTypes.length === 0}
                  <option value="">No reusable property types available</option>
                {:else}
                  {#each attachableSharedPropertyTypes as sharedProperty (sharedProperty.id)}
                    <option value={sharedProperty.id}>
                      {sharedProperty.display_name} ({sharedProperty.property_type})
                    </option>
                  {/each}
                {/if}
              </select>
            </div>
            <div class="flex items-end">
              <button
                type="button"
                class="rounded-full bg-slate-900 px-4 py-2 text-sm font-medium text-white hover:bg-slate-700 disabled:opacity-50 dark:bg-slate-100 dark:text-slate-900 dark:hover:bg-slate-200"
                disabled={attachingSharedPropertyType || attachableSharedPropertyTypes.length === 0}
                onclick={() => void handleAttachSharedPropertyType()}
              >
                {attachingSharedPropertyType ? 'Attaching...' : 'Attach existing'}
              </button>
            </div>
          </div>

          <form class="mt-4 space-y-4 rounded-2xl border border-slate-200 p-4 dark:border-slate-800" onsubmit={handleCreateSharedPropertyType}>
            <div class="flex items-center justify-between gap-3">
              <div>
                <h4 class="text-sm font-semibold text-slate-900 dark:text-slate-100">Create reusable property type</h4>
                <p class="mt-1 text-sm text-slate-500">This creates a shared property definition and immediately attaches it to the current object type.</p>
              </div>
            </div>

            <div class="grid gap-4 md:grid-cols-2">
              <div>
                <label for="shared-property-name" class="mb-1 block text-sm font-medium text-slate-700 dark:text-slate-200">Name</label>
                <input
                  id="shared-property-name"
                  bind:value={sharedPropertyName}
                  class="w-full rounded-2xl border border-slate-300 px-4 py-2.5 font-mono text-sm dark:border-slate-700 dark:bg-slate-950 dark:text-slate-100"
                  placeholder="status"
                />
              </div>
              <div>
                <label for="shared-property-display-name" class="mb-1 block text-sm font-medium text-slate-700 dark:text-slate-200">Display name</label>
                <input
                  id="shared-property-display-name"
                  bind:value={sharedPropertyDisplayName}
                  class="w-full rounded-2xl border border-slate-300 px-4 py-2.5 text-sm dark:border-slate-700 dark:bg-slate-950 dark:text-slate-100"
                  placeholder="Status"
                />
              </div>
            </div>

            <div class="grid gap-4 md:grid-cols-[1fr_auto]">
              <div>
                <label for="shared-property-description" class="mb-1 block text-sm font-medium text-slate-700 dark:text-slate-200">Description</label>
                <input
                  id="shared-property-description"
                  bind:value={sharedPropertyDescription}
                  class="w-full rounded-2xl border border-slate-300 px-4 py-2.5 text-sm dark:border-slate-700 dark:bg-slate-950 dark:text-slate-100"
                  placeholder="Reusable lifecycle status across operational types"
                />
              </div>
              <div>
                <label for="shared-property-type" class="mb-1 block text-sm font-medium text-slate-700 dark:text-slate-200">Type</label>
                <select id="shared-property-type" bind:value={sharedPropertyType} class="w-full rounded-2xl border border-slate-300 px-4 py-2.5 text-sm dark:border-slate-700 dark:bg-slate-950 dark:text-slate-100">
                  {#each propertyTypeOptions as option}
                    <option value={option}>{option}</option>
                  {/each}
                </select>
              </div>
            </div>

            <div class="flex flex-wrap gap-4 text-sm text-slate-600 dark:text-slate-300">
              <label class="inline-flex items-center gap-2">
                <input bind:checked={sharedPropertyRequired} type="checkbox" class="rounded border-slate-300 text-indigo-600 focus:ring-indigo-500 dark:border-slate-700" />
                Required
              </label>
              <label class="inline-flex items-center gap-2">
                <input bind:checked={sharedPropertyUniqueConstraint} type="checkbox" class="rounded border-slate-300 text-indigo-600 focus:ring-indigo-500 dark:border-slate-700" />
                Unique
              </label>
              <label class="inline-flex items-center gap-2">
                <input bind:checked={sharedPropertyTimeDependent} type="checkbox" class="rounded border-slate-300 text-indigo-600 focus:ring-indigo-500 dark:border-slate-700" />
                Time-dependent
              </label>
            </div>

            <div class="flex justify-end">
              <button type="submit" disabled={creatingSharedPropertyType} class="rounded-full bg-indigo-600 px-4 py-2 text-sm font-medium text-white hover:bg-indigo-700 disabled:opacity-50">
                {creatingSharedPropertyType ? 'Creating...' : 'Create and attach'}
              </button>
            </div>
          </form>
        </div>
      </section>

      <section class="rounded-[1.75rem] border border-slate-200 bg-white p-6 shadow-sm dark:border-slate-800 dark:bg-slate-900">
        <h2 class="text-lg font-semibold text-slate-950 dark:text-slate-50">Link Types</h2>
        <p class="mt-1 text-sm text-slate-500">Create-link actions and function responses can target any of these IDs.</p>

        {#if linkTypes.length === 0}
          <div class="mt-4 rounded-2xl border border-dashed border-slate-300 px-4 py-6 text-sm text-slate-500 dark:border-slate-700">
            No link types reference this object type yet.
          </div>
        {:else}
          <div class="mt-4 space-y-3">
            {#each linkTypes as linkType (linkType.id)}
              <div class="rounded-2xl border border-slate-200 px-4 py-3 dark:border-slate-800">
                <div class="flex flex-wrap items-center gap-2">
                  <h3 class="font-medium text-slate-900 dark:text-slate-100">{linkType.display_name}</h3>
                  <span class="rounded-full bg-slate-100 px-2 py-0.5 font-mono text-xs text-slate-600 dark:bg-slate-800 dark:text-slate-300">{linkType.id}</span>
                </div>
                <p class="mt-2 text-xs text-slate-500">{linkType.source_type_id} -> {linkType.target_type_id} ({linkType.cardinality})</p>
              </div>
            {/each}
          </div>
        {/if}
      </section>
    </div>

    <div class="grid gap-6 xl:grid-cols-[1.05fr_0.95fr]">
      <section class="rounded-[1.75rem] border border-slate-200 bg-white p-6 shadow-sm dark:border-slate-800 dark:bg-slate-900">
        <div class="flex items-center justify-between gap-3">
          <div>
            <h2 class="text-lg font-semibold text-slate-950 dark:text-slate-50">Functions Platform</h2>
            <p class="mt-1 text-sm text-slate-500">Register reusable TypeScript/Python packages with execution capabilities and reuse them from action types.</p>
          </div>
          <span class="rounded-full bg-slate-100 px-3 py-1 text-xs text-slate-600 dark:bg-slate-800 dark:text-slate-300">{functionPackages.length} packages</span>
        </div>

        {#if functionFormError}
          <div class="mt-4 rounded-2xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-700 dark:border-rose-900/40 dark:bg-rose-950/20 dark:text-rose-300">
            {functionFormError}
          </div>
        {/if}

        {#if functionFormSuccess}
          <div class="mt-4 rounded-2xl border border-emerald-200 bg-emerald-50 px-4 py-3 text-sm text-emerald-700 dark:border-emerald-900/40 dark:bg-emerald-950/20 dark:text-emerald-300">
            {functionFormSuccess}
          </div>
        {/if}

        {#if functionAuthoringSurface}
          <div class="mt-4 rounded-2xl border border-slate-200 p-4 dark:border-slate-800">
            <div class="flex flex-wrap items-start justify-between gap-3">
              <div>
                <h3 class="text-sm font-semibold text-slate-900 dark:text-slate-100">Authoring kits</h3>
                <p class="mt-1 text-xs text-slate-500">Backend-defined templates, SDK references, and CLI scaffolds for reusable ontology functions.</p>
              </div>
              <div class="rounded-full bg-slate-100 px-3 py-1 text-xs text-slate-600 dark:bg-slate-800 dark:text-slate-300">
                {functionAuthoringSurface.templates.length} templates
              </div>
            </div>

            <div class="mt-4 grid gap-3 xl:grid-cols-3">
              {#each functionAuthoringSurface.templates as template (template.id)}
                <article class="rounded-2xl border border-slate-200 p-4 dark:border-slate-800">
                  <div class="flex flex-wrap items-center gap-2">
                    <h4 class="font-medium text-slate-900 dark:text-slate-100">{template.display_name}</h4>
                    <span class="rounded-full bg-slate-100 px-2 py-0.5 text-xs text-slate-600 dark:bg-slate-800 dark:text-slate-300">{template.runtime}</span>
                  </div>
                  <p class="mt-2 text-sm text-slate-500">{template.description}</p>
                  <div class="mt-3 flex flex-wrap gap-2">
                    {#each template.recommended_use_cases as useCase}
                      <span class="rounded-full bg-indigo-50 px-2 py-0.5 text-xs text-indigo-700 dark:bg-indigo-950/30 dark:text-indigo-200">{useCase}</span>
                    {/each}
                  </div>
                  <div class="mt-3 text-xs text-slate-500">
                    Entrypoint: <span class="font-mono">{template.entrypoint}</span>
                    {#if template.cli_scaffold_template}
                      · CLI scaffold: <span class="font-mono">{template.cli_scaffold_template}</span>
                    {/if}
                  </div>
                  <div class="mt-4 flex justify-end">
                    <button
                      type="button"
                      class="rounded-full border border-indigo-300 px-3 py-1 text-xs font-medium text-indigo-700 hover:bg-indigo-50 dark:border-indigo-600 dark:text-indigo-200 dark:hover:bg-indigo-950/30"
                      onclick={() => applyFunctionAuthoringTemplate(template)}
                    >
                      Apply template
                    </button>
                  </div>
                </article>
              {/each}
            </div>

            <div class="mt-4 grid gap-4 lg:grid-cols-2">
              <div class="rounded-2xl bg-slate-50 p-4 dark:bg-slate-900/60">
                <h4 class="text-sm font-semibold text-slate-900 dark:text-slate-100">SDK packages</h4>
                <div class="mt-3 space-y-3 text-xs text-slate-500">
                  {#each functionAuthoringSurface.sdk_packages as sdkPackage (sdkPackage.language)}
                    <div class="rounded-2xl border border-slate-200 bg-white px-3 py-3 dark:border-slate-800 dark:bg-slate-950">
                      <div class="flex flex-wrap items-center gap-2">
                        <span class="font-medium text-slate-900 dark:text-slate-100">{sdkPackage.language}</span>
                        <span class="rounded-full bg-slate-100 px-2 py-0.5 font-mono text-slate-600 dark:bg-slate-800 dark:text-slate-300">{sdkPackage.package_name}</span>
                      </div>
                      <div class="mt-2 font-mono text-[11px]">{sdkPackage.path}</div>
                    </div>
                  {/each}
                </div>
              </div>

              <div class="rounded-2xl bg-slate-50 p-4 dark:bg-slate-900/60">
                <h4 class="text-sm font-semibold text-slate-900 dark:text-slate-100">CLI scaffolds</h4>
                <div class="mt-3 space-y-3">
                  {#each functionAuthoringSurface.cli_commands as command}
                    <pre class="overflow-x-auto rounded-2xl bg-slate-950 px-3 py-3 text-[11px] text-slate-100">{command}</pre>
                  {/each}
                </div>
              </div>
            </div>
          </div>
        {/if}

        <form class="mt-4 space-y-4" onsubmit={handleCreateFunctionPackage}>
          <div class="grid gap-4 md:grid-cols-2">
            <div>
              <label for="function-package-name" class="mb-1 block text-sm font-medium text-slate-700 dark:text-slate-200">Package name</label>
              <input
                id="function-package-name"
                bind:value={functionName}
                class="w-full rounded-2xl border border-slate-300 px-4 py-2.5 font-mono text-sm dark:border-slate-700 dark:bg-slate-950 dark:text-slate-100"
                placeholder="customer_triage"
              />
            </div>
            <div>
              <label for="function-package-version" class="mb-1 block text-sm font-medium text-slate-700 dark:text-slate-200">Version</label>
              <input
                id="function-package-version"
                bind:value={functionVersion}
                class="w-full rounded-2xl border border-slate-300 px-4 py-2.5 font-mono text-sm dark:border-slate-700 dark:bg-slate-950 dark:text-slate-100"
                placeholder="1.0.0"
              />
            </div>
          </div>

          <div class="grid gap-4 md:grid-cols-1">
            <div>
              <label for="function-package-display-name" class="mb-1 block text-sm font-medium text-slate-700 dark:text-slate-200">Display name</label>
              <input
                id="function-package-display-name"
                bind:value={functionDisplayName}
                class="w-full rounded-2xl border border-slate-300 px-4 py-2.5 text-sm dark:border-slate-700 dark:bg-slate-950 dark:text-slate-100"
                placeholder="Customer triage"
              />
            </div>
          </div>

          <div class="grid gap-4 md:grid-cols-[1fr_auto_auto]">
            <div>
              <label for="function-package-description" class="mb-1 block text-sm font-medium text-slate-700 dark:text-slate-200">Description</label>
              <input
                id="function-package-description"
                bind:value={functionDescription}
                class="w-full rounded-2xl border border-slate-300 px-4 py-2.5 text-sm dark:border-slate-700 dark:bg-slate-950 dark:text-slate-100"
                placeholder="Reusable object triage flow"
              />
            </div>
            <div>
              <label for="function-package-runtime" class="mb-1 block text-sm font-medium text-slate-700 dark:text-slate-200">Runtime</label>
              <select id="function-package-runtime" bind:value={functionRuntime} class="w-full rounded-2xl border border-slate-300 px-4 py-2.5 text-sm dark:border-slate-700 dark:bg-slate-950 dark:text-slate-100">
                <option value="typescript">typescript</option>
                <option value="python">python</option>
              </select>
            </div>
            <div>
              <label for="function-package-entrypoint" class="mb-1 block text-sm font-medium text-slate-700 dark:text-slate-200">Entrypoint</label>
              <select id="function-package-entrypoint" bind:value={functionEntrypoint} class="w-full rounded-2xl border border-slate-300 px-4 py-2.5 text-sm dark:border-slate-700 dark:bg-slate-950 dark:text-slate-100">
                <option value="default">default</option>
                <option value="handler">handler</option>
              </select>
            </div>
          </div>

          <div class="grid gap-4 lg:grid-cols-2">
            <div>
              <label for="function-package-capabilities" class="mb-1 block text-sm font-medium text-slate-700 dark:text-slate-200">Capabilities JSON</label>
              <textarea id="function-package-capabilities" bind:value={functionCapabilitiesText} rows={10} class="w-full rounded-2xl border border-slate-300 bg-slate-950 px-4 py-3 font-mono text-xs text-slate-100 dark:border-slate-700" spellcheck="false"></textarea>
            </div>
            <div>
              <div class="mb-1 flex items-center justify-between gap-3">
                <label for="function-package-source" class="block text-sm font-medium text-slate-700 dark:text-slate-200">Source</label>
                <button
                  type="button"
                  class="rounded-full border border-slate-300 px-3 py-1 text-xs font-medium text-slate-700 hover:bg-slate-100 dark:border-slate-700 dark:text-slate-200 dark:hover:bg-slate-800"
                  onclick={() => applyFunctionRuntimeTemplate(functionRuntime)}
                >
                  Load {functionRuntime} template
                </button>
              </div>
              <textarea id="function-package-source" bind:value={functionSourceText} rows={10} class="w-full rounded-2xl border border-slate-300 bg-slate-950 px-4 py-3 font-mono text-xs text-slate-100 dark:border-slate-700" spellcheck="false"></textarea>
              <p class="mt-2 text-xs text-slate-500">
                Both runtimes expose `context.sdk.ontology`, `context.sdk.ai`, and `context.llm.complete(...)`. Python packages can now use `def handler(context): ...` or `def default(context): ...`.
              </p>
            </div>
          </div>

          <div class="flex justify-end">
            <button type="submit" disabled={creatingFunctionPackage} class="rounded-full bg-indigo-600 px-4 py-2 text-sm font-medium text-white hover:bg-indigo-700 disabled:opacity-50">
              {creatingFunctionPackage ? 'Saving...' : 'Create function package'}
            </button>
          </div>
        </form>

        <div class="mt-6 space-y-3">
          {#each functionPackages as functionPackage (functionPackage.id)}
            <article class={`rounded-2xl border px-4 py-4 ${selectedFunctionPackageId === functionPackage.id ? 'border-indigo-400 bg-indigo-50 dark:border-indigo-500/60 dark:bg-indigo-950/20' : 'border-slate-200 dark:border-slate-800'}`}>
              <div class="flex flex-wrap items-start justify-between gap-3">
                <div>
                  <div class="font-medium text-slate-900 dark:text-slate-100">{functionPackage.display_name}</div>
                  <div class="mt-1 flex flex-wrap items-center gap-2 text-xs text-slate-500">
                    <span class="font-mono">{functionPackage.name}</span>
                    <span class="rounded-full bg-indigo-100 px-2 py-0.5 font-mono text-indigo-700 dark:bg-indigo-950/40 dark:text-indigo-200">{functionPackage.version}</span>
                    <span class="rounded-full bg-slate-100 px-2 py-0.5 dark:bg-slate-800">{functionPackage.runtime}</span>
                    <span class="rounded-full bg-slate-100 px-2 py-0.5 dark:bg-slate-800">{functionPackage.entrypoint}</span>
                  </div>
                  {#if functionPackage.description}
                    <p class="mt-2 text-sm text-slate-500">{functionPackage.description}</p>
                  {/if}
                </div>
                <div class="flex flex-wrap gap-2">
                  <button type="button" class="rounded-full border border-slate-300 px-3 py-1 text-xs font-medium text-slate-700 hover:bg-slate-100 dark:border-slate-700 dark:text-slate-200 dark:hover:bg-slate-800" onclick={() => void handleSelectFunctionPackage(functionPackage.id)}>
                    View metrics
                  </button>
                  <button type="button" class="rounded-full border border-slate-300 px-3 py-1 text-xs font-medium text-slate-700 hover:bg-slate-100 dark:border-slate-700 dark:text-slate-200 dark:hover:bg-slate-800" onclick={() => { void handleSelectFunctionPackage(functionPackage.id); usePinnedFunctionPackageInActionConfig(functionPackage.id); }}>
                    Pin by id
                  </button>
                  <button type="button" class="rounded-full border border-indigo-300 px-3 py-1 text-xs font-medium text-indigo-700 hover:bg-indigo-50 dark:border-indigo-600 dark:text-indigo-200 dark:hover:bg-indigo-950/30" onclick={() => { void handleSelectFunctionPackage(functionPackage.id); useVersionedFunctionPackageInActionConfig(functionPackage); }}>
                    Use version
                  </button>
                  <button type="button" class="rounded-full border border-emerald-300 px-3 py-1 text-xs font-medium text-emerald-700 hover:bg-emerald-50 dark:border-emerald-600 dark:text-emerald-200 dark:hover:bg-emerald-950/30" onclick={() => { void handleSelectFunctionPackage(functionPackage.id); useVersionedFunctionPackageInActionConfig(functionPackage, true); }}>
                    Auto-upgrade
                  </button>
                  <button type="button" class="rounded-full bg-indigo-600 px-3 py-1 text-xs font-medium text-white hover:bg-indigo-700" onclick={() => handleSimulateFunctionPackage(functionPackage.id)}>
                    {functionRuntimeLoading && selectedFunctionPackageId === functionPackage.id ? 'Running...' : 'Simulate'}
                  </button>
                </div>
              </div>
              <div class="mt-3 grid gap-2 md:grid-cols-2">
                <div class="rounded-2xl bg-slate-100 px-3 py-2 text-xs text-slate-600 dark:bg-slate-800/70 dark:text-slate-300">
                  AI: {functionPackage.capabilities.allow_ai ? 'enabled' : 'disabled'} · Network: {functionPackage.capabilities.allow_network ? 'enabled' : 'disabled'}
                </div>
                <div class="rounded-2xl bg-slate-100 px-3 py-2 text-xs text-slate-600 dark:bg-slate-800/70 dark:text-slate-300">
                  Ontology write: {functionPackage.capabilities.allow_ontology_write ? 'enabled' : 'disabled'} · Timeout: {functionPackage.capabilities.timeout_seconds}s
                </div>
              </div>
            </article>
          {/each}
        </div>

        {#if functionRuntimeError}
          <div class="mt-4 rounded-2xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-700 dark:border-rose-900/40 dark:bg-rose-950/20 dark:text-rose-300">
            {functionRuntimeError}
          </div>
        {/if}

        {#if functionRuntimeResult}
          <pre class="mt-4 overflow-x-auto rounded-2xl bg-slate-950 p-4 text-xs text-slate-100">{formatJson(functionRuntimeResult)}</pre>
        {/if}

        <div class="mt-6 rounded-2xl border border-slate-200 p-4 dark:border-slate-800">
          <div class="flex flex-wrap items-center justify-between gap-3">
            <div>
              <h3 class="text-sm font-semibold text-slate-900 dark:text-slate-100">Function monitoring</h3>
              <p class="mt-1 text-xs text-slate-500">Recent executions and aggregate metrics for the selected reusable package.</p>
            </div>
            {#if functionMetrics}
              <div class="rounded-full bg-slate-100 px-3 py-1 text-xs text-slate-600 dark:bg-slate-800 dark:text-slate-300">
                {functionMetrics.package.display_name} · {functionMetrics.package.version}
              </div>
            {/if}
          </div>

          {#if functionMonitoringError}
            <div class="mt-4 rounded-2xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-700 dark:border-rose-900/40 dark:bg-rose-950/20 dark:text-rose-300">
              {functionMonitoringError}
            </div>
          {/if}

          {#if functionMonitoringLoading}
            <div class="mt-4 rounded-2xl border border-dashed border-slate-300 px-4 py-6 text-sm text-slate-500 dark:border-slate-700">
              Loading function run history...
            </div>
          {:else if functionMetrics}
            <div class="mt-4 grid gap-3 md:grid-cols-2 xl:grid-cols-4">
              <div class="rounded-2xl bg-slate-100 px-4 py-3 dark:bg-slate-800/70">
                <div class="text-xs uppercase tracking-[0.18em] text-slate-500">Executions</div>
                <div class="mt-2 text-2xl font-semibold text-slate-900 dark:text-slate-100">{functionMetrics.total_runs}</div>
                <div class="mt-1 text-xs text-slate-500">{functionMetrics.action_runs} action · {functionMetrics.simulation_runs} simulation</div>
              </div>
              <div class="rounded-2xl bg-slate-100 px-4 py-3 dark:bg-slate-800/70">
                <div class="text-xs uppercase tracking-[0.18em] text-slate-500">Success rate</div>
                <div class="mt-2 text-2xl font-semibold text-slate-900 dark:text-slate-100">{formatPercent(functionMetrics.success_rate)}</div>
                <div class="mt-1 text-xs text-slate-500">{functionMetrics.successful_runs} success · {functionMetrics.failed_runs} failure</div>
              </div>
              <div class="rounded-2xl bg-slate-100 px-4 py-3 dark:bg-slate-800/70">
                <div class="text-xs uppercase tracking-[0.18em] text-slate-500">Average duration</div>
                <div class="mt-2 text-2xl font-semibold text-slate-900 dark:text-slate-100">{formatDuration(functionMetrics.avg_duration_ms)}</div>
                <div class="mt-1 text-xs text-slate-500">P95 {formatDuration(functionMetrics.p95_duration_ms)}</div>
              </div>
              <div class="rounded-2xl bg-slate-100 px-4 py-3 dark:bg-slate-800/70">
                <div class="text-xs uppercase tracking-[0.18em] text-slate-500">Last run</div>
                <div class="mt-2 text-sm font-semibold text-slate-900 dark:text-slate-100">{formatTimestamp(functionMetrics.last_run_at)}</div>
                <div class="mt-1 text-xs text-slate-500">Last success {formatTimestamp(functionMetrics.last_success_at)}</div>
              </div>
            </div>

            {#if functionRuns.length === 0}
              <div class="mt-4 rounded-2xl border border-dashed border-slate-300 px-4 py-6 text-sm text-slate-500 dark:border-slate-700">
                No runs have been recorded for this package yet.
              </div>
            {:else}
              <div class="mt-4 overflow-x-auto rounded-2xl border border-slate-200 dark:border-slate-800">
                <table class="min-w-full divide-y divide-slate-200 text-left text-sm dark:divide-slate-800">
                  <thead class="bg-slate-50 text-xs uppercase tracking-[0.16em] text-slate-500 dark:bg-slate-900/70">
                    <tr>
                      <th class="px-4 py-3">When</th>
                      <th class="px-4 py-3">Kind</th>
                      <th class="px-4 py-3">Status</th>
                      <th class="px-4 py-3">Duration</th>
                      <th class="px-4 py-3">Context</th>
                      <th class="px-4 py-3">Error</th>
                    </tr>
                  </thead>
                  <tbody class="divide-y divide-slate-200 dark:divide-slate-800">
                    {#each functionRuns as run (run.id)}
                      <tr class="align-top">
                        <td class="px-4 py-3 text-slate-600 dark:text-slate-300">{formatTimestamp(run.completed_at)}</td>
                        <td class="px-4 py-3">
                          <span class="rounded-full bg-slate-100 px-2 py-0.5 text-xs text-slate-700 dark:bg-slate-800 dark:text-slate-200">{run.invocation_kind}</span>
                        </td>
                        <td class="px-4 py-3">
                          <span class={`rounded-full px-2 py-0.5 text-xs ${run.status === 'success' ? 'bg-emerald-100 text-emerald-700 dark:bg-emerald-950/40 dark:text-emerald-200' : 'bg-rose-100 text-rose-700 dark:bg-rose-950/40 dark:text-rose-200'}`}>{run.status}</span>
                        </td>
                        <td class="px-4 py-3 text-slate-600 dark:text-slate-300">{formatDuration(run.duration_ms)}</td>
                        <td class="px-4 py-3 text-xs text-slate-500">
                          <div>{run.action_name ?? 'Standalone simulation'}</div>
                          <div class="mt-1 font-mono">{run.target_object_id ?? 'no target object'}</div>
                        </td>
                        <td class="px-4 py-3 text-xs text-slate-500">{run.error_message ?? '—'}</td>
                      </tr>
                    {/each}
                  </tbody>
                </table>
              </div>
            {/if}
          {:else}
            <div class="mt-4 rounded-2xl border border-dashed border-slate-300 px-4 py-6 text-sm text-slate-500 dark:border-slate-700">
              Select a function package to inspect its run history.
            </div>
          {/if}
        </div>
      </section>

      <section class="rounded-[1.75rem] border border-slate-200 bg-white p-6 shadow-sm dark:border-slate-800 dark:bg-slate-900">
        <div class="flex items-center justify-between gap-3">
          <div>
            <h2 class="text-lg font-semibold text-slate-950 dark:text-slate-50">Rules & Machinery</h2>
            <p class="mt-1 text-sm text-slate-500">Model rule triggers, scheduling and alerts, then inspect run history and machinery pressure for this object type.</p>
          </div>
          <span class="rounded-full bg-slate-100 px-3 py-1 text-xs text-slate-600 dark:bg-slate-800 dark:text-slate-300">{rules.length} rules</span>
        </div>

        {#if ruleFormError}
          <div class="mt-4 rounded-2xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-700 dark:border-rose-900/40 dark:bg-rose-950/20 dark:text-rose-300">
            {ruleFormError}
          </div>
        {/if}

        {#if ruleFormSuccess}
          <div class="mt-4 rounded-2xl border border-emerald-200 bg-emerald-50 px-4 py-3 text-sm text-emerald-700 dark:border-emerald-900/40 dark:bg-emerald-950/20 dark:text-emerald-300">
            {ruleFormSuccess}
          </div>
        {/if}

        <form class="mt-4 space-y-4" onsubmit={handleCreateRule}>
          <div class="grid gap-4 md:grid-cols-[1fr_1fr_auto]">
            <div>
              <label for="ontology-rule-name" class="mb-1 block text-sm font-medium text-slate-700 dark:text-slate-200">Rule name</label>
              <input id="ontology-rule-name" bind:value={ruleName} class="w-full rounded-2xl border border-slate-300 px-4 py-2.5 font-mono text-sm dark:border-slate-700 dark:bg-slate-950 dark:text-slate-100" placeholder="escalate_high_risk" />
            </div>
            <div>
              <label for="ontology-rule-display-name" class="mb-1 block text-sm font-medium text-slate-700 dark:text-slate-200">Display name</label>
              <input id="ontology-rule-display-name" bind:value={ruleDisplayName} class="w-full rounded-2xl border border-slate-300 px-4 py-2.5 text-sm dark:border-slate-700 dark:bg-slate-950 dark:text-slate-100" placeholder="Escalate high risk" />
            </div>
            <div>
              <label for="ontology-rule-mode" class="mb-1 block text-sm font-medium text-slate-700 dark:text-slate-200">Mode</label>
              <select id="ontology-rule-mode" bind:value={ruleEvaluationMode} class="w-full rounded-2xl border border-slate-300 px-4 py-2.5 text-sm dark:border-slate-700 dark:bg-slate-950 dark:text-slate-100">
                <option value="advisory">advisory</option>
                <option value="automatic">automatic</option>
              </select>
            </div>
          </div>

          <div>
            <label for="ontology-rule-description" class="mb-1 block text-sm font-medium text-slate-700 dark:text-slate-200">Description</label>
            <input id="ontology-rule-description" bind:value={ruleDescription} class="w-full rounded-2xl border border-slate-300 px-4 py-2.5 text-sm dark:border-slate-700 dark:bg-slate-950 dark:text-slate-100" placeholder="Escalate any case above the configured risk threshold" />
          </div>

          <div class="grid gap-4 lg:grid-cols-2">
            <div>
              <label for="ontology-rule-trigger-spec" class="mb-1 block text-sm font-medium text-slate-700 dark:text-slate-200">Trigger spec JSON</label>
              <textarea id="ontology-rule-trigger-spec" bind:value={ruleTriggerSpecText} rows={9} class="w-full rounded-2xl border border-slate-300 bg-slate-950 px-4 py-3 font-mono text-xs text-slate-100 dark:border-slate-700" spellcheck="false"></textarea>
            </div>
            <div>
              <label for="ontology-rule-effect-spec" class="mb-1 block text-sm font-medium text-slate-700 dark:text-slate-200">Effect spec JSON</label>
              <textarea id="ontology-rule-effect-spec" bind:value={ruleEffectSpecText} rows={9} class="w-full rounded-2xl border border-slate-300 bg-slate-950 px-4 py-3 font-mono text-xs text-slate-100 dark:border-slate-700" spellcheck="false"></textarea>
            </div>
          </div>

          <div class="flex justify-end">
            <button type="submit" disabled={creatingRule} class="rounded-full bg-fuchsia-600 px-4 py-2 text-sm font-medium text-white hover:bg-fuchsia-700 disabled:opacity-50">
              {creatingRule ? 'Saving...' : 'Create rule'}
            </button>
          </div>
        </form>

        <div class="mt-6 grid gap-3 md:grid-cols-2">
          {#each machineryInsights as insight (insight.rule_id)}
            <div class="rounded-2xl border border-slate-200 p-4 dark:border-slate-800">
              <div class="flex flex-wrap items-center justify-between gap-3">
                <div class="font-medium text-slate-900 dark:text-slate-100">{insight.display_name}</div>
                <span class={`rounded-full px-3 py-1 text-[11px] font-medium uppercase tracking-[0.18em] ${pressureToneClass(insight.dynamic_pressure)}`}>
                  {insight.dynamic_pressure}
                </span>
              </div>
              <div class="mt-2 grid grid-cols-2 gap-2 text-xs text-slate-500">
                <span>Runs: {insight.total_runs}</span>
                <span>Matched: {insight.matched_runs}</span>
                <span>Pending schedules: {insight.pending_schedules}</span>
                <span>Overdue: {insight.overdue_schedules}</span>
                <span>Avg lead: {insight.avg_schedule_lead_hours?.toFixed(1) ?? 'n/a'}h</span>
                <span>Mode: {insight.evaluation_mode}</span>
              </div>
            </div>
          {/each}
        </div>

        <div class="mt-6 rounded-2xl border border-slate-200 p-4 dark:border-slate-800">
          <div class="flex flex-wrap items-start justify-between gap-3">
            <div>
              <h3 class="font-medium text-slate-900 dark:text-slate-100">Machinery queue</h3>
              <p class="mt-1 text-sm text-slate-500">
                Dynamic scheduling recommendations across pending rule schedules, capabilities, and due dates.
              </p>
            </div>
            {#if machineryQueue}
              <span class="rounded-full bg-slate-100 px-3 py-1 text-xs text-slate-700 dark:bg-slate-800 dark:text-slate-200">
                {machineryQueue.recommendation.strategy}
              </span>
            {/if}
          </div>

          {#if machineryQueue}
            <div class="mt-4 grid gap-3 md:grid-cols-4">
              <div class="rounded-2xl bg-slate-100 px-4 py-3 dark:bg-slate-800/70">
                <div class="text-xs uppercase tracking-[0.2em] text-slate-500">Queue depth</div>
                <div class="mt-1 text-xl font-semibold text-slate-900 dark:text-slate-100">{machineryQueue.recommendation.queue_depth}</div>
              </div>
              <div class="rounded-2xl bg-slate-100 px-4 py-3 dark:bg-slate-800/70">
                <div class="text-xs uppercase tracking-[0.2em] text-slate-500">Overdue</div>
                <div class="mt-1 text-xl font-semibold text-slate-900 dark:text-slate-100">{machineryQueue.recommendation.overdue_count}</div>
              </div>
              <div class="rounded-2xl bg-slate-100 px-4 py-3 dark:bg-slate-800/70">
                <div class="text-xs uppercase tracking-[0.2em] text-slate-500">Estimated load</div>
                <div class="mt-1 text-xl font-semibold text-slate-900 dark:text-slate-100">{machineryQueue.recommendation.total_estimated_minutes}m</div>
              </div>
              <div class="rounded-2xl bg-slate-100 px-4 py-3 dark:bg-slate-800/70">
                <div class="text-xs uppercase tracking-[0.2em] text-slate-500">Next due</div>
                <div class="mt-1 text-sm font-semibold text-slate-900 dark:text-slate-100">
                  {machineryQueue.recommendation.next_due_at ? formatTimestamp(machineryQueue.recommendation.next_due_at) : 'n/a'}
                </div>
              </div>
            </div>

            {#if machineryQueue.recommendation.capability_load.length > 0}
              <div class="mt-4">
                <p class="text-xs uppercase tracking-[0.2em] text-slate-500">Capability load</p>
                <div class="mt-2 flex flex-wrap gap-2">
                  {#each machineryQueue.recommendation.capability_load as capability}
                    <span class="rounded-full bg-slate-100 px-3 py-1 text-xs text-slate-700 dark:bg-slate-800 dark:text-slate-200">
                      {capability.capability} · {capability.pending_count} items · {capability.total_estimated_minutes}m
                    </span>
                  {/each}
                </div>
              </div>
            {/if}

            {#if machineryQueue.data.length === 0}
              <div class="mt-4 rounded-2xl border border-dashed border-slate-300 px-4 py-8 text-center text-sm text-slate-500 dark:border-slate-700">
                No pending Machinery queue items yet.
              </div>
            {:else}
              <div class="mt-4 space-y-3">
                {#each machineryQueue.data as item, index (item.id)}
                  <div class="rounded-2xl border border-slate-200 p-4 dark:border-slate-800">
                    <div class="flex flex-wrap items-start justify-between gap-3">
                      <div>
                        <div class="flex flex-wrap items-center gap-2">
                          <div class="font-medium text-slate-900 dark:text-slate-100">{item.rule_display_name}</div>
                          <span class="rounded-full bg-slate-100 px-2 py-0.5 text-[11px] text-slate-600 dark:bg-slate-800 dark:text-slate-300">
                            rank {machineryQueue.recommendation.recommended_order.indexOf(item.id) >= 0 ? machineryQueue.recommendation.recommended_order.indexOf(item.id) + 1 : index + 1}
                          </span>
                          <span class={`rounded-full px-2 py-0.5 text-[11px] ${item.status === 'pending' && new Date(item.scheduled_for) < new Date() ? 'bg-rose-50 text-rose-700 dark:bg-rose-950/40 dark:text-rose-300' : 'bg-slate-100 text-slate-600 dark:bg-slate-800 dark:text-slate-300'}`}>
                            {item.status}
                          </span>
                        </div>
                        <div class="mt-2 flex flex-wrap gap-3 text-xs text-slate-500">
                          <span>Due: {formatTimestamp(item.scheduled_for)}</span>
                          <span>Priority: {item.priority_score}</span>
                          <span>ETA: {item.estimated_duration_minutes}m</span>
                          <span>Capability: {item.required_capability ?? 'general'}</span>
                        </div>
                      </div>
                      <div class="flex flex-wrap gap-2">
                        <button
                          type="button"
                          class="rounded-full border border-slate-300 px-3 py-1 text-xs font-medium text-slate-700 hover:bg-slate-100 disabled:opacity-50 dark:border-slate-700 dark:text-slate-200 dark:hover:bg-slate-800"
                          disabled={updatingMachineryQueueItemId === item.id}
                          onclick={() => void handleMachineryQueueTransition(item.id, 'in_progress')}
                        >
                          Start
                        </button>
                        <button
                          type="button"
                          class="rounded-full bg-emerald-600 px-3 py-1 text-xs font-medium text-white hover:bg-emerald-500 disabled:opacity-50"
                          disabled={updatingMachineryQueueItemId === item.id}
                          onclick={() => void handleMachineryQueueTransition(item.id, 'completed')}
                        >
                          Complete
                        </button>
                      </div>
                    </div>
                  </div>
                {/each}
              </div>
            {/if}
          {/if}
        </div>

        <div class="mt-6 space-y-3">
          {#each rules as rule (rule.id)}
            <article class="rounded-2xl border border-slate-200 p-4 dark:border-slate-800">
              <div class="flex flex-wrap items-start justify-between gap-3">
                <div>
                  <div class="font-medium text-slate-900 dark:text-slate-100">{rule.display_name}</div>
                  <div class="mt-1 flex flex-wrap gap-2 text-xs text-slate-500">
                    <span class="font-mono">{rule.name}</span>
                    <span class="rounded-full bg-slate-100 px-2 py-0.5 dark:bg-slate-800">{rule.evaluation_mode}</span>
                  </div>
                  {#if rule.description}
                    <p class="mt-2 text-sm text-slate-500">{rule.description}</p>
                  {/if}
                </div>
                <div class="flex flex-wrap gap-2">
                  <button type="button" class="rounded-full border border-slate-300 px-3 py-1 text-xs font-medium text-slate-700 hover:bg-slate-100 dark:border-slate-700 dark:text-slate-200 dark:hover:bg-slate-800" onclick={() => handleRuleRuntime(rule.id, 'simulate')}>
                    Simulate
                  </button>
                  <button type="button" class="rounded-full bg-fuchsia-600 px-3 py-1 text-xs font-medium text-white hover:bg-fuchsia-700" onclick={() => handleRuleRuntime(rule.id, 'apply')}>
                    Apply
                  </button>
                </div>
              </div>
            </article>
          {/each}
        </div>

        {#if ruleRuntimeError}
          <div class="mt-4 rounded-2xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-700 dark:border-rose-900/40 dark:bg-rose-950/20 dark:text-rose-300">
            {ruleRuntimeError}
          </div>
        {/if}

        {#if ruleRuntimeResult}
          <pre class="mt-4 overflow-x-auto rounded-2xl bg-slate-950 p-4 text-xs text-slate-100">{formatJson(ruleRuntimeResult)}</pre>
        {/if}
      </section>
    </div>

    <div class="grid gap-6 xl:grid-cols-[1.05fr_0.95fr]">
      <section class="rounded-[1.75rem] border border-slate-200 bg-white p-6 shadow-sm dark:border-slate-800 dark:bg-slate-900">
        <div class="flex items-center justify-between gap-3">
          <div>
            <h2 class="text-lg font-semibold text-slate-950 dark:text-slate-50">Object Lab</h2>
            <p class="mt-1 text-sm text-slate-500">Create test objects to validate and execute action types against real instances.</p>
          </div>
          <span class="rounded-full bg-slate-100 px-3 py-1 text-xs text-slate-600 dark:bg-slate-800 dark:text-slate-300">{objects.length} objects</span>
        </div>

        {#if objectError}
          <div class="mt-4 rounded-2xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-700 dark:border-rose-900/40 dark:bg-rose-950/20 dark:text-rose-300">
            {objectError}
          </div>
        {/if}

        {#if objectSuccess}
          <div class="mt-4 rounded-2xl border border-emerald-200 bg-emerald-50 px-4 py-3 text-sm text-emerald-700 dark:border-emerald-900/40 dark:bg-emerald-950/20 dark:text-emerald-300">
            {objectSuccess}
          </div>
        {/if}

        <form class="mt-4 space-y-3" onsubmit={handleCreateObject}>
          <label class="block text-sm font-medium text-slate-700 dark:text-slate-200" for="object-properties-json">
            New object properties JSON
          </label>
          <textarea
            id="object-properties-json"
            bind:value={objectPropertiesText}
            rows={8}
            class="w-full rounded-2xl border border-slate-300 bg-slate-950 px-4 py-3 font-mono text-sm text-slate-100 dark:border-slate-700"
            spellcheck="false"
          ></textarea>
          <div class="flex items-center justify-between gap-3">
            <p class="text-xs text-slate-500">Match property names exactly. Unknown properties are still stored, but typed ones are validated on action execution.</p>
            <button
              type="submit"
              disabled={creatingObject}
              class="rounded-full bg-teal-600 px-4 py-2 text-sm font-medium text-white hover:bg-teal-700 disabled:opacity-50"
            >
              {creatingObject ? 'Creating...' : 'Create object'}
            </button>
          </div>
        </form>

        <div class="mt-6 space-y-3">
          {#if objects.length === 0}
            <div class="rounded-2xl border border-dashed border-slate-300 px-4 py-6 text-sm text-slate-500 dark:border-slate-700">
              No objects exist yet for this type.
            </div>
          {:else}
            {#each objects as object (object.id)}
              <div id={`object-${object.id}`} class="rounded-2xl border border-slate-200 p-4 dark:border-slate-800">
                <div class="flex flex-wrap items-center justify-between gap-3">
                  <div class="space-y-1">
                    <button
                      type="button"
                      class={`rounded-full px-3 py-1 text-left text-xs font-medium ${selectedTargetObjectId === object.id ? 'bg-teal-600 text-white' : 'bg-slate-100 text-slate-700 dark:bg-slate-800 dark:text-slate-200'}`}
                      onclick={() => {
                        selectedTargetObjectId = object.id;
                        void loadObjectInspector(object.id);
                      }}
                    >
                      {selectedTargetObjectId === object.id ? 'Selected target' : 'Use as target'}
                    </button>
                    <div class="font-mono text-xs text-slate-500">{object.id}</div>
                  </div>
                  <button
                    type="button"
                    class="text-sm font-medium text-rose-600 hover:text-rose-700"
                    onclick={() => handleDeleteObject(object.id)}
                  >
                    Delete
                  </button>
                </div>
                {#if properties.some((property) => property.inline_edit_config)}
                  <div class="mt-3 rounded-2xl border border-slate-200 bg-slate-50 p-4 dark:border-slate-800 dark:bg-slate-950/40">
                    <div class="flex flex-wrap items-center justify-between gap-2">
                      <div>
                        <p class="text-sm font-medium text-slate-900 dark:text-slate-100">Inline edits backed by actions</p>
                        <p class="mt-1 text-xs text-slate-500">
                          Each field below runs its configured action type instead of patching the object directly.
                        </p>
                      </div>
                      <span class="rounded-full bg-slate-200 px-2.5 py-1 text-xs font-medium text-slate-700 dark:bg-slate-800 dark:text-slate-300">
                        {properties.filter((property) => property.inline_edit_config).length} configured
                      </span>
                    </div>

                    <div class="mt-4 space-y-3">
                      {#each properties.filter((property) => property.inline_edit_config) as property (property.id)}
                        <div class="grid gap-3 md:grid-cols-[minmax(0,1fr)_auto] md:items-end">
                          <div>
                            <label
                              class="mb-1 block text-sm font-medium text-slate-700 dark:text-slate-200"
                              for={`object-inline-${object.id}-${property.id}`}
                            >
                              {property.display_name}
                            </label>
                            <input
                              id={`object-inline-${object.id}-${property.id}`}
                              bind:value={objectInlineEditDrafts[inlineEditFieldKey(object.id, property.id)]}
                              class="w-full rounded-2xl border border-slate-300 px-4 py-2.5 text-sm dark:border-slate-700 dark:bg-slate-950 dark:text-slate-100"
                              placeholder={property.name}
                            />
                            <p class="mt-1 text-xs text-slate-500">
                              Action: {actionLabelById(property.inline_edit_config?.action_type_id)} • Type: {property.property_type}
                            </p>
                          </div>
                          <button
                            type="button"
                            class="rounded-full bg-sky-600 px-4 py-2 text-sm font-medium text-white hover:bg-sky-700 disabled:opacity-50"
                            disabled={executingInlineEditKey === inlineEditFieldKey(object.id, property.id)}
                            onclick={() => void handleExecuteInlineEdit(object.id, property)}
                          >
                            {executingInlineEditKey === inlineEditFieldKey(object.id, property.id) ? 'Applying...' : 'Apply inline edit'}
                          </button>
                        </div>
                      {/each}
                    </div>
                  </div>
                {/if}
                <pre class="mt-3 overflow-x-auto rounded-2xl bg-slate-950 p-4 text-xs text-slate-100">{formatJson(object.properties)}</pre>
              </div>
            {/each}
          {/if}
        </div>
      </section>

      <section class="rounded-[1.75rem] border border-slate-200 bg-white p-6 shadow-sm dark:border-slate-800 dark:bg-slate-900">
        <div class="flex items-center justify-between gap-3">
          <div>
            <h2 class="text-lg font-semibold text-slate-950 dark:text-slate-50">Action Types</h2>
            <p class="mt-1 text-sm text-slate-500">Create HTTP-backed functions, webhooks, or object-mutating actions directly from the frontend.</p>
          </div>
          <span class="rounded-full bg-slate-100 px-3 py-1 text-xs text-slate-600 dark:bg-slate-800 dark:text-slate-300">{actions.length} actions</span>
        </div>

        {#if actionFormError}
          <div class="mt-4 rounded-2xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-700 dark:border-rose-900/40 dark:bg-rose-950/20 dark:text-rose-300">
            {actionFormError}
          </div>
        {/if}

        {#if actionFormSuccess}
          <div class="mt-4 rounded-2xl border border-emerald-200 bg-emerald-50 px-4 py-3 text-sm text-emerald-700 dark:border-emerald-900/40 dark:bg-emerald-950/20 dark:text-emerald-300">
            {actionFormSuccess}
          </div>
        {/if}

        <form class="mt-4 space-y-4" onsubmit={handleCreateAction}>
          <div class="grid gap-4 md:grid-cols-2">
            <div>
              <label class="mb-1 block text-sm font-medium text-slate-700 dark:text-slate-200" for="action-name">Name</label>
              <input
                id="action-name"
                bind:value={actionName}
                class="w-full rounded-2xl border border-slate-300 px-4 py-2.5 font-mono text-sm dark:border-slate-700 dark:bg-slate-950 dark:text-slate-100"
                placeholder="enrich_customer"
              />
            </div>
            <div>
              <label class="mb-1 block text-sm font-medium text-slate-700 dark:text-slate-200" for="action-display-name">Display name</label>
              <input
                id="action-display-name"
                bind:value={actionDisplayName}
                class="w-full rounded-2xl border border-slate-300 px-4 py-2.5 text-sm dark:border-slate-700 dark:bg-slate-950 dark:text-slate-100"
                placeholder="Enrich customer"
              />
            </div>
          </div>

          <div>
            <label class="mb-1 block text-sm font-medium text-slate-700 dark:text-slate-200" for="action-description">Description</label>
            <textarea
              id="action-description"
              bind:value={actionDescription}
              rows={2}
              class="w-full rounded-2xl border border-slate-300 px-4 py-2.5 text-sm dark:border-slate-700 dark:bg-slate-950 dark:text-slate-100"
              placeholder="What should this action do?"
            ></textarea>
          </div>

          <div class="grid gap-4 md:grid-cols-[1fr_1fr_auto]">
            <div>
              <label class="mb-1 block text-sm font-medium text-slate-700 dark:text-slate-200" for="action-kind">Operation kind</label>
              <select
                id="action-kind"
                bind:value={actionOperationKind}
                onchange={() => applyTemplate(actionOperationKind)}
                class="w-full rounded-2xl border border-slate-300 px-4 py-2.5 text-sm dark:border-slate-700 dark:bg-slate-950 dark:text-slate-100"
              >
                <option value="update_object">update_object</option>
                <option value="create_link">create_link</option>
                <option value="delete_object">delete_object</option>
                <option value="invoke_function">invoke_function</option>
                <option value="invoke_webhook">invoke_webhook</option>
              </select>
            </div>
            <div>
              <label class="mb-1 block text-sm font-medium text-slate-700 dark:text-slate-200" for="permission-key">Permission key</label>
              <input
                id="permission-key"
                bind:value={actionPermissionKey}
                class="w-full rounded-2xl border border-slate-300 px-4 py-2.5 text-sm dark:border-slate-700 dark:bg-slate-950 dark:text-slate-100"
                placeholder="ontology.actions.execute"
              />
            </div>
            <div class="flex items-end">
              <label class="flex items-center gap-2 rounded-2xl border border-slate-300 px-4 py-2.5 text-sm dark:border-slate-700 dark:bg-slate-950 dark:text-slate-100">
                <input type="checkbox" bind:checked={actionConfirmationRequired} />
                Requires confirmation
              </label>
            </div>
          </div>

          <div class="rounded-2xl bg-slate-100 px-4 py-3 text-sm text-slate-600 dark:bg-slate-800/70 dark:text-slate-300">
            {actionTemplates[actionOperationKind].notes}
          </div>

          <div class="grid gap-4 xl:grid-cols-2">
            <div>
              <div class="mb-1 flex items-center justify-between gap-3">
                <label class="block text-sm font-medium text-slate-700 dark:text-slate-200" for="action-input-schema">Input schema JSON</label>
                <button
                  type="button"
                  class="rounded-full border border-slate-300 px-3 py-1 text-xs font-medium text-slate-700 hover:bg-slate-100 dark:border-slate-700 dark:text-slate-200 dark:hover:bg-slate-800"
                  onclick={() => applyTemplate(actionOperationKind)}
                >
                  Load template
                </button>
              </div>
              <textarea
                id="action-input-schema"
                bind:value={actionInputSchemaText}
                rows={12}
                class="w-full rounded-2xl border border-slate-300 bg-slate-950 px-4 py-3 font-mono text-xs text-slate-100 dark:border-slate-700"
                spellcheck="false"
              ></textarea>
              <p class="mt-2 text-xs text-slate-500">
                Inputs reuse ontology property types, including `vector` for numeric JSON arrays and `media_reference` for URI/URL strings or JSON objects with `uri` or `url`.
              </p>
            </div>
            <div>
              <div class="mb-1 flex items-center justify-between gap-3">
                <label class="block text-sm font-medium text-slate-700 dark:text-slate-200" for="action-form-schema">Form schema JSON</label>
                <button
                  type="button"
                  class="rounded-full border border-slate-300 px-3 py-1 text-xs font-medium text-slate-700 hover:bg-slate-100 dark:border-slate-700 dark:text-slate-200 dark:hover:bg-slate-800"
                  onclick={() => {
                    try {
                      const inputSchema = parseJsonArray<ActionInputField>(actionInputSchemaText, 'Action input schema');
                      actionFormSchemaText = formatJson(buildDefaultActionFormSchema(inputSchema));
                    } catch (cause) {
                      actionFormError = cause instanceof Error ? cause.message : 'Failed to generate form schema';
                    }
                  }}
                >
                  Generate form schema
                </button>
              </div>
              <textarea
                id="action-form-schema"
                bind:value={actionFormSchemaText}
                rows={12}
                class="w-full rounded-2xl border border-slate-300 bg-slate-950 px-4 py-3 font-mono text-xs text-slate-100 dark:border-slate-700"
                spellcheck="false"
              ></textarea>
              <p class="mt-2 text-xs text-slate-500">
                Supports `sections`, per-section conditional overrides, and parameter-level overrides for requiredness, visibility, labels, and defaults.
              </p>
            </div>
            <div>
              <label class="mb-1 block text-sm font-medium text-slate-700 dark:text-slate-200" for="action-config">Config JSON</label>
              <textarea
                id="action-config"
                bind:value={actionConfigText}
                rows={12}
                class="w-full rounded-2xl border border-slate-300 bg-slate-950 px-4 py-3 font-mono text-xs text-slate-100 dark:border-slate-700"
                spellcheck="false"
              ></textarea>
              <p class="mt-2 text-xs text-slate-500">
                Function-backed actions can reference packages by pinned `function_package_id` or by `function_package_name` + `function_package_version`, with optional `function_package_auto_upgrade: true` for stable same-major upgrades.
              </p>
            </div>
            <div>
              <label class="mb-1 block text-sm font-medium text-slate-700 dark:text-slate-200" for="action-authorization-policy">Authorization policy JSON</label>
              <textarea
                id="action-authorization-policy"
                bind:value={actionAuthorizationPolicyText}
                rows={12}
                class="w-full rounded-2xl border border-slate-300 bg-slate-950 px-4 py-3 font-mono text-xs text-slate-100 dark:border-slate-700"
                spellcheck="false"
              ></textarea>
              <p class="mt-2 text-xs text-slate-500">
                Supports `required_permission_keys`, `any_role`, `all_roles`, `attribute_equals`, `allowed_markings`, `minimum_clearance` and `deny_guest_sessions`.
              </p>
            </div>
          </div>

          <div class="flex items-center justify-end gap-3">
            <button
              type="submit"
              disabled={creatingAction}
              class="rounded-full bg-sky-600 px-4 py-2 text-sm font-medium text-white hover:bg-sky-700 disabled:opacity-50"
            >
              {creatingAction ? 'Saving...' : 'Create action type'}
            </button>
          </div>
        </form>

        <div class="mt-6 space-y-3">
          {#if actions.length === 0}
            <div class="rounded-2xl border border-dashed border-slate-300 px-4 py-6 text-sm text-slate-500 dark:border-slate-700">
              No action types have been defined for this object type yet.
            </div>
          {:else}
            {#each actions as action (action.id)}
              <div class={`rounded-2xl border px-4 py-3 ${selectedActionId === action.id ? 'border-sky-400 bg-sky-50 dark:border-sky-500/60 dark:bg-sky-950/20' : 'border-slate-200 dark:border-slate-800'}`}>
                <div class="flex flex-wrap items-center justify-between gap-3">
                  <button
                    type="button"
                    class="text-left"
                    onclick={() => void handleSelectAction(action.id)}
                  >
                    <div class="font-medium text-slate-900 dark:text-slate-100">{action.display_name}</div>
                    <div class="mt-1 flex flex-wrap items-center gap-2 text-xs text-slate-500">
                      <span class="font-mono">{action.name}</span>
                      <span class="rounded-full bg-slate-100 px-2 py-0.5 text-slate-700 dark:bg-slate-800 dark:text-slate-300">{action.operation_kind}</span>
                      {#if action.permission_key}
                        <span class="rounded-full bg-sky-50 px-2 py-0.5 text-sky-700 dark:bg-sky-950/40 dark:text-sky-300">{action.permission_key}</span>
                      {/if}
                      {#if hasAuthorizationPolicy(action.authorization_policy)}
                        <span class="rounded-full bg-violet-50 px-2 py-0.5 text-violet-700 dark:bg-violet-950/40 dark:text-violet-300">policy</span>
                      {/if}
                      {#if action.confirmation_required}
                        <span class="rounded-full bg-amber-50 px-2 py-0.5 text-amber-700 dark:bg-amber-950/40 dark:text-amber-300">confirm</span>
                      {/if}
                    </div>
                  </button>
                  <button
                    type="button"
                    class="text-sm font-medium text-rose-600 hover:text-rose-700"
                    onclick={() => handleDeleteAction(action.id)}
                  >
                    Delete
                  </button>
                </div>
                {#if action.description}
                  <p class="mt-2 text-sm text-slate-500">{action.description}</p>
                {/if}
              </div>
            {/each}
          {/if}
        </div>
      </section>
    </div>

    <section class="rounded-[1.75rem] border border-slate-200 bg-white p-6 shadow-sm dark:border-slate-800 dark:bg-slate-900">
      <div class="flex flex-wrap items-start justify-between gap-4">
        <div>
          <h2 class="text-lg font-semibold text-slate-950 dark:text-slate-50">Action Console</h2>
          <p class="mt-1 text-sm text-slate-500">Validate and execute the selected action type against current object instances.</p>
        </div>
        <div class="rounded-2xl bg-slate-100 px-4 py-3 text-xs text-slate-600 dark:bg-slate-800/70 dark:text-slate-300">
          {#if getSelectedAction()}
            Selected: <span class="font-mono">{getSelectedAction()?.name}</span>
          {:else}
            Select an action from the list above.
          {/if}
        </div>
      </div>

      {#if runtimeError}
        <div class="mt-4 rounded-2xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-700 dark:border-rose-900/40 dark:bg-rose-950/20 dark:text-rose-300">
          {runtimeError}
        </div>
      {/if}

      <div class="mt-4 grid gap-6 xl:grid-cols-[0.95fr_1.05fr]">
        <div class="space-y-4">
          <div>
            <label class="mb-1 block text-sm font-medium text-slate-700 dark:text-slate-200" for="selected-action">Action type</label>
            <select
              id="selected-action"
              bind:value={selectedActionId}
              onchange={() => void handleSelectAction(selectedActionId)}
              class="w-full rounded-2xl border border-slate-300 px-4 py-2.5 text-sm dark:border-slate-700 dark:bg-slate-950 dark:text-slate-100"
            >
              <option value="">Select an action</option>
              {#each actions as action (action.id)}
                <option value={action.id}>{action.display_name} ({action.operation_kind})</option>
              {/each}
            </select>
          </div>

          {#if getSelectedAction()}
            <div class="rounded-2xl border border-slate-200 p-4 dark:border-slate-800">
              <div class="flex items-center justify-between gap-3">
                <h3 class="font-medium text-slate-900 dark:text-slate-100">Authorization policy</h3>
                {#if hasAuthorizationPolicy(getSelectedAction()?.authorization_policy)}
                  <span class="rounded-full bg-violet-50 px-3 py-1 text-xs font-medium text-violet-700 dark:bg-violet-950/40 dark:text-violet-300">granular policy</span>
                {:else}
                  <span class="rounded-full bg-slate-100 px-3 py-1 text-xs text-slate-600 dark:bg-slate-800 dark:text-slate-300">default access</span>
                {/if}
              </div>
              <div class="mt-3 flex flex-wrap gap-2 text-xs text-slate-500">
                {#if getSelectedAction()?.permission_key}
                  <span class="rounded-full bg-sky-50 px-2 py-1 text-sky-700 dark:bg-sky-950/40 dark:text-sky-300">
                    permission: {getSelectedAction()?.permission_key}
                  </span>
                {/if}
                {#if getSelectedAction()?.confirmation_required}
                  <span class="rounded-full bg-amber-50 px-2 py-1 text-amber-700 dark:bg-amber-950/40 dark:text-amber-300">
                    confirmation required
                  </span>
                {/if}
              </div>
              <pre class="mt-3 overflow-x-auto rounded-2xl bg-slate-950 p-4 text-xs text-slate-100">{formatJson(getSelectedAction()?.authorization_policy ?? {})}</pre>
            </div>
          {/if}

          <div>
            <label class="mb-1 block text-sm font-medium text-slate-700 dark:text-slate-200" for="selected-target-object">Target object</label>
            <select
              id="selected-target-object"
              bind:value={selectedTargetObjectId}
              onchange={() => void loadObjectInspector(selectedTargetObjectId)}
              class="w-full rounded-2xl border border-slate-300 px-4 py-2.5 text-sm dark:border-slate-700 dark:bg-slate-950 dark:text-slate-100"
            >
              <option value="">No target object</option>
              {#each objects as object (object.id)}
                <option value={object.id}>{object.id}</option>
              {/each}
            </select>
            {#if getSelectedAction()}
              <p class="mt-2 text-xs text-slate-500">
                {#if operationRequiresTarget(getSelectedAction()?.operation_kind)}
                  This action kind requires a target object.
                {:else}
                  Target object is optional for this action kind.
                {/if}
              </p>
            {/if}
          </div>

          <div>
            <p class="mb-1 block text-sm font-medium text-slate-700 dark:text-slate-200">Structured form</p>
            <ActionExecutor
              action={getSelectedAction()}
              targetObject={getSelectedTargetObject()}
              parameters={actionParametersDraft}
              title="Action inputs"
              emptyMessage="This action can run without any user-supplied parameters."
              on:change={handleStructuredActionParametersChange}
            />
          </div>

          <div>
            <label class="mb-1 block text-sm font-medium text-slate-700 dark:text-slate-200" for="action-parameters">Invocation parameters JSON</label>
            <textarea
              id="action-parameters"
              value={actionParametersText}
              rows={12}
              class="w-full rounded-2xl border border-slate-300 bg-slate-950 px-4 py-3 font-mono text-xs text-slate-100 dark:border-slate-700"
              spellcheck="false"
              oninput={handleActionParametersTextInput}
            ></textarea>
            {#if actionParametersInputError}
              <p class="mt-2 text-xs text-rose-600 dark:text-rose-300">{actionParametersInputError}</p>
            {:else}
              <p class="mt-2 text-xs text-slate-500">
                Advanced override for direct JSON editing. The structured form above stays in sync with the last valid payload.
              </p>
            {/if}
          </div>

          <div>
            <label class="mb-1 block text-sm font-medium text-slate-700 dark:text-slate-200" for="action-justification">Execution justification</label>
            <textarea
              id="action-justification"
              bind:value={actionJustification}
              rows={3}
              class="w-full rounded-2xl border border-slate-300 px-4 py-2.5 text-sm dark:border-slate-700 dark:bg-slate-950 dark:text-slate-100"
              placeholder="Required for confirmation-gated actions and useful for audit trails."
            ></textarea>
          </div>

          <div class="flex flex-wrap items-center gap-3">
            <button
              type="button"
              disabled={!selectedActionId || validatingAction}
              class="rounded-full border border-slate-300 px-4 py-2 text-sm font-medium text-slate-700 hover:bg-slate-100 disabled:opacity-50 dark:border-slate-700 dark:text-slate-200 dark:hover:bg-slate-800"
              onclick={handleValidateAction}
            >
              {validatingAction ? 'Validating...' : 'Validate'}
            </button>
            <button
              type="button"
              disabled={!selectedActionId || executingAction}
              class="rounded-full bg-emerald-600 px-4 py-2 text-sm font-medium text-white hover:bg-emerald-700 disabled:opacity-50"
              onclick={handleExecuteAction}
            >
              {executingAction ? 'Executing...' : 'Execute'}
            </button>
          </div>

          <div class="rounded-2xl border border-slate-200 p-4 dark:border-slate-800">
            <div class="flex items-center justify-between gap-3">
              <div>
                <h3 class="font-medium text-slate-900 dark:text-slate-100">What-if branch</h3>
                <p class="mt-1 text-sm text-slate-500">Persist a preview branch for the selected action and target without mutating live ontology data.</p>
              </div>
            </div>

            <div class="mt-4 grid gap-4 md:grid-cols-2">
              <div>
                <label class="mb-1 block text-sm font-medium text-slate-700 dark:text-slate-200" for="what-if-branch-name">Branch name</label>
                <input
                  id="what-if-branch-name"
                  bind:value={whatIfBranchName}
                  class="w-full rounded-2xl border border-slate-300 px-4 py-2.5 text-sm dark:border-slate-700 dark:bg-slate-950 dark:text-slate-100"
                  placeholder="High-risk customer escalation"
                />
              </div>
              <div>
                <label class="mb-1 block text-sm font-medium text-slate-700 dark:text-slate-200" for="what-if-branch-description">Description</label>
                <input
                  id="what-if-branch-description"
                  bind:value={whatIfBranchDescription}
                  class="w-full rounded-2xl border border-slate-300 px-4 py-2.5 text-sm dark:border-slate-700 dark:bg-slate-950 dark:text-slate-100"
                  placeholder="Dry-run before applying to customer operations"
                />
              </div>
            </div>

            <div class="mt-4 flex flex-wrap items-center gap-3">
              <button
                type="button"
                disabled={!selectedActionId || creatingWhatIfBranch}
                class="rounded-full bg-sky-600 px-4 py-2 text-sm font-medium text-white hover:bg-sky-700 disabled:opacity-50"
                onclick={handleCreateWhatIfBranch}
              >
                {creatingWhatIfBranch ? 'Saving branch...' : 'Save what-if branch'}
              </button>
            </div>
          </div>
        </div>

        <div class="space-y-4">
          <details class="rounded-2xl border border-slate-200 px-4 py-3 text-sm text-slate-600 dark:border-slate-800 dark:text-slate-300">
            <summary class="cursor-pointer font-medium text-slate-900 dark:text-slate-100">Function response contract</summary>
            <pre class="mt-3 overflow-x-auto rounded-2xl bg-slate-950 p-4 text-xs text-slate-100">{`{
  "output": { "summary": "external result" },
  "object_patch": { "status": "enriched" },
  "link": {
    "link_type_id": "uuid",
    "target_object_id": "uuid",
    "source_role": "source",
    "properties": { "confidence": 0.92 }
  },
  "delete_object": false
}`}</pre>
          </details>

          {#if validation}
            <div class="rounded-2xl border border-slate-200 p-4 dark:border-slate-800">
              <div class="flex items-center justify-between gap-3">
                <h3 class="font-medium text-slate-900 dark:text-slate-100">Validation</h3>
                <span class={`rounded-full px-3 py-1 text-xs font-medium ${validation.valid ? 'bg-emerald-50 text-emerald-700 dark:bg-emerald-950/40 dark:text-emerald-300' : 'bg-rose-50 text-rose-700 dark:bg-rose-950/40 dark:text-rose-300'}`}>
                  {validation.valid ? 'valid' : 'invalid'}
                </span>
              </div>
              {#if validation.errors.length > 0}
                <ul class="mt-3 space-y-2 text-sm text-rose-700 dark:text-rose-300">
                  {#each validation.errors as item}
                    <li class="rounded-xl bg-rose-50 px-3 py-2 dark:bg-rose-950/20">{item}</li>
                  {/each}
                </ul>
              {/if}
              <pre class="mt-3 overflow-x-auto rounded-2xl bg-slate-950 p-4 text-xs text-slate-100">{formatJson(validation.preview)}</pre>
            </div>
          {/if}

          {#if execution}
            <div class="rounded-2xl border border-slate-200 p-4 dark:border-slate-800">
              <div class="flex items-center justify-between gap-3">
                <h3 class="font-medium text-slate-900 dark:text-slate-100">Execution result</h3>
                {#if execution.deleted}
                  <span class="rounded-full bg-amber-50 px-3 py-1 text-xs font-medium text-amber-700 dark:bg-amber-950/40 dark:text-amber-300">object deleted</span>
                {/if}
              </div>
              <div class="mt-3 grid gap-4">
                <div>
                  <p class="mb-2 text-xs font-medium uppercase tracking-[0.2em] text-slate-500">Preview</p>
                  <pre class="overflow-x-auto rounded-2xl bg-slate-950 p-4 text-xs text-slate-100">{formatJson(execution.preview)}</pre>
                </div>
                {#if execution.object}
                  <div>
                    <p class="mb-2 text-xs font-medium uppercase tracking-[0.2em] text-slate-500">Object payload</p>
                    <pre class="overflow-x-auto rounded-2xl bg-slate-950 p-4 text-xs text-slate-100">{formatJson(execution.object)}</pre>
                  </div>
                {/if}
                {#if execution.link}
                  <div>
                    <p class="mb-2 text-xs font-medium uppercase tracking-[0.2em] text-slate-500">Link payload</p>
                    <pre class="overflow-x-auto rounded-2xl bg-slate-950 p-4 text-xs text-slate-100">{formatJson(execution.link)}</pre>
                  </div>
                {/if}
                {#if execution.result !== null}
                  <div>
                    <p class="mb-2 text-xs font-medium uppercase tracking-[0.2em] text-slate-500">External result</p>
                    <pre class="overflow-x-auto rounded-2xl bg-slate-950 p-4 text-xs text-slate-100">{formatJson(execution.result)}</pre>
                  </div>
                {/if}
              </div>
            </div>
          {/if}

          <div class="rounded-2xl border border-slate-200 p-4 dark:border-slate-800">
            <div class="flex items-center justify-between gap-3">
              <h3 class="font-medium text-slate-900 dark:text-slate-100">Saved what-if branches</h3>
              <span class="rounded-full bg-slate-100 px-3 py-1 text-xs text-slate-600 dark:bg-slate-800 dark:text-slate-300">{actionWhatIfBranches.length} branches</span>
            </div>

            {#if actionWhatIfBranches.length === 0}
              <div class="mt-4 rounded-2xl border border-dashed border-slate-300 px-4 py-6 text-sm text-slate-500 dark:border-slate-700">
                No what-if branches have been saved for this action yet.
              </div>
            {:else}
              <div class="mt-4 space-y-3">
                {#each actionWhatIfBranches as branch (branch.id)}
                  <div class="rounded-2xl border border-slate-200 p-4 dark:border-slate-800">
                    <div class="flex flex-wrap items-start justify-between gap-3">
                      <div>
                        <div class="flex flex-wrap items-center gap-2">
                          <h4 class="font-medium text-slate-900 dark:text-slate-100">{branch.name}</h4>
                          {#if branch.deleted}
                            <span class="rounded-full bg-amber-50 px-2 py-0.5 text-xs text-amber-700 dark:bg-amber-950/40 dark:text-amber-300">delete branch</span>
                          {/if}
                          {#if branch.target_object_id}
                            <span class="rounded-full bg-slate-100 px-2 py-0.5 font-mono text-xs text-slate-600 dark:bg-slate-800 dark:text-slate-300">{branch.target_object_id}</span>
                          {/if}
                        </div>
                        <p class="mt-1 text-xs text-slate-500">{new Date(branch.created_at).toLocaleString()}</p>
                        {#if branch.description}
                          <p class="mt-2 text-sm text-slate-500">{branch.description}</p>
                        {/if}
                      </div>
                      <button
                        type="button"
                        class="rounded-full border border-slate-300 px-3 py-1 text-xs font-medium text-slate-700 hover:bg-slate-100 disabled:opacity-50 dark:border-slate-700 dark:text-slate-200 dark:hover:bg-slate-800"
                        disabled={deletingWhatIfBranchId === branch.id}
                        onclick={() => void handleDeleteWhatIfBranch(branch.id)}
                      >
                        {deletingWhatIfBranchId === branch.id ? 'Deleting...' : 'Delete'}
                      </button>
                    </div>

                    <details class="mt-3 rounded-2xl bg-slate-50 px-4 py-3 dark:bg-slate-950/40">
                      <summary class="cursor-pointer text-sm font-medium text-slate-900 dark:text-slate-100">Branch payload</summary>
                      <div class="mt-3 grid gap-3">
                        <div>
                          <p class="mb-2 text-xs font-medium uppercase tracking-[0.2em] text-slate-500">Preview</p>
                          <pre class="overflow-x-auto rounded-2xl bg-slate-950 p-4 text-xs text-slate-100">{formatJson(branch.preview)}</pre>
                        </div>
                        {#if branch.before_object}
                          <div>
                            <p class="mb-2 text-xs font-medium uppercase tracking-[0.2em] text-slate-500">Before</p>
                            <pre class="overflow-x-auto rounded-2xl bg-slate-950 p-4 text-xs text-slate-100">{formatJson(branch.before_object)}</pre>
                          </div>
                        {/if}
                        {#if branch.after_object}
                          <div>
                            <p class="mb-2 text-xs font-medium uppercase tracking-[0.2em] text-slate-500">After</p>
                            <pre class="overflow-x-auto rounded-2xl bg-slate-950 p-4 text-xs text-slate-100">{formatJson(branch.after_object)}</pre>
                          </div>
                        {/if}
                      </div>
                    </details>
                  </div>
                {/each}
              </div>
            {/if}
          </div>
        </div>
      </div>
    </section>

    <section class="rounded-[1.75rem] border border-slate-200 bg-white p-6 shadow-sm dark:border-slate-800 dark:bg-slate-900">
      <div class="flex flex-wrap items-start justify-between gap-4">
        <div>
          <h2 class="text-lg font-semibold text-slate-950 dark:text-slate-50">Object View & Simulation</h2>
          <p class="mt-1 text-sm text-slate-500">Inspect the selected object as a digital twin: graph neighborhood, matching rules, recent machinery runs and projected impact.</p>
        </div>
        <div class="rounded-2xl bg-slate-100 px-4 py-3 text-xs text-slate-600 dark:bg-slate-800/70 dark:text-slate-300">
          {selectedTargetObjectId || 'Select an object from Object Lab first'}
        </div>
      </div>

      <div class="mt-4 grid gap-6 xl:grid-cols-[0.92fr_1.08fr]">
        <div class="space-y-4">
          <div>
            <label for="ontology-simulation-patch" class="mb-1 block text-sm font-medium text-slate-700 dark:text-slate-200">Simulation patch JSON</label>
            <textarea id="ontology-simulation-patch" bind:value={simulationPatchText} rows={12} class="w-full rounded-2xl border border-slate-300 bg-slate-950 px-4 py-3 font-mono text-xs text-slate-100 dark:border-slate-700" spellcheck="false"></textarea>
          </div>

          <div class="flex flex-wrap items-center gap-3">
            <button
              type="button"
              disabled={!selectedTargetObjectId || simulationLoading}
              class="rounded-full bg-cyan-600 px-4 py-2 text-sm font-medium text-white hover:bg-cyan-700 disabled:opacity-50"
              onclick={handleSimulateObject}
            >
              {simulationLoading ? 'Simulating...' : 'Simulate selected object'}
            </button>
            {#if selectedTargetObjectId}
              <button
                type="button"
                class="rounded-full border border-slate-300 px-4 py-2 text-sm font-medium text-slate-700 hover:bg-slate-100 dark:border-slate-700 dark:text-slate-200 dark:hover:bg-slate-800"
                onclick={() => void loadObjectInspector(selectedTargetObjectId)}
              >
                Refresh inspector
              </button>
            {/if}
          </div>

          {#if simulation}
            <div class="rounded-2xl border border-slate-200 p-4 dark:border-slate-800">
              <div class="flex items-center justify-between gap-3">
                <h3 class="font-medium text-slate-900 dark:text-slate-100">Simulation result</h3>
                <div class="flex flex-wrap gap-2">
                  <span class="rounded-full bg-slate-100 px-3 py-1 text-xs font-medium text-slate-700 dark:bg-slate-800 dark:text-slate-200">
                    {formatScope(simulation.impact_summary.scope)}
                  </span>
                  {#if simulation.deleted}
                    <span class="rounded-full bg-amber-50 px-3 py-1 text-xs font-medium text-amber-700 dark:bg-amber-950/40 dark:text-amber-300">predicted delete</span>
                  {/if}
                </div>
              </div>
              <div class="mt-3 grid gap-3 md:grid-cols-3">
                <div class="rounded-2xl bg-slate-100 px-4 py-3 dark:bg-slate-800/70">
                  <div class="text-xs uppercase tracking-[0.2em] text-slate-500">Impacted objects</div>
                  <div class="mt-1 text-xl font-semibold text-slate-900 dark:text-slate-100">
                    {simulation.impact_summary.impacted_object_count}
                  </div>
                </div>
                <div class="rounded-2xl bg-slate-100 px-4 py-3 dark:bg-slate-800/70">
                  <div class="text-xs uppercase tracking-[0.2em] text-slate-500">Direct neighbors</div>
                  <div class="mt-1 text-xl font-semibold text-slate-900 dark:text-slate-100">
                    {simulation.impact_summary.direct_neighbors}
                  </div>
                </div>
                <div class="rounded-2xl bg-slate-100 px-4 py-3 dark:bg-slate-800/70">
                  <div class="text-xs uppercase tracking-[0.2em] text-slate-500">Sensitive objects</div>
                  <div class="mt-1 text-xl font-semibold text-slate-900 dark:text-slate-100">
                    {simulation.impact_summary.sensitive_objects}
                  </div>
                </div>
                <div class="rounded-2xl bg-slate-100 px-4 py-3 dark:bg-slate-800/70">
                  <div class="text-xs uppercase tracking-[0.2em] text-slate-500">Boundary crossings</div>
                  <div class="mt-1 text-xl font-semibold text-slate-900 dark:text-slate-100">
                    {simulation.impact_summary.boundary_crossings}
                  </div>
                </div>
                <div class="rounded-2xl bg-slate-100 px-4 py-3 dark:bg-slate-800/70">
                  <div class="text-xs uppercase tracking-[0.2em] text-slate-500">Matching rules</div>
                  <div class="mt-1 text-xl font-semibold text-slate-900 dark:text-slate-100">
                    {simulation.impact_summary.matching_rules}
                  </div>
                </div>
                <div class="rounded-2xl bg-slate-100 px-4 py-3 dark:bg-slate-800/70">
                  <div class="text-xs uppercase tracking-[0.2em] text-slate-500">Max hops</div>
                  <div class="mt-1 text-xl font-semibold text-slate-900 dark:text-slate-100">
                    {simulation.impact_summary.max_hops_reached}
                  </div>
                </div>
              </div>

              {#if simulation.impact_summary.impacted_types.length > 0}
                <div class="mt-4">
                  <p class="text-xs uppercase tracking-[0.2em] text-slate-500">Types in blast radius</p>
                  <div class="mt-2 flex flex-wrap gap-2">
                    {#each simulation.impact_summary.impacted_types as impactedType}
                      <span class="rounded-full bg-cyan-50 px-3 py-1 text-xs font-medium text-cyan-700 dark:bg-cyan-950/40 dark:text-cyan-300">
                        {impactedType}
                      </span>
                    {/each}
                  </div>
                </div>
              {/if}

              {#if simulation.impact_summary.changed_properties.length > 0}
                <div class="mt-4">
                  <p class="text-xs uppercase tracking-[0.2em] text-slate-500">Changed properties</p>
                  <div class="mt-2 flex flex-wrap gap-2">
                    {#each simulation.impact_summary.changed_properties as propertyName}
                      <span class="rounded-full bg-slate-100 px-3 py-1 font-mono text-xs text-slate-700 dark:bg-slate-800 dark:text-slate-200">
                        {propertyName}
                      </span>
                    {/each}
                  </div>
                </div>
              {/if}

              {#if simulation.impact_summary.sensitive_markings.length > 0}
                <div class="mt-4">
                  <p class="text-xs uppercase tracking-[0.2em] text-slate-500">Sensitive markings</p>
                  <div class="mt-2 flex flex-wrap gap-2">
                    {#each simulation.impact_summary.sensitive_markings as marking}
                      <span class="rounded-full bg-amber-50 px-3 py-1 text-xs font-medium text-amber-700 dark:bg-amber-950/40 dark:text-amber-300">
                        {marking}
                      </span>
                    {/each}
                  </div>
                </div>
              {/if}

              <details class="mt-4 rounded-2xl border border-slate-200 px-4 py-3 dark:border-slate-800">
                <summary class="cursor-pointer font-medium text-slate-900 dark:text-slate-100">Raw simulation payload</summary>
                <pre class="mt-3 overflow-x-auto rounded-2xl bg-slate-950 p-4 text-xs text-slate-100">{formatJson(simulation)}</pre>
              </details>
            </div>
          {/if}

          <div class="rounded-2xl border border-slate-200 p-4 dark:border-slate-800">
            <div class="flex flex-wrap items-start justify-between gap-3">
              <div>
                <h3 class="font-medium text-slate-900 dark:text-slate-100">Scenario Lab</h3>
                <p class="mt-1 text-sm text-slate-500">
                  Compare multi-object scenarios, propagate automatic rules across the graph neighborhood, and
                  score each candidate against constraints and goals. Use `null` as `target_object_id` to target
                  the selected root object.
                </p>
              </div>
              {#if scenarioComparison}
                <span class="rounded-full bg-slate-100 px-3 py-1 text-xs font-medium text-slate-700 dark:bg-slate-800 dark:text-slate-200">
                  Compared {formatTimestamp(scenarioComparison.compared_at)}
                </span>
              {/if}
            </div>

            <div class="mt-4 grid gap-4 xl:grid-cols-3">
              <div>
                <label for="scenario-candidates" class="mb-1 block text-sm font-medium text-slate-700 dark:text-slate-200">Scenario candidates</label>
                <textarea
                  id="scenario-candidates"
                  bind:value={scenarioCandidatesText}
                  rows={18}
                  class="w-full rounded-2xl border border-slate-300 bg-slate-950 px-4 py-3 font-mono text-xs text-slate-100 dark:border-slate-700"
                  spellcheck="false"
                ></textarea>
              </div>
              <div>
                <label for="scenario-constraints" class="mb-1 block text-sm font-medium text-slate-700 dark:text-slate-200">Constraints</label>
                <textarea
                  id="scenario-constraints"
                  bind:value={scenarioConstraintsText}
                  rows={18}
                  class="w-full rounded-2xl border border-slate-300 bg-slate-950 px-4 py-3 font-mono text-xs text-slate-100 dark:border-slate-700"
                  spellcheck="false"
                ></textarea>
              </div>
              <div>
                <label for="scenario-goals" class="mb-1 block text-sm font-medium text-slate-700 dark:text-slate-200">Goals</label>
                <textarea
                  id="scenario-goals"
                  bind:value={scenarioGoalsText}
                  rows={18}
                  class="w-full rounded-2xl border border-slate-300 bg-slate-950 px-4 py-3 font-mono text-xs text-slate-100 dark:border-slate-700"
                  spellcheck="false"
                ></textarea>
              </div>
            </div>

            <div class="mt-4 flex flex-wrap items-center gap-3">
              <button
                type="button"
                class="rounded-full border border-slate-300 px-4 py-2 text-sm font-medium text-slate-700 hover:bg-slate-100 dark:border-slate-700 dark:text-slate-200 dark:hover:bg-slate-800"
                onclick={seedScenarioDraftFromSelection}
              >
                Seed from current selection
              </button>
              <button
                type="button"
                disabled={!selectedTargetObjectId || scenarioLoading}
                class="rounded-full bg-indigo-600 px-4 py-2 text-sm font-medium text-white hover:bg-indigo-700 disabled:opacity-50"
                onclick={handleSimulateScenarios}
              >
                {scenarioLoading ? 'Comparing scenarios...' : 'Compare scenarios'}
              </button>
            </div>

            {#if scenarioComparison}
              <div class="mt-4 space-y-4">
                {#if scenarioComparison.baseline}
                  <div class="rounded-2xl border border-slate-200 bg-slate-50 p-4 dark:border-slate-800 dark:bg-slate-950/40">
                    <div class="flex flex-wrap items-center justify-between gap-3">
                      <div>
                        <h4 class="font-medium text-slate-900 dark:text-slate-100">Baseline neighborhood</h4>
                        <p class="mt-1 text-sm text-slate-500">
                          Current propagated state before applying any candidate scenario.
                        </p>
                      </div>
                      <span class="rounded-full bg-slate-200 px-3 py-1 text-xs font-medium text-slate-700 dark:bg-slate-800 dark:text-slate-200">
                        Goal score {scenarioGoalScore(scenarioComparison.baseline)}
                      </span>
                    </div>

                    <div class="mt-4 grid gap-3 md:grid-cols-5">
                      <div class="rounded-2xl bg-white px-4 py-3 dark:bg-slate-900">
                        <div class="text-xs uppercase tracking-[0.2em] text-slate-500">Impacted</div>
                        <div class="mt-1 text-xl font-semibold text-slate-900 dark:text-slate-100">
                          {scenarioComparison.baseline.summary.impacted_object_count}
                        </div>
                      </div>
                      <div class="rounded-2xl bg-white px-4 py-3 dark:bg-slate-900">
                        <div class="text-xs uppercase tracking-[0.2em] text-slate-500">Changed</div>
                        <div class="mt-1 text-xl font-semibold text-slate-900 dark:text-slate-100">
                          {scenarioComparison.baseline.summary.changed_object_count}
                        </div>
                      </div>
                      <div class="rounded-2xl bg-white px-4 py-3 dark:bg-slate-900">
                        <div class="text-xs uppercase tracking-[0.2em] text-slate-500">Auto rules</div>
                        <div class="mt-1 text-xl font-semibold text-slate-900 dark:text-slate-100">
                          {scenarioComparison.baseline.summary.automatic_rule_applications}
                        </div>
                      </div>
                      <div class="rounded-2xl bg-white px-4 py-3 dark:bg-slate-900">
                        <div class="text-xs uppercase tracking-[0.2em] text-slate-500">Failed constraints</div>
                        <div class="mt-1 text-xl font-semibold text-slate-900 dark:text-slate-100">
                          {scenarioComparison.baseline.summary.failed_constraints}
                        </div>
                      </div>
                      <div class="rounded-2xl bg-white px-4 py-3 dark:bg-slate-900">
                        <div class="text-xs uppercase tracking-[0.2em] text-slate-500">Schedules</div>
                        <div class="mt-1 text-xl font-semibold text-slate-900 dark:text-slate-100">
                          {scenarioComparison.baseline.summary.schedule_count}
                        </div>
                      </div>
                    </div>

                    {#if scenarioComparison.baseline.summary.impacted_types.length > 0}
                      <div class="mt-4">
                        <p class="text-xs uppercase tracking-[0.2em] text-slate-500">Impacted types</p>
                        <div class="mt-2 flex flex-wrap gap-2">
                          {#each scenarioComparison.baseline.summary.impacted_types as impactedType}
                            <span class="rounded-full bg-cyan-50 px-3 py-1 text-xs font-medium text-cyan-700 dark:bg-cyan-950/40 dark:text-cyan-300">
                              {impactedType}
                            </span>
                          {/each}
                        </div>
                      </div>
                    {/if}
                  </div>
                {/if}

                {#if scenarioComparison.scenarios.length === 0}
                  <div class="rounded-2xl border border-dashed border-slate-300 px-4 py-8 text-center text-sm text-slate-500 dark:border-slate-700">
                    No scenarios were returned. Add at least one candidate in the editor above.
                  </div>
                {:else}
                  {#each scenarioComparison.scenarios as result (result.scenario_id)}
                    <article class="rounded-2xl border border-slate-200 p-4 dark:border-slate-800">
                      <div class="flex flex-wrap items-start justify-between gap-3">
                        <div>
                          <div class="flex flex-wrap items-center gap-2">
                            <h4 class="font-medium text-slate-900 dark:text-slate-100">{result.name}</h4>
                            <span class="rounded-full bg-indigo-50 px-3 py-1 text-xs font-medium text-indigo-700 dark:bg-indigo-950/40 dark:text-indigo-300">
                              Goal score {scenarioGoalScore(result)}
                            </span>
                            <span class={`rounded-full px-3 py-1 text-xs font-medium ${result.summary.failed_constraints === 0 ? 'bg-emerald-50 text-emerald-700 dark:bg-emerald-950/40 dark:text-emerald-300' : 'bg-rose-50 text-rose-700 dark:bg-rose-950/40 dark:text-rose-300'}`}>
                              {result.summary.failed_constraints} failed constraints
                            </span>
                            {#if result.summary.deleted_object_count > 0}
                              <span class="rounded-full bg-amber-50 px-3 py-1 text-xs font-medium text-amber-700 dark:bg-amber-950/40 dark:text-amber-300">
                                {result.summary.deleted_object_count} deleted objects
                              </span>
                            {/if}
                          </div>
                          {#if result.description}
                            <p class="mt-1 text-sm text-slate-500">{result.description}</p>
                          {/if}
                        </div>
                        <span class="rounded-full bg-slate-100 px-3 py-1 text-xs font-medium text-slate-700 dark:bg-slate-800 dark:text-slate-200">
                          {formatScope(result.graph.summary.scope)}
                        </span>
                      </div>

                      <div class="mt-4 grid gap-3 md:grid-cols-3 xl:grid-cols-6">
                        <div class="rounded-2xl bg-slate-100 px-4 py-3 dark:bg-slate-800/70">
                          <div class="text-xs uppercase tracking-[0.2em] text-slate-500">Impacted</div>
                          <div class="mt-1 text-xl font-semibold text-slate-900 dark:text-slate-100">
                            {result.summary.impacted_object_count}
                          </div>
                          {#if result.delta_from_baseline}
                            <div class={`mt-1 text-xs ${scenarioDeltaClass(result.delta_from_baseline.impacted_object_count, 'lower')}`}>
                              {scenarioDeltaPrefix(result.delta_from_baseline.impacted_object_count)}{result.delta_from_baseline.impacted_object_count} vs baseline
                            </div>
                          {/if}
                        </div>
                        <div class="rounded-2xl bg-slate-100 px-4 py-3 dark:bg-slate-800/70">
                          <div class="text-xs uppercase tracking-[0.2em] text-slate-500">Changed</div>
                          <div class="mt-1 text-xl font-semibold text-slate-900 dark:text-slate-100">
                            {result.summary.changed_object_count}
                          </div>
                          {#if result.delta_from_baseline}
                            <div class={`mt-1 text-xs ${scenarioDeltaClass(result.delta_from_baseline.changed_object_count, 'lower')}`}>
                              {scenarioDeltaPrefix(result.delta_from_baseline.changed_object_count)}{result.delta_from_baseline.changed_object_count} vs baseline
                            </div>
                          {/if}
                        </div>
                        <div class="rounded-2xl bg-slate-100 px-4 py-3 dark:bg-slate-800/70">
                          <div class="text-xs uppercase tracking-[0.2em] text-slate-500">Auto apps</div>
                          <div class="mt-1 text-xl font-semibold text-slate-900 dark:text-slate-100">
                            {result.summary.automatic_rule_applications}
                          </div>
                          {#if result.delta_from_baseline}
                            <div class={`mt-1 text-xs ${scenarioDeltaClass(result.delta_from_baseline.automatic_rule_applications, 'lower')}`}>
                              {scenarioDeltaPrefix(result.delta_from_baseline.automatic_rule_applications)}{result.delta_from_baseline.automatic_rule_applications} vs baseline
                            </div>
                          {/if}
                        </div>
                        <div class="rounded-2xl bg-slate-100 px-4 py-3 dark:bg-slate-800/70">
                          <div class="text-xs uppercase tracking-[0.2em] text-slate-500">Schedules</div>
                          <div class="mt-1 text-xl font-semibold text-slate-900 dark:text-slate-100">
                            {result.summary.schedule_count}
                          </div>
                          {#if result.delta_from_baseline}
                            <div class={`mt-1 text-xs ${scenarioDeltaClass(result.delta_from_baseline.schedule_count, 'lower')}`}>
                              {scenarioDeltaPrefix(result.delta_from_baseline.schedule_count)}{result.delta_from_baseline.schedule_count} vs baseline
                            </div>
                          {/if}
                        </div>
                        <div class="rounded-2xl bg-slate-100 px-4 py-3 dark:bg-slate-800/70">
                          <div class="text-xs uppercase tracking-[0.2em] text-slate-500">Failed constraints</div>
                          <div class="mt-1 text-xl font-semibold text-slate-900 dark:text-slate-100">
                            {result.summary.failed_constraints}
                          </div>
                          {#if result.delta_from_baseline}
                            <div class={`mt-1 text-xs ${scenarioDeltaClass(result.delta_from_baseline.failed_constraints, 'lower')}`}>
                              {scenarioDeltaPrefix(result.delta_from_baseline.failed_constraints)}{result.delta_from_baseline.failed_constraints} vs baseline
                            </div>
                          {/if}
                        </div>
                        <div class="rounded-2xl bg-slate-100 px-4 py-3 dark:bg-slate-800/70">
                          <div class="text-xs uppercase tracking-[0.2em] text-slate-500">Goal score</div>
                          <div class="mt-1 text-xl font-semibold text-slate-900 dark:text-slate-100">
                            {scenarioGoalScore(result)}
                          </div>
                          {#if result.delta_from_baseline}
                            <div class={`mt-1 text-xs ${scenarioDeltaClass(result.delta_from_baseline.goal_score, 'higher')}`}>
                              {scenarioDeltaPrefix(result.delta_from_baseline.goal_score)}{Number(result.delta_from_baseline.goal_score ?? 0).toFixed(2)} vs baseline
                            </div>
                          {/if}
                        </div>
                      </div>

                      {#if result.summary.impacted_types.length > 0}
                        <div class="mt-4">
                          <p class="text-xs uppercase tracking-[0.2em] text-slate-500">Impacted types</p>
                          <div class="mt-2 flex flex-wrap gap-2">
                            {#each result.summary.impacted_types as impactedType}
                              <span class="rounded-full bg-cyan-50 px-3 py-1 text-xs font-medium text-cyan-700 dark:bg-cyan-950/40 dark:text-cyan-300">
                                {impactedType}
                              </span>
                            {/each}
                          </div>
                        </div>
                      {/if}

                      {#if result.summary.changed_properties.length > 0}
                        <div class="mt-4">
                          <p class="text-xs uppercase tracking-[0.2em] text-slate-500">Changed properties</p>
                          <div class="mt-2 flex flex-wrap gap-2">
                            {#each result.summary.changed_properties as propertyName}
                              <span class="rounded-full bg-slate-100 px-3 py-1 font-mono text-xs text-slate-700 dark:bg-slate-800 dark:text-slate-200">
                                {propertyName}
                              </span>
                            {/each}
                          </div>
                        </div>
                      {/if}

                      <div class="mt-4 grid gap-4 lg:grid-cols-2">
                        <div class="rounded-2xl border border-slate-200 p-4 dark:border-slate-800">
                          <div class="flex items-center justify-between gap-3">
                            <h5 class="font-medium text-slate-900 dark:text-slate-100">Constraints</h5>
                            <span class="rounded-full bg-slate-100 px-3 py-1 text-xs text-slate-600 dark:bg-slate-800 dark:text-slate-300">
                              {result.constraints.length}
                            </span>
                          </div>
                          {#if result.constraints.length === 0}
                            <p class="mt-3 text-sm text-slate-500">No constraints were evaluated for this scenario.</p>
                          {:else}
                            <div class="mt-3 space-y-3">
                              {#each result.constraints as metric}
                                <div class={`rounded-2xl px-3 py-3 ${scenarioMetricToneClass(metric.passed)}`}>
                                  <div class="flex flex-wrap items-center justify-between gap-2">
                                    <p class="font-medium">{metric.name}</p>
                                    <span class="rounded-full bg-white/70 px-2 py-0.5 text-xs font-medium dark:bg-slate-900/40">
                                      {metric.passed ? 'pass' : 'fail'}
                                    </span>
                                  </div>
                                  <p class="mt-1 text-xs">
                                    {metric.metric} {metric.comparator} {formatScenarioMetricValue(metric.target)}
                                  </p>
                                  <p class="mt-2 text-sm">{metric.message}</p>
                                  <div class="mt-2 flex flex-wrap gap-2 text-xs">
                                    <span class="rounded-full bg-white/70 px-2 py-0.5 font-mono dark:bg-slate-900/40">
                                      observed {formatScenarioMetricValue(metric.observed)}
                                    </span>
                                    {#if metric.score !== null}
                                      <span class="rounded-full bg-white/70 px-2 py-0.5 font-mono dark:bg-slate-900/40">
                                        score {Number(metric.score).toFixed(2)}
                                      </span>
                                    {/if}
                                  </div>
                                </div>
                              {/each}
                            </div>
                          {/if}
                        </div>

                        <div class="rounded-2xl border border-slate-200 p-4 dark:border-slate-800">
                          <div class="flex items-center justify-between gap-3">
                            <h5 class="font-medium text-slate-900 dark:text-slate-100">Goals</h5>
                            <span class="rounded-full bg-slate-100 px-3 py-1 text-xs text-slate-600 dark:bg-slate-800 dark:text-slate-300">
                              {result.summary.achieved_goals}/{result.summary.total_goals} achieved
                            </span>
                          </div>
                          {#if result.goals.length === 0}
                            <p class="mt-3 text-sm text-slate-500">No goals were evaluated for this scenario.</p>
                          {:else}
                            <div class="mt-3 space-y-3">
                              {#each result.goals as metric}
                                <div class={`rounded-2xl px-3 py-3 ${scenarioMetricToneClass(metric.passed)}`}>
                                  <div class="flex flex-wrap items-center justify-between gap-2">
                                    <p class="font-medium">{metric.name}</p>
                                    <span class="rounded-full bg-white/70 px-2 py-0.5 text-xs font-medium dark:bg-slate-900/40">
                                      {metric.passed ? 'achieved' : 'missed'}
                                    </span>
                                  </div>
                                  <p class="mt-1 text-xs">
                                    {metric.metric} {metric.comparator} {formatScenarioMetricValue(metric.target)}
                                  </p>
                                  <p class="mt-2 text-sm">{metric.message}</p>
                                  <div class="mt-2 flex flex-wrap gap-2 text-xs">
                                    <span class="rounded-full bg-white/70 px-2 py-0.5 font-mono dark:bg-slate-900/40">
                                      observed {formatScenarioMetricValue(metric.observed)}
                                    </span>
                                    {#if metric.score !== null}
                                      <span class="rounded-full bg-white/70 px-2 py-0.5 font-mono dark:bg-slate-900/40">
                                        score {Number(metric.score).toFixed(2)}
                                      </span>
                                    {/if}
                                  </div>
                                </div>
                              {/each}
                            </div>
                          {/if}
                        </div>
                      </div>

                      <div class="mt-4 grid gap-4 xl:grid-cols-[1.15fr_0.85fr]">
                        <div class="rounded-2xl border border-slate-200 p-4 dark:border-slate-800">
                          <div class="flex items-center justify-between gap-3">
                            <h5 class="font-medium text-slate-900 dark:text-slate-100">Changed objects</h5>
                            <span class="rounded-full bg-slate-100 px-3 py-1 text-xs text-slate-600 dark:bg-slate-800 dark:text-slate-300">
                              {result.object_changes.length}
                            </span>
                          </div>
                          {#if result.object_changes.length === 0}
                            <p class="mt-3 text-sm text-slate-500">This scenario did not mutate any objects.</p>
                          {:else}
                            <div class="mt-3 space-y-3">
                              {#each result.object_changes as change (change.object_id)}
                                <div class="rounded-2xl border border-slate-200 p-4 dark:border-slate-800">
                                  <div class="flex flex-wrap items-start justify-between gap-3">
                                    <div>
                                      <div class="flex flex-wrap items-center gap-2">
                                        <h6 class="font-medium text-slate-900 dark:text-slate-100">{scenarioObjectLabel(change)}</h6>
                                        <span class="rounded-full bg-slate-100 px-2 py-0.5 text-xs text-slate-600 dark:bg-slate-800 dark:text-slate-300">
                                          {change.object_type_label}
                                        </span>
                                        {#if change.deleted}
                                          <span class="rounded-full bg-amber-50 px-2 py-0.5 text-xs text-amber-700 dark:bg-amber-950/40 dark:text-amber-300">
                                            deleted
                                          </span>
                                        {/if}
                                      </div>
                                      <p class="mt-1 font-mono text-xs text-slate-500">{change.object_id}</p>
                                    </div>
                                    <div class="flex flex-wrap gap-2">
                                      {#each change.sources as source}
                                        <span class="rounded-full bg-indigo-50 px-2 py-0.5 text-xs font-medium text-indigo-700 dark:bg-indigo-950/40 dark:text-indigo-300">
                                          {source}
                                        </span>
                                      {/each}
                                    </div>
                                  </div>

                                  {#if change.changed_properties.length > 0}
                                    <div class="mt-3 flex flex-wrap gap-2">
                                      {#each change.changed_properties as propertyName}
                                        <span class="rounded-full bg-slate-100 px-3 py-1 font-mono text-xs text-slate-700 dark:bg-slate-800 dark:text-slate-200">
                                          {propertyName}
                                        </span>
                                      {/each}
                                    </div>
                                  {/if}

                                  <details class="mt-3 rounded-2xl bg-slate-50 px-4 py-3 dark:bg-slate-950/40">
                                    <summary class="cursor-pointer text-sm font-medium text-slate-900 dark:text-slate-100">Before / after payload</summary>
                                    <pre class="mt-3 overflow-x-auto rounded-2xl bg-slate-950 p-4 text-xs text-slate-100">{formatJson({ before: change.before, after: change.after })}</pre>
                                  </details>
                                </div>
                              {/each}
                            </div>
                          {/if}
                        </div>

                        <div class="space-y-4">
                          <div class="rounded-2xl border border-slate-200 p-4 dark:border-slate-800">
                            <div class="flex items-center justify-between gap-3">
                              <h5 class="font-medium text-slate-900 dark:text-slate-100">Rule outcomes</h5>
                              <span class="rounded-full bg-slate-100 px-3 py-1 text-xs text-slate-600 dark:bg-slate-800 dark:text-slate-300">
                                {result.rule_outcomes.length}
                              </span>
                            </div>
                            {#if result.rule_outcomes.length === 0}
                              <p class="mt-3 text-sm text-slate-500">No rule evaluations were emitted for this scenario.</p>
                            {:else}
                              <div class="mt-3 space-y-3">
                                {#each result.rule_outcomes as rule, index (`${rule.rule_id}-${rule.object_id}-${index}`)}
                                  <div class="rounded-2xl bg-slate-50 px-3 py-3 dark:bg-slate-950/40">
                                    <div class="flex flex-wrap items-center gap-2">
                                      <p class="font-medium text-slate-900 dark:text-slate-100">{rule.rule_display_name || rule.rule_name}</p>
                                      <span class={`rounded-full px-2 py-0.5 text-xs font-medium ${rule.matched ? 'bg-emerald-50 text-emerald-700 dark:bg-emerald-950/40 dark:text-emerald-300' : 'bg-slate-200 text-slate-600 dark:bg-slate-800 dark:text-slate-300'}`}>
                                        {rule.matched ? 'matched' : 'not matched'}
                                      </span>
                                      {#if rule.auto_applied}
                                        <span class="rounded-full bg-indigo-50 px-2 py-0.5 text-xs font-medium text-indigo-700 dark:bg-indigo-950/40 dark:text-indigo-300">
                                          auto applied
                                        </span>
                                      {/if}
                                    </div>
                                    <p class="mt-1 font-mono text-xs text-slate-500">{rule.object_id}</p>
                                    <p class="mt-2 text-xs uppercase tracking-[0.2em] text-slate-500">{rule.evaluation_mode}</p>
                                    <details class="mt-3 rounded-2xl border border-slate-200 px-3 py-2 dark:border-slate-800">
                                      <summary class="cursor-pointer text-sm text-slate-700 dark:text-slate-200">Trigger & effect preview</summary>
                                      <pre class="mt-3 overflow-x-auto rounded-2xl bg-slate-950 p-3 text-xs text-slate-100">{formatJson({ trigger: rule.trigger_payload, effect: rule.effect_preview })}</pre>
                                    </details>
                                  </div>
                                {/each}
                              </div>
                            {/if}
                          </div>

                          <div class="rounded-2xl border border-slate-200 p-4 dark:border-slate-800">
                            <div class="flex items-center justify-between gap-3">
                              <h5 class="font-medium text-slate-900 dark:text-slate-100">Link previews</h5>
                              <span class="rounded-full bg-slate-100 px-3 py-1 text-xs text-slate-600 dark:bg-slate-800 dark:text-slate-300">
                                {result.link_previews.length}
                              </span>
                            </div>
                            {#if result.link_previews.length === 0}
                              <p class="mt-3 text-sm text-slate-500">This scenario did not stage any link changes.</p>
                            {:else}
                              <div class="mt-3 space-y-3">
                                {#each result.link_previews as preview, index (`${preview.link_type_id ?? 'link'}-${index}`)}
                                  <div class="rounded-2xl bg-slate-50 px-3 py-3 dark:bg-slate-950/40">
                                    <div class="flex flex-wrap items-center gap-2 text-xs text-slate-500">
                                      <span class="rounded-full bg-slate-200 px-2 py-0.5 font-mono dark:bg-slate-800">
                                        {preview.source_object_id ?? 'n/a'}
                                      </span>
                                      <span>→</span>
                                      <span class="rounded-full bg-slate-200 px-2 py-0.5 font-mono dark:bg-slate-800">
                                        {preview.target_object_id ?? 'n/a'}
                                      </span>
                                    </div>
                                    <p class="mt-2 font-mono text-xs text-slate-500">{preview.link_type_id ?? 'link type unavailable'}</p>
                                    <pre class="mt-3 overflow-x-auto rounded-2xl bg-slate-950 p-3 text-xs text-slate-100">{formatJson(preview.preview)}</pre>
                                  </div>
                                {/each}
                              </div>
                            {/if}
                          </div>
                        </div>
                      </div>

                      <details class="mt-4 rounded-2xl border border-slate-200 px-4 py-3 dark:border-slate-800">
                        <summary class="cursor-pointer font-medium text-slate-900 dark:text-slate-100">Raw scenario payload</summary>
                        <pre class="mt-3 overflow-x-auto rounded-2xl bg-slate-950 p-4 text-xs text-slate-100">{formatJson(result)}</pre>
                      </details>
                    </article>
                  {/each}
                {/if}
              </div>
            {/if}
          </div>
        </div>

        <div class="space-y-4">
          {#if objectViewLoading}
            <div class="rounded-2xl border border-dashed border-slate-300 px-4 py-10 text-center text-sm text-slate-500 dark:border-slate-700">
              Loading object inspector...
            </div>
          {:else if objectView}
            <div class="grid gap-3 md:grid-cols-2">
              {#each Object.entries(objectView.summary) as [label, value]}
                <div class="rounded-2xl bg-slate-100 px-4 py-3 dark:bg-slate-800/70">
                  <div class="text-xs uppercase tracking-[0.2em] text-slate-500">{label.replaceAll('_', ' ')}</div>
                  <div class="mt-1 text-xl font-semibold text-slate-900 dark:text-slate-100">{String(value)}</div>
                </div>
              {/each}
            </div>

            <div class="rounded-2xl border border-slate-200 p-4 dark:border-slate-800">
              <div class="flex flex-wrap items-center justify-between gap-3">
                <div>
                  <h3 class="font-medium text-slate-900 dark:text-slate-100">Graph impact summary</h3>
                  <p class="mt-1 text-sm text-slate-500">
                    Vertex now classifies this neighborhood by blast radius, sensitivity and boundary crossings.
                  </p>
                </div>
                <span class="rounded-full bg-slate-100 px-3 py-1 text-xs font-medium text-slate-700 dark:bg-slate-800 dark:text-slate-200">
                  {formatScope(objectView.graph.summary.scope)}
                </span>
              </div>

              <div class="mt-4 grid gap-3 md:grid-cols-4">
                <div class="rounded-2xl bg-slate-100 px-4 py-3 dark:bg-slate-800/70">
                  <div class="text-xs uppercase tracking-[0.2em] text-slate-500">Direct neighbors</div>
                  <div class="mt-1 text-xl font-semibold text-slate-900 dark:text-slate-100">
                    {objectView.graph.summary.root_neighbor_count}
                  </div>
                </div>
                <div class="rounded-2xl bg-slate-100 px-4 py-3 dark:bg-slate-800/70">
                  <div class="text-xs uppercase tracking-[0.2em] text-slate-500">Max hops</div>
                  <div class="mt-1 text-xl font-semibold text-slate-900 dark:text-slate-100">
                    {objectView.graph.summary.max_hops_reached}
                  </div>
                </div>
                <div class="rounded-2xl bg-slate-100 px-4 py-3 dark:bg-slate-800/70">
                  <div class="text-xs uppercase tracking-[0.2em] text-slate-500">Sensitive objects</div>
                  <div class="mt-1 text-xl font-semibold text-slate-900 dark:text-slate-100">
                    {objectView.graph.summary.sensitive_objects}
                  </div>
                </div>
                <div class="rounded-2xl bg-slate-100 px-4 py-3 dark:bg-slate-800/70">
                  <div class="text-xs uppercase tracking-[0.2em] text-slate-500">Boundary crossings</div>
                  <div class="mt-1 text-xl font-semibold text-slate-900 dark:text-slate-100">
                    {objectView.graph.summary.boundary_crossings}
                  </div>
                </div>
              </div>

              {#if countEntries(objectView.graph.summary.object_types).length > 0}
                <div class="mt-4">
                  <p class="text-xs uppercase tracking-[0.2em] text-slate-500">Types in neighborhood</p>
                  <div class="mt-2 flex flex-wrap gap-2">
                    {#each countEntries(objectView.graph.summary.object_types) as [label, count]}
                      <span class="rounded-full bg-cyan-50 px-3 py-1 text-xs font-medium text-cyan-700 dark:bg-cyan-950/40 dark:text-cyan-300">
                        {label} · {count}
                      </span>
                    {/each}
                  </div>
                </div>
              {/if}

              {#if objectView.graph.summary.sensitive_markings.length > 0}
                <div class="mt-4">
                  <p class="text-xs uppercase tracking-[0.2em] text-slate-500">Sensitive markings</p>
                  <div class="mt-2 flex flex-wrap gap-2">
                    {#each objectView.graph.summary.sensitive_markings as marking}
                      <span class="rounded-full bg-amber-50 px-3 py-1 text-xs font-medium text-amber-700 dark:bg-amber-950/40 dark:text-amber-300">
                        {marking}
                      </span>
                    {/each}
                  </div>
                </div>
              {/if}
            </div>

            <details class="rounded-2xl border border-slate-200 px-4 py-3 dark:border-slate-800">
              <summary class="cursor-pointer font-medium text-slate-900 dark:text-slate-100">Object snapshot</summary>
              <pre class="mt-3 overflow-x-auto rounded-2xl bg-slate-950 p-4 text-xs text-slate-100">{formatJson(objectView.object)}</pre>
            </details>

            <div class="grid gap-4 lg:grid-cols-2">
              <div class="rounded-2xl border border-slate-200 p-4 dark:border-slate-800">
                <h3 class="font-medium text-slate-900 dark:text-slate-100">Matching rules</h3>
                {#if objectView.matching_rules.length === 0}
                  <p class="mt-3 text-sm text-slate-500">No rules currently match this object.</p>
                {:else}
                  <div class="mt-3 space-y-2">
                    {#each objectView.matching_rules as matchResult (matchResult.rule_id)}
                      <pre class="overflow-x-auto rounded-2xl bg-slate-950 p-3 text-xs text-slate-100">{formatJson(matchResult)}</pre>
                    {/each}
                  </div>
                {/if}
              </div>

              <div class="rounded-2xl border border-slate-200 p-4 dark:border-slate-800">
                <h3 class="font-medium text-slate-900 dark:text-slate-100">Applicable actions</h3>
                <div class="mt-3 flex flex-wrap gap-2">
                  {#each objectView.applicable_actions as action (action.id)}
                    <button
                      type="button"
                      class={`rounded-full px-3 py-1 text-xs font-medium ${selectedActionId === action.id ? 'bg-sky-600 text-white' : 'bg-slate-100 text-slate-700 dark:bg-slate-800 dark:text-slate-200'}`}
                      onclick={() => void handleSelectAction(action.id)}
                    >
                      {action.display_name}
                    </button>
                  {/each}
                </div>
              </div>
            </div>

            <details class="rounded-2xl border border-slate-200 px-4 py-3 dark:border-slate-800">
              <summary class="cursor-pointer font-medium text-slate-900 dark:text-slate-100">Timeline & machinery history</summary>
              <pre class="mt-3 overflow-x-auto rounded-2xl bg-slate-950 p-4 text-xs text-slate-100">{formatJson(objectView.timeline)}</pre>
            </details>

            <details class="rounded-2xl border border-slate-200 px-4 py-3 dark:border-slate-800">
              <summary class="cursor-pointer font-medium text-slate-900 dark:text-slate-100">Graph neighborhood snapshot</summary>
              <pre class="mt-3 overflow-x-auto rounded-2xl bg-slate-950 p-4 text-xs text-slate-100">{formatJson(objectView.graph)}</pre>
            </details>
          {:else}
            <div class="rounded-2xl border border-dashed border-slate-300 px-4 py-10 text-center text-sm text-slate-500 dark:border-slate-700">
              Select an object to inspect its graph, rules, timeline, and projected simulations.
            </div>
          {/if}
        </div>
      </div>
    </section>
  </div>
{/if}
