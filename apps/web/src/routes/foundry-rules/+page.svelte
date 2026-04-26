<script lang="ts">
  import Glyph from '$components/ui/Glyph.svelte';
  import {
    applyRule,
    createRule,
    getMachineryInsights,
    getMachineryQueue,
    listObjectTypes,
    listRules,
    simulateRule,
    updateMachineryQueueItem,
    updateRule,
    type MachineryInsight,
    type MachineryQueueItem,
    type MachineryQueueResponse,
    type ObjectType,
    type OntologyRule,
    type RuleEffectSpec,
    type RuleEvaluationMode,
    type RuleTriggerSpec
  } from '$lib/api/ontology';
  import {
    evaluateCronWorkflows,
    listWorkflowApprovals,
    listWorkflowRuns,
    listWorkflows,
    startWorkflowRun,
    type WorkflowApproval,
    type WorkflowDefinition,
    type WorkflowRun
  } from '$lib/api/workflows';
  import { notifications } from '$stores/notifications';

  type RuleDraft = {
    id?: string;
    name: string;
    display_name: string;
    description: string;
    object_type_id: string;
    evaluation_mode: RuleEvaluationMode;
    triggerText: string;
    effectText: string;
  };

  type RuleEvaluationPreview = {
    mode: 'simulation' | 'application';
    matched: boolean;
    objectId: string;
    trigger_payload: Record<string, unknown>;
    effect_preview: Record<string, unknown> | null;
    evaluatedAt: string;
  };

  type Bucket = {
    label: string;
    value: number;
  };

  const triggerTemplateAlert: RuleTriggerSpec = {
    numeric_gte: { risk_score: 80 },
    exists: ['transaction_id'],
    changed_properties: ['risk_score']
  };

  const effectTemplateAlert: RuleEffectSpec = {
    alert: {
      severity: 'high',
      title: 'Suspicious activity threshold exceeded',
      message: 'Escalate this object into analyst review.'
    }
  };

  const triggerTemplateSchedule: RuleTriggerSpec = {
    equals: { status: 'pending_review' },
    numeric_lte: { hours_until_deadline: 24 }
  };

  const effectTemplateSchedule: RuleEffectSpec = {
    schedule: {
      property_name: 'due_at',
      offset_hours: -4,
      priority_score: 85,
      estimated_duration_minutes: 90,
      required_capability: 'review',
      constraint_tags: ['deadline', 'manual-review'],
      hard_deadline_hours: 0
    },
    alert: {
      severity: 'medium',
      title: 'Review window approaching',
      message: 'Place this object into the operations schedule before SLA breach.'
    }
  };

  const triggerTemplateCohort: RuleTriggerSpec = {
    equals: { region: 'emea' },
    numeric_gte: { lifetime_value: 10000 }
  };

  const effectTemplateCohort: RuleEffectSpec = {
    object_patch: {
      customer_segment: 'priority_growth',
      rule_source: 'foundry_rules'
    }
  };

  let objectTypes = $state<ObjectType[]>([]);
  let rules = $state<OntologyRule[]>([]);
  let insights = $state<MachineryInsight[]>([]);
  let queue = $state<MachineryQueueResponse | null>(null);
  let workflows = $state<WorkflowDefinition[]>([]);
  let approvals = $state<WorkflowApproval[]>([]);
  let workflowRuns = $state<WorkflowRun[]>([]);

  let selectedObjectTypeId = $state('');
  let selectedRuleId = $state('');
  let selectedWorkflowId = $state('');
  let ruleSearch = $state('');
  let evaluationObjectId = $state('');

  let loading = $state(true);
  let savingRule = $state(false);
  let evaluatingRule = $state(false);
  let queueBusy = $state('');
  let workflowBusy = $state('');
  let error = $state('');

  let evaluationPreview = $state<RuleEvaluationPreview | null>(null);
  let draft = $state<RuleDraft>(createEmptyDraft(''));

  let enableSelfManagedTransforms = $state(true);
  let enableStreamingConfiguration = $state(false);
  let selectedRepository = $state('rules.transforms/FoundryRulesTransform');
  let customPropertyName = $state('');
  let customProperties = $state<string[]>([]);
  let defaultSeverity = $state<'low' | 'medium' | 'high' | 'critical'>('medium');
  let enableRuleActions = $state(true);
  let enableTimeSeriesOutputs = $state(true);
  let enableMarketplacePackaging = $state(false);

  function createEmptyDraft(objectTypeId: string): RuleDraft {
    return {
      name: '',
      display_name: '',
      description: '',
      object_type_id: objectTypeId,
      evaluation_mode: 'advisory',
      triggerText: JSON.stringify({}, null, 2),
      effectText: JSON.stringify({}, null, 2)
    };
  }

  function titleizeCapability(value: string | null | undefined) {
    const normalized = (value || 'general').replaceAll('_', ' ');
    return normalized.charAt(0).toUpperCase() + normalized.slice(1);
  }

  function formatTimestamp(value: string | null | undefined) {
    if (!value) return 'n/a';
    return new Intl.DateTimeFormat('en', {
      month: 'short',
      day: 'numeric',
      hour: 'numeric',
      minute: '2-digit'
    }).format(new Date(value));
  }

  function formatDay(value: Date) {
    return new Intl.DateTimeFormat('en', {
      weekday: 'short',
      month: 'short',
      day: 'numeric'
    }).format(value);
  }

  function formatCompactDuration(minutes: number) {
    if (minutes >= 60) {
      const hours = Math.floor(minutes / 60);
      const remainder = minutes % 60;
      return remainder === 0 ? `${hours}h` : `${hours}h ${remainder}m`;
    }
    return `${minutes}m`;
  }

  function parseJson<T>(value: string, label: string): T {
    try {
      return JSON.parse(value) as T;
    } catch (cause) {
      throw new Error(`${label} must be valid JSON: ${cause instanceof Error ? cause.message : 'parse failure'}`);
    }
  }

  function selectedObjectType() {
    return objectTypes.find((typeItem) => typeItem.id === selectedObjectTypeId) ?? null;
  }

  function selectedRule() {
    return rules.find((rule) => rule.id === selectedRuleId) ?? null;
  }

  function selectedWorkflow() {
    return workflows.find((workflow) => workflow.id === selectedWorkflowId) ?? null;
  }

  function selectedInsight() {
    return insights.find((item) => item.rule_id === selectedRuleId) ?? null;
  }

  function selectedRuleQueueItems() {
    return (queue?.data ?? []).filter((item) => item.rule_id === selectedRuleId);
  }

  function maxBucketValue(buckets: Bucket[]) {
    return Math.max(1, ...buckets.map((bucket) => bucket.value));
  }

  function filteredRules() {
    const query = ruleSearch.trim().toLowerCase();
    return rules.filter((rule) => {
      if (!query) return true;
      return (
        rule.name.toLowerCase().includes(query) ||
        rule.display_name.toLowerCase().includes(query) ||
        rule.description.toLowerCase().includes(query)
      );
    });
  }

  function automaticRuleCount() {
    return rules.filter((rule) => rule.evaluation_mode === 'automatic').length;
  }

  function advisoryRuleCount() {
    return rules.filter((rule) => rule.evaluation_mode === 'advisory').length;
  }

  function pendingSchedules() {
    return queue?.recommendation.queue_depth ?? 0;
  }

  function overdueSchedules() {
    return queue?.recommendation.overdue_count ?? 0;
  }

  function approvalBacklog() {
    return approvals.filter((approval) => approval.status === 'pending').length;
  }

  function currentRuleTriggerSpec() {
    return parseJson<RuleTriggerSpec>(draft.triggerText, 'Trigger spec');
  }

  function currentRuleEffectSpec() {
    return parseJson<RuleEffectSpec>(draft.effectText, 'Effect spec');
  }

  function conditionsForRule(rule: OntologyRule | null) {
    if (!rule) return [] as string[];
    const items: string[] = [];
    for (const [key, value] of Object.entries(rule.trigger_spec.equals ?? {})) {
      items.push(`${key} = ${String(value)}`);
    }
    for (const [key, value] of Object.entries(rule.trigger_spec.numeric_gte ?? {})) {
      items.push(`${key} >= ${value}`);
    }
    for (const [key, value] of Object.entries(rule.trigger_spec.numeric_lte ?? {})) {
      items.push(`${key} <= ${value}`);
    }
    for (const key of rule.trigger_spec.exists ?? []) {
      items.push(`${key} exists`);
    }
    for (const key of rule.trigger_spec.changed_properties ?? []) {
      items.push(`${key} changed`);
    }
    for (const key of rule.trigger_spec.markings ?? []) {
      items.push(`marking:${key}`);
    }
    return items;
  }

  function outputContract(rule: OntologyRule | null) {
    if (!rule) return [] as { label: string; value: string }[];
    const items: { label: string; value: string }[] = [];
    const schedule = rule.effect_spec.schedule;
    const alert = rule.effect_spec.alert;
    const objectPatch = rule.effect_spec.object_patch ?? {};

    if (schedule) {
      items.push({ label: 'Schedule offset', value: `${schedule.offset_hours}h on ${schedule.property_name}` });
      items.push({ label: 'Capability', value: titleizeCapability(schedule.required_capability) });
      items.push({
        label: 'Duration',
        value: formatCompactDuration(schedule.estimated_duration_minutes ?? 60)
      });
    }

    if (alert) {
      items.push({ label: 'Alert severity', value: alert.severity });
      items.push({ label: 'Alert title', value: alert.title });
    }

    const patchKeys = Object.keys(objectPatch);
    if (patchKeys.length > 0) {
      items.push({ label: 'Output patch', value: patchKeys.join(', ') });
    }

    return items;
  }

  function suggestedObjectIds() {
    const items = selectedRuleQueueItems();
    const ids = new Set<string>();
    for (const item of items) {
      ids.add(item.object_id);
    }
    return Array.from(ids).slice(0, 5);
  }

  function scheduleBuckets() {
    const start = new Date();
    start.setHours(0, 0, 0, 0);
    const buckets: Bucket[] = [];
    for (let index = 0; index < 7; index += 1) {
      const day = new Date(start);
      day.setDate(start.getDate() + index);
      const next = new Date(day);
      next.setDate(day.getDate() + 1);
      const value = (queue?.data ?? []).filter((item) => {
        const scheduled = new Date(item.scheduled_for);
        return scheduled >= day && scheduled < next;
      }).length;
      buckets.push({ label: formatDay(day), value });
    }
    return buckets;
  }

  function runBuckets() {
    const end = new Date();
    end.setHours(23, 59, 59, 999);
    const buckets: Bucket[] = [];
    for (let offset = 6; offset >= 0; offset -= 1) {
      const day = new Date(end);
      day.setDate(end.getDate() - offset);
      day.setHours(0, 0, 0, 0);
      const next = new Date(day);
      next.setDate(day.getDate() + 1);
      const value = workflowRuns.filter((run) => {
        const started = new Date(run.started_at);
        return started >= day && started < next;
      }).length;
      buckets.push({ label: formatDay(day), value });
    }
    return buckets;
  }

  function generatedPipelineCode() {
    const activeRule = selectedRule();
    const activeConditions = conditionsForRule(activeRule);
    const activeOutputs = outputContract(activeRule);
    const propertyLines = [...customProperties];

    if (activeRule?.effect_spec.object_patch) {
      propertyLines.push(...Object.keys(activeRule.effect_spec.object_patch));
    }

    return `export const foundryRulesWorkflow = {
  selfManagedTransforms: ${enableSelfManagedTransforms},
  streamingEnabled: ${enableStreamingConfiguration},
  repository: "${selectedRepository}",
  optionalFeatures: {
    ruleActions: ${enableRuleActions},
    timeSeriesOutputs: ${enableTimeSeriesOutputs},
    marketplacePackaging: ${enableMarketplacePackaging}
  },
  defaultSeverity: "${defaultSeverity}",
  selectedRule: ${activeRule ? `"${activeRule.name}"` : 'null'},
  ruleConditions: ${JSON.stringify(activeConditions, null, 2)},
  outputContract: ${JSON.stringify(activeOutputs, null, 2)},
  customProperties: ${JSON.stringify(propertyLines, null, 2)}
};

export function FoundryRulesTransform(input) {
  return {
    ...input,
    metadata: foundryRulesWorkflow
  };
}`;
  }

  function capabilityLoadRows() {
    return queue?.recommendation.capability_load ?? [];
  }

  function selectRule(rule: OntologyRule) {
    selectedRuleId = rule.id;
    evaluationObjectId = suggestedObjectIds()[0] ?? evaluationObjectId;
    draft = {
      id: rule.id,
      name: rule.name,
      display_name: rule.display_name,
      description: rule.description,
      object_type_id: rule.object_type_id,
      evaluation_mode: rule.evaluation_mode,
      triggerText: JSON.stringify(rule.trigger_spec ?? {}, null, 2),
      effectText: JSON.stringify(rule.effect_spec ?? {}, null, 2)
    };
  }

  function startNewRule(template: 'blank' | 'alert' | 'schedule' | 'cohort' = 'blank') {
    const next = createEmptyDraft(selectedObjectTypeId);
    if (template === 'alert') {
      next.name = 'high_risk_alert';
      next.display_name = 'High risk alert';
      next.description = 'Raise alerts when an object crosses a high-risk threshold.';
      next.triggerText = JSON.stringify(triggerTemplateAlert, null, 2);
      next.effectText = JSON.stringify(effectTemplateAlert, null, 2);
    } else if (template === 'schedule') {
      next.name = 'deadline_review_schedule';
      next.display_name = 'Deadline review schedule';
      next.description = 'Schedule manual review work before the deadline window closes.';
      next.evaluation_mode = 'automatic';
      next.triggerText = JSON.stringify(triggerTemplateSchedule, null, 2);
      next.effectText = JSON.stringify(effectTemplateSchedule, null, 2);
    } else if (template === 'cohort') {
      next.name = 'priority_growth_cohort';
      next.display_name = 'Priority growth cohort';
      next.description = 'Tag strategic customers into a reusable cohort for operations.';
      next.triggerText = JSON.stringify(triggerTemplateCohort, null, 2);
      next.effectText = JSON.stringify(effectTemplateCohort, null, 2);
    }

    selectedRuleId = '';
    evaluationPreview = null;
    draft = next;
  }

  async function loadRuleData() {
    const params = selectedObjectTypeId ? { object_type_id: selectedObjectTypeId, per_page: 100 } : { per_page: 100 };
    const [ruleResponse, insightResponse, queueResponse] = await Promise.all([
      listRules(params),
      getMachineryInsights(selectedObjectTypeId ? { object_type_id: selectedObjectTypeId } : undefined),
      getMachineryQueue(selectedObjectTypeId ? { object_type_id: selectedObjectTypeId } : undefined)
    ]);

    rules = ruleResponse.data;
    insights = insightResponse.data;
    queue = queueResponse;

    if (selectedRuleId) {
      const found = ruleResponse.data.find((rule) => rule.id === selectedRuleId);
      if (found) {
        selectRule(found);
      } else if (ruleResponse.data.length > 0) {
        selectRule(ruleResponse.data[0]);
      } else {
        startNewRule();
      }
    } else if (ruleResponse.data.length > 0) {
      selectRule(ruleResponse.data[0]);
    } else {
      startNewRule();
    }
  }

  async function loadWorkflowData() {
    const workflowResponse = await listWorkflows({ per_page: 50 });
    workflows = workflowResponse.data;

    if (!selectedWorkflowId && workflows.length > 0) {
      selectedWorkflowId = workflows[0].id;
    } else if (selectedWorkflowId && !workflows.find((workflow) => workflow.id === selectedWorkflowId)) {
      selectedWorkflowId = workflows[0]?.id ?? '';
    }

    const [approvalResponse, runResponse] = await Promise.all([
      listWorkflowApprovals({ per_page: 50, status: 'pending', workflow_id: selectedWorkflowId || undefined }),
      selectedWorkflowId ? listWorkflowRuns(selectedWorkflowId, { per_page: 20 }) : Promise.resolve({ data: [], page: 1, per_page: 20, total: 0 })
    ]);

    approvals = approvalResponse.data;
    workflowRuns = runResponse.data;
  }

  async function load() {
    loading = true;
    error = '';
    try {
      const typeResponse = await listObjectTypes({ page: 1, per_page: 100 });
      objectTypes = typeResponse.data;
      if (!selectedObjectTypeId && objectTypes.length > 0) {
        selectedObjectTypeId = objectTypes[0].id;
      }

      await Promise.all([loadRuleData(), loadWorkflowData()]);
    } catch (cause) {
      error = cause instanceof Error ? cause.message : 'Failed to load Foundry Rules surfaces';
    } finally {
      loading = false;
    }
  }

  async function refreshForObjectType(nextObjectTypeId: string) {
    selectedObjectTypeId = nextObjectTypeId;
    await loadRuleData();
  }

  async function saveCurrentRule() {
    if (!draft.object_type_id) {
      notifications.error('Select an object type before saving the rule.');
      return;
    }

    savingRule = true;
    error = '';
    try {
      const trigger_spec = currentRuleTriggerSpec();
      const effect_spec = currentRuleEffectSpec();

      if (draft.id) {
        await updateRule(draft.id, {
          display_name: draft.display_name || draft.name,
          description: draft.description,
          evaluation_mode: draft.evaluation_mode,
          trigger_spec,
          effect_spec
        });
        notifications.success('Rule updated');
      } else {
        await createRule({
          name: draft.name.trim(),
          display_name: draft.display_name || draft.name.trim(),
          description: draft.description,
          object_type_id: draft.object_type_id,
          evaluation_mode: draft.evaluation_mode,
          trigger_spec,
          effect_spec
        });
        notifications.success('Rule created');
      }

      await loadRuleData();
    } catch (cause) {
      const message = cause instanceof Error ? cause.message : 'Failed to save rule';
      error = message;
      notifications.error(message);
    } finally {
      savingRule = false;
    }
  }

  async function evaluateCurrentRule(mode: 'simulation' | 'application') {
    const activeRule = selectedRule();
    if (!activeRule) {
      notifications.error('Select an existing rule before running evaluation.');
      return;
    }
    if (!evaluationObjectId.trim()) {
      notifications.error('Provide an object id to evaluate the selected rule.');
      return;
    }

    evaluatingRule = true;
    error = '';
    try {
      const response = mode === 'simulation'
        ? await simulateRule(activeRule.id, { object_id: evaluationObjectId.trim() })
        : await applyRule(activeRule.id, { object_id: evaluationObjectId.trim() });

      evaluationPreview = {
        mode,
        matched: response.matched,
        objectId: evaluationObjectId.trim(),
        trigger_payload: response.trigger_payload,
        effect_preview: response.effect_preview,
        evaluatedAt: new Date().toISOString()
      };

      notifications.success(mode === 'simulation' ? 'Rule simulated' : 'Rule applied');
      if (mode === 'application') {
        await loadRuleData();
      }
    } catch (cause) {
      const message = cause instanceof Error ? cause.message : `Failed to ${mode === 'simulation' ? 'simulate' : 'apply'} rule`;
      error = message;
      notifications.error(message);
    } finally {
      evaluatingRule = false;
    }
  }

  async function updateQueueItemStatus(itemId: string, status: string) {
    queueBusy = itemId;
    try {
      await updateMachineryQueueItem(itemId, { status });
      notifications.success(`Queue item marked as ${status}`);
      await loadRuleData();
    } catch (cause) {
      notifications.error(cause instanceof Error ? cause.message : 'Failed to update queue item');
    } finally {
      queueBusy = '';
    }
  }

  async function runSelectedWorkflow() {
    if (!selectedWorkflowId) {
      notifications.error('Select a workflow first.');
      return;
    }

    workflowBusy = selectedWorkflowId;
    try {
      await startWorkflowRun(selectedWorkflowId, { initiated_from: 'foundry-rules' });
      notifications.success('Workflow run started');
      await loadWorkflowData();
    } catch (cause) {
      notifications.error(cause instanceof Error ? cause.message : 'Failed to run workflow');
    } finally {
      workflowBusy = '';
    }
  }

  async function runDueCronWorkflows() {
    workflowBusy = 'cron';
    try {
      const response = await evaluateCronWorkflows();
      notifications.success(`Triggered ${response.triggered_runs} due cron workflow(s)`);
      await loadWorkflowData();
    } catch (cause) {
      notifications.error(cause instanceof Error ? cause.message : 'Failed to run due cron workflows');
    } finally {
      workflowBusy = '';
    }
  }

  function addCustomProperty() {
    const trimmed = customPropertyName.trim();
    if (!trimmed) return;
    if (!customProperties.includes(trimmed)) {
      customProperties = [...customProperties, trimmed];
    }
    customPropertyName = '';
  }

  function removeCustomProperty(name: string) {
    customProperties = customProperties.filter((item) => item !== name);
  }

  const selectedRuleConditions = $derived(conditionsForRule(selectedRule()));
  const selectedRuleOutputs = $derived(outputContract(selectedRule()));
  const selectedScheduleBuckets = $derived(scheduleBuckets());
  const selectedRunBuckets = $derived(runBuckets());
  const recommendedRepository = $derived(
    selectedObjectType()
      ? `rules.transforms/${selectedObjectType()?.name.replaceAll('.', '_') || 'FoundryRulesTransform'}`
      : 'rules.transforms/FoundryRulesTransform'
  );

  $effect(() => {
    void load();
  });
