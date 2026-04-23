<script lang="ts">
  import { onMount } from 'svelte';
  import { page as pageStore } from '$app/stores';
  import {
    createActionType,
    createObject,
    deleteActionType,
    deleteObject,
    executeAction,
    getObjectType,
    listActionTypes,
    listLinkTypes,
    listObjects,
    listProperties,
    validateAction,
    type ActionInputField,
    type ActionOperationKind,
    type ActionType,
    type ExecuteActionResponse,
    type LinkType,
    type ObjectInstance,
    type ObjectType,
    type Property,
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
        url: 'https://example.com/functions/enrich',
        method: 'POST',
        headers: {
          'x-openfoundry-action': 'invoke_function',
        },
      },
      notes: 'The HTTP endpoint can return output, object_patch, link, or delete_object instructions.',
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

  let objectType = $state<ObjectType | null>(null);
  let properties = $state<Property[]>([]);
  let linkTypes = $state<LinkType[]>([]);
  let objects = $state<ObjectInstance[]>([]);
  let actions = $state<ActionType[]>([]);
  let loading = $state(true);
  let error = $state('');

  let actionFormError = $state('');
  let actionFormSuccess = $state('');
  let objectError = $state('');
  let runtimeError = $state('');

  let creatingAction = $state(false);
  let creatingObject = $state(false);
  let validatingAction = $state(false);
  let executingAction = $state(false);

  let selectedActionId = $state('');
  let selectedTargetObjectId = $state('');
  let validation = $state<ValidateActionResponse | null>(null);
  let execution = $state<ExecuteActionResponse | null>(null);

  let actionName = $state('');
  let actionDisplayName = $state('');
  let actionDescription = $state('');
  let actionOperationKind = $state<ActionOperationKind>('update_object');
  let actionConfirmationRequired = $state(false);
  let actionPermissionKey = $state('');
  let actionInputSchemaText = $state(JSON.stringify(actionTemplates.update_object.inputSchema, null, 2));
  let actionConfigText = $state(JSON.stringify(actionTemplates.update_object.config, null, 2));

  let objectPropertiesText = $state('{}');
  let actionParametersText = $state('{}');

  function formatJson(value: unknown): string {
    return JSON.stringify(value ?? null, null, 2);
  }

  function parseJsonValue(source: string, label: string, fallback: unknown): unknown {
    try {
      return source.trim() ? JSON.parse(source) : fallback;
    } catch (cause) {
      throw new Error(`${label} must be valid JSON`);
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

  function getSelectedAction(): ActionType | null {
    return actions.find((action) => action.id === selectedActionId) ?? null;
  }

  function operationRequiresTarget(kind: ActionOperationKind | undefined): boolean {
    return kind === 'update_object' || kind === 'create_link' || kind === 'delete_object';
  }

  function applyTemplate(kind: ActionOperationKind) {
    actionInputSchemaText = formatJson(actionTemplates[kind].inputSchema);
    actionConfigText = formatJson(actionTemplates[kind].config);
  }

  function syncSelections(nextActions: ActionType[], nextObjects: ObjectInstance[]) {
    if (!nextActions.some((action) => action.id === selectedActionId)) {
      selectedActionId = nextActions[0]?.id ?? '';
    }

    if (!nextObjects.some((object) => object.id === selectedTargetObjectId)) {
      selectedTargetObjectId = '';
    }

    if (!selectedTargetObjectId && nextObjects[0]) {
      selectedTargetObjectId = nextObjects[0].id;
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
      const [nextType, nextProperties, nextLinkTypes, nextObjects, nextActions] = await Promise.all([
        getObjectType(objectTypeId),
        listProperties(objectTypeId),
        listLinkTypes({ object_type_id: objectTypeId, page: 1, per_page: 100 }),
        listObjects(objectTypeId, { page: 1, per_page: 50 }),
        listActionTypes({ object_type_id: objectTypeId, page: 1, per_page: 100 }),
      ]);

      objectType = nextType;
      properties = nextProperties;
      linkTypes = nextLinkTypes.data;
      objects = nextObjects.data;
      actions = nextActions.data;
      syncSelections(nextActions.data, nextObjects.data);
    } catch (cause) {
      error = cause instanceof Error ? cause.message : 'Failed to load ontology details';
    } finally {
      loading = false;
    }
  }

  async function handleCreateObject(event: Event) {
    event.preventDefault();
    if (!objectTypeId) {
      return;
    }

    creatingObject = true;
    objectError = '';

    try {
      const propertiesPayload = parseJsonObject(objectPropertiesText, 'Object properties');
      const created = await createObject(objectTypeId, propertiesPayload);
      objectPropertiesText = '{}';
      selectedTargetObjectId = created.id;
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

    try {
      await deleteObject(objectTypeId, id);
      if (selectedTargetObjectId === id) {
        selectedTargetObjectId = '';
      }
      await load();
    } catch (cause) {
      objectError = cause instanceof Error ? cause.message : 'Failed to delete object';
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
      const config = parseJsonValue(actionConfigText, 'Action config', {});
      const created = await createActionType({
        name: actionName.trim(),
        display_name: actionDisplayName.trim() || undefined,
        description: actionDescription.trim() || undefined,
        object_type_id: objectTypeId,
        operation_kind: actionOperationKind,
        input_schema: inputSchema,
        config,
        confirmation_required: actionConfirmationRequired,
        permission_key: actionPermissionKey.trim() || undefined,
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
      }
      await load();
    } catch (cause) {
      actionFormError = cause instanceof Error ? cause.message : 'Failed to delete action type';
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
      execution = await executeAction(action.id, buildInvocationBody(action));
      await load();
    } catch (cause) {
      runtimeError = cause instanceof Error ? cause.message : 'Failed to execute action';
    } finally {
      executingAction = false;
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
            <p class="mt-1 text-sm text-slate-500">These definitions drive input validation for update actions.</p>
          </div>
          <a href="/ontology/graph" class="rounded-full border border-slate-300 px-3 py-1.5 text-xs font-medium text-slate-700 hover:bg-slate-100 dark:border-slate-700 dark:text-slate-200 dark:hover:bg-slate-800">
            Open graph view
          </a>
        </div>

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
                </div>
                {#if property.description}
                  <p class="mt-2 text-sm text-slate-500">{property.description}</p>
                {/if}
              </div>
            {/each}
          </div>
        {/if}
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
              <div class="rounded-2xl border border-slate-200 p-4 dark:border-slate-800">
                <div class="flex flex-wrap items-center justify-between gap-3">
                  <div class="space-y-1">
                    <button
                      type="button"
                      class={`rounded-full px-3 py-1 text-left text-xs font-medium ${selectedTargetObjectId === object.id ? 'bg-teal-600 text-white' : 'bg-slate-100 text-slate-700 dark:bg-slate-800 dark:text-slate-200'}`}
                      onclick={() => {
                        selectedTargetObjectId = object.id;
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

          <div class="grid gap-4 lg:grid-cols-2">
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
                    onclick={() => {
                      selectedActionId = action.id;
                      runtimeError = '';
                    }}
                  >
                    <div class="font-medium text-slate-900 dark:text-slate-100">{action.display_name}</div>
                    <div class="mt-1 flex flex-wrap items-center gap-2 text-xs text-slate-500">
                      <span class="font-mono">{action.name}</span>
                      <span class="rounded-full bg-slate-100 px-2 py-0.5 text-slate-700 dark:bg-slate-800 dark:text-slate-300">{action.operation_kind}</span>
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
              class="w-full rounded-2xl border border-slate-300 px-4 py-2.5 text-sm dark:border-slate-700 dark:bg-slate-950 dark:text-slate-100"
            >
              <option value="">Select an action</option>
              {#each actions as action (action.id)}
                <option value={action.id}>{action.display_name} ({action.operation_kind})</option>
              {/each}
            </select>
          </div>

          <div>
            <label class="mb-1 block text-sm font-medium text-slate-700 dark:text-slate-200" for="selected-target-object">Target object</label>
            <select
              id="selected-target-object"
              bind:value={selectedTargetObjectId}
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
            <label class="mb-1 block text-sm font-medium text-slate-700 dark:text-slate-200" for="action-parameters">Invocation parameters JSON</label>
            <textarea
              id="action-parameters"
              bind:value={actionParametersText}
              rows={12}
              class="w-full rounded-2xl border border-slate-300 bg-slate-950 px-4 py-3 font-mono text-xs text-slate-100 dark:border-slate-700"
              spellcheck="false"
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
        </div>
      </div>
    </section>
  </div>
{/if}
