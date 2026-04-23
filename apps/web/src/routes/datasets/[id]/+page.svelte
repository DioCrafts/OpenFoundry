<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/stores';
  import { listUsers, type UserProfile } from '$lib/api/auth';
  import {
    checkoutDatasetBranch,
    createDatasetBranch,
    createDatasetQualityRule,
    deleteDatasetQualityRule,
    getDataset,
    getDatasetQuality,
    getVersions,
    listBranches,
    refreshDatasetQualityProfile,
    updateDataset,
    updateDatasetQualityRule,
    type CreateDatasetBranchParams,
    type DatasetBranch,
    type CreateDatasetQualityRuleParams,
    type Dataset,
    type DatasetColumnProfile,
    type DatasetQualityResponse,
    type DatasetQualityRule,
    type DatasetRuleResult,
    type DatasetVersion,
  } from '$lib/api/datasets';

  type RuleType = 'null_check' | 'range' | 'regex' | 'custom_sql';
  type RuleSeverity = 'low' | 'medium' | 'high';
  type RuleOperator = 'gt' | 'gte' | 'eq' | 'lte' | 'lt';

  let dataset = $state<Dataset | null>(null);
  let versions = $state<DatasetVersion[]>([]);
  let branches = $state<DatasetBranch[]>([]);
  let users = $state<UserProfile[]>([]);
  let quality = $state<DatasetQualityResponse | null>(null);
  let loading = $state(true);
  let activeTab = $state<'schema' | 'versions' | 'branches' | 'preview' | 'quality'>('quality');
  let descriptionInput = $state('');
  let tagsInput = $state('');
  let ownerId = $state('');
  let metadataError = $state('');
  let qualityError = $state('');
  let branchError = $state('');
  let savingMetadata = $state(false);
  let refreshingQuality = $state(false);
  let creatingBranch = $state(false);
  let checkingOutBranch = $state('');
  let savingRule = $state(false);
  let ruleFormMode = $state<'create' | 'edit'>('create');
  let editingRuleId = $state<string | null>(null);
  let branchName = $state('feature-experiment');
  let branchDescription = $state('');
  let branchSourceVersion = $state('');
  let ruleName = $state('Completeness threshold');
  let ruleType = $state<RuleType>('null_check');
  let ruleSeverity = $state<RuleSeverity>('medium');
  let ruleEnabled = $state(true);
  let columnName = $state('');
  let maxNullRatio = $state('5');
  let rangeMin = $state('');
  let rangeMax = $state('');
  let regexPattern = $state('');
  let regexAllowNulls = $state(true);
  let customSql = $state('SELECT COUNT(*) AS value FROM dataset');
  let comparisonOperator = $state<RuleOperator>('gte');
  let threshold = $state('1');

  const datasetId = $derived($page.params.id ?? '');

  function normalizeRuleType(value: string): RuleType {
    if (value === 'range' || value === 'regex' || value === 'custom_sql') return value;
    return 'null_check';
  }

  function normalizeRuleSeverity(value: string): RuleSeverity {
    if (value === 'low' || value === 'high') return value;
    return 'medium';
  }

  function normalizeRuleOperator(value: string): RuleOperator {
    if (value === 'gt' || value === 'eq' || value === 'lte' || value === 'lt') return value;
    return 'gte';
  }

  function ownerName(userId: string) {
    return users.find((user) => user.id === userId)?.name ?? userId.slice(0, 8);
  }

  function toneFor(score: number | null) {
    if (score === null) return 'text-gray-500';
    if (score >= 90) return 'text-emerald-600 dark:text-emerald-300';
    if (score >= 75) return 'text-amber-600 dark:text-amber-300';
    return 'text-rose-600 dark:text-rose-300';
  }

  function parseTags(value: string) {
    return value.split(',').map((tag) => tag.trim()).filter(Boolean);
  }

  function columns(): DatasetColumnProfile[] {
    return quality?.profile?.columns ?? [];
  }

  function activeAlerts() {
    return quality?.alerts.filter((alert) => alert.status === 'active') ?? [];
  }

  function ruleResultFor(rule: DatasetQualityRule): DatasetRuleResult | undefined {
    return quality?.profile?.rule_results.find((result) => result.rule_id === rule.id);
  }

  function resetRuleForm() {
    ruleFormMode = 'create';
    editingRuleId = null;
    ruleName = 'Completeness threshold';
    ruleType = 'null_check';
    ruleSeverity = 'medium';
    ruleEnabled = true;
    columnName = columns()[0]?.name ?? '';
    maxNullRatio = '5';
    rangeMin = '';
    rangeMax = '';
    regexPattern = '';
    regexAllowNulls = true;
    customSql = 'SELECT COUNT(*) AS value FROM dataset';
    comparisonOperator = 'gte';
    threshold = '1';
  }

  function editRule(rule: DatasetQualityRule) {
    const config = rule.config ?? {};
    ruleFormMode = 'edit';
    editingRuleId = rule.id;
    ruleName = rule.name;
    ruleType = normalizeRuleType(rule.rule_type);
    ruleSeverity = normalizeRuleSeverity(rule.severity);
    ruleEnabled = rule.enabled;
    columnName = typeof config['column'] === 'string' ? String(config['column']) : '';
    maxNullRatio = typeof config['max_null_ratio'] === 'number'
      ? String(Number(config['max_null_ratio']) * 100)
      : '5';
    rangeMin = typeof config['min'] === 'number' ? String(config['min']) : '';
    rangeMax = typeof config['max'] === 'number' ? String(config['max']) : '';
    regexPattern = typeof config['pattern'] === 'string' ? String(config['pattern']) : '';
    regexAllowNulls = typeof config['allow_nulls'] === 'boolean' ? Boolean(config['allow_nulls']) : true;
    customSql = typeof config['sql'] === 'string' ? String(config['sql']) : 'SELECT COUNT(*) AS value FROM dataset';
    comparisonOperator = typeof config['operator'] === 'string'
      ? normalizeRuleOperator(String(config['operator']))
      : 'gte';
    threshold = typeof config['threshold'] === 'number' ? String(config['threshold']) : '1';
  }

  function buildRulePayload(): CreateDatasetQualityRuleParams {
    if (ruleType === 'null_check') {
      return {
        name: ruleName,
        rule_type: ruleType,
        severity: ruleSeverity,
        enabled: ruleEnabled,
        config: {
          column: columnName,
          max_null_ratio: Math.max(Number(maxNullRatio) || 0, 0) / 100,
        },
      };
    }

    if (ruleType === 'range') {
      const config: Record<string, unknown> = { column: columnName };
      if (rangeMin) config.min = Number(rangeMin);
      if (rangeMax) config.max = Number(rangeMax);
      return {
        name: ruleName,
        rule_type: ruleType,
        severity: ruleSeverity,
        enabled: ruleEnabled,
        config,
      };
    }

    if (ruleType === 'regex') {
      return {
        name: ruleName,
        rule_type: ruleType,
        severity: ruleSeverity,
        enabled: ruleEnabled,
        config: {
          column: columnName,
          pattern: regexPattern,
          allow_nulls: regexAllowNulls,
        },
      };
    }

    return {
      name: ruleName,
      rule_type: ruleType,
      severity: ruleSeverity,
      enabled: ruleEnabled,
      config: {
        sql: customSql,
        operator: comparisonOperator,
        threshold: Number(threshold) || 0,
      },
    };
  }

  async function load() {
    loading = true;
    try {
      const [nextDataset, nextVersions, nextBranches, nextUsers, nextQuality] = await Promise.all([
        getDataset(datasetId),
        getVersions(datasetId),
        listBranches(datasetId).catch(() => [] as DatasetBranch[]),
        listUsers().catch(() => [] as UserProfile[]),
        getDatasetQuality(datasetId).catch(() => null as DatasetQualityResponse | null),
      ]);
      dataset = nextDataset;
      versions = nextVersions;
      branches = nextBranches;
      users = nextUsers;
      quality = nextQuality;
      descriptionInput = nextDataset.description;
      tagsInput = nextDataset.tags.join(', ');
      ownerId = nextDataset.owner_id;
      branchSourceVersion = String(nextDataset.current_version);
      if (!columnName) {
        columnName = nextQuality?.profile?.columns[0]?.name ?? '';
      }
    } catch (cause) {
      console.error('Failed to load dataset', cause);
    } finally {
      loading = false;
    }
  }

  async function saveMetadata() {
    if (!dataset) return;
    savingMetadata = true;
    metadataError = '';
    try {
      dataset = await updateDataset(dataset.id, {
        description: descriptionInput,
        owner_id: ownerId || undefined,
        tags: parseTags(tagsInput),
      });
    } catch (cause) {
      metadataError = cause instanceof Error ? cause.message : 'Failed to save metadata';
    } finally {
      savingMetadata = false;
    }
  }

  async function refreshQuality() {
    if (!dataset) return;
    refreshingQuality = true;
    qualityError = '';
    try {
      quality = await refreshDatasetQualityProfile(dataset.id);
      dataset = await getDataset(dataset.id);
      if (!columnName) {
        columnName = quality.profile?.columns[0]?.name ?? '';
      }
    } catch (cause) {
      qualityError = cause instanceof Error ? cause.message : 'Failed to refresh quality profile';
    } finally {
      refreshingQuality = false;
    }
  }

  async function saveBranch() {
    if (!dataset) return;
    creatingBranch = true;
    branchError = '';
    try {
      const payload: CreateDatasetBranchParams = {
        name: branchName.trim(),
        description: branchDescription.trim() || undefined,
        source_version: branchSourceVersion ? Number(branchSourceVersion) : undefined,
      };
      await createDatasetBranch(dataset.id, payload);
      branches = await listBranches(dataset.id);
      branchName = 'feature-experiment';
      branchDescription = '';
    } catch (cause) {
      branchError = cause instanceof Error ? cause.message : 'Failed to create branch';
    } finally {
      creatingBranch = false;
    }
  }

  async function checkoutBranch(name: string) {
    if (!dataset) return;
    checkingOutBranch = name;
    branchError = '';
    try {
      dataset = await checkoutDatasetBranch(dataset.id, name);
      branches = await listBranches(dataset.id);
      versions = await getVersions(dataset.id);
      branchSourceVersion = String(dataset.current_version);
    } catch (cause) {
      branchError = cause instanceof Error ? cause.message : 'Failed to switch branch';
    } finally {
      checkingOutBranch = '';
    }
  }

  async function saveRule() {
    if (!dataset) return;
    savingRule = true;
    qualityError = '';
    try {
      const payload = buildRulePayload();
      quality = ruleFormMode === 'edit' && editingRuleId
        ? await updateDatasetQualityRule(dataset.id, editingRuleId, payload)
        : await createDatasetQualityRule(dataset.id, payload);
      resetRuleForm();
    } catch (cause) {
      qualityError = cause instanceof Error ? cause.message : 'Failed to save quality rule';
    } finally {
      savingRule = false;
    }
  }

  async function removeRule(ruleId: string) {
    if (!dataset || !confirm('Delete this quality rule?')) return;
    qualityError = '';
    try {
      quality = await deleteDatasetQualityRule(dataset.id, ruleId);
      if (editingRuleId === ruleId) {
        resetRuleForm();
      }
    } catch (cause) {
      qualityError = cause instanceof Error ? cause.message : 'Failed to delete quality rule';
    }
  }

  onMount(() => {
    if (datasetId) {
      void load();
    }
  });