</script>

<svelte:head>
  <title>OpenFoundry - Foundry Rules</title>
</svelte:head>

<div class="space-y-6">
  <section class="overflow-hidden rounded-[28px] border border-[var(--border-default)] bg-[linear-gradient(135deg,#fbfcff_0%,#f6faf7_45%,#fff8ef_100%)] shadow-[var(--shadow-panel)]">
    <div class="grid gap-8 px-6 py-7 lg:grid-cols-[minmax(0,1.3fr)_360px] lg:px-8">
      <div>
        <div class="of-eyebrow">Foundry Rules</div>
        <h1 class="mt-3 max-w-4xl text-[34px] font-semibold tracking-[-0.03em] text-[var(--text-strong)]">
          Author, deploy, schedule, and customize governed rule logic across ontology objects and operational workflows.
        </h1>
        <p class="mt-4 max-w-3xl text-[15px] leading-7 text-[var(--text-muted)]">
          This surface turns the rules runtime into a dedicated product: low-code authoring, simulation,
          deployment workflows, queue operations, time-series scheduling, and self-managed pipeline customization.
        </p>

        <div class="mt-6 flex flex-wrap gap-3">
          <button type="button" class="of-btn of-btn-primary" onclick={() => startNewRule('alert')}>
            <Glyph name="plus" size={16} />
            <span>Author rule</span>
          </button>
          <a href="#deploy" class="of-btn">
            <Glyph name="run" size={16} />
            <span>Deploy workflows</span>
          </a>
          <a href="#customization" class="of-btn">
            <Glyph name="code" size={16} />
            <span>Customize pipeline</span>
          </a>
        </div>

        <div class="mt-7 grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
          <article class="rounded-[18px] border border-white/75 bg-white/85 px-4 py-4 backdrop-blur">
            <div class="text-[11px] font-semibold uppercase tracking-[0.18em] text-[var(--text-soft)]">Rules</div>
            <div class="mt-2 text-[30px] font-semibold tracking-[-0.03em] text-[var(--text-strong)]">{rules.length}</div>
            <div class="mt-1 text-sm leading-6 text-[var(--text-muted)]">Point-and-click rule logic bound to ontology objects.</div>
          </article>
          <article class="rounded-[18px] border border-white/75 bg-white/85 px-4 py-4 backdrop-blur">
            <div class="text-[11px] font-semibold uppercase tracking-[0.18em] text-[var(--text-soft)]">Automatic / advisory</div>
            <div class="mt-2 text-[30px] font-semibold tracking-[-0.03em] text-[var(--text-strong)]">{automaticRuleCount()} / {advisoryRuleCount()}</div>
            <div class="mt-1 text-sm leading-6 text-[var(--text-muted)]">Mix of auto-application and analyst-facing review rules.</div>
          </article>
          <article class="rounded-[18px] border border-white/75 bg-white/85 px-4 py-4 backdrop-blur">
            <div class="text-[11px] font-semibold uppercase tracking-[0.18em] text-[var(--text-soft)]">Pending schedules</div>
            <div class="mt-2 text-[30px] font-semibold tracking-[-0.03em] text-[var(--text-strong)]">{pendingSchedules()}</div>
            <div class="mt-1 text-sm leading-6 text-[var(--text-muted)]">{overdueSchedules()} overdue items across the machinery queue.</div>
          </article>
          <article class="rounded-[18px] border border-white/75 bg-white/85 px-4 py-4 backdrop-blur">
            <div class="text-[11px] font-semibold uppercase tracking-[0.18em] text-[var(--text-soft)]">Workflow control</div>
            <div class="mt-2 text-[30px] font-semibold tracking-[-0.03em] text-[var(--text-strong)]">{workflows.length}</div>
            <div class="mt-1 text-sm leading-6 text-[var(--text-muted)]">{approvalBacklog()} pending approvals waiting in the review queue.</div>
          </article>
        </div>
      </div>

      <div class="rounded-[24px] border border-white/75 bg-white/80 p-5 shadow-[var(--shadow-panel)] backdrop-blur">
        <div class="flex items-center justify-between">
          <div>
            <div class="text-[11px] font-semibold uppercase tracking-[0.18em] text-[var(--text-soft)]">Coverage</div>
            <div class="mt-1 text-lg font-semibold text-[var(--text-strong)]">Product stack</div>
          </div>
          <span class="rounded-full border border-[#d9e5f6] bg-[#f4f8ff] px-3 py-1 text-xs font-semibold text-[#335ea8]">Dedicated surface</span>
        </div>
        <div class="mt-5 grid gap-3">
          <article class="rounded-[18px] border border-[#d8e3f4] bg-[#f8fbff] p-4">
            <div class="text-sm font-semibold text-[var(--text-strong)]">Rule logic</div>
            <div class="mt-2 text-sm leading-6 text-[var(--text-muted)]">Conditions, effect previews, schedule outputs, and alert contracts for ontology objects.</div>
          </article>
          <article class="rounded-[18px] border border-[#e6dcc5] bg-[#fffaf1] p-4">
            <div class="text-sm font-semibold text-[var(--text-strong)]">Deploy and workflows</div>
            <div class="mt-2 text-sm leading-6 text-[var(--text-muted)]">Operational workflows, approvals, cron triggers, and runtime deployment hooks.</div>
          </article>
          <article class="rounded-[18px] border border-[#d8eadf] bg-[#f4fbf7] p-4">
            <div class="text-sm font-semibold text-[var(--text-strong)]">Time-series and customization</div>
            <div class="mt-2 text-sm leading-6 text-[var(--text-muted)]">Queue pressure, schedule buckets, custom properties, and self-managed transform code generation.</div>
          </article>
        </div>
      </div>
    </div>
  </section>

  {#if error}
    <div class="of-inline-note">{error}</div>
  {/if}

  <div class="grid gap-6 xl:grid-cols-[280px_minmax(0,1fr)_320px]">
    <aside class="space-y-4 xl:sticky xl:top-5 xl:self-start">
      <section class="of-panel overflow-hidden">
        <div class="border-b border-[var(--border-subtle)] bg-[#f7fafc] px-4 py-3">
          <div class="text-[11px] font-semibold uppercase tracking-[0.18em] text-[var(--text-soft)]">Contents</div>
        </div>
        <div class="space-y-1 px-4 py-4">
          <a href="#overview" class="flex items-center gap-2 rounded-[10px] px-3 py-2 text-sm text-[var(--text-default)] hover:bg-[var(--bg-hover)]">
            <Glyph name="chevron-right" size={14} />
            <span>Overview</span>
          </a>
          <a href="#rule-logic" class="flex items-center gap-2 rounded-[10px] px-3 py-2 text-sm text-[var(--text-default)] hover:bg-[var(--bg-hover)]">
            <Glyph name="chevron-right" size={14} />
            <span>Rule logic</span>
          </a>
          <a href="#deploy" class="flex items-center gap-2 rounded-[10px] px-3 py-2 text-sm text-[var(--text-default)] hover:bg-[var(--bg-hover)]">
            <Glyph name="chevron-right" size={14} />
            <span>Deploy</span>
          </a>
          <a href="#time-series" class="flex items-center gap-2 rounded-[10px] px-3 py-2 text-sm text-[var(--text-default)] hover:bg-[var(--bg-hover)]">
            <Glyph name="chevron-right" size={14} />
            <span>Time series</span>
          </a>
          <a href="#customization" class="flex items-center gap-2 rounded-[10px] px-3 py-2 text-sm text-[var(--text-default)] hover:bg-[var(--bg-hover)]">
            <Glyph name="chevron-right" size={14} />
            <span>Settings & customization</span>
          </a>
        </div>
      </section>

      <section class="of-panel overflow-hidden">
        <div class="border-b border-[var(--border-subtle)] bg-[#f7fafc] px-4 py-3">
          <div class="text-[11px] font-semibold uppercase tracking-[0.18em] text-[var(--text-soft)]">Object model</div>
        </div>
        <div class="space-y-4 px-4 py-4">
          <div>
            <label for="object-type" class="mb-2 block text-sm font-medium text-[var(--text-strong)]">Object type</label>
            <select
              id="object-type"
              bind:value={selectedObjectTypeId}
              class="w-full rounded-xl border border-[var(--border-default)] bg-white px-3 py-2 text-sm"
              onchange={(event) => void refreshForObjectType((event.currentTarget as HTMLSelectElement).value)}
            >
              {#each objectTypes as typeItem (typeItem.id)}
                <option value={typeItem.id}>{typeItem.display_name}</option>
              {/each}
            </select>
          </div>

          <div>
            <label for="rule-search" class="mb-2 block text-sm font-medium text-[var(--text-strong)]">Rule catalog</label>
            <input
              id="rule-search"
              bind:value={ruleSearch}
              placeholder="Search rules..."
              class="w-full rounded-xl border border-[var(--border-default)] bg-white px-3 py-2 text-sm"
            />
          </div>

          <div class="space-y-2">
            {#if loading}
              <div class="rounded-[14px] border border-dashed border-[var(--border-default)] px-4 py-8 text-center text-sm text-[var(--text-muted)]">Loading rules...</div>
            {:else if filteredRules().length === 0}
              <div class="rounded-[14px] border border-dashed border-[var(--border-default)] px-4 py-8 text-center text-sm text-[var(--text-muted)]">No rules match the current filter.</div>
            {:else}
              {#each filteredRules() as rule (rule.id)}
                <button
                  type="button"
                  onclick={() => selectRule(rule)}
                  class={`w-full rounded-[16px] border px-4 py-3 text-left transition ${selectedRuleId === rule.id ? 'border-[#95b6e8] bg-[#f4f8ff]' : 'border-[var(--border-default)] bg-white hover:border-[#bfd0ea] hover:bg-[#fbfcfe]'}`}
                >
                  <div class="flex items-start justify-between gap-3">
                    <div class="min-w-0">
                      <div class="truncate text-sm font-semibold text-[var(--text-strong)]">{rule.display_name}</div>
                      <div class="mt-1 text-xs text-[var(--text-soft)]">{rule.name}</div>
                    </div>
                    <span class={`rounded-full px-2 py-1 text-[11px] font-semibold ${rule.evaluation_mode === 'automatic' ? 'bg-[#eef5e8] text-[#356b3c]' : 'bg-[#edf3ff] text-[#335ea8]'}`}>
                      {rule.evaluation_mode}
                    </span>
                  </div>
                  <div class="mt-2 text-sm leading-6 text-[var(--text-muted)]">{rule.description || 'No description yet.'}</div>
                </button>
              {/each}
            {/if}
          </div>

          <div class="grid gap-2 sm:grid-cols-2">
            <button type="button" class="of-btn" onclick={() => startNewRule('schedule')}>
              <Glyph name="plus" size={15} />
              <span>Schedule rule</span>
            </button>
            <button type="button" class="of-btn" onclick={() => startNewRule('cohort')}>
              <Glyph name="plus" size={15} />
              <span>Cohort rule</span>
            </button>
          </div>
        </div>
      </section>
    </aside>

    <main class="space-y-6">
      <section id="overview" class="of-panel overflow-hidden">
        <div class="border-b border-[var(--border-subtle)] bg-[#fbfcfe] px-5 py-4">
          <div class="of-heading-sm">Overview</div>
          <div class="mt-1 text-sm text-[var(--text-muted)]">A dedicated low-code product for creating, managing, and applying rules to ontology objects and operational workflows.</div>
        </div>
        <div class="space-y-6 px-5 py-5">
          <div class="grid gap-4 lg:grid-cols-3">
            <article class="rounded-[18px] border border-[var(--border-default)] bg-white p-4">
              <div class="text-sm font-semibold text-[var(--text-strong)]">Anti-Money Laundering</div>
              <div class="mt-2 text-sm leading-6 text-[var(--text-muted)]">Flag suspicious transactions with threshold logic, approvals, and downstream analyst scheduling.</div>
            </article>
            <article class="rounded-[18px] border border-[var(--border-default)] bg-white p-4">
              <div class="text-sm font-semibold text-[var(--text-strong)]">Equipment monitoring</div>
              <div class="mt-2 text-sm leading-6 text-[var(--text-muted)]">Raise alerts for degrading assets and automatically stage inspections into the machinery queue.</div>
            </article>
            <article class="rounded-[18px] border border-[var(--border-default)] bg-white p-4">
              <div class="text-sm font-semibold text-[var(--text-strong)]">Cohorting</div>
              <div class="mt-2 text-sm leading-6 text-[var(--text-muted)]">Classify entities into reusable operational groups with rule-driven object patches and outputs.</div>
            </article>
          </div>

          <div class="rounded-[22px] border border-[#dbe7f6] bg-[#f9fbff] p-5">
            <div class="flex items-center justify-between gap-4">
              <div>
                <div class="text-xs font-semibold uppercase tracking-[0.16em] text-[var(--text-soft)]">Rule logic preview</div>
                <div class="mt-1 text-lg font-semibold text-[var(--text-strong)]">{selectedRule()?.display_name || 'Select a rule'}</div>
              </div>
              {#if selectedRule()}
                <span class={`rounded-full px-3 py-1 text-xs font-semibold ${selectedRule()?.evaluation_mode === 'automatic' ? 'bg-[#eef5e8] text-[#356b3c]' : 'bg-[#edf3ff] text-[#335ea8]'}`}>
                  {selectedRule()?.evaluation_mode}
                </span>
              {/if}
            </div>

            <div class="mt-4 rounded-[18px] border border-[#d9e3ef] bg-white p-4">
              <div class="flex flex-wrap items-center gap-3 text-sm text-[var(--text-muted)]">
                <span class="rounded-full bg-[#e7f5ed] px-3 py-1 font-semibold text-[#356b3c]">All of the following is true</span>
                <span>Rule conditions expand into reusable filter groups.</span>
              </div>
              <div class="mt-4 space-y-3">
                {#if selectedRuleConditions.length === 0}
                  <div class="rounded-[14px] border border-dashed border-[var(--border-default)] px-4 py-5 text-sm text-[var(--text-muted)]">This rule does not define conditions yet.</div>
                {:else}
                  {#each selectedRuleConditions as condition, index (condition + index)}
                    <div class="flex items-center gap-3 rounded-[14px] border border-[var(--border-default)] px-4 py-3">
                      <span class="rounded-full bg-[#f3ead6] px-2 py-1 text-[11px] font-semibold uppercase tracking-[0.08em] text-[#9a6c2f]">
                        {index === 0 ? 'AND' : 'OR'}
                      </span>
                      <div class="text-sm font-medium text-[var(--text-strong)]">{condition}</div>
                    </div>
                  {/each}
                {/if}
              </div>
            </div>
          </div>
        </div>
      </section>

      <section id="rule-logic" class="grid gap-6 xl:grid-cols-[minmax(0,1.15fr)_380px]">
        <section class="of-panel overflow-hidden">
          <div class="border-b border-[var(--border-subtle)] bg-[#fbfcfe] px-5 py-4">
            <div class="flex flex-wrap items-center justify-between gap-3">
              <div>
                <div class="of-heading-sm">Author and run a rule</div>
                <div class="mt-1 text-sm text-[var(--text-muted)]">Edit metadata, conditions, schedule outputs, alerts, and object patch behavior from a dedicated rule workbench.</div>
              </div>
              <div class="flex flex-wrap gap-2">
                <button type="button" class="of-btn" onclick={() => startNewRule('blank')}>
                  <Glyph name="plus" size={15} />
                  <span>Blank</span>
                </button>
                <button type="button" class="of-btn" onclick={() => startNewRule('alert')}>
                  <Glyph name="bell" size={15} />
                  <span>Alerting</span>
                </button>
                <button type="button" class="of-btn" onclick={() => startNewRule('schedule')}>
                  <Glyph name="run" size={15} />
                  <span>Scheduling</span>
                </button>
              </div>
            </div>
          </div>

          <div class="space-y-5 px-5 py-5">
            <div class="grid gap-4 md:grid-cols-2">
              <div>
                <label for="rule-name" class="mb-2 block text-sm font-medium text-[var(--text-strong)]">Rule name</label>
                <input
                  id="rule-name"
                  bind:value={draft.name}
                  disabled={Boolean(draft.id)}
                  placeholder="high_risk_alert"
                  class="w-full rounded-xl border border-[var(--border-default)] bg-white px-3 py-2 text-sm disabled:bg-[#f4f6f8] disabled:text-[var(--text-soft)]"
                />
              </div>
              <div>
                <label for="rule-display-name" class="mb-2 block text-sm font-medium text-[var(--text-strong)]">Display name</label>
                <input
                  id="rule-display-name"
                  bind:value={draft.display_name}
                  placeholder="High risk alert"
                  class="w-full rounded-xl border border-[var(--border-default)] bg-white px-3 py-2 text-sm"
                />
              </div>
            </div>

            <div class="grid gap-4 md:grid-cols-[minmax(0,1fr)_220px]">
              <div>
                <label for="rule-description" class="mb-2 block text-sm font-medium text-[var(--text-strong)]">Description</label>
                <textarea
                  id="rule-description"
                  bind:value={draft.description}
                  rows="3"
                  class="w-full rounded-xl border border-[var(--border-default)] bg-white px-3 py-2 text-sm"
                ></textarea>
              </div>
              <div>
                <label for="rule-mode" class="mb-2 block text-sm font-medium text-[var(--text-strong)]">Evaluation mode</label>
                <select id="rule-mode" bind:value={draft.evaluation_mode} class="w-full rounded-xl border border-[var(--border-default)] bg-white px-3 py-2 text-sm">
                  <option value="advisory">advisory</option>
                  <option value="automatic">automatic</option>
                </select>
                <div class="mt-3 rounded-[14px] border border-[#e7edf5] bg-[#fbfcfe] px-3 py-3 text-sm text-[var(--text-muted)]">
                  Existing object type:
                  <span class="font-semibold text-[var(--text-strong)]"> {selectedObjectType()?.display_name || 'n/a'}</span>
                </div>
              </div>
            </div>

            <div class="grid gap-4 xl:grid-cols-2">
              <div>
                <label for="trigger-json" class="mb-2 block text-sm font-medium text-[var(--text-strong)]">Trigger spec</label>
                <textarea
                  id="trigger-json"
                  bind:value={draft.triggerText}
                  rows="14"
                  class="w-full rounded-xl border border-[var(--border-default)] bg-[#0f172a] px-3 py-3 font-mono text-sm text-slate-100"
                ></textarea>
              </div>
              <div>
                <label for="effect-json" class="mb-2 block text-sm font-medium text-[var(--text-strong)]">Effect spec</label>
                <textarea
                  id="effect-json"
                  bind:value={draft.effectText}
                  rows="14"
                  class="w-full rounded-xl border border-[var(--border-default)] bg-[#0f172a] px-3 py-3 font-mono text-sm text-slate-100"
                ></textarea>
              </div>
            </div>

            <div class="flex flex-wrap gap-3">
              <button type="button" class="of-btn of-btn-primary" onclick={() => void saveCurrentRule()} disabled={savingRule}>
                <Glyph name="bookmark" size={15} />
                <span>{savingRule ? 'Saving...' : draft.id ? 'Save changes' : 'Create rule'}</span>
              </button>
              <button type="button" class="of-btn" onclick={() => startNewRule('cohort')}>
                <Glyph name="cube" size={15} />
                <span>Load cohort template</span>
              </button>
              <button type="button" class="of-btn" onclick={() => startNewRule('schedule')}>
                <Glyph name="run" size={15} />
                <span>Load scheduling template</span>
              </button>
            </div>
          </div>
        </section>

        <section class="of-panel overflow-hidden">
          <div class="border-b border-[var(--border-subtle)] bg-[#fbfcfe] px-5 py-4">
            <div class="of-heading-sm">Simulation and apply</div>
            <div class="mt-1 text-sm text-[var(--text-muted)]">Run the selected rule against a live ontology object id, preview outputs, then apply when you are ready.</div>
          </div>

          <div class="space-y-4 px-5 py-5">
            <div>
              <label for="evaluation-object-id" class="mb-2 block text-sm font-medium text-[var(--text-strong)]">Object id</label>
              <input
                id="evaluation-object-id"
                bind:value={evaluationObjectId}
                placeholder="Paste an object id from the current object type"
                class="w-full rounded-xl border border-[var(--border-default)] bg-white px-3 py-2 text-sm"
              />
            </div>

            {#if suggestedObjectIds().length > 0}
              <div>
                <div class="mb-2 text-xs font-semibold uppercase tracking-[0.12em] text-[var(--text-soft)]">Suggested from queue</div>
                <div class="flex flex-wrap gap-2">
                  {#each suggestedObjectIds() as objectId}
                    <button type="button" class="rounded-full border border-[#d8e3f4] bg-[#f8fbff] px-3 py-1.5 text-xs font-medium text-[#335ea8]" onclick={() => evaluationObjectId = objectId}>
                      {objectId}
                    </button>
                  {/each}
                </div>
              </div>
            {/if}

            <div class="flex flex-wrap gap-3">
              <button type="button" class="of-btn" onclick={() => void evaluateCurrentRule('simulation')} disabled={evaluatingRule}>
                <Glyph name="search" size={15} />
                <span>{evaluatingRule ? 'Running...' : 'Simulate'}</span>
              </button>
              <button type="button" class="of-btn of-btn-primary" onclick={() => void evaluateCurrentRule('application')} disabled={evaluatingRule}>
                <Glyph name="run" size={15} />
                <span>{evaluatingRule ? 'Applying...' : 'Apply rule'}</span>
              </button>
            </div>

            {#if evaluationPreview}
              <div class="rounded-[18px] border border-[var(--border-default)] bg-[#fbfcfe] p-4">
                <div class="flex items-center justify-between gap-3">
                  <div>
                    <div class="text-sm font-semibold text-[var(--text-strong)]">{evaluationPreview.mode === 'simulation' ? 'Simulation preview' : 'Applied result'}</div>
                    <div class="mt-1 text-sm text-[var(--text-muted)]">Object {evaluationPreview.objectId} evaluated at {formatTimestamp(evaluationPreview.evaluatedAt)}</div>
                  </div>
                  <span class={`rounded-full px-3 py-1 text-xs font-semibold ${evaluationPreview.matched ? 'bg-[#eef5e8] text-[#356b3c]' : 'bg-[#fff4e5] text-[#9a6c2f]'}`}>
                    {evaluationPreview.matched ? 'Matched' : 'No match'}
                  </span>
                </div>
                <div class="mt-4 grid gap-4">
                  <div>
                    <div class="mb-2 text-xs font-semibold uppercase tracking-[0.12em] text-[var(--text-soft)]">Trigger payload</div>
                    <pre class="overflow-x-auto rounded-[14px] bg-[#0f172a] px-3 py-3 text-xs text-slate-100">{JSON.stringify(evaluationPreview.trigger_payload, null, 2)}</pre>
                  </div>
                  <div>
                    <div class="mb-2 text-xs font-semibold uppercase tracking-[0.12em] text-[var(--text-soft)]">Effect preview</div>
                    <pre class="overflow-x-auto rounded-[14px] bg-[#0f172a] px-3 py-3 text-xs text-slate-100">{JSON.stringify(evaluationPreview.effect_preview ?? {}, null, 2)}</pre>
                  </div>
                </div>
              </div>
            {/if}
          </div>
        </section>
      </section>

      <section id="deploy" class="of-panel overflow-hidden">
        <div class="border-b border-[var(--border-subtle)] bg-[#fbfcfe] px-5 py-4">
          <div class="flex flex-wrap items-center justify-between gap-3">
            <div>
              <div class="of-heading-sm">Deploy and workflow control</div>
              <div class="mt-1 text-sm text-[var(--text-muted)]">Configure how rules get operationalized into manual, cron, event, and approval-driven workflows.</div>
            </div>
            <div class="flex flex-wrap gap-2">
              <button type="button" class="of-btn" onclick={() => void runDueCronWorkflows()} disabled={workflowBusy === 'cron'}>
                <Glyph name="run" size={15} />
                <span>{workflowBusy === 'cron' ? 'Running...' : 'Run due cron workflows'}</span>
              </button>
              <a href="/workflows" class="of-btn">
                <Glyph name="chevron-right" size={15} />
                <span>Open workflow builder</span>
              </a>
            </div>
          </div>
        </div>

        <div class="grid gap-6 px-5 py-5 xl:grid-cols-[minmax(0,1fr)_360px]">
          <div class="space-y-4">
            <div class="grid gap-4 lg:grid-cols-3">
              <article class="rounded-[18px] border border-[var(--border-default)] bg-white p-4">
                <div class="text-xs font-semibold uppercase tracking-[0.12em] text-[var(--text-soft)]">Workflows</div>
                <div class="mt-2 text-2xl font-semibold text-[var(--text-strong)]">{workflows.length}</div>
                <div class="mt-1 text-sm text-[var(--text-muted)]">Deployment surfaces available for operational rules.</div>
              </article>
              <article class="rounded-[18px] border border-[var(--border-default)] bg-white p-4">
                <div class="text-xs font-semibold uppercase tracking-[0.12em] text-[var(--text-soft)]">Pending approvals</div>
                <div class="mt-2 text-2xl font-semibold text-[var(--text-strong)]">{approvalBacklog()}</div>
                <div class="mt-1 text-sm text-[var(--text-muted)]">Human-in-the-loop proposals waiting for decision.</div>
              </article>
              <article class="rounded-[18px] border border-[var(--border-default)] bg-white p-4">
                <div class="text-xs font-semibold uppercase tracking-[0.12em] text-[var(--text-soft)]">Recent runs</div>
                <div class="mt-2 text-2xl font-semibold text-[var(--text-strong)]">{workflowRuns.length}</div>
                <div class="mt-1 text-sm text-[var(--text-muted)]">Latest executions and state transitions for the selected workflow.</div>
              </article>
            </div>

            <div class="rounded-[18px] border border-[var(--border-default)] bg-white p-4">
              <div class="mb-3 flex items-center justify-between gap-3">
                <div class="text-sm font-semibold text-[var(--text-strong)]">Workflow configuration</div>
                <button type="button" class="of-btn" onclick={() => void runSelectedWorkflow()} disabled={!selectedWorkflowId || workflowBusy === selectedWorkflowId}>
                  <Glyph name="run" size={15} />
                  <span>{workflowBusy === selectedWorkflowId ? 'Starting...' : 'Start workflow run'}</span>
                </button>
              </div>

              <div class="grid gap-4 lg:grid-cols-[260px_minmax(0,1fr)]">
                <div class="space-y-2">
                  {#if workflows.length === 0}
                    <div class="rounded-[14px] border border-dashed border-[var(--border-default)] px-4 py-8 text-center text-sm text-[var(--text-muted)]">No workflows available yet.</div>
                  {:else}
                    {#each workflows as workflow (workflow.id)}
                      <button
                        type="button"
                        onclick={() => { selectedWorkflowId = workflow.id; void loadWorkflowData(); }}
                        class={`w-full rounded-[14px] border px-4 py-3 text-left transition ${selectedWorkflowId === workflow.id ? 'border-[#95b6e8] bg-[#f4f8ff]' : 'border-[var(--border-default)] hover:border-[#bfd0ea] hover:bg-[#fbfcfe]'}`}
                      >
                        <div class="text-sm font-semibold text-[var(--text-strong)]">{workflow.name}</div>
                        <div class="mt-1 text-xs text-[var(--text-soft)]">{workflow.trigger_type} · {workflow.status}</div>
                        <div class="mt-2 text-sm leading-6 text-[var(--text-muted)]">{workflow.description || 'No description'}</div>
                      </button>
                    {/each}
                  {/if}
                </div>

                <div class="space-y-4">
                  {#if selectedWorkflow()}
                    <div class="rounded-[16px] border border-[#dbe7f6] bg-[#f9fbff] p-4">
                      <div class="text-sm font-semibold text-[var(--text-strong)]">{selectedWorkflow()?.name}</div>
                      <div class="mt-2 grid gap-3 md:grid-cols-2">
                        <div class="text-sm text-[var(--text-muted)]">Trigger type: <span class="font-semibold text-[var(--text-strong)]">{selectedWorkflow()?.trigger_type}</span></div>
                        <div class="text-sm text-[var(--text-muted)]">Next run: <span class="font-semibold text-[var(--text-strong)]">{formatTimestamp(selectedWorkflow()?.next_run_at)}</span></div>
                        <div class="text-sm text-[var(--text-muted)]">Last triggered: <span class="font-semibold text-[var(--text-strong)]">{formatTimestamp(selectedWorkflow()?.last_triggered_at)}</span></div>
                        <div class="text-sm text-[var(--text-muted)]">Steps: <span class="font-semibold text-[var(--text-strong)]">{selectedWorkflow()?.steps.length}</span></div>
                      </div>
                    </div>
                  {/if}

                  <div class="rounded-[16px] border border-[var(--border-default)] bg-white p-4">
                    <div class="text-sm font-semibold text-[var(--text-strong)]">Recent workflow runs</div>
                    <div class="mt-3 space-y-3">
                      {#if workflowRuns.length === 0}
                        <div class="text-sm text-[var(--text-muted)]">No runs yet for this workflow.</div>
                      {:else}
                        {#each workflowRuns.slice(0, 8) as run (run.id)}
                          <div class="rounded-[14px] border border-[var(--border-subtle)] px-4 py-3">
                            <div class="flex items-center justify-between gap-3">
                              <div class="text-sm font-semibold text-[var(--text-strong)]">{run.status}</div>
                              <div class="text-xs text-[var(--text-soft)]">{formatTimestamp(run.started_at)}</div>
                            </div>
                            <div class="mt-2 text-sm text-[var(--text-muted)]">Trigger {run.trigger_type} · step {run.current_step_id || 'completed'}</div>
                          </div>
                        {/each}
                      {/if}
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>

          <div class="space-y-4">
            <div class="rounded-[18px] border border-[var(--border-default)] bg-white p-4">
              <div class="text-sm font-semibold text-[var(--text-strong)]">Approval queue</div>
              <div class="mt-1 text-sm text-[var(--text-muted)]">Human review steps generated by workflow proposals and ontology actions.</div>
              <div class="mt-4 space-y-3">
                {#if approvals.length === 0}
                  <div class="rounded-[14px] border border-dashed border-[var(--border-default)] px-4 py-8 text-center text-sm text-[var(--text-muted)]">No pending approvals for the selected workflow scope.</div>
                {:else}
                  {#each approvals.slice(0, 8) as approval (approval.id)}
                    <div class="rounded-[14px] border border-[var(--border-subtle)] px-4 py-3">
                      <div class="flex items-start justify-between gap-3">
                        <div>
                          <div class="text-sm font-semibold text-[var(--text-strong)]">{approval.title}</div>
                          <div class="mt-1 text-sm text-[var(--text-muted)]">{approval.instructions}</div>
                        </div>
                        <span class="rounded-full bg-[#fff4e5] px-2 py-1 text-[11px] font-semibold text-[#9a6c2f]">{approval.status}</span>
                      </div>
                      <div class="mt-2 text-xs text-[var(--text-soft)]">Requested {formatTimestamp(approval.requested_at)}</div>
                    </div>
                  {/each}
                {/if}
              </div>
            </div>
          </div>
        </div>
      </section>

      <section id="time-series" class="of-panel overflow-hidden">
        <div class="border-b border-[var(--border-subtle)] bg-[#fbfcfe] px-5 py-4">
          <div class="of-heading-sm">Time series and operations</div>
          <div class="mt-1 text-sm text-[var(--text-muted)]">Schedule density, workflow activity, queue pressure, and capability load across the current rules horizon.</div>
        </div>

        <div class="space-y-6 px-5 py-5">
          <div class="grid gap-4 xl:grid-cols-2">
            <div class="rounded-[18px] border border-[var(--border-default)] bg-white p-4">
              <div class="text-sm font-semibold text-[var(--text-strong)]">Scheduled rule outputs</div>
              <div class="mt-4 grid grid-cols-7 gap-3">
                {#each selectedScheduleBuckets as bucket (bucket.label)}
                  <div class="flex flex-col items-center gap-2">
                    <div class="flex h-32 w-full items-end rounded-[14px] bg-[#f6f8fb] px-2 pb-2">
                      <div
                        class="w-full rounded-[10px] bg-[linear-gradient(180deg,#78a6ee_0%,#3d6dd8_100%)]"
                        style={`height:${Math.max(10, Math.round((bucket.value / maxBucketValue(selectedScheduleBuckets)) * 100))}%`}
                      ></div>
                    </div>
                    <div class="text-center text-[11px] text-[var(--text-soft)]">{bucket.label}</div>
                    <div class="text-sm font-semibold text-[var(--text-strong)]">{bucket.value}</div>
                  </div>
                {/each}
              </div>
            </div>

            <div class="rounded-[18px] border border-[var(--border-default)] bg-white p-4">
              <div class="text-sm font-semibold text-[var(--text-strong)]">Workflow run cadence</div>
              <div class="mt-4 grid grid-cols-7 gap-3">
                {#each selectedRunBuckets as bucket (bucket.label)}
                  <div class="flex flex-col items-center gap-2">
                    <div class="flex h-32 w-full items-end rounded-[14px] bg-[#f7faf7] px-2 pb-2">
                      <div
                        class="w-full rounded-[10px] bg-[linear-gradient(180deg,#71c19c_0%,#2d8a67_100%)]"
                        style={`height:${Math.max(10, Math.round((bucket.value / maxBucketValue(selectedRunBuckets)) * 100))}%`}
                      ></div>
                    </div>
                    <div class="text-center text-[11px] text-[var(--text-soft)]">{bucket.label}</div>
                    <div class="text-sm font-semibold text-[var(--text-strong)]">{bucket.value}</div>
                  </div>
                {/each}
              </div>
            </div>
          </div>

          <div class="grid gap-4 xl:grid-cols-[minmax(0,1fr)_360px]">
            <div class="rounded-[18px] border border-[var(--border-default)] bg-white p-4">
              <div class="mb-4 flex items-center justify-between gap-3">
                <div>
                  <div class="text-sm font-semibold text-[var(--text-strong)]">Machinery queue</div>
                  <div class="mt-1 text-sm text-[var(--text-muted)]">Operational schedule items emitted by rule outputs and prioritized by queue recommendation.</div>
                </div>
                <a href="/dynamic-scheduling" class="of-link text-sm">Open Dynamic Scheduling</a>
              </div>
              <div class="space-y-3">
                {#if (queue?.data ?? []).length === 0}
                  <div class="rounded-[14px] border border-dashed border-[var(--border-default)] px-4 py-8 text-center text-sm text-[var(--text-muted)]">No scheduled queue items for the current object type.</div>
                {:else}
                  {#each (queue?.data ?? []).slice(0, 8) as item (item.id)}
                    <div class="rounded-[14px] border border-[var(--border-subtle)] px-4 py-3">
                      <div class="flex flex-wrap items-start justify-between gap-3">
                        <div>
                          <div class="text-sm font-semibold text-[var(--text-strong)]">{item.rule_display_name}</div>
                          <div class="mt-1 text-sm text-[var(--text-muted)]">Object {item.object_id} · {formatTimestamp(item.scheduled_for)}</div>
                        </div>
                        <span class={`rounded-full px-2 py-1 text-[11px] font-semibold ${item.status === 'pending' ? 'bg-[#fff4e5] text-[#9a6c2f]' : item.status === 'in_progress' ? 'bg-[#edf3ff] text-[#335ea8]' : 'bg-[#eef5e8] text-[#356b3c]'}`}>
                          {item.status}
                        </span>
                      </div>
                      <div class="mt-2 flex flex-wrap gap-2 text-xs text-[var(--text-soft)]">
                        <span>Priority {item.priority_score}</span>
                        <span>{formatCompactDuration(item.estimated_duration_minutes)}</span>
                        <span>{titleizeCapability(item.required_capability)}</span>
                      </div>
                      <div class="mt-3 flex flex-wrap gap-2">
                        <button type="button" class="of-btn text-[13px]" onclick={() => void updateQueueItemStatus(item.id, 'in_progress')} disabled={queueBusy === item.id}>Start</button>
                        <button type="button" class="of-btn text-[13px]" onclick={() => void updateQueueItemStatus(item.id, 'completed')} disabled={queueBusy === item.id}>Complete</button>
                        <button type="button" class="of-btn text-[13px]" onclick={() => void updateQueueItemStatus(item.id, 'pending')} disabled={queueBusy === item.id}>Reset</button>
                        <button type="button" class="of-btn text-[13px]" onclick={() => void updateQueueItemStatus(item.id, 'cancelled')} disabled={queueBusy === item.id}>Cancel</button>
                      </div>
                    </div>
                  {/each}
                {/if}
              </div>
            </div>

            <div class="rounded-[18px] border border-[var(--border-default)] bg-white p-4">
              <div class="text-sm font-semibold text-[var(--text-strong)]">Capability load</div>
              <div class="mt-1 text-sm text-[var(--text-muted)]">Time-series rules feed queue pressure that can be rebalanced by capability.</div>
              <div class="mt-4 space-y-3">
                {#if capabilityLoadRows().length === 0}
                  <div class="text-sm text-[var(--text-muted)]">No capability load available yet.</div>
                {:else}
                  {#each capabilityLoadRows() as item (item.capability)}
                    <div class="rounded-[14px] border border-[var(--border-subtle)] px-4 py-3">
                      <div class="flex items-center justify-between gap-3">
                        <div class="text-sm font-semibold text-[var(--text-strong)]">{titleizeCapability(item.capability)}</div>
                        <div class="text-xs text-[var(--text-soft)]">{item.pending_count} items</div>
                      </div>
                      <div class="mt-2 h-2 rounded-full bg-[#eef2f7]">
                        <div
                          class="h-2 rounded-full bg-[linear-gradient(90deg,#6ea7f7_0%,#335ea8_100%)]"
                          style={`width:${Math.min(100, Math.max(8, item.pending_count * 16))}%`}
                        ></div>
                      </div>
                      <div class="mt-2 text-sm text-[var(--text-muted)]">{formatCompactDuration(item.total_estimated_minutes)}</div>
                    </div>
                  {/each}
                {/if}
              </div>
            </div>
          </div>
        </div>
      </section>

      <section id="customization" class="of-panel overflow-hidden">
        <div class="border-b border-[var(--border-subtle)] bg-[#fbfcfe] px-5 py-4">
          <div class="of-heading-sm">Settings & customization</div>
          <div class="mt-1 text-sm text-[var(--text-muted)]">Advanced controls for self-managed transforms, optional features, custom properties, and generated pipeline code.</div>
        </div>

        <div class="grid gap-6 px-5 py-5 xl:grid-cols-[minmax(0,1fr)_420px]">
          <div class="space-y-4">
            <div class="rounded-[18px] border border-[#ead8a6] bg-[#fffaf0] p-4">
              <div class="flex items-start gap-3">
                <span class="mt-0.5 inline-flex h-8 w-8 items-center justify-center rounded-full bg-[#ffe7a8] text-[#9a6c2f]">
                  <Glyph name="settings" size={16} />
                </span>
                <div>
                  <div class="text-sm font-semibold text-[var(--text-strong)]">Advanced feature</div>
                  <div class="mt-1 text-sm leading-6 text-[var(--text-muted)]">Self-managed transforms and custom pipelines unlock granular control, but they also increase implementation and maintenance burden.</div>
                </div>
              </div>
            </div>

            <div class="grid gap-4 md:grid-cols-2">
              <label class="rounded-[16px] border border-[var(--border-default)] bg-white px-4 py-4">
                <div class="flex items-center justify-between gap-3">
                  <div>
                    <div class="text-sm font-semibold text-[var(--text-strong)]">Enable self-managed transforms</div>
                    <div class="mt-1 text-sm text-[var(--text-muted)]">Generate and own the transform code path.</div>
                  </div>
                  <input type="checkbox" bind:checked={enableSelfManagedTransforms} class="h-4 w-4" />
                </div>
              </label>

              <label class="rounded-[16px] border border-[var(--border-default)] bg-white px-4 py-4">
                <div class="flex items-center justify-between gap-3">
                  <div>
                    <div class="text-sm font-semibold text-[var(--text-strong)]">Enable streaming configuration</div>
                    <div class="mt-1 text-sm text-[var(--text-muted)]">Keep the workflow ready for streaming constraints.</div>
                  </div>
                  <input type="checkbox" bind:checked={enableStreamingConfiguration} class="h-4 w-4" />
                </div>
              </label>

              <label class="rounded-[16px] border border-[var(--border-default)] bg-white px-4 py-4">
                <div class="flex items-center justify-between gap-3">
                  <div>
                    <div class="text-sm font-semibold text-[var(--text-strong)]">Rule Actions</div>
                    <div class="mt-1 text-sm text-[var(--text-muted)]">Expose action execution from rule outcomes.</div>
                  </div>
                  <input type="checkbox" bind:checked={enableRuleActions} class="h-4 w-4" />
                </div>
              </label>

              <label class="rounded-[16px] border border-[var(--border-default)] bg-white px-4 py-4">
                <div class="flex items-center justify-between gap-3">
                  <div>
                    <div class="text-sm font-semibold text-[var(--text-strong)]">Time-series outputs</div>
                    <div class="mt-1 text-sm text-[var(--text-muted)]">Publish schedules and temporal rule outputs.</div>
                  </div>
                  <input type="checkbox" bind:checked={enableTimeSeriesOutputs} class="h-4 w-4" />
                </div>
              </label>
            </div>

            <div class="grid gap-4 md:grid-cols-[minmax(0,1fr)_220px]">
              <div>
                <label for="repository" class="mb-2 block text-sm font-medium text-[var(--text-strong)]">Transform repository</label>
                <input
                  id="repository"
                  bind:value={selectedRepository}
                  placeholder={recommendedRepository}
                  class="w-full rounded-xl border border-[var(--border-default)] bg-white px-3 py-2 text-sm"
                />
              </div>
              <div>
                <label for="default-severity" class="mb-2 block text-sm font-medium text-[var(--text-strong)]">Default output severity</label>
                <select id="default-severity" bind:value={defaultSeverity} class="w-full rounded-xl border border-[var(--border-default)] bg-white px-3 py-2 text-sm">
                  <option value="low">low</option>
                  <option value="medium">medium</option>
                  <option value="high">high</option>
                  <option value="critical">critical</option>
                </select>
              </div>
            </div>

            <div class="rounded-[18px] border border-[var(--border-default)] bg-white p-4">
              <div class="text-sm font-semibold text-[var(--text-strong)]">Custom properties</div>
              <div class="mt-1 text-sm text-[var(--text-muted)]">Add output columns or metadata fields to the generated rules pipeline contract.</div>
              <div class="mt-4 flex flex-wrap gap-3">
                <input
                  bind:value={customPropertyName}
                  placeholder="review_bucket"
                  class="min-w-[220px] flex-1 rounded-xl border border-[var(--border-default)] bg-white px-3 py-2 text-sm"
                />
                <button type="button" class="of-btn" onclick={addCustomProperty}>
                  <Glyph name="plus" size={15} />
                  <span>Add property</span>
                </button>
              </div>
              <div class="mt-4 flex flex-wrap gap-2">
                {#each customProperties as propertyName (propertyName)}
                  <button type="button" class="rounded-full border border-[#d8e3f4] bg-[#f8fbff] px-3 py-1.5 text-xs font-medium text-[#335ea8]" onclick={() => removeCustomProperty(propertyName)}>
                    {propertyName}
                  </button>
                {/each}
                {#if customProperties.length === 0}
                  <div class="text-sm text-[var(--text-muted)]">No custom properties added yet.</div>
                {/if}
              </div>
            </div>
          </div>

          <div class="rounded-[18px] border border-[var(--border-default)] bg-white p-4">
            <div class="flex items-center justify-between gap-3">
              <div>
                <div class="text-sm font-semibold text-[var(--text-strong)]">Generated pipeline code</div>
                <div class="mt-1 text-sm text-[var(--text-muted)]">A self-managed transform scaffold based on the current rule contract and customization settings.</div>
              </div>
              <span class="rounded-full bg-[#eef5e8] px-3 py-1 text-xs font-semibold text-[#356b3c]">
                {enableSelfManagedTransforms ? 'self-managed' : 'managed'}
              </span>
            </div>
            <pre class="mt-4 overflow-x-auto rounded-[16px] bg-[#0f172a] px-4 py-4 text-xs leading-6 text-slate-100">{generatedPipelineCode()}</pre>
          </div>
        </div>
      </section>
    </main>

    <aside class="space-y-4 xl:sticky xl:top-5 xl:self-start">
      <section class="of-panel overflow-hidden">
        <div class="border-b border-[var(--border-subtle)] bg-[#f7fafc] px-4 py-3">
          <div class="text-[11px] font-semibold uppercase tracking-[0.18em] text-[var(--text-soft)]">Selected rule</div>
        </div>
        <div class="space-y-4 px-4 py-4">
          {#if selectedRule()}
            <div>
              <div class="text-lg font-semibold text-[var(--text-strong)]">{selectedRule()?.display_name}</div>
              <div class="mt-1 text-sm text-[var(--text-muted)]">{selectedRule()?.description || 'No description for this rule yet.'}</div>
            </div>

            <div class="rounded-[16px] border border-[#e7edf5] bg-[#fbfcfe] px-4 py-4">
              <div class="text-xs font-semibold uppercase tracking-[0.12em] text-[var(--text-soft)]">Rule conditions</div>
              <div class="mt-3 flex flex-wrap gap-2">
                {#each selectedRuleConditions as condition (condition)}
                  <span class="rounded-full bg-[#f4f8ff] px-3 py-1.5 text-xs font-medium text-[#335ea8]">{condition}</span>
                {/each}
                {#if selectedRuleConditions.length === 0}
                  <span class="text-sm text-[var(--text-muted)]">No conditions declared.</span>
                {/if}
              </div>
            </div>

            <div class="rounded-[16px] border border-[#e7edf5] bg-[#fbfcfe] px-4 py-4">
              <div class="text-xs font-semibold uppercase tracking-[0.12em] text-[var(--text-soft)]">Output contract</div>
              <div class="mt-3 space-y-3">
                {#each selectedRuleOutputs as item (item.label + item.value)}
                  <div>
                    <div class="text-xs font-semibold uppercase tracking-[0.08em] text-[var(--text-soft)]">{item.label}</div>
                    <div class="mt-1 text-sm text-[var(--text-strong)]">{item.value}</div>
                  </div>
                {/each}
                {#if selectedRuleOutputs.length === 0}
                  <div class="text-sm text-[var(--text-muted)]">No output effects declared.</div>
                {/if}
              </div>
            </div>

            {#if selectedInsight()}
              <div class="rounded-[16px] border border-[#d8eadf] bg-[#f4fbf7] px-4 py-4">
                <div class="text-xs font-semibold uppercase tracking-[0.12em] text-[var(--text-soft)]">Operational insight</div>
                <div class="mt-3 grid gap-3">
                  <div class="text-sm text-[var(--text-muted)]">Dynamic pressure: <span class="font-semibold text-[var(--text-strong)]">{selectedInsight()?.dynamic_pressure}</span></div>
                  <div class="text-sm text-[var(--text-muted)]">Matched runs: <span class="font-semibold text-[var(--text-strong)]">{selectedInsight()?.matched_runs} / {selectedInsight()?.total_runs}</span></div>
                  <div class="text-sm text-[var(--text-muted)]">Pending schedules: <span class="font-semibold text-[var(--text-strong)]">{selectedInsight()?.pending_schedules}</span></div>
                  <div class="text-sm text-[var(--text-muted)]">Average lead: <span class="font-semibold text-[var(--text-strong)]">{selectedInsight()?.avg_schedule_lead_hours?.toFixed(1) ?? 'n/a'}h</span></div>
                </div>
              </div>
            {/if}
          {:else}
            <div class="text-sm text-[var(--text-muted)]">Select a rule to inspect its contract and operational footprint.</div>
          {/if}
        </div>
      </section>
    </aside>
  </div>
</div>
