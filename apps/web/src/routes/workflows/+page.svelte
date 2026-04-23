<script lang="ts">
  import { onMount } from 'svelte';

  import { listUsers, type UserProfile } from '$lib/api/auth';
  import {
    createWorkflow,
    decideWorkflowApproval,
    deleteWorkflow,
    evaluateCronWorkflows,
    getWorkflow,
    listWorkflowApprovals,
    listWorkflowRuns,
    listWorkflows,
    startWorkflowRun,
    triggerWorkflowEvent,
    updateWorkflow,
    type WorkflowApproval,
    type WorkflowBranch,
    type WorkflowDefinition,
    type WorkflowRun,
    type WorkflowStep,
  } from '$lib/api/workflows';
  import { notifications } from '$stores/notifications';

  type WorkflowDraft = {
    id?: string;
    name: string;
    description: string;
    status: string;
    trigger_type: string;
    trigger_config: Record<string, unknown>;
    steps: WorkflowStep[];
  };

  let workflows = $state<WorkflowDefinition[]>([]);
  let approvals = $state<WorkflowApproval[]>([]);
  let runs = $state<WorkflowRun[]>([]);
  let users = $state<UserProfile[]>([]);
  let search = $state('');
  let loading = $state(true);
  let saving = $state(false);
  let triggering = $state(false);
  let selectedWorkflowId = $state('');
  let eventPayloadText = $state('{\n  "source": "workflow-builder"\n}');
  let error = $state('');
  let stepConfigText = $state<Record<string, string>>({});

  let draft = $state<WorkflowDraft>(createEmptyWorkflow());

  function createStep(stepType = 'action'): WorkflowStep {
    return {
      id: crypto.randomUUID(),
      name: `New ${stepType}`,
      step_type: stepType,
      description: '',
      config: {},
      next_step_id: null,
      branches: [],
    };
  }

  function createEmptyWorkflow(): WorkflowDraft {
    return {
      name: 'New workflow',
      description: '',
      status: 'draft',
      trigger_type: 'manual',
      trigger_config: {},
      steps: [createStep('action')],
    };
  }

  function normalizeWorkflow(workflow: WorkflowDefinition): WorkflowDraft {
    return {
      id: workflow.id,
      name: workflow.name,
      description: workflow.description,
      status: workflow.status,
      trigger_type: workflow.trigger_type,
      trigger_config: workflow.trigger_config ?? {},
      steps: Array.isArray(workflow.steps) ? workflow.steps : [],
    };
  }

  function syncStepConfigText() {
    stepConfigText = Object.fromEntries(
      draft.steps.map((step) => [step.id, JSON.stringify(step.config ?? {}, null, 2)]),
    );
  }

  function selectedWorkflow(): WorkflowDefinition | undefined {
    return workflows.find((workflow) => workflow.id === selectedWorkflowId);
  }

  async function loadWorkflows() {
    const response = await listWorkflows({ search: search || undefined, per_page: 50 });
    workflows = response.data;
  }

  async function loadApprovals() {
    const response = await listWorkflowApprovals({ per_page: 50, status: 'pending', workflow_id: selectedWorkflowId || undefined });
    approvals = response.data;
  }

  async function loadRuns() {
    if (!selectedWorkflowId) {
      runs = [];
      return;
    }

    const response = await listWorkflowRuns(selectedWorkflowId, { per_page: 20 });
    runs = response.data;
  }

  async function selectWorkflow(id: string) {
    selectedWorkflowId = id;
    const workflow = await getWorkflow(id);
    draft = normalizeWorkflow(workflow);
    syncStepConfigText();
    await Promise.all([loadRuns(), loadApprovals()]);
  }

  async function load() {
    loading = true;
    error = '';
    try {
      const [allUsers] = await Promise.all([
        listUsers().catch(() => [] as UserProfile[]),
        loadWorkflows(),
      ]);
      users = allUsers;

      if (selectedWorkflowId) {
        await selectWorkflow(selectedWorkflowId);
      } else if (workflows.length > 0) {
        await selectWorkflow(workflows[0].id);
      } else {
        draft = createEmptyWorkflow();
        selectedWorkflowId = '';
        syncStepConfigText();
      }
    } catch (cause) {
      console.error('Failed to load workflows', cause);
      error = cause instanceof Error ? cause.message : 'Failed to load workflows';
    } finally {
      loading = false;
    }
  }

  function newWorkflow() {
    selectedWorkflowId = '';
    draft = createEmptyWorkflow();
    runs = [];
    approvals = [];
    syncStepConfigText();
  }

  async function saveWorkflow() {
    saving = true;
    error = '';
    try {
      const payload = {
        name: draft.name,
        description: draft.description,
        status: draft.status,
        trigger_type: draft.trigger_type,
        trigger_config: draft.trigger_config,
        steps: draft.steps,
      };

      const workflow = draft.id
        ? await updateWorkflow(draft.id, payload)
        : await createWorkflow(payload);

      notifications.success(`Workflow ${draft.id ? 'updated' : 'created'}`);
      await loadWorkflows();
      await selectWorkflow(workflow.id);
    } catch (cause) {
      error = cause instanceof Error ? cause.message : 'Failed to save workflow';
    } finally {
      saving = false;
    }
  }

  async function removeCurrentWorkflow() {
    if (!draft.id || !confirm('Delete this workflow?')) {
      return;
    }

    await deleteWorkflow(draft.id);
    notifications.success('Workflow deleted');
    newWorkflow();
    await loadWorkflows();
    if (workflows.length > 0) {
      await selectWorkflow(workflows[0].id);
    }
  }

  function addStep(stepType = 'action') {
    const step = createStep(stepType);
    draft.steps = [...draft.steps, step];
    stepConfigText = { ...stepConfigText, [step.id]: '{}' };
  }

  function duplicateStep(stepId: string) {
    const source = draft.steps.find((step) => step.id === stepId);
    if (!source) return;
    const copy = {
      ...structuredClone(source),
      id: crypto.randomUUID(),
      name: `${source.name} copy`,
    };
    draft.steps = [...draft.steps, copy];
    stepConfigText = { ...stepConfigText, [copy.id]: JSON.stringify(copy.config ?? {}, null, 2) };
  }

  function removeStep(stepId: string) {
    if (draft.steps.length <= 1) {
      return;
    }

    draft.steps = draft.steps.filter((step) => step.id !== stepId).map((step) => ({
      ...step,
      next_step_id: step.next_step_id === stepId ? null : step.next_step_id,
      branches: step.branches.filter((branch) => branch.next_step_id !== stepId),
    }));
    const nextConfigText = { ...stepConfigText };
    delete nextConfigText[stepId];
    stepConfigText = nextConfigText;
  }

  function addBranch(stepId: string) {
    const step = draft.steps.find((item) => item.id === stepId);
    if (!step) return;
    step.branches = [
      ...step.branches,
      {
        condition: { field: 'last_approval_decision.decision', operator: 'eq', value: 'approved' },
        next_step_id: '',
      },
    ];
  }

  function removeBranch(stepId: string, branchIndex: number) {
    const step = draft.steps.find((item) => item.id === stepId);
    if (!step) return;
    step.branches = step.branches.filter((_, index) => index !== branchIndex);
  }

  function updateStepConfig(stepId: string, raw: string) {
    stepConfigText = { ...stepConfigText, [stepId]: raw };
    const step = draft.steps.find((item) => item.id === stepId);
    if (!step) return;
    try {
      step.config = JSON.parse(raw) as Record<string, unknown>;
    } catch {
      // Keep showing the raw text until it becomes valid JSON.
    }
  }

  function ownerName(userId: string | null) {
    if (!userId) return 'Unassigned';
    return users.find((user) => user.id === userId)?.name ?? userId.slice(0, 8);
  }

  function stepName(stepId: string | null) {
    return draft.steps.find((step) => step.id === stepId)?.name ?? 'End';
  }

  function updateTriggerField(key: string, value: string) {
    draft.trigger_config = {
      ...draft.trigger_config,
      [key]: value,
    };
  }

  async function runManual() {
    if (!draft.id) return;
    triggering = true;
    try {
      await startWorkflowRun(draft.id, { initiated_from: 'workflow-builder' });
      notifications.success('Manual run started');
      await Promise.all([loadRuns(), loadApprovals()]);
    } finally {
      triggering = false;
    }
  }

  async function fireEvent() {
    const eventName = typeof draft.trigger_config['event_name'] === 'string' ? String(draft.trigger_config['event_name']) : '';
    if (!eventName) {
      error = 'Set an event name before firing an event trigger';
      return;
    }

    triggering = true;
    error = '';
    try {
      const payload = JSON.parse(eventPayloadText) as Record<string, unknown>;
      await triggerWorkflowEvent(eventName, payload);
      notifications.success('Event trigger dispatched');
      await Promise.all([loadRuns(), loadApprovals(), loadWorkflows()]);
    } catch (cause) {
      error = cause instanceof Error ? cause.message : 'Failed to trigger workflow event';
    } finally {
      triggering = false;
    }
  }

  async function runCronEvaluation() {
    triggering = true;
    try {
      const response = await evaluateCronWorkflows();
      notifications.info(`Triggered ${response.triggered_runs} due cron workflow(s)`);
      await Promise.all([loadRuns(), loadApprovals(), loadWorkflows()]);
    } finally {
      triggering = false;
    }
  }

  async function decide(approval: WorkflowApproval, decision: 'approved' | 'rejected') {
    await decideWorkflowApproval(approval.id, { decision, payload: {} });
    notifications.success(`Approval ${decision}`);
    await Promise.all([loadApprovals(), loadRuns()]);
  }

  onMount(() => {
    syncStepConfigText();
    void load();
  });