</script>

{#if loading}
  <div class="py-12 text-center text-gray-500">Loading...</div>
{:else if !dataset}
  <div class="py-12 text-center text-gray-500">Dataset not found</div>
{:else}
  <div class="space-y-6">
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-2xl font-bold">{dataset.name}</h1>
        <p class="mt-1 text-gray-500">{dataset.description || 'No description'}</p>
      </div>
      <div class="flex gap-2">
        <span class="rounded bg-gray-100 px-3 py-1 text-sm dark:bg-gray-800">{dataset.format}</span>
        <span class="rounded bg-amber-100 px-3 py-1 text-sm dark:bg-amber-900">{dataset.active_branch}</span>
        <span class="rounded bg-blue-100 px-3 py-1 text-sm dark:bg-blue-900">v{dataset.current_version}</span>
      </div>
    </div>

    <div class="grid gap-4 xl:grid-cols-[2fr,1fr]">
      <div class="grid grid-cols-2 gap-4 lg:grid-cols-4">
        <div class="rounded-xl border p-4 dark:border-gray-700">
          <div class="text-sm text-gray-500">Size</div>
          <div class="text-lg font-semibold">{(dataset.size_bytes / 1024).toFixed(1)} KB</div>
        </div>
        <div class="rounded-xl border p-4 dark:border-gray-700">
          <div class="text-sm text-gray-500">Rows</div>
          <div class="text-lg font-semibold">{dataset.row_count.toLocaleString()}</div>
        </div>
        <div class="rounded-xl border p-4 dark:border-gray-700">
          <div class="text-sm text-gray-500">Version</div>
          <div class="text-lg font-semibold">{dataset.current_version}</div>
        </div>
        <div class="rounded-xl border p-4 dark:border-gray-700">
          <div class="text-sm text-gray-500">Quality Score</div>
          <div class={`text-lg font-semibold ${toneFor(quality?.score ?? null)}`}>
            {#if quality?.score !== null && quality?.score !== undefined}
              {quality.score.toFixed(1)}
            {:else}
              --
            {/if}
          </div>
        </div>
      </div>

      <div class="rounded-2xl border border-slate-200 bg-white p-5 shadow-sm dark:border-gray-700 dark:bg-gray-900">
        <div class="flex items-center justify-between">
          <div>
            <div class="text-xs uppercase tracking-[0.22em] text-gray-400">Catalog Metadata</div>
            <div class="mt-1 text-sm text-gray-500">Tagging and ownership live with the dataset.</div>
          </div>
          <button onclick={saveMetadata} disabled={savingMetadata} class="rounded-xl bg-slate-900 px-4 py-2 text-sm text-white disabled:opacity-50 dark:bg-white dark:text-slate-900">
            {savingMetadata ? 'Saving...' : 'Save'}
          </button>
        </div>

        <div class="mt-4 space-y-4">
          <div>
            <label for="owner" class="mb-1 block text-sm font-medium">Owner</label>
            <select id="owner" bind:value={ownerId} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-gray-700 dark:bg-gray-800">
              {#each users as user (user.id)}
                <option value={user.id}>{user.name}</option>
              {/each}
            </select>
          </div>

          <div>
            <label for="description" class="mb-1 block text-sm font-medium">Description</label>
            <textarea id="description" bind:value={descriptionInput} rows="3" class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-gray-700 dark:bg-gray-800"></textarea>
          </div>

          <div>
            <label for="tags" class="mb-1 block text-sm font-medium">Tags</label>
            <input id="tags" bind:value={tagsInput} placeholder="finance, monthly, curated" class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-gray-700 dark:bg-gray-800" />
            <div class="mt-2 flex flex-wrap gap-2">
              {#each dataset.tags as tag}
                <span class="rounded-full bg-blue-100 px-2.5 py-1 text-xs font-medium text-blue-700 dark:bg-blue-900/40 dark:text-blue-300">{tag}</span>
              {/each}
            </div>
          </div>

          <div class="text-sm text-gray-500">Current owner: {ownerName(dataset.owner_id)}</div>

          {#if metadataError}
            <div class="rounded-xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-700 dark:border-rose-900/40 dark:bg-rose-950/40 dark:text-rose-300">{metadataError}</div>
          {/if}
        </div>
      </div>
    </div>

    <div class="border-b dark:border-gray-700">
      <nav class="flex gap-4">
        <button
          class="border-b-2 pb-2 px-1 text-sm font-medium transition-colors"
          class:border-blue-600={activeTab === 'preview'}
          class:text-blue-600={activeTab === 'preview'}
          class:border-transparent={activeTab !== 'preview'}
          onclick={() => activeTab = 'preview'}
        >Preview</button>
        <button
          class="border-b-2 pb-2 px-1 text-sm font-medium transition-colors"
          class:border-blue-600={activeTab === 'schema'}
          class:text-blue-600={activeTab === 'schema'}
          class:border-transparent={activeTab !== 'schema'}
          onclick={() => activeTab = 'schema'}
        >Schema</button>
        <button
          class="border-b-2 pb-2 px-1 text-sm font-medium transition-colors"
          class:border-blue-600={activeTab === 'versions'}
          class:text-blue-600={activeTab === 'versions'}
          class:border-transparent={activeTab !== 'versions'}
          onclick={() => activeTab = 'versions'}
        >Versions ({versions.length})</button>
        <button
          class="border-b-2 pb-2 px-1 text-sm font-medium transition-colors"
          class:border-blue-600={activeTab === 'branches'}
          class:text-blue-600={activeTab === 'branches'}
          class:border-transparent={activeTab !== 'branches'}
          onclick={() => activeTab = 'branches'}
        >Branches ({branches.length})</button>
        <button
          class="border-b-2 pb-2 px-1 text-sm font-medium transition-colors"
          class:border-blue-600={activeTab === 'quality'}
          class:text-blue-600={activeTab === 'quality'}
          class:border-transparent={activeTab !== 'quality'}
          onclick={() => activeTab = 'quality'}
        >Quality</button>
      </nav>
    </div>

    {#if activeTab === 'preview'}
      <div class="rounded border py-8 text-center text-gray-500 dark:border-gray-700">
        Data preview will be available after file upload and Parquet integration.
      </div>
    {:else if activeTab === 'schema'}
      <div class="rounded border py-8 text-center text-gray-500 dark:border-gray-700">
        Schema is now inferred during quality profiling and will appear after the first profile refresh.
      </div>
    {:else if activeTab === 'versions'}
      <div class="space-y-2">
        {#each versions as version (version.id)}
          <div class="flex items-center justify-between rounded border p-3 dark:border-gray-700">
            <div>
              <span class="font-medium">v{version.version}</span>
              <span class="ml-2 text-sm text-gray-500">{version.message || 'No message'}</span>
            </div>
            <div class="text-sm text-gray-500">
              {(version.size_bytes / 1024).toFixed(1)} KB · {new Date(version.created_at).toLocaleDateString()}
            </div>
          </div>
        {/each}
        {#if versions.length === 0}
          <div class="py-4 text-center text-gray-500">No versions yet</div>
        {/if}
      </div>
    {:else if activeTab === 'branches'}
      <div class="grid gap-6 xl:grid-cols-[1.1fr,0.9fr]">
        <div class="rounded-2xl border border-slate-200 bg-white p-5 shadow-sm dark:border-gray-700 dark:bg-gray-900">
          <div class="flex items-center justify-between">
            <div>
              <div class="text-xs uppercase tracking-[0.22em] text-gray-400">Branch Selector</div>
              <div class="mt-1 text-sm text-gray-500">Switch the active dataset branch or inspect which version each branch points to.</div>
            </div>
            <span class="rounded-full bg-slate-100 px-3 py-1 text-xs font-medium text-slate-700 dark:bg-gray-800 dark:text-gray-200">
              Active {dataset.active_branch}
            </span>
          </div>

          <div class="mt-4 space-y-3">
            {#each branches as branch (branch.id)}
              <div class="rounded-xl border border-slate-200 p-4 dark:border-gray-700">
                <div class="flex items-center justify-between gap-3">
                  <div>
                    <div class="flex items-center gap-2">
                      <div class="font-medium">{branch.name}</div>
                      {#if branch.is_default}
                        <span class="rounded-full bg-slate-100 px-2 py-0.5 text-[10px] font-semibold uppercase tracking-[0.16em] text-slate-600 dark:bg-gray-800 dark:text-gray-300">default</span>
                      {/if}
                    </div>
                    <div class="mt-1 text-sm text-gray-500">{branch.description || 'No description'}</div>
                    <div class="mt-2 text-xs text-gray-500">Version {branch.version} · Updated {new Date(branch.updated_at).toLocaleString()}</div>
                  </div>
                  <button
                    type="button"
                    onclick={() => checkoutBranch(branch.name)}
                    disabled={checkingOutBranch === branch.name || branch.name === dataset.active_branch}
                    class="rounded-xl border border-slate-200 px-3 py-2 text-sm disabled:opacity-50 hover:bg-slate-50 dark:border-gray-700 dark:hover:bg-gray-800"
                  >
                    {branch.name === dataset.active_branch ? 'Current' : checkingOutBranch === branch.name ? 'Switching...' : 'Checkout'}
                  </button>
                </div>
              </div>
            {/each}
          </div>
        </div>

        <div class="rounded-2xl border border-slate-200 bg-white p-5 shadow-sm dark:border-gray-700 dark:bg-gray-900">
          <div class="text-xs uppercase tracking-[0.22em] text-gray-400">Create Branch</div>
          <div class="mt-1 text-sm text-gray-500">Start from the current or any existing dataset version.</div>

          <div class="mt-4 space-y-4">
            <div>
              <label for="branch-name" class="mb-1 block text-sm font-medium">Branch name</label>
              <input id="branch-name" bind:value={branchName} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-gray-700 dark:bg-gray-800" />
            </div>

            <div>
              <label for="branch-description" class="mb-1 block text-sm font-medium">Description</label>
              <textarea id="branch-description" bind:value={branchDescription} rows="3" class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-gray-700 dark:bg-gray-800"></textarea>
            </div>

            <div>
              <label for="branch-source-version" class="mb-1 block text-sm font-medium">Source version</label>
              <select id="branch-source-version" bind:value={branchSourceVersion} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-gray-700 dark:bg-gray-800">
                <option value={String(dataset.current_version)}>Current version ({dataset.current_version})</option>
                {#each versions as version (version.id)}
                  <option value={String(version.version)}>Version {version.version}</option>
                {/each}
              </select>
            </div>

            <button type="button" onclick={saveBranch} disabled={creatingBranch} class="w-full rounded-xl bg-slate-900 px-4 py-2 text-white disabled:opacity-50 dark:bg-white dark:text-slate-900">
              {creatingBranch ? 'Creating...' : 'Create branch'}
            </button>

            {#if branchError}
              <div class="rounded-xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-700 dark:border-rose-900/40 dark:bg-rose-950/40 dark:text-rose-300">{branchError}</div>
            {/if}
          </div>
        </div>
      </div>
    {:else}
      <div class="space-y-6">
        <div class="flex flex-col gap-3 rounded-2xl border border-slate-200 bg-white p-5 shadow-sm dark:border-gray-700 dark:bg-gray-900 lg:flex-row lg:items-center lg:justify-between">
          <div>
            <div class="text-xs uppercase tracking-[0.22em] text-gray-400">Quality Dashboard</div>
            <div class="mt-1 text-sm text-gray-500">Profiling report, score trend, alerts, and rule management.</div>
          </div>
          <button onclick={refreshQuality} disabled={refreshingQuality} class="rounded-xl bg-blue-600 px-4 py-2 text-white disabled:opacity-50 hover:bg-blue-700">
            {refreshingQuality ? 'Refreshing...' : 'Refresh Profile'}
          </button>
        </div>

        {#if qualityError}
          <div class="rounded-xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-700 dark:border-rose-900/40 dark:bg-rose-950/40 dark:text-rose-300">{qualityError}</div>
        {/if}

        {#if !quality?.profile}
          <div class="rounded-2xl border border-dashed border-slate-300 px-6 py-10 text-center text-gray-500 dark:border-gray-700">
            Generate the first quality profile after uploading data to unlock profiling, scoring, alerts, and rules.
          </div>
        {:else}
          <div class="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
            <div class="rounded-2xl border border-slate-200 bg-white p-4 shadow-sm dark:border-gray-700 dark:bg-gray-900">
              <div class="text-xs uppercase tracking-[0.22em] text-gray-400">Quality Score</div>
              <div class={`mt-3 text-3xl font-semibold ${toneFor(quality.score)}`}>{quality.score?.toFixed(1) ?? '--'}</div>
            </div>
            <div class="rounded-2xl border border-slate-200 bg-white p-4 shadow-sm dark:border-gray-700 dark:bg-gray-900">
              <div class="text-xs uppercase tracking-[0.22em] text-gray-400">Completeness</div>
              <div class="mt-3 text-3xl font-semibold">{(quality.profile.completeness_ratio * 100).toFixed(1)}%</div>
            </div>
            <div class="rounded-2xl border border-slate-200 bg-white p-4 shadow-sm dark:border-gray-700 dark:bg-gray-900">
              <div class="text-xs uppercase tracking-[0.22em] text-gray-400">Uniqueness</div>
              <div class="mt-3 text-3xl font-semibold">{(quality.profile.uniqueness_ratio * 100).toFixed(1)}%</div>
            </div>
            <div class="rounded-2xl border border-slate-200 bg-white p-4 shadow-sm dark:border-gray-700 dark:bg-gray-900">
              <div class="text-xs uppercase tracking-[0.22em] text-gray-400">Duplicate Rows</div>
              <div class="mt-3 text-3xl font-semibold">{quality.profile.duplicate_rows.toLocaleString()}</div>
            </div>
          </div>

          <div class="grid gap-6 xl:grid-cols-[1.2fr,0.8fr]">
            <div class="rounded-2xl border border-slate-200 bg-white p-5 shadow-sm dark:border-gray-700 dark:bg-gray-900">
              <div class="flex items-center justify-between">
                <h2 class="text-lg font-semibold">Quality Trend</h2>
                <div class="text-sm text-gray-500">{quality.history.length} run{quality.history.length === 1 ? '' : 's'}</div>
              </div>
              <div class="mt-4 space-y-3">
                {#each quality.history.slice(-8) as point (point.id)}
                  <div class="grid grid-cols-[96px,1fr,56px] items-center gap-3 text-sm">
                    <div class="text-gray-500">{new Date(point.created_at).toLocaleDateString()}</div>
                    <div class="h-2 rounded-full bg-slate-100 dark:bg-gray-800">
                      <div class="h-2 rounded-full bg-blue-500" style={`width:${Math.max(point.score, 4)}%`}></div>
                    </div>
                    <div class="text-right font-medium">{point.score.toFixed(1)}</div>
                  </div>
                {/each}
              </div>
            </div>

            <div class="rounded-2xl border border-slate-200 bg-white p-5 shadow-sm dark:border-gray-700 dark:bg-gray-900">
              <h2 class="text-lg font-semibold">Active Alerts</h2>
              <div class="mt-4 space-y-3">
                {#each activeAlerts() as alert (alert.id)}
                  <div class="rounded-xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-700 dark:border-rose-900/40 dark:bg-rose-950/40 dark:text-rose-300">
                    <div class="font-medium uppercase tracking-[0.16em]">{alert.level}</div>
                    <div class="mt-1">{alert.message}</div>
                  </div>
                {/each}
                {#if activeAlerts().length === 0}
                  <div class="rounded-xl border border-emerald-200 bg-emerald-50 px-4 py-3 text-sm text-emerald-700 dark:border-emerald-900/40 dark:bg-emerald-950/40 dark:text-emerald-300">
                    No active alerts on the latest quality run.
                  </div>
                {/if}
              </div>
            </div>
          </div>

          <div class="rounded-2xl border border-slate-200 bg-white p-5 shadow-sm dark:border-gray-700 dark:bg-gray-900">
            <div class="flex items-center justify-between">
              <h2 class="text-lg font-semibold">Column Profiling</h2>
              <div class="text-sm text-gray-500">{quality.profile.column_count} columns</div>
            </div>
            <div class="mt-4 overflow-x-auto">
              <table class="min-w-full divide-y divide-slate-200 text-sm dark:divide-gray-700">
                <thead>
                  <tr class="text-left text-gray-500">
                    <th class="pb-3 pr-4 font-medium">Column</th>
                    <th class="pb-3 pr-4 font-medium">Type</th>
                    <th class="pb-3 pr-4 font-medium">Null Rate</th>
                    <th class="pb-3 pr-4 font-medium">Uniqueness</th>
                    <th class="pb-3 pr-4 font-medium">Distribution</th>
                    <th class="pb-3 pr-4 font-medium">Min / Max / Avg</th>
                  </tr>
                </thead>
                <tbody class="divide-y divide-slate-100 dark:divide-gray-800">
                  {#each quality.profile.columns as column (column.name)}
                    <tr>
                      <td class="py-3 pr-4 font-medium">{column.name}</td>
                      <td class="py-3 pr-4 text-gray-500">{column.field_type}</td>
                      <td class="py-3 pr-4">{(column.null_rate * 100).toFixed(1)}%</td>
                      <td class="py-3 pr-4">{(column.uniqueness_rate * 100).toFixed(1)}%</td>
                      <td class="py-3 pr-4 text-gray-500">
                        {#if column.sample_values.length > 0}
                          {column.sample_values.map((sample) => `${sample.value} (${sample.count})`).join(', ')}
                        {:else}
                          --
                        {/if}
                      </td>
                      <td class="py-3 pr-4 text-gray-500">
                        {column.min_value ?? '--'} / {column.max_value ?? '--'} / {column.average_value !== null ? column.average_value.toFixed(2) : '--'}
                      </td>
                    </tr>
                  {/each}
                </tbody>
              </table>
            </div>
          </div>
        {/if}

        <div class="grid gap-6 xl:grid-cols-[0.95fr,1.05fr]">
          <div class="rounded-2xl border border-slate-200 bg-white p-5 shadow-sm dark:border-gray-700 dark:bg-gray-900">
            <div class="flex items-center justify-between">
              <h2 class="text-lg font-semibold">Rule Builder</h2>
              {#if ruleFormMode === 'edit'}
                <button onclick={resetRuleForm} class="text-sm text-gray-500 hover:text-gray-700">Cancel edit</button>
              {/if}
            </div>

            <div class="mt-4 grid gap-4">
              <input bind:value={ruleName} placeholder="Rule name" class="rounded-xl border border-slate-200 px-3 py-2 dark:border-gray-700 dark:bg-gray-800" />

              <div class="grid gap-4 md:grid-cols-3">
                <select bind:value={ruleType} class="rounded-xl border border-slate-200 px-3 py-2 dark:border-gray-700 dark:bg-gray-800">
                  <option value="null_check">Null check</option>
                  <option value="range">Range</option>
                  <option value="regex">Regex</option>
                  <option value="custom_sql">Custom SQL</option>
                </select>
                <select bind:value={ruleSeverity} class="rounded-xl border border-slate-200 px-3 py-2 dark:border-gray-700 dark:bg-gray-800">
                  <option value="low">Low</option>
                  <option value="medium">Medium</option>
                  <option value="high">High</option>
                </select>
                <label class="flex items-center gap-2 rounded-xl border border-slate-200 px-3 py-2 dark:border-gray-700">
                  <input type="checkbox" bind:checked={ruleEnabled} />
                  <span>Enabled</span>
                </label>
              </div>

              <datalist id="quality-columns">
                {#each columns() as column (column.name)}
                  <option value={column.name}></option>
                {/each}
              </datalist>

              {#if ruleType === 'null_check'}
                <div class="grid gap-4 md:grid-cols-2">
                  <input bind:value={columnName} list="quality-columns" placeholder="Column name" class="rounded-xl border border-slate-200 px-3 py-2 dark:border-gray-700 dark:bg-gray-800" />
                  <input bind:value={maxNullRatio} type="number" min="0" max="100" placeholder="Max null %" class="rounded-xl border border-slate-200 px-3 py-2 dark:border-gray-700 dark:bg-gray-800" />
                </div>
              {:else if ruleType === 'range'}
                <div class="grid gap-4 md:grid-cols-3">
                  <input bind:value={columnName} list="quality-columns" placeholder="Column name" class="rounded-xl border border-slate-200 px-3 py-2 dark:border-gray-700 dark:bg-gray-800" />
                  <input bind:value={rangeMin} type="number" placeholder="Min" class="rounded-xl border border-slate-200 px-3 py-2 dark:border-gray-700 dark:bg-gray-800" />
                  <input bind:value={rangeMax} type="number" placeholder="Max" class="rounded-xl border border-slate-200 px-3 py-2 dark:border-gray-700 dark:bg-gray-800" />
                </div>
              {:else if ruleType === 'regex'}
                <div class="grid gap-4 md:grid-cols-[1fr,1fr,auto]">
                  <input bind:value={columnName} list="quality-columns" placeholder="Column name" class="rounded-xl border border-slate-200 px-3 py-2 dark:border-gray-700 dark:bg-gray-800" />
                  <input bind:value={regexPattern} placeholder="Regex pattern" class="rounded-xl border border-slate-200 px-3 py-2 dark:border-gray-700 dark:bg-gray-800" />
                  <label class="flex items-center gap-2 rounded-xl border border-slate-200 px-3 py-2 dark:border-gray-700">
                    <input type="checkbox" bind:checked={regexAllowNulls} />
                    <span>Allow nulls</span>
                  </label>
                </div>
              {:else}
                <div class="grid gap-4">
                  <textarea bind:value={customSql} rows="4" class="rounded-xl border border-slate-200 px-3 py-2 font-mono text-sm dark:border-gray-700 dark:bg-gray-800"></textarea>
                  <div class="grid gap-4 md:grid-cols-2">
                    <select bind:value={comparisonOperator} class="rounded-xl border border-slate-200 px-3 py-2 dark:border-gray-700 dark:bg-gray-800">
                      <option value="gt">&gt;</option>
                      <option value="gte">&gt;=</option>
                      <option value="eq">=</option>
                      <option value="lte">&lt;=</option>
                      <option value="lt">&lt;</option>
                    </select>
                    <input bind:value={threshold} type="number" placeholder="Threshold" class="rounded-xl border border-slate-200 px-3 py-2 dark:border-gray-700 dark:bg-gray-800" />
                  </div>
                </div>
              {/if}

              <button onclick={saveRule} disabled={savingRule} class="rounded-xl bg-slate-900 px-4 py-2 text-white disabled:opacity-50 dark:bg-white dark:text-slate-900">
                {savingRule ? 'Saving...' : ruleFormMode === 'edit' ? 'Update Rule' : 'Add Rule'}
              </button>
            </div>
          </div>

          <div class="rounded-2xl border border-slate-200 bg-white p-5 shadow-sm dark:border-gray-700 dark:bg-gray-900">
            <h2 class="text-lg font-semibold">Rules</h2>
            <div class="mt-4 space-y-3">
              {#each quality?.rules ?? [] as rule (rule.id)}
                {@const result = ruleResultFor(rule)}
                <div class="rounded-xl border border-slate-200 p-4 dark:border-gray-700">
                  <div class="flex flex-col gap-3 lg:flex-row lg:items-start lg:justify-between">
                    <div class="space-y-2">
                      <div class="flex flex-wrap items-center gap-2">
                        <div class="font-medium">{rule.name}</div>
                        <span class="rounded-full bg-slate-100 px-2.5 py-1 text-xs font-medium text-slate-700 dark:bg-gray-800 dark:text-gray-300">{rule.rule_type}</span>
                        <span class="rounded-full bg-slate-100 px-2.5 py-1 text-xs font-medium text-slate-700 dark:bg-gray-800 dark:text-gray-300">{rule.severity}</span>
                        <span class={`rounded-full px-2.5 py-1 text-xs font-medium ${result?.passed ? 'bg-emerald-100 text-emerald-700 dark:bg-emerald-900/40 dark:text-emerald-300' : 'bg-rose-100 text-rose-700 dark:bg-rose-900/40 dark:text-rose-300'}`}>
                          {result?.passed ? 'Passing' : 'Failing'}
                        </span>
                      </div>
                      <div class="text-sm text-gray-500">{JSON.stringify(rule.config)}</div>
                      {#if result}
                        <div class="text-sm text-gray-500">{result.message}{result.measured_value ? ` (${result.measured_value})` : ''}</div>
                      {/if}
                    </div>

                    <div class="flex gap-2">
                      <button onclick={() => editRule(rule)} class="rounded-xl border border-slate-200 px-3 py-2 text-sm hover:bg-slate-50 dark:border-gray-700 dark:hover:bg-gray-800">Edit</button>
                      <button onclick={() => removeRule(rule.id)} class="rounded-xl border border-rose-200 px-3 py-2 text-sm text-rose-600 hover:bg-rose-50 dark:border-rose-900/40 dark:hover:bg-rose-950/30">Delete</button>
                    </div>
                  </div>
                </div>
              {/each}

              {#if !quality || quality.rules.length === 0}
                <div class="rounded-xl border border-dashed border-slate-300 px-4 py-6 text-center text-sm text-gray-500 dark:border-gray-700">
                  No quality rules yet.
                </div>
              {/if}
            </div>
          </div>
        </div>
      </div>
    {/if}
  </div>
{/if}