</script>

<div class="space-y-6">
  <div class="flex items-center justify-between">
    <div>
      <h1 class="text-2xl font-bold">Workflows & Notifications</h1>
      <p class="mt-1 text-sm text-gray-500">Build trigger-driven workflows, route them through approval queues, and pair them with notification delivery.</p>
    </div>
    <div class="flex gap-2">
      <button onclick={newWorkflow} class="rounded-xl border border-slate-200 px-4 py-2 hover:bg-slate-50 dark:border-gray-700 dark:hover:bg-gray-800">New workflow</button>
      <button onclick={saveWorkflow} disabled={saving} class="rounded-xl bg-blue-600 px-4 py-2 text-white disabled:opacity-50 hover:bg-blue-700">
        {saving ? 'Saving...' : draft.id ? 'Save changes' : 'Create workflow'}
      </button>
    </div>
  </div>

  {#if error}
    <div class="rounded-xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-700 dark:border-rose-900/40 dark:bg-rose-950/40 dark:text-rose-300">{error}</div>
  {/if}

  <div class="grid gap-6 xl:grid-cols-[0.92fr,1.08fr]">
    <section class="space-y-4 rounded-2xl border border-slate-200 bg-white p-5 shadow-sm dark:border-gray-700 dark:bg-gray-900">
      <div class="flex items-center justify-between">
        <div>
          <div class="text-xs uppercase tracking-[0.22em] text-gray-400">Workflow registry</div>
          <div class="mt-1 text-sm text-gray-500">Definitions, trigger modes, and approval queue entry points.</div>
        </div>
        <div class="rounded-full bg-slate-100 px-3 py-1 text-xs font-medium text-slate-700 dark:bg-gray-800 dark:text-gray-200">
          {workflows.length} workflows
        </div>
      </div>

      <input bind:value={search} placeholder="Search workflows..." oninput={() => void loadWorkflows()} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-gray-700 dark:bg-gray-800" />

      {#if loading}
        <div class="py-12 text-center text-gray-500">Loading workflows...</div>
      {:else if workflows.length === 0}
        <div class="rounded-xl border border-dashed border-slate-300 px-4 py-10 text-center text-sm text-gray-500 dark:border-gray-700">
          No workflows yet. Create the first workflow from the builder.
        </div>
      {:else}
        <div class="space-y-3">
          {#each workflows as workflow (workflow.id)}
            <button
              type="button"
              onclick={() => void selectWorkflow(workflow.id)}
              class={`w-full rounded-xl border p-4 text-left transition-colors ${selectedWorkflowId === workflow.id ? 'border-blue-500 bg-blue-50/70 dark:border-blue-400 dark:bg-blue-950/30' : 'border-slate-200 hover:bg-slate-50 dark:border-gray-700 dark:hover:bg-gray-800'}`}
            >
              <div class="flex items-center justify-between gap-3">
                <div>
                  <div class="font-medium">{workflow.name}</div>
                  <div class="mt-1 text-sm text-gray-500">{workflow.description || 'No description'}</div>
                </div>
                <span class="rounded-full bg-slate-100 px-2.5 py-1 text-xs font-medium text-slate-700 dark:bg-gray-800 dark:text-gray-300">
                  {workflow.trigger_type}
                </span>
              </div>
              <div class="mt-3 flex flex-wrap gap-2 text-xs text-gray-500">
                <span>Status {workflow.status}</span>
                <span>{Array.isArray(workflow.steps) ? workflow.steps.length : 0} steps</span>
                <span>Owner {ownerName(workflow.owner_id)}</span>
              </div>
            </button>
          {/each}
        </div>
      {/if}

      <div class="rounded-xl border border-slate-200 p-4 dark:border-gray-700">
        <div class="flex items-center justify-between">
          <div>
            <div class="font-medium">Pending approvals</div>
            <div class="text-sm text-gray-500">Human-in-the-loop queue for the selected workflow.</div>
          </div>
          <span class="rounded-full bg-amber-100 px-2.5 py-1 text-xs font-medium text-amber-700 dark:bg-amber-900/30 dark:text-amber-300">{approvals.length}</span>
        </div>

        <div class="mt-4 space-y-3">
          {#each approvals as approval (approval.id)}
            <div class="rounded-xl border border-slate-200 p-3 dark:border-gray-700">
              <div class="flex items-start justify-between gap-3">
                <div>
                  <div class="font-medium">{approval.title}</div>
                  <div class="mt-1 text-sm text-gray-500">{approval.instructions || 'No instructions'}</div>
                  <div class="mt-2 text-xs text-gray-500">Assigned to {ownerName(approval.assigned_to)}</div>
                </div>
                <div class="flex gap-2">
                  <button onclick={() => void decide(approval, 'approved')} class="rounded-lg bg-emerald-600 px-3 py-1.5 text-xs font-medium text-white hover:bg-emerald-700">Approve</button>
                  <button onclick={() => void decide(approval, 'rejected')} class="rounded-lg bg-rose-600 px-3 py-1.5 text-xs font-medium text-white hover:bg-rose-700">Reject</button>
                </div>
              </div>
            </div>
          {/each}
          {#if approvals.length === 0}
            <div class="text-sm text-gray-500">No pending approvals.</div>
          {/if}
        </div>
      </div>
    </section>

    <section class="space-y-6">
      <div class="rounded-2xl border border-slate-200 bg-white p-5 shadow-sm dark:border-gray-700 dark:bg-gray-900">
        <div class="grid gap-4 md:grid-cols-2">
          <div>
            <label for="workflow-name" class="mb-1 block text-sm font-medium">Workflow name</label>
            <input id="workflow-name" bind:value={draft.name} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-gray-700 dark:bg-gray-800" />
          </div>
          <div>
            <label for="workflow-status" class="mb-1 block text-sm font-medium">Status</label>
            <select id="workflow-status" bind:value={draft.status} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-gray-700 dark:bg-gray-800">
              <option value="draft">Draft</option>
              <option value="active">Active</option>
              <option value="paused">Paused</option>
            </select>
          </div>
        </div>

        <div class="mt-4">
          <label for="workflow-description" class="mb-1 block text-sm font-medium">Description</label>
          <textarea id="workflow-description" bind:value={draft.description} rows="3" class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-gray-700 dark:bg-gray-800"></textarea>
        </div>

        <div class="mt-4 grid gap-4 md:grid-cols-2">
          <div>
            <label for="workflow-trigger-type" class="mb-1 block text-sm font-medium">Trigger type</label>
            <select id="workflow-trigger-type" bind:value={draft.trigger_type} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-gray-700 dark:bg-gray-800">
              <option value="manual">Manual</option>
              <option value="cron">Cron</option>
              <option value="event">Event-driven</option>
              <option value="webhook">Webhook</option>
            </select>
          </div>
          <div>
            <label for="workflow-trigger-config" class="mb-1 block text-sm font-medium">Trigger config</label>
            {#if draft.trigger_type === 'cron'}
              <input id="workflow-trigger-config" value={String(draft.trigger_config['cron'] ?? '')} oninput={(event) => updateTriggerField('cron', (event.currentTarget as HTMLInputElement).value)} placeholder="*/15 * * * * *" class="w-full rounded-xl border border-slate-200 px-3 py-2 font-mono text-sm dark:border-gray-700 dark:bg-gray-800" />
            {:else if draft.trigger_type === 'event'}
              <input id="workflow-trigger-config" value={String(draft.trigger_config['event_name'] ?? '')} oninput={(event) => updateTriggerField('event_name', (event.currentTarget as HTMLInputElement).value)} placeholder="dataset.quality.degraded" class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-gray-700 dark:bg-gray-800" />
            {:else if draft.trigger_type === 'webhook'}
              <input id="workflow-trigger-config" value={String(draft.trigger_config['secret'] ?? '')} oninput={(event) => updateTriggerField('secret', (event.currentTarget as HTMLInputElement).value)} placeholder="Shared secret" class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-gray-700 dark:bg-gray-800" />
            {:else}
              <div id="workflow-trigger-config" class="rounded-xl border border-dashed border-slate-300 px-3 py-2 text-sm text-gray-500 dark:border-gray-700">Manual trigger uses the run button below.</div>
            {/if}
          </div>
        </div>

        <div class="mt-4 flex flex-wrap gap-2">
          <button onclick={() => void runManual()} disabled={!draft.id || triggering} class="rounded-xl bg-slate-900 px-4 py-2 text-white disabled:opacity-50 dark:bg-white dark:text-slate-900">
            {triggering ? 'Running...' : 'Run manually'}
          </button>
          <button onclick={() => void fireEvent()} disabled={triggering || draft.trigger_type !== 'event'} class="rounded-xl border border-slate-200 px-4 py-2 disabled:opacity-50 hover:bg-slate-50 dark:border-gray-700 dark:hover:bg-gray-800">
            Fire event trigger
          </button>
          <button onclick={() => void runCronEvaluation()} disabled={triggering || draft.trigger_type !== 'cron'} class="rounded-xl border border-slate-200 px-4 py-2 disabled:opacity-50 hover:bg-slate-50 dark:border-gray-700 dark:hover:bg-gray-800">
            Run due cron workflows
          </button>
          {#if draft.id}
            <button onclick={() => void removeCurrentWorkflow()} class="rounded-xl border border-rose-200 px-4 py-2 text-rose-600 hover:bg-rose-50 dark:border-rose-900/40 dark:hover:bg-rose-950/30">
              Delete workflow
            </button>
          {/if}
        </div>

        {#if draft.trigger_type === 'event'}
          <div class="mt-4">
            <label for="workflow-event-payload" class="mb-1 block text-sm font-medium">Event payload</label>
            <textarea id="workflow-event-payload" bind:value={eventPayloadText} rows="5" class="w-full rounded-xl border border-slate-200 px-3 py-2 font-mono text-sm dark:border-gray-700 dark:bg-gray-800"></textarea>
          </div>
        {/if}
      </div>

      <div class="rounded-2xl border border-slate-200 bg-white p-5 shadow-sm dark:border-gray-700 dark:bg-gray-900">
        <div class="flex items-center justify-between">
          <div>
            <div class="text-xs uppercase tracking-[0.22em] text-gray-400">Workflow builder</div>
            <div class="mt-1 text-sm text-gray-500">A visual step lane with config JSON, branching, approval routing, and notifications.</div>
          </div>
          <div class="flex gap-2">
            <button onclick={() => addStep('action')} class="rounded-xl border border-slate-200 px-3 py-2 text-sm hover:bg-slate-50 dark:border-gray-700 dark:hover:bg-gray-800">Add action</button>
            <button onclick={() => addStep('approval')} class="rounded-xl border border-slate-200 px-3 py-2 text-sm hover:bg-slate-50 dark:border-gray-700 dark:hover:bg-gray-800">Add approval</button>
            <button onclick={() => addStep('notification')} class="rounded-xl border border-slate-200 px-3 py-2 text-sm hover:bg-slate-50 dark:border-gray-700 dark:hover:bg-gray-800">Add notification</button>
          </div>
        </div>

        <div class="mt-5 overflow-x-auto pb-2">
          <div class="flex min-w-max gap-4">
            {#each draft.steps as step, index (step.id)}
              <div class="w-[22rem] rounded-2xl border border-slate-200 bg-slate-50/70 p-4 shadow-sm dark:border-gray-700 dark:bg-gray-950/40">
                <div class="flex items-center justify-between">
                  <span class="rounded-full bg-white px-2.5 py-1 text-xs font-semibold uppercase tracking-[0.16em] text-slate-600 dark:bg-gray-900 dark:text-gray-300">{step.step_type}</span>
                  <div class="flex gap-2 text-xs">
                    <button onclick={() => duplicateStep(step.id)} class="text-blue-600 hover:underline">Duplicate</button>
                    <button onclick={() => removeStep(step.id)} class="text-rose-600 hover:underline">Remove</button>
                  </div>
                </div>

                <div class="mt-4 space-y-3">
                  <input bind:value={step.name} placeholder="Step name" class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-gray-700 dark:bg-gray-900" />
                  <textarea bind:value={step.description} rows="2" placeholder="Step description" class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-gray-700 dark:bg-gray-900"></textarea>
                  <select bind:value={step.next_step_id} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-gray-700 dark:bg-gray-900">
                    <option value={null}>End workflow</option>
                    {#each draft.steps.filter((candidate) => candidate.id !== step.id) as option (option.id)}
                      <option value={option.id}>{option.name}</option>
                    {/each}
                  </select>
                  <textarea
                    rows="8"
                    value={stepConfigText[step.id] ?? '{}'}
                    oninput={(event) => updateStepConfig(step.id, (event.currentTarget as HTMLTextAreaElement).value)}
                    class="w-full rounded-xl border border-slate-200 px-3 py-2 font-mono text-xs dark:border-gray-700 dark:bg-gray-900"
                  ></textarea>
                </div>

                <div class="mt-4 rounded-xl border border-slate-200 p-3 dark:border-gray-700">
                  <div class="flex items-center justify-between">
                    <div class="text-sm font-medium">Branches</div>
                    <button onclick={() => addBranch(step.id)} class="text-xs text-blue-600 hover:underline">Add branch</button>
                  </div>
                  <div class="mt-3 space-y-3">
                    {#each step.branches as branch, branchIndex}
                      <div class="rounded-xl border border-slate-200 p-3 dark:border-gray-700">
                        <div class="grid gap-3 md:grid-cols-2">
                          <input bind:value={branch.condition.field} placeholder="Context path" class="rounded-lg border border-slate-200 px-3 py-2 text-xs dark:border-gray-700 dark:bg-gray-900" />
                          <select bind:value={branch.condition.operator} class="rounded-lg border border-slate-200 px-3 py-2 text-xs dark:border-gray-700 dark:bg-gray-900">
                            <option value="eq">equals</option>
                            <option value="ne">not equal</option>
                            <option value="contains">contains</option>
                            <option value="gt">&gt;</option>
                            <option value="gte">&gt;=</option>
                            <option value="lt">&lt;</option>
                            <option value="lte">&lt;=</option>
                          </select>
                          <input
                            value={typeof branch.condition.value === 'string' ? branch.condition.value : JSON.stringify(branch.condition.value)}
                            oninput={(event) => branch.condition.value = (event.currentTarget as HTMLInputElement).value}
                            placeholder="Match value"
                            class="rounded-lg border border-slate-200 px-3 py-2 text-xs dark:border-gray-700 dark:bg-gray-900"
                          />
                          <select bind:value={branch.next_step_id} class="rounded-lg border border-slate-200 px-3 py-2 text-xs dark:border-gray-700 dark:bg-gray-900">
                            <option value="">Choose target step</option>
                            {#each draft.steps.filter((candidate) => candidate.id !== step.id) as option (option.id)}
                              <option value={option.id}>{option.name}</option>
                            {/each}
                          </select>
                        </div>
                        <div class="mt-2 flex justify-end">
                          <button onclick={() => removeBranch(step.id, branchIndex)} class="text-xs text-rose-600 hover:underline">Remove branch</button>
                        </div>
                      </div>
                    {/each}
                    {#if step.branches.length === 0}
                      <div class="text-xs text-gray-500">No conditional branches on this step.</div>
                    {/if}
                  </div>
                </div>

                <div class="mt-4 text-xs text-gray-500">
                  <span class="font-medium">Next:</span> {stepName(step.next_step_id)}
                </div>

                {#if index < draft.steps.length - 1}
                  <div class="mt-4 text-center text-xs text-gray-400">↓ then</div>
                {/if}
              </div>
            {/each}
          </div>
        </div>
      </div>

      <div class="rounded-2xl border border-slate-200 bg-white p-5 shadow-sm dark:border-gray-700 dark:bg-gray-900">
        <div class="flex items-center justify-between">
          <div>
            <div class="text-xs uppercase tracking-[0.22em] text-gray-400">Run history</div>
            <div class="mt-1 text-sm text-gray-500">Latest executions, trigger source, and workflow state transitions.</div>
          </div>
          <span class="rounded-full bg-slate-100 px-3 py-1 text-xs font-medium text-slate-700 dark:bg-gray-800 dark:text-gray-200">{runs.length} recent runs</span>
        </div>

        <div class="mt-4 space-y-3">
          {#each runs as run (run.id)}
            <div class="rounded-xl border border-slate-200 p-4 dark:border-gray-700">
              <div class="flex flex-wrap items-center justify-between gap-2">
                <div class="font-medium">{run.trigger_type} trigger</div>
                <span class="rounded-full bg-slate-100 px-2.5 py-1 text-xs font-medium text-slate-700 dark:bg-gray-800 dark:text-gray-300">{run.status}</span>
              </div>
              <div class="mt-2 text-sm text-gray-500">Started {new Date(run.started_at).toLocaleString()}</div>
              <div class="mt-1 text-xs text-gray-500">Current step: {stepName(run.current_step_id)}</div>
              {#if run.error_message}
                <div class="mt-2 rounded-lg border border-rose-200 bg-rose-50 px-3 py-2 text-xs text-rose-700 dark:border-rose-900/40 dark:bg-rose-950/40 dark:text-rose-300">{run.error_message}</div>
              {/if}
            </div>
          {/each}
          {#if runs.length === 0}
            <div class="text-sm text-gray-500">No runs yet for this workflow.</div>
          {/if}
        </div>
      </div>
    </section>
  </div>
</div>