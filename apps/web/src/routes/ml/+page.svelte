<script lang="ts">
  import { onMount } from 'svelte';

  import { listDatasets, type Dataset } from '$lib/api/datasets';
  import {
    compareRuns,
    createBatchPrediction,
    createDeployment,
    createExperiment,
    createFeature,
    createModel,
    createModelVersion,
    createRun,
    createTrainingJob,
    generateDriftReport,
    getOnlineFeatureSnapshot,
    getOverview,
    listBatchPredictions,
    listDeployments,
    listExperiments,
    listFeatures,
    listModelVersions,
    listModels,
    listRuns,
    listTrainingJobs,
    materializeFeature,
    realtimePredict,
    transitionModelVersion,
    updateDeployment,
    updateExperiment,
    updateFeature,
    updateModel,
    type ArtifactReference,
    type BatchPredictionJob,
    type CompareRunsResponse,
    type Experiment,
    type ExperimentRun,
    type FeatureDefinition,
    type FeatureSample,
    type MetricValue,
    type MlStudioOverview,
    type ModelDeployment,
    type ModelVersion,
    type OnlineFeatureSnapshot,
    type RegisteredModel,
    type RealtimePredictionResponse,
    type TrafficSplitEntry,
    type TrainingJob,
  } from '$lib/api/ml';
  import { notifications } from '$stores/notifications';

  type ExperimentDraft = {
    id?: string;
    name: string;
    description: string;
    objective: string;
    task_type: string;
    primary_metric: string;
    status: string;
    tags_text: string;
  };

  type RunDraft = {
    name: string;
    status: string;
    params_text: string;
    metrics_text: string;
    artifacts_text: string;
    notes: string;
    source_dataset_ids_text: string;
  };

  type ModelDraft = {
    id?: string;
    name: string;
    description: string;
    problem_type: string;
    status: string;
    tags_text: string;
  };

  type ModelVersionDraft = {
    version_label: string;
    stage: string;
    artifact_uri: string;
    hyperparameters_text: string;
    metrics_text: string;
    schema_text: string;
  };

  type FeatureDraft = {
    id?: string;
    name: string;
    entity_name: string;
    data_type: string;
    description: string;
    status: string;
    offline_source: string;
    transformation: string;
    online_enabled: boolean;
    online_namespace: string;
    batch_schedule: string;
    freshness_sla_minutes: number;
    tags_text: string;
    samples_text: string;
  };

  type TrainingJobDraft = {
    name: string;
    experiment_id: string;
    model_id: string;
    dataset_ids_text: string;
    objective_metric_name: string;
    training_config_text: string;
    hyperparameter_search_text: string;
    auto_register_model_version: boolean;
  };

  type DeploymentDraft = {
    id?: string;
    model_id: string;
    name: string;
    status: string;
    strategy_type: string;
    endpoint_path: string;
    traffic_split_text: string;
    monitoring_window: string;
    baseline_dataset_id: string;
  };

  let overview = $state<MlStudioOverview | null>(null);
  let experiments = $state<Experiment[]>([]);
  let runs = $state<ExperimentRun[]>([]);
  let comparedRuns = $state<CompareRunsResponse | null>(null);
  let models = $state<RegisteredModel[]>([]);
  let modelVersions = $state<ModelVersion[]>([]);
  let features = $state<FeatureDefinition[]>([]);
  let onlineFeature = $state<OnlineFeatureSnapshot | null>(null);
  let trainingJobs = $state<TrainingJob[]>([]);
  let deployments = $state<ModelDeployment[]>([]);
  let batchPredictions = $state<BatchPredictionJob[]>([]);
  let predictionPreview = $state<RealtimePredictionResponse | null>(null);
  let datasets = $state<Dataset[]>([]);

  let selectedExperimentId = $state('');
  let selectedModelId = $state('');
  let selectedFeatureId = $state('');
  let selectedDeploymentId = $state('');
  let compareRunIds = $state<string[]>([]);

  let loading = $state(true);
  let busy = $state(false);
  let uiError = $state('');

  let experimentDraft = $state<ExperimentDraft>(createEmptyExperimentDraft());
  let runDraft = $state<RunDraft>(createEmptyRunDraft());
  let modelDraft = $state<ModelDraft>(createEmptyModelDraft());
  let modelVersionDraft = $state<ModelVersionDraft>(createEmptyModelVersionDraft());
  let featureDraft = $state<FeatureDraft>(createEmptyFeatureDraft());
  let trainingJobDraft = $state<TrainingJobDraft>(createEmptyTrainingJobDraft());
  let deploymentDraft = $state<DeploymentDraft>(createEmptyDeploymentDraft());

  let realtimeInputText = $state('[\n  {\n    "usage_decline": 0.34,\n    "open_tickets": 6,\n    "resolution_hours": 14\n  }\n]');
  let batchInputText = $state('[\n  {\n    "usage_decline": 0.42,\n    "open_tickets": 8\n  },\n  {\n    "usage_decline": 0.11,\n    "open_tickets": 2\n  }\n]');
  let batchOutputDestination = $state('');
  let driftBaselineRows = $state('10000');
  let driftObservedRows = $state('12100');
  let autoRetrain = $state(false);

  function createEmptyExperimentDraft(): ExperimentDraft {
    return {
      name: 'New experiment',
      description: '',
      objective: '',
      task_type: 'classification',
      primary_metric: 'accuracy',
      status: 'active',
      tags_text: '',
    };
  }

  function createEmptyRunDraft(): RunDraft {
    return {
      name: 'manual-run',
      status: 'completed',
      params_text: '{\n  "learning_rate": 0.05,\n  "max_depth": 6\n}',
      metrics_text: '[\n  { "name": "accuracy", "value": 0.81 },\n  { "name": "f1", "value": 0.84 }\n]',
      artifacts_text: '[]',
      notes: '',
      source_dataset_ids_text: '',
    };
  }

  function createEmptyModelDraft(): ModelDraft {
    return {
      name: 'New model',
      description: '',
      problem_type: 'classification',
      status: 'active',
      tags_text: '',
    };
  }

  function createEmptyModelVersionDraft(): ModelVersionDraft {
    return {
      version_label: '',
      stage: 'candidate',
      artifact_uri: '',
      hyperparameters_text: '{\n  "learning_rate": 0.05,\n  "max_depth": 6\n}',
      metrics_text: '[\n  { "name": "accuracy", "value": 0.82 }\n]',
      schema_text: '{\n  "signature": "tabular"\n}',
    };
  }

  function createEmptyFeatureDraft(): FeatureDraft {
    return {
      name: 'new_feature',
      entity_name: 'account',
      data_type: 'float',
      description: '',
      status: 'active',
      offline_source: '',
      transformation: '',
      online_enabled: true,
      online_namespace: 'ml:features:account',
      batch_schedule: '0 * * * *',
      freshness_sla_minutes: 60,
      tags_text: '',
      samples_text: '[\n  { "entity_key": "acct_001", "value": 12.4 }\n]',
    };
  }

  function createEmptyTrainingJobDraft(): TrainingJobDraft {
    return {
      name: 'nightly-training-job',
      experiment_id: '',
      model_id: '',
      dataset_ids_text: '',
      objective_metric_name: 'accuracy',
      training_config_text: '{\n  "engine": "xgboost",\n  "workers": 3\n}',
      hyperparameter_search_text: '{\n  "method": "random-search",\n  "candidates": [\n    { "learning_rate": 0.03, "max_depth": 5 },\n    { "learning_rate": 0.05, "max_depth": 6 }\n  ]\n}',
      auto_register_model_version: true,
    };
  }

  function createEmptyDeploymentDraft(modelId = '', modelVersionId = ''): DeploymentDraft {
    return {
      model_id: modelId,
      name: 'new-deployment',
      status: 'active',
      strategy_type: 'single',
      endpoint_path: '/api/v1/ml/runtime/new-endpoint',
      traffic_split_text: formatJson([
        { model_version_id: modelVersionId, label: 'champion', allocation: 100 },
      ]),
      monitoring_window: '24h',
      baseline_dataset_id: '',
    };
  }

  function formatJson(value: unknown): string {
    return JSON.stringify(value, null, 2);
  }

  function parseTags(text: string): string[] {
    return text
      .split(',')
      .map((value) => value.trim())
      .filter(Boolean);
  }

  function parseIdList(text: string): string[] {
    return text
      .split(',')
      .map((value) => value.trim())
      .filter(Boolean);
  }

  function parseJsonText<T>(text: string, fallback: T): T {
    const trimmed = text.trim();
    if (!trimmed) return fallback;
    return JSON.parse(trimmed) as T;
  }

  function ensureSelection(currentId: string, ids: string[]): string {
    return ids.includes(currentId) ? currentId : (ids[0] ?? '');
  }

  function toMessage(cause: unknown, fallback: string): string {
    return cause instanceof Error ? cause.message : fallback;
  }

  function formatTimestamp(value: string | null): string {
    return value ? new Date(value).toLocaleString() : 'n/a';
  }

  function selectedExperiment(): Experiment | undefined {
    return experiments.find((experiment) => experiment.id === selectedExperimentId);
  }

  function selectedModel(): RegisteredModel | undefined {
    return models.find((model) => model.id === selectedModelId);
  }

  function selectedFeature(): FeatureDefinition | undefined {
    return features.find((feature) => feature.id === selectedFeatureId);
  }

  function selectedDeployment(): ModelDeployment | undefined {
    return deployments.find((deployment) => deployment.id === selectedDeploymentId);
  }

  function syncExperimentDraft() {
    const experiment = selectedExperiment();
    experimentDraft = experiment
      ? {
          id: experiment.id,
          name: experiment.name,
          description: experiment.description,
          objective: experiment.objective,
          task_type: experiment.task_type,
          primary_metric: experiment.primary_metric,
          status: experiment.status,
          tags_text: experiment.tags.join(', '),
        }
      : createEmptyExperimentDraft();
    runDraft = createEmptyRunDraft();
  }

  function syncModelDraft() {
    const model = selectedModel();
    modelDraft = model
      ? {
          id: model.id,
          name: model.name,
          description: model.description,
          problem_type: model.problem_type,
          status: model.status,
          tags_text: model.tags.join(', '),
        }
      : createEmptyModelDraft();
    modelVersionDraft = createEmptyModelVersionDraft();
  }

  function syncFeatureDraft() {
    const feature = selectedFeature();
    featureDraft = feature
      ? {
          id: feature.id,
          name: feature.name,
          entity_name: feature.entity_name,
          data_type: feature.data_type,
          description: feature.description,
          status: feature.status,
          offline_source: feature.offline_source,
          transformation: feature.transformation,
          online_enabled: feature.online_enabled,
          online_namespace: feature.online_namespace,
          batch_schedule: feature.batch_schedule,
          freshness_sla_minutes: feature.freshness_sla_minutes,
          tags_text: feature.tags.join(', '),
          samples_text: formatJson(feature.samples),
        }
      : createEmptyFeatureDraft();
  }

  function syncDeploymentDraft() {
    const deployment = selectedDeployment();
    if (deployment) {
      deploymentDraft = {
        id: deployment.id,
        model_id: deployment.model_id,
        name: deployment.name,
        status: deployment.status,
        strategy_type: deployment.strategy_type,
        endpoint_path: deployment.endpoint_path,
        traffic_split_text: formatJson(deployment.traffic_split),
        monitoring_window: deployment.monitoring_window,
        baseline_dataset_id: deployment.baseline_dataset_id ?? '',
      };
      return;
    }

    deploymentDraft = createEmptyDeploymentDraft(
      selectedModelId || models[0]?.id || '',
      modelVersions[0]?.id || '',
    );
  }

  async function loadRunsForSelection() {
    runs = selectedExperimentId ? (await listRuns(selectedExperimentId)).data : [];
    compareRunIds = [];
    comparedRuns = null;
  }

  async function loadVersionsForSelection() {
    modelVersions = selectedModelId ? (await listModelVersions(selectedModelId)).data : [];
  }

  async function loadOnlineFeatureForSelection() {
    if (!selectedFeatureId) {
      onlineFeature = null;
      return;
    }

    try {
      onlineFeature = await getOnlineFeatureSnapshot(selectedFeatureId);
    } catch {
      onlineFeature = null;
    }
  }

  async function loadAll() {
    loading = true;
    uiError = '';

    try {
      const [
        overviewResponse,
        experimentsResponse,
        modelsResponse,
        featuresResponse,
        trainingJobsResponse,
        deploymentsResponse,
        batchPredictionsResponse,
        datasetsResponse,
      ] = await Promise.all([
        getOverview(),
        listExperiments(),
        listModels(),
        listFeatures(),
        listTrainingJobs(),
        listDeployments(),
        listBatchPredictions(),
        listDatasets({ per_page: 100 }),
      ]);

      overview = overviewResponse;
      experiments = experimentsResponse.data;
      models = modelsResponse.data;
      features = featuresResponse.data;
      trainingJobs = trainingJobsResponse.data;
      deployments = deploymentsResponse.data;
      batchPredictions = batchPredictionsResponse.data;
      datasets = datasetsResponse.data;

      selectedExperimentId = ensureSelection(selectedExperimentId, experiments.map((experiment) => experiment.id));
      selectedModelId = ensureSelection(selectedModelId, models.map((model) => model.id));
      selectedFeatureId = ensureSelection(selectedFeatureId, features.map((feature) => feature.id));
      selectedDeploymentId = ensureSelection(selectedDeploymentId, deployments.map((deployment) => deployment.id));

      syncExperimentDraft();
      syncModelDraft();
      syncFeatureDraft();

      await Promise.all([
        loadRunsForSelection(),
        loadVersionsForSelection(),
        loadOnlineFeatureForSelection(),
      ]);

      syncDeploymentDraft();

      if (!trainingJobDraft.experiment_id) {
        trainingJobDraft.experiment_id = selectedExperimentId;
      }
      if (!trainingJobDraft.model_id) {
        trainingJobDraft.model_id = selectedModelId;
      }
    } catch (cause) {
      uiError = toMessage(cause, 'Failed to load ML Studio');
    } finally {
      loading = false;
    }
  }

  async function selectExperiment(id: string) {
    selectedExperimentId = id;
    syncExperimentDraft();
    await loadRunsForSelection();
  }

  async function selectModel(id: string) {
    selectedModelId = id;
    syncModelDraft();
    await loadVersionsForSelection();
    syncDeploymentDraft();
  }

  async function selectFeature(id: string) {
    selectedFeatureId = id;
    syncFeatureDraft();
    await loadOnlineFeatureForSelection();
  }

  function selectDeployment(id: string) {
    selectedDeploymentId = id;
    syncDeploymentDraft();
    predictionPreview = null;
  }

  function newExperiment() {
    selectedExperimentId = '';
    experimentDraft = createEmptyExperimentDraft();
    runs = [];
    comparedRuns = null;
    compareRunIds = [];
  }

  function newModel() {
    selectedModelId = '';
    modelDraft = createEmptyModelDraft();
    modelVersions = [];
  }

  function newFeature() {
    selectedFeatureId = '';
    featureDraft = createEmptyFeatureDraft();
    onlineFeature = null;
  }

  function newDeployment() {
    selectedDeploymentId = '';
    deploymentDraft = createEmptyDeploymentDraft(selectedModelId, modelVersions[0]?.id || '');
    predictionPreview = null;
  }

  async function saveExperiment() {
    busy = true;
    uiError = '';

    try {
      const payload = {
        name: experimentDraft.name,
        description: experimentDraft.description,
        objective: experimentDraft.objective,
        task_type: experimentDraft.task_type,
        primary_metric: experimentDraft.primary_metric,
        status: experimentDraft.status,
        tags: parseTags(experimentDraft.tags_text),
      };

      const experiment = experimentDraft.id
        ? await updateExperiment(experimentDraft.id, payload)
        : await createExperiment(payload);

      selectedExperimentId = experiment.id;
      notifications.success(`Experiment ${experimentDraft.id ? 'updated' : 'created'}`);
      await loadAll();
    } catch (cause) {
      uiError = toMessage(cause, 'Failed to save experiment');
    } finally {
      busy = false;
    }
  }

  async function logRun() {
    if (!selectedExperimentId) {
      notifications.warning('Select an experiment before logging a run');
      return;
    }

    busy = true;
    uiError = '';

    try {
      await createRun(selectedExperimentId, {
        name: runDraft.name,
        status: runDraft.status,
        params: parseJsonText<Record<string, unknown>>(runDraft.params_text, {}),
        metrics: parseJsonText<MetricValue[]>(runDraft.metrics_text, []),
        artifacts: parseJsonText<ArtifactReference[]>(runDraft.artifacts_text, []),
        notes: runDraft.notes,
        source_dataset_ids: parseIdList(runDraft.source_dataset_ids_text),
      });

      notifications.success('Run logged');
      runDraft = createEmptyRunDraft();
      await loadAll();
    } catch (cause) {
      uiError = toMessage(cause, 'Failed to log run');
    } finally {
      busy = false;
    }
  }

  async function compareSelectedRunsAction() {
    if (compareRunIds.length < 2) {
      notifications.warning('Select at least two runs to compare');
      return;
    }

    busy = true;
    uiError = '';

    try {
      comparedRuns = await compareRuns(compareRunIds);
    } catch (cause) {
      uiError = toMessage(cause, 'Failed to compare runs');
    } finally {
      busy = false;
    }
  }

  function toggleCompareRun(runId: string, checked: boolean) {
    compareRunIds = checked
      ? [...compareRunIds, runId]
      : compareRunIds.filter((candidate) => candidate !== runId);
  }

  async function saveModel() {
    busy = true;
    uiError = '';

    try {
      const payload = {
        name: modelDraft.name,
        description: modelDraft.description,
        problem_type: modelDraft.problem_type,
        status: modelDraft.status,
        tags: parseTags(modelDraft.tags_text),
      };

      const model = modelDraft.id
        ? await updateModel(modelDraft.id, payload)
        : await createModel(payload);

      selectedModelId = model.id;
      notifications.success(`Model ${modelDraft.id ? 'updated' : 'created'}`);
      await loadAll();
    } catch (cause) {
      uiError = toMessage(cause, 'Failed to save model');
    } finally {
      busy = false;
    }
  }

  async function saveModelVersionAction() {
    if (!selectedModelId) {
      notifications.warning('Select a model before creating a version');
      return;
    }

    busy = true;
    uiError = '';

    try {
      await createModelVersion(selectedModelId, {
        version_label: modelVersionDraft.version_label || undefined,
        stage: modelVersionDraft.stage,
        artifact_uri: modelVersionDraft.artifact_uri || null,
        hyperparameters: parseJsonText<Record<string, unknown>>(modelVersionDraft.hyperparameters_text, {}),
        metrics: parseJsonText<MetricValue[]>(modelVersionDraft.metrics_text, []),
        schema: parseJsonText<Record<string, unknown>>(modelVersionDraft.schema_text, {}),
      });

      notifications.success('Model version registered');
      modelVersionDraft = createEmptyModelVersionDraft();
      await loadAll();
    } catch (cause) {
      uiError = toMessage(cause, 'Failed to create model version');
    } finally {
      busy = false;
    }
  }

  async function transitionVersionAction(versionId: string, stage: string) {
    busy = true;
    uiError = '';

    try {
      await transitionModelVersion(versionId, stage);
      notifications.success(`Model version moved to ${stage}`);
      await loadAll();
    } catch (cause) {
      uiError = toMessage(cause, 'Failed to transition model version');
    } finally {
      busy = false;
    }
  }

  async function saveFeature() {
    busy = true;
    uiError = '';

    try {
      const payload = {
        name: featureDraft.name,
        entity_name: featureDraft.entity_name,
        data_type: featureDraft.data_type,
        description: featureDraft.description,
        status: featureDraft.status,
        offline_source: featureDraft.offline_source,
        transformation: featureDraft.transformation,
        online_enabled: featureDraft.online_enabled,
        online_namespace: featureDraft.online_namespace,
        batch_schedule: featureDraft.batch_schedule,
        freshness_sla_minutes: Number(featureDraft.freshness_sla_minutes),
        tags: parseTags(featureDraft.tags_text),
        samples: parseJsonText<FeatureSample[]>(featureDraft.samples_text, []),
      };

      const feature = featureDraft.id
        ? await updateFeature(featureDraft.id, payload)
        : await createFeature(payload);

      selectedFeatureId = feature.id;
      notifications.success(`Feature ${featureDraft.id ? 'updated' : 'created'}`);
      await loadAll();
    } catch (cause) {
      uiError = toMessage(cause, 'Failed to save feature');
    } finally {
      busy = false;
    }
  }

  async function materializeSelectedFeature() {
    if (!selectedFeatureId) {
      notifications.warning('Select a feature before materializing it');
      return;
    }

    busy = true;
    uiError = '';

    try {
      await materializeFeature(selectedFeatureId, {
        samples: parseJsonText<FeatureSample[]>(featureDraft.samples_text, []),
        mode: 'manual',
      });
      notifications.success('Feature materialized');
      await loadAll();
    } catch (cause) {
      uiError = toMessage(cause, 'Failed to materialize feature');
    } finally {
      busy = false;
    }
  }

  async function createTrainingJobAction() {
    busy = true;
    uiError = '';

    try {
      await createTrainingJob({
        experiment_id: trainingJobDraft.experiment_id || null,
        model_id: trainingJobDraft.model_id || null,
        name: trainingJobDraft.name,
        dataset_ids: parseIdList(trainingJobDraft.dataset_ids_text),
        training_config: parseJsonText<Record<string, unknown>>(trainingJobDraft.training_config_text, {}),
        hyperparameter_search: parseJsonText<Record<string, unknown>>(trainingJobDraft.hyperparameter_search_text, {}),
        objective_metric_name: trainingJobDraft.objective_metric_name,
        auto_register_model_version: trainingJobDraft.auto_register_model_version,
      });

      notifications.success('Training job submitted');
      trainingJobDraft = createEmptyTrainingJobDraft();
      trainingJobDraft.experiment_id = selectedExperimentId;
      trainingJobDraft.model_id = selectedModelId;
      await loadAll();
    } catch (cause) {
      uiError = toMessage(cause, 'Failed to create training job');
    } finally {
      busy = false;
    }
  }

  async function saveDeployment() {
    busy = true;
    uiError = '';

    try {
      const payload = {
        model_id: deploymentDraft.model_id,
        name: deploymentDraft.name,
        status: deploymentDraft.status,
        strategy_type: deploymentDraft.strategy_type,
        endpoint_path: deploymentDraft.endpoint_path,
        traffic_split: parseJsonText<TrafficSplitEntry[]>(deploymentDraft.traffic_split_text, []),
        monitoring_window: deploymentDraft.monitoring_window,
        baseline_dataset_id: deploymentDraft.baseline_dataset_id || null,
      };

      const deployment = deploymentDraft.id
        ? await updateDeployment(deploymentDraft.id, payload)
        : await createDeployment(payload);

      selectedDeploymentId = deployment.id;
      notifications.success(`Deployment ${deploymentDraft.id ? 'updated' : 'created'}`);
      await loadAll();
    } catch (cause) {
      uiError = toMessage(cause, 'Failed to save deployment');
    } finally {
      busy = false;
    }
  }

  async function evaluateDriftAction() {
    if (!selectedDeploymentId) {
      notifications.warning('Select a deployment before running drift checks');
      return;
    }

    busy = true;
    uiError = '';

    try {
      await generateDriftReport(selectedDeploymentId, {
        baseline_rows: Number(driftBaselineRows) || 10_000,
        observed_rows: Number(driftObservedRows) || 10_000,
        auto_retrain: autoRetrain,
      });

      notifications.success('Drift report generated');
      await loadAll();
    } catch (cause) {
      uiError = toMessage(cause, 'Failed to generate drift report');
    } finally {
      busy = false;
    }
  }

  async function predictRealtimeAction() {
    if (!selectedDeploymentId) {
      notifications.warning('Select a deployment before testing predictions');
      return;
    }

    busy = true;
    uiError = '';

    try {
      predictionPreview = await realtimePredict(selectedDeploymentId, {
        inputs: parseJsonText<unknown[]>(realtimeInputText, []),
        explain: true,
      });
    } catch (cause) {
      uiError = toMessage(cause, 'Failed to run realtime prediction');
    } finally {
      busy = false;
    }
  }

  async function runBatchPredictionAction() {
    if (!selectedDeploymentId) {
      notifications.warning('Select a deployment before starting a batch prediction');
      return;
    }

    busy = true;
    uiError = '';

    try {
      await createBatchPrediction({
        deployment_id: selectedDeploymentId,
        records: parseJsonText<unknown[]>(batchInputText, []),
        output_destination: batchOutputDestination || null,
      });

      notifications.success('Batch prediction completed');
      await loadAll();
    } catch (cause) {
      uiError = toMessage(cause, 'Failed to run batch prediction');
    } finally {
      busy = false;
    }
  }

  onMount(() => {
    void loadAll();
  });
</script>

<svelte:head>
  <title>OpenFoundry — ML Studio</title>
</svelte:head>

{#if loading}
  <div class="mx-auto max-w-7xl rounded-[2rem] border border-dashed border-slate-300 px-6 py-24 text-center text-sm text-slate-500 dark:border-slate-700">
    Loading ML Studio...
  </div>
{:else}
  <div class="mx-auto max-w-7xl space-y-8">
    <section class="rounded-[2rem] border border-slate-200 bg-[linear-gradient(135deg,_rgba(14,165,233,0.14),_rgba(16,185,129,0.12)_45%,_rgba(255,255,255,0.94)_100%)] p-8 shadow-sm dark:border-slate-800 dark:bg-[linear-gradient(135deg,_rgba(14,165,233,0.24),_rgba(16,185,129,0.18)_45%,_rgba(15,23,42,0.94)_100%)]">
      <div class="flex flex-wrap items-start justify-between gap-4">
        <div class="max-w-3xl space-y-3">
          <div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Milestone 3.2</div>
          <h1 class="text-4xl font-semibold tracking-tight text-slate-950 dark:text-slate-50">ML Studio</h1>
          <p class="text-base text-slate-600 dark:text-slate-300">
            Track experiments, tune training jobs, register model versions, materialize online features, run A/B deployments, and inspect drift and prediction flows from a single control plane.
          </p>
        </div>

        <button
          type="button"
          class="rounded-2xl bg-slate-900 px-5 py-3 text-sm font-medium text-white hover:bg-slate-800 disabled:opacity-50 dark:bg-slate-100 dark:text-slate-950 dark:hover:bg-white"
          onclick={() => void loadAll()}
          disabled={busy}
        >
          Refresh Studio
        </button>
      </div>
    </section>

    {#if uiError}
      <div class="rounded-2xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-700 dark:border-rose-900/40 dark:bg-rose-950/20 dark:text-rose-300">{uiError}</div>
    {/if}

    {#if overview}
      <section class="grid gap-4 sm:grid-cols-2 xl:grid-cols-5">
        <div class="rounded-2xl border border-slate-200 bg-white p-5 shadow-sm dark:border-slate-800 dark:bg-slate-950">
          <div class="text-xs uppercase tracking-[0.2em] text-slate-400">Experiments</div>
          <div class="mt-3 text-3xl font-semibold text-slate-950 dark:text-slate-50">{overview.experiment_count}</div>
          <div class="mt-1 text-sm text-slate-500">{overview.active_run_count} tracked runs</div>
        </div>
        <div class="rounded-2xl border border-slate-200 bg-white p-5 shadow-sm dark:border-slate-800 dark:bg-slate-950">
          <div class="text-xs uppercase tracking-[0.2em] text-slate-400">Registry</div>
          <div class="mt-3 text-3xl font-semibold text-slate-950 dark:text-slate-50">{overview.model_count}</div>
          <div class="mt-1 text-sm text-slate-500">{overview.production_model_count} production version(s)</div>
        </div>
        <div class="rounded-2xl border border-slate-200 bg-white p-5 shadow-sm dark:border-slate-800 dark:bg-slate-950">
          <div class="text-xs uppercase tracking-[0.2em] text-slate-400">Feature Store</div>
          <div class="mt-3 text-3xl font-semibold text-slate-950 dark:text-slate-50">{overview.feature_count}</div>
          <div class="mt-1 text-sm text-slate-500">{overview.online_feature_count} online features</div>
        </div>
        <div class="rounded-2xl border border-slate-200 bg-white p-5 shadow-sm dark:border-slate-800 dark:bg-slate-950">
          <div class="text-xs uppercase tracking-[0.2em] text-slate-400">Serving</div>
          <div class="mt-3 text-3xl font-semibold text-slate-950 dark:text-slate-50">{overview.deployment_count}</div>
          <div class="mt-1 text-sm text-slate-500">{overview.ab_test_count} live A/B deployment(s)</div>
        </div>
        <div class="rounded-2xl border border-slate-200 bg-white p-5 shadow-sm dark:border-slate-800 dark:bg-slate-950">
          <div class="text-xs uppercase tracking-[0.2em] text-slate-400">Monitoring</div>
          <div class="mt-3 text-3xl font-semibold text-slate-950 dark:text-slate-50">{overview.drift_alert_count}</div>
          <div class="mt-1 text-sm text-slate-500">{overview.queued_training_jobs} queued/running retrain jobs</div>
        </div>
      </section>
    {/if}

    <section class="rounded-[1.75rem] border border-slate-200 bg-white p-6 shadow-sm dark:border-slate-800 dark:bg-slate-950">
      <div class="mb-5 flex flex-wrap items-center justify-between gap-3">
        <div>
          <div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-400">Experiment Tracking</div>
          <h2 class="mt-1 text-2xl font-semibold text-slate-950 dark:text-slate-50">Experiments, runs, metrics, and artifacts</h2>
        </div>
        <button type="button" class="rounded-xl border border-slate-200 px-4 py-2 text-sm hover:bg-slate-50 dark:border-slate-700 dark:hover:bg-slate-900" onclick={newExperiment}>New experiment</button>
      </div>

      <div class="grid gap-6 xl:grid-cols-[280px,1fr]">
        <div class="space-y-3">
          {#if experiments.length === 0}
            <div class="rounded-2xl border border-dashed border-slate-300 px-4 py-10 text-center text-sm text-slate-500 dark:border-slate-700">No experiments yet.</div>
          {:else}
            {#each experiments as experiment (experiment.id)}
              <button
                type="button"
                class={`w-full rounded-2xl border p-4 text-left transition ${selectedExperimentId === experiment.id ? 'border-sky-500 bg-sky-50 dark:bg-sky-950/20' : 'border-slate-200 hover:bg-slate-50 dark:border-slate-700 dark:hover:bg-slate-900'}`}
                onclick={() => void selectExperiment(experiment.id)}
              >
                <div class="flex items-start justify-between gap-3">
                  <div>
                    <div class="font-medium text-slate-900 dark:text-slate-100">{experiment.name}</div>
                    <div class="mt-1 text-sm text-slate-500">{experiment.task_type}</div>
                  </div>
                  <span class="rounded-full border border-slate-200 px-2 py-1 text-[11px] uppercase tracking-[0.18em] text-slate-500 dark:border-slate-700">{experiment.status}</span>
                </div>
                <div class="mt-3 flex flex-wrap gap-2 text-xs text-slate-400">
                  <span>{experiment.run_count} run(s)</span>
                  {#if experiment.best_metric}
                    <span>{experiment.best_metric.name}: {experiment.best_metric.value}</span>
                  {/if}
                </div>
              </button>
            {/each}
          {/if}
        </div>

        <div class="space-y-6">
          <div class="grid gap-4 md:grid-cols-2">
            <label class="space-y-1 text-sm">
              <span class="font-medium text-slate-600 dark:text-slate-300">Name</span>
              <input bind:value={experimentDraft.name} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-slate-700 dark:bg-slate-900" />
            </label>
            <label class="space-y-1 text-sm">
              <span class="font-medium text-slate-600 dark:text-slate-300">Primary metric</span>
              <input bind:value={experimentDraft.primary_metric} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-slate-700 dark:bg-slate-900" />
            </label>
            <label class="space-y-1 text-sm md:col-span-2">
              <span class="font-medium text-slate-600 dark:text-slate-300">Objective</span>
              <textarea rows="2" bind:value={experimentDraft.objective} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-slate-700 dark:bg-slate-900"></textarea>
            </label>
            <label class="space-y-1 text-sm md:col-span-2">
              <span class="font-medium text-slate-600 dark:text-slate-300">Description</span>
              <textarea rows="3" bind:value={experimentDraft.description} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-slate-700 dark:bg-slate-900"></textarea>
            </label>
            <label class="space-y-1 text-sm">
              <span class="font-medium text-slate-600 dark:text-slate-300">Task type</span>
              <select bind:value={experimentDraft.task_type} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-slate-700 dark:bg-slate-900">
                <option value="classification">Classification</option>
                <option value="regression">Regression</option>
                <option value="ranking">Ranking</option>
              </select>
            </label>
            <label class="space-y-1 text-sm">
              <span class="font-medium text-slate-600 dark:text-slate-300">Status</span>
              <select bind:value={experimentDraft.status} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-slate-700 dark:bg-slate-900">
                <option value="active">Active</option>
                <option value="archived">Archived</option>
              </select>
            </label>
            <label class="space-y-1 text-sm md:col-span-2">
              <span class="font-medium text-slate-600 dark:text-slate-300">Tags</span>
              <input bind:value={experimentDraft.tags_text} placeholder="retention, b2b" class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-slate-700 dark:bg-slate-900" />
            </label>
          </div>

          <div class="flex gap-3">
            <button type="button" class="rounded-xl bg-slate-900 px-4 py-2 text-sm font-medium text-white hover:bg-slate-800 disabled:opacity-50 dark:bg-slate-100 dark:text-slate-950 dark:hover:bg-white" onclick={() => void saveExperiment()} disabled={busy}>
              {experimentDraft.id ? 'Save experiment' : 'Create experiment'}
            </button>
          </div>

          <div class="rounded-[1.5rem] border border-slate-200 p-4 dark:border-slate-800">
            <div class="mb-4 flex items-center justify-between gap-3">
              <div>
                <div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-400">Runs</div>
                <div class="mt-1 text-sm text-slate-500">Log params, metrics, and artifact manifests for the selected experiment.</div>
              </div>
            </div>

            <div class="grid gap-4 lg:grid-cols-2">
              <div class="space-y-3">
                <label class="space-y-1 text-sm">
                  <span class="font-medium text-slate-600 dark:text-slate-300">Run name</span>
                  <input bind:value={runDraft.name} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-slate-700 dark:bg-slate-900" />
                </label>
                <label class="space-y-1 text-sm">
                  <span class="font-medium text-slate-600 dark:text-slate-300">Status</span>
                  <select bind:value={runDraft.status} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-slate-700 dark:bg-slate-900">
                    <option value="completed">Completed</option>
                    <option value="running">Running</option>
                    <option value="queued">Queued</option>
                    <option value="failed">Failed</option>
                  </select>
                </label>
                <label class="space-y-1 text-sm">
                  <span class="font-medium text-slate-600 dark:text-slate-300">Source dataset ids</span>
                  <input bind:value={runDraft.source_dataset_ids_text} placeholder="uuid-1, uuid-2" class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-slate-700 dark:bg-slate-900" />
                </label>
                <label class="space-y-1 text-sm">
                  <span class="font-medium text-slate-600 dark:text-slate-300">Notes</span>
                  <textarea rows="3" bind:value={runDraft.notes} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-slate-700 dark:bg-slate-900"></textarea>
                </label>
              </div>

              <div class="space-y-3">
                <label class="space-y-1 text-sm">
                  <span class="font-medium text-slate-600 dark:text-slate-300">Params JSON</span>
                  <textarea rows="4" bind:value={runDraft.params_text} class="w-full rounded-xl border border-slate-200 px-3 py-2 font-mono text-xs dark:border-slate-700 dark:bg-slate-900"></textarea>
                </label>
                <label class="space-y-1 text-sm">
                  <span class="font-medium text-slate-600 dark:text-slate-300">Metrics JSON</span>
                  <textarea rows="4" bind:value={runDraft.metrics_text} class="w-full rounded-xl border border-slate-200 px-3 py-2 font-mono text-xs dark:border-slate-700 dark:bg-slate-900"></textarea>
                </label>
                <label class="space-y-1 text-sm">
                  <span class="font-medium text-slate-600 dark:text-slate-300">Artifacts JSON</span>
                  <textarea rows="4" bind:value={runDraft.artifacts_text} class="w-full rounded-xl border border-slate-200 px-3 py-2 font-mono text-xs dark:border-slate-700 dark:bg-slate-900"></textarea>
                </label>
              </div>
            </div>

            <div class="mt-4 flex gap-3">
              <button type="button" class="rounded-xl bg-sky-600 px-4 py-2 text-sm font-medium text-white hover:bg-sky-500 disabled:opacity-50" onclick={() => void logRun()} disabled={busy || !selectedExperimentId}>
                Log run
              </button>
              <button type="button" class="rounded-xl border border-slate-200 px-4 py-2 text-sm hover:bg-slate-50 disabled:opacity-50 dark:border-slate-700 dark:hover:bg-slate-900" onclick={() => void compareSelectedRunsAction()} disabled={busy || compareRunIds.length < 2}>
                Compare selected runs
              </button>
            </div>

            <div class="mt-6 grid gap-4 lg:grid-cols-[1fr,1fr]">
              <div class="space-y-3">
                {#if runs.length === 0}
                  <div class="rounded-2xl border border-dashed border-slate-300 px-4 py-10 text-center text-sm text-slate-500 dark:border-slate-700">No runs logged yet.</div>
                {:else}
                  {#each runs as run (run.id)}
                    <div class="rounded-2xl border border-slate-200 p-4 dark:border-slate-700">
                      <div class="flex items-start justify-between gap-3">
                        <div>
                          <div class="font-medium text-slate-900 dark:text-slate-100">{run.name}</div>
                          <div class="mt-1 text-xs text-slate-500">{formatTimestamp(run.started_at)} → {formatTimestamp(run.finished_at)}</div>
                        </div>
                        <label class="flex items-center gap-2 text-xs text-slate-500">
                          <input
                            type="checkbox"
                            checked={compareRunIds.includes(run.id)}
                            onchange={(event) => toggleCompareRun(run.id, (event.currentTarget as HTMLInputElement).checked)}
                          />
                          compare
                        </label>
                      </div>
                      <div class="mt-3 flex flex-wrap gap-2 text-xs text-slate-400">
                        <span class="rounded-full border border-slate-200 px-2 py-1 dark:border-slate-700">{run.status}</span>
                        {#each run.metrics as metric}
                          <span>{metric.name}: {metric.value}</span>
                        {/each}
                      </div>
                    </div>
                  {/each}
                {/if}
              </div>

              <div>
                {#if comparedRuns}
                  <div class="overflow-x-auto rounded-2xl border border-slate-200 dark:border-slate-700">
                    <table class="min-w-full text-sm">
                      <thead class="bg-slate-50 dark:bg-slate-900">
                        <tr>
                          <th class="px-3 py-2 text-left font-medium text-slate-500">Run</th>
                          {#each comparedRuns.metric_names as metricName}
                            <th class="px-3 py-2 text-left font-medium text-slate-500">{metricName}</th>
                          {/each}
                        </tr>
                      </thead>
                      <tbody>
                        {#each comparedRuns.data as run (run.id)}
                          <tr class="border-t border-slate-200 dark:border-slate-800">
                            <td class="px-3 py-2 font-medium text-slate-900 dark:text-slate-100">{run.name}</td>
                            {#each comparedRuns.metric_names as metricName}
                              <td class="px-3 py-2 text-slate-600 dark:text-slate-300">{run.metrics.find((metric) => metric.name === metricName)?.value ?? '—'}</td>
                            {/each}
                          </tr>
                        {/each}
                      </tbody>
                    </table>
                  </div>
                {:else}
                  <div class="rounded-2xl border border-dashed border-slate-300 px-4 py-10 text-center text-sm text-slate-500 dark:border-slate-700">Select two or more runs to compare primary metrics side by side.</div>
                {/if}
              </div>
            </div>
          </div>
        </div>
      </div>
    </section>

    <section class="rounded-[1.75rem] border border-slate-200 bg-white p-6 shadow-sm dark:border-slate-800 dark:bg-slate-950">
      <div class="mb-5 flex flex-wrap items-center justify-between gap-3">
        <div>
          <div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-400">Model Registry</div>
          <h2 class="mt-1 text-2xl font-semibold text-slate-950 dark:text-slate-50">Versions, stages, and promotion flows</h2>
        </div>
        <button type="button" class="rounded-xl border border-slate-200 px-4 py-2 text-sm hover:bg-slate-50 dark:border-slate-700 dark:hover:bg-slate-900" onclick={newModel}>New model</button>
      </div>

      <div class="grid gap-6 xl:grid-cols-[280px,1fr]">
        <div class="space-y-3">
          {#if models.length === 0}
            <div class="rounded-2xl border border-dashed border-slate-300 px-4 py-10 text-center text-sm text-slate-500 dark:border-slate-700">No models registered yet.</div>
          {:else}
            {#each models as model (model.id)}
              <button
                type="button"
                class={`w-full rounded-2xl border p-4 text-left transition ${selectedModelId === model.id ? 'border-emerald-500 bg-emerald-50 dark:bg-emerald-950/20' : 'border-slate-200 hover:bg-slate-50 dark:border-slate-700 dark:hover:bg-slate-900'}`}
                onclick={() => void selectModel(model.id)}
              >
                <div class="flex items-start justify-between gap-3">
                  <div>
                    <div class="font-medium text-slate-900 dark:text-slate-100">{model.name}</div>
                    <div class="mt-1 text-sm text-slate-500">{model.problem_type}</div>
                  </div>
                  <span class="rounded-full border border-slate-200 px-2 py-1 text-[11px] uppercase tracking-[0.18em] text-slate-500 dark:border-slate-700">{model.current_stage}</span>
                </div>
                <div class="mt-3 text-xs text-slate-400">Latest version: {model.latest_version_number ?? 'n/a'}</div>
              </button>
            {/each}
          {/if}
        </div>

        <div class="space-y-6">
          <div class="grid gap-4 md:grid-cols-2">
            <label class="space-y-1 text-sm">
              <span class="font-medium text-slate-600 dark:text-slate-300">Name</span>
              <input bind:value={modelDraft.name} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-slate-700 dark:bg-slate-900" />
            </label>
            <label class="space-y-1 text-sm">
              <span class="font-medium text-slate-600 dark:text-slate-300">Problem type</span>
              <select bind:value={modelDraft.problem_type} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-slate-700 dark:bg-slate-900">
                <option value="classification">Classification</option>
                <option value="regression">Regression</option>
                <option value="ranking">Ranking</option>
              </select>
            </label>
            <label class="space-y-1 text-sm md:col-span-2">
              <span class="font-medium text-slate-600 dark:text-slate-300">Description</span>
              <textarea rows="3" bind:value={modelDraft.description} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-slate-700 dark:bg-slate-900"></textarea>
            </label>
            <label class="space-y-1 text-sm">
              <span class="font-medium text-slate-600 dark:text-slate-300">Status</span>
              <select bind:value={modelDraft.status} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-slate-700 dark:bg-slate-900">
                <option value="active">Active</option>
                <option value="archived">Archived</option>
              </select>
            </label>
            <label class="space-y-1 text-sm">
              <span class="font-medium text-slate-600 dark:text-slate-300">Tags</span>
              <input bind:value={modelDraft.tags_text} placeholder="xgboost, champion" class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-slate-700 dark:bg-slate-900" />
            </label>
          </div>

          <div class="flex gap-3">
            <button type="button" class="rounded-xl bg-slate-900 px-4 py-2 text-sm font-medium text-white hover:bg-slate-800 disabled:opacity-50 dark:bg-slate-100 dark:text-slate-950 dark:hover:bg-white" onclick={() => void saveModel()} disabled={busy}>
              {modelDraft.id ? 'Save model' : 'Create model'}
            </button>
          </div>

          <div class="rounded-[1.5rem] border border-slate-200 p-4 dark:border-slate-800">
            <div class="mb-4 flex items-center justify-between gap-3">
              <div>
                <div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-400">Versioning</div>
                <div class="mt-1 text-sm text-slate-500">Register new artifacts and transition them across candidate, staging, and production.</div>
              </div>
            </div>

            <div class="grid gap-4 lg:grid-cols-2">
              <div class="space-y-3">
                <label class="space-y-1 text-sm">
                  <span class="font-medium text-slate-600 dark:text-slate-300">Version label</span>
                  <input bind:value={modelVersionDraft.version_label} placeholder="v3-gradient-boosting" class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-slate-700 dark:bg-slate-900" />
                </label>
                <label class="space-y-1 text-sm">
                  <span class="font-medium text-slate-600 dark:text-slate-300">Stage</span>
                  <select bind:value={modelVersionDraft.stage} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-slate-700 dark:bg-slate-900">
                    <option value="candidate">Candidate</option>
                    <option value="staging">Staging</option>
                    <option value="production">Production</option>
                    <option value="archived">Archived</option>
                  </select>
                </label>
                <label class="space-y-1 text-sm">
                  <span class="font-medium text-slate-600 dark:text-slate-300">Artifact URI</span>
                  <input bind:value={modelVersionDraft.artifact_uri} placeholder="ml://models/churn/v3" class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-slate-700 dark:bg-slate-900" />
                </label>
              </div>

              <div class="space-y-3">
                <label class="space-y-1 text-sm">
                  <span class="font-medium text-slate-600 dark:text-slate-300">Hyperparameters JSON</span>
                  <textarea rows="4" bind:value={modelVersionDraft.hyperparameters_text} class="w-full rounded-xl border border-slate-200 px-3 py-2 font-mono text-xs dark:border-slate-700 dark:bg-slate-900"></textarea>
                </label>
                <label class="space-y-1 text-sm">
                  <span class="font-medium text-slate-600 dark:text-slate-300">Metrics JSON</span>
                  <textarea rows="4" bind:value={modelVersionDraft.metrics_text} class="w-full rounded-xl border border-slate-200 px-3 py-2 font-mono text-xs dark:border-slate-700 dark:bg-slate-900"></textarea>
                </label>
                <label class="space-y-1 text-sm">
                  <span class="font-medium text-slate-600 dark:text-slate-300">Schema JSON</span>
                  <textarea rows="3" bind:value={modelVersionDraft.schema_text} class="w-full rounded-xl border border-slate-200 px-3 py-2 font-mono text-xs dark:border-slate-700 dark:bg-slate-900"></textarea>
                </label>
              </div>
            </div>

            <div class="mt-4">
              <button type="button" class="rounded-xl bg-emerald-600 px-4 py-2 text-sm font-medium text-white hover:bg-emerald-500 disabled:opacity-50" onclick={() => void saveModelVersionAction()} disabled={busy || !selectedModelId}>
                Register version
              </button>
            </div>

            <div class="mt-6 space-y-3">
              {#if modelVersions.length === 0}
                <div class="rounded-2xl border border-dashed border-slate-300 px-4 py-10 text-center text-sm text-slate-500 dark:border-slate-700">No versions registered yet.</div>
              {:else}
                {#each modelVersions as version (version.id)}
                  <div class="rounded-2xl border border-slate-200 p-4 dark:border-slate-700">
                    <div class="flex flex-wrap items-start justify-between gap-3">
                      <div>
                        <div class="font-medium text-slate-900 dark:text-slate-100">{version.version_label} (v{version.version_number})</div>
                        <div class="mt-1 text-sm text-slate-500">Artifact: {version.artifact_uri ?? 'n/a'}</div>
                      </div>
                      <span class="rounded-full border border-slate-200 px-3 py-1 text-[11px] uppercase tracking-[0.18em] text-slate-500 dark:border-slate-700">{version.stage}</span>
                    </div>
                    <div class="mt-3 flex flex-wrap gap-2 text-xs text-slate-400">
                      {#each version.metrics as metric}
                        <span>{metric.name}: {metric.value}</span>
                      {/each}
                    </div>
                    <div class="mt-4 flex flex-wrap gap-2">
                      <button type="button" class="rounded-lg border border-slate-200 px-3 py-2 text-xs hover:bg-slate-50 dark:border-slate-700 dark:hover:bg-slate-900" onclick={() => void transitionVersionAction(version.id, 'candidate')} disabled={busy}>Candidate</button>
                      <button type="button" class="rounded-lg border border-slate-200 px-3 py-2 text-xs hover:bg-slate-50 dark:border-slate-700 dark:hover:bg-slate-900" onclick={() => void transitionVersionAction(version.id, 'staging')} disabled={busy}>Staging</button>
                      <button type="button" class="rounded-lg border border-slate-200 px-3 py-2 text-xs hover:bg-slate-50 dark:border-slate-700 dark:hover:bg-slate-900" onclick={() => void transitionVersionAction(version.id, 'production')} disabled={busy}>Production</button>
                      <button type="button" class="rounded-lg border border-slate-200 px-3 py-2 text-xs hover:bg-slate-50 dark:border-slate-700 dark:hover:bg-slate-900" onclick={() => void transitionVersionAction(version.id, 'archived')} disabled={busy}>Archive</button>
                    </div>
                  </div>
                {/each}
              {/if}
            </div>
          </div>
        </div>
      </div>
    </section>

    <section class="rounded-[1.75rem] border border-slate-200 bg-white p-6 shadow-sm dark:border-slate-800 dark:bg-slate-950">
      <div class="mb-5 flex flex-wrap items-center justify-between gap-3">
        <div>
          <div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-400">Feature Store</div>
          <h2 class="mt-1 text-2xl font-semibold text-slate-950 dark:text-slate-50">Offline definitions and online serving snapshots</h2>
        </div>
        <button type="button" class="rounded-xl border border-slate-200 px-4 py-2 text-sm hover:bg-slate-50 dark:border-slate-700 dark:hover:bg-slate-900" onclick={newFeature}>New feature</button>
      </div>

      <div class="grid gap-6 xl:grid-cols-[280px,1fr]">
        <div class="space-y-3">
          {#if features.length === 0}
            <div class="rounded-2xl border border-dashed border-slate-300 px-4 py-10 text-center text-sm text-slate-500 dark:border-slate-700">No features defined yet.</div>
          {:else}
            {#each features as feature (feature.id)}
              <button
                type="button"
                class={`w-full rounded-2xl border p-4 text-left transition ${selectedFeatureId === feature.id ? 'border-fuchsia-500 bg-fuchsia-50 dark:bg-fuchsia-950/20' : 'border-slate-200 hover:bg-slate-50 dark:border-slate-700 dark:hover:bg-slate-900'}`}
                onclick={() => void selectFeature(feature.id)}
              >
                <div class="flex items-start justify-between gap-3">
                  <div>
                    <div class="font-medium text-slate-900 dark:text-slate-100">{feature.name}</div>
                    <div class="mt-1 text-sm text-slate-500">{feature.entity_name}</div>
                  </div>
                  <span class="rounded-full border border-slate-200 px-2 py-1 text-[11px] uppercase tracking-[0.18em] text-slate-500 dark:border-slate-700">{feature.online_enabled ? 'online' : 'offline'}</span>
                </div>
              </button>
            {/each}
          {/if}
        </div>

        <div class="space-y-6">
          <div class="grid gap-4 md:grid-cols-2">
            <label class="space-y-1 text-sm">
              <span class="font-medium text-slate-600 dark:text-slate-300">Name</span>
              <input bind:value={featureDraft.name} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-slate-700 dark:bg-slate-900" />
            </label>
            <label class="space-y-1 text-sm">
              <span class="font-medium text-slate-600 dark:text-slate-300">Entity</span>
              <input bind:value={featureDraft.entity_name} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-slate-700 dark:bg-slate-900" />
            </label>
            <label class="space-y-1 text-sm">
              <span class="font-medium text-slate-600 dark:text-slate-300">Data type</span>
              <input bind:value={featureDraft.data_type} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-slate-700 dark:bg-slate-900" />
            </label>
            <label class="space-y-1 text-sm">
              <span class="font-medium text-slate-600 dark:text-slate-300">Batch schedule</span>
              <input bind:value={featureDraft.batch_schedule} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-slate-700 dark:bg-slate-900" />
            </label>
            <label class="space-y-1 text-sm md:col-span-2">
              <span class="font-medium text-slate-600 dark:text-slate-300">Description</span>
              <textarea rows="2" bind:value={featureDraft.description} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-slate-700 dark:bg-slate-900"></textarea>
            </label>
            <label class="space-y-1 text-sm md:col-span-2">
              <span class="font-medium text-slate-600 dark:text-slate-300">Offline source</span>
              <textarea rows="3" bind:value={featureDraft.offline_source} class="w-full rounded-xl border border-slate-200 px-3 py-2 font-mono text-xs dark:border-slate-700 dark:bg-slate-900"></textarea>
            </label>
            <label class="space-y-1 text-sm md:col-span-2">
              <span class="font-medium text-slate-600 dark:text-slate-300">Transformation</span>
              <textarea rows="2" bind:value={featureDraft.transformation} class="w-full rounded-xl border border-slate-200 px-3 py-2 font-mono text-xs dark:border-slate-700 dark:bg-slate-900"></textarea>
            </label>
            <label class="space-y-1 text-sm">
              <span class="font-medium text-slate-600 dark:text-slate-300">Online namespace</span>
              <input bind:value={featureDraft.online_namespace} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-slate-700 dark:bg-slate-900" />
            </label>
            <label class="space-y-1 text-sm">
              <span class="font-medium text-slate-600 dark:text-slate-300">Freshness SLA (min)</span>
              <input type="number" bind:value={featureDraft.freshness_sla_minutes} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-slate-700 dark:bg-slate-900" />
            </label>
            <label class="space-y-1 text-sm md:col-span-2">
              <span class="font-medium text-slate-600 dark:text-slate-300">Tags</span>
              <input bind:value={featureDraft.tags_text} placeholder="support, freshness-critical" class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-slate-700 dark:bg-slate-900" />
            </label>
            <label class="flex items-center gap-3 rounded-xl border border-slate-200 px-3 py-2 text-sm dark:border-slate-700">
              <input type="checkbox" bind:checked={featureDraft.online_enabled} />
              <span class="font-medium text-slate-600 dark:text-slate-300">Enable online serving</span>
            </label>
            <label class="space-y-1 text-sm md:col-span-2">
              <span class="font-medium text-slate-600 dark:text-slate-300">Sample values JSON</span>
              <textarea rows="5" bind:value={featureDraft.samples_text} class="w-full rounded-xl border border-slate-200 px-3 py-2 font-mono text-xs dark:border-slate-700 dark:bg-slate-900"></textarea>
            </label>
          </div>

          <div class="flex flex-wrap gap-3">
            <button type="button" class="rounded-xl bg-slate-900 px-4 py-2 text-sm font-medium text-white hover:bg-slate-800 disabled:opacity-50 dark:bg-slate-100 dark:text-slate-950 dark:hover:bg-white" onclick={() => void saveFeature()} disabled={busy}>
              {featureDraft.id ? 'Save feature' : 'Create feature'}
            </button>
            <button type="button" class="rounded-xl border border-slate-200 px-4 py-2 text-sm hover:bg-slate-50 disabled:opacity-50 dark:border-slate-700 dark:hover:bg-slate-900" onclick={() => void materializeSelectedFeature()} disabled={busy || !selectedFeatureId}>
              Materialize offline + online
            </button>
          </div>

          <div class="grid gap-4 lg:grid-cols-2">
            <div class="rounded-2xl border border-slate-200 p-4 dark:border-slate-700">
              <div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-400">Online Snapshot</div>
              {#if onlineFeature}
                <div class="mt-3 space-y-3">
                  <div class="text-sm text-slate-500">Source: {onlineFeature.source}</div>
                  <div class="overflow-x-auto">
                    <table class="min-w-full text-sm">
                      <thead>
                        <tr class="text-left text-slate-500">
                          <th class="px-2 py-2">Entity</th>
                          <th class="px-2 py-2">Value</th>
                        </tr>
                      </thead>
                      <tbody>
                        {#each onlineFeature.values as sample, index (`${sample.entity_key}-${index}`)}
                          <tr class="border-t border-slate-200 dark:border-slate-800">
                            <td class="px-2 py-2 font-medium text-slate-900 dark:text-slate-100">{sample.entity_key}</td>
                            <td class="px-2 py-2 text-slate-600 dark:text-slate-300">{JSON.stringify(sample.value)}</td>
                          </tr>
                        {/each}
                      </tbody>
                    </table>
                  </div>
                </div>
              {:else}
                <div class="mt-3 text-sm text-slate-500">No online snapshot available yet.</div>
              {/if}
            </div>

            <div class="rounded-2xl border border-slate-200 p-4 dark:border-slate-700">
              <div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-400">Dataset context</div>
              <div class="mt-3 space-y-2 text-sm text-slate-500">
                <div>{datasets.length} dataset(s) available for offline feature definitions.</div>
                {#if datasets.length > 0}
                  <div class="rounded-xl bg-slate-50 px-3 py-2 dark:bg-slate-900">
                    Example dataset: <span class="font-medium text-slate-700 dark:text-slate-200">{datasets[0].name}</span>
                  </div>
                {/if}
                {#if selectedFeature()}
                  <div>Last materialized: {formatTimestamp(selectedFeature()?.last_materialized_at ?? null)}</div>
                {/if}
              </div>
            </div>
          </div>
        </div>
      </div>
    </section>

    <section class="rounded-[1.75rem] border border-slate-200 bg-white p-6 shadow-sm dark:border-slate-800 dark:bg-slate-950">
      <div class="mb-5">
        <div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-400">Training Orchestration</div>
        <h2 class="mt-1 text-2xl font-semibold text-slate-950 dark:text-slate-50">Jobs, hyperparameter tuning, and auto-registration</h2>
      </div>

      <div class="grid gap-6 xl:grid-cols-[360px,1fr]">
        <div class="space-y-3 rounded-[1.5rem] border border-slate-200 p-4 dark:border-slate-800">
          <label class="space-y-1 text-sm">
            <span class="font-medium text-slate-600 dark:text-slate-300">Job name</span>
            <input bind:value={trainingJobDraft.name} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-slate-700 dark:bg-slate-900" />
          </label>
          <label class="space-y-1 text-sm">
            <span class="font-medium text-slate-600 dark:text-slate-300">Experiment</span>
            <select bind:value={trainingJobDraft.experiment_id} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-slate-700 dark:bg-slate-900">
              <option value="">No linked experiment</option>
              {#each experiments as experiment (experiment.id)}
                <option value={experiment.id}>{experiment.name}</option>
              {/each}
            </select>
          </label>
          <label class="space-y-1 text-sm">
            <span class="font-medium text-slate-600 dark:text-slate-300">Model</span>
            <select bind:value={trainingJobDraft.model_id} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-slate-700 dark:bg-slate-900">
              <option value="">No linked model</option>
              {#each models as model (model.id)}
                <option value={model.id}>{model.name}</option>
              {/each}
            </select>
          </label>
          <label class="space-y-1 text-sm">
            <span class="font-medium text-slate-600 dark:text-slate-300">Dataset ids</span>
            <input bind:value={trainingJobDraft.dataset_ids_text} placeholder="uuid-1, uuid-2" class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-slate-700 dark:bg-slate-900" />
          </label>
          <label class="space-y-1 text-sm">
            <span class="font-medium text-slate-600 dark:text-slate-300">Objective metric</span>
            <input bind:value={trainingJobDraft.objective_metric_name} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-slate-700 dark:bg-slate-900" />
          </label>
          <label class="space-y-1 text-sm">
            <span class="font-medium text-slate-600 dark:text-slate-300">Training config JSON</span>
            <textarea rows="5" bind:value={trainingJobDraft.training_config_text} class="w-full rounded-xl border border-slate-200 px-3 py-2 font-mono text-xs dark:border-slate-700 dark:bg-slate-900"></textarea>
          </label>
          <label class="space-y-1 text-sm">
            <span class="font-medium text-slate-600 dark:text-slate-300">Hyperparameter search JSON</span>
            <textarea rows="6" bind:value={trainingJobDraft.hyperparameter_search_text} class="w-full rounded-xl border border-slate-200 px-3 py-2 font-mono text-xs dark:border-slate-700 dark:bg-slate-900"></textarea>
          </label>
          <label class="flex items-center gap-3 rounded-xl border border-slate-200 px-3 py-2 text-sm dark:border-slate-700">
            <input type="checkbox" bind:checked={trainingJobDraft.auto_register_model_version} />
            <span class="font-medium text-slate-600 dark:text-slate-300">Auto-register best model version</span>
          </label>

          <button type="button" class="rounded-xl bg-emerald-600 px-4 py-2 text-sm font-medium text-white hover:bg-emerald-500 disabled:opacity-50" onclick={() => void createTrainingJobAction()} disabled={busy}>
            Submit training job
          </button>
        </div>

        <div class="space-y-3">
          {#if trainingJobs.length === 0}
            <div class="rounded-2xl border border-dashed border-slate-300 px-4 py-10 text-center text-sm text-slate-500 dark:border-slate-700">No training jobs submitted yet.</div>
          {:else}
            {#each trainingJobs as job (job.id)}
              <div class="rounded-2xl border border-slate-200 p-4 dark:border-slate-700">
                <div class="flex flex-wrap items-start justify-between gap-3">
                  <div>
                    <div class="font-medium text-slate-900 dark:text-slate-100">{job.name}</div>
                    <div class="mt-1 text-sm text-slate-500">Objective: {job.objective_metric_name}</div>
                  </div>
                  <span class="rounded-full border border-slate-200 px-3 py-1 text-[11px] uppercase tracking-[0.18em] text-slate-500 dark:border-slate-700">{job.status}</span>
                </div>
                <div class="mt-3 flex flex-wrap gap-2 text-xs text-slate-400">
                  <span>{job.trials.length} trial(s)</span>
                  {#if job.best_model_version_id}<span>best version: {job.best_model_version_id}</span>{/if}
                </div>
                <div class="mt-4 overflow-x-auto rounded-xl bg-slate-50 p-3 dark:bg-slate-900">
                  <table class="min-w-full text-sm">
                    <thead>
                      <tr class="text-left text-slate-500">
                        <th class="px-2 py-2">Trial</th>
                        <th class="px-2 py-2">Metric</th>
                        <th class="px-2 py-2">Hyperparameters</th>
                      </tr>
                    </thead>
                    <tbody>
                      {#each job.trials as trial (trial.id)}
                        <tr class="border-t border-slate-200 dark:border-slate-800">
                          <td class="px-2 py-2 font-medium text-slate-900 dark:text-slate-100">{trial.id}</td>
                          <td class="px-2 py-2 text-slate-600 dark:text-slate-300">{trial.objective_metric.name}: {trial.objective_metric.value}</td>
                          <td class="px-2 py-2 text-slate-600 dark:text-slate-300">{JSON.stringify(trial.hyperparameters)}</td>
                        </tr>
                      {/each}
                    </tbody>
                  </table>
                </div>
              </div>
            {/each}
          {/if}
        </div>
      </div>
    </section>

    <section class="rounded-[1.75rem] border border-slate-200 bg-white p-6 shadow-sm dark:border-slate-800 dark:bg-slate-950">
      <div class="mb-5 flex flex-wrap items-center justify-between gap-3">
        <div>
          <div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-400">Serving, A/B Testing, and Drift</div>
          <h2 class="mt-1 text-2xl font-semibold text-slate-950 dark:text-slate-50">Deployments, routing, predictions, and monitoring</h2>
        </div>
        <button type="button" class="rounded-xl border border-slate-200 px-4 py-2 text-sm hover:bg-slate-50 dark:border-slate-700 dark:hover:bg-slate-900" onclick={newDeployment}>New deployment</button>
      </div>

      <div class="grid gap-6 xl:grid-cols-[300px,1fr]">
        <div class="space-y-3">
          {#if deployments.length === 0}
            <div class="rounded-2xl border border-dashed border-slate-300 px-4 py-10 text-center text-sm text-slate-500 dark:border-slate-700">No deployments yet.</div>
          {:else}
            {#each deployments as deployment (deployment.id)}
              <button
                type="button"
                class={`w-full rounded-2xl border p-4 text-left transition ${selectedDeploymentId === deployment.id ? 'border-amber-500 bg-amber-50 dark:bg-amber-950/20' : 'border-slate-200 hover:bg-slate-50 dark:border-slate-700 dark:hover:bg-slate-900'}`}
                onclick={() => selectDeployment(deployment.id)}
              >
                <div class="flex items-start justify-between gap-3">
                  <div>
                    <div class="font-medium text-slate-900 dark:text-slate-100">{deployment.name}</div>
                    <div class="mt-1 text-sm text-slate-500">{deployment.endpoint_path}</div>
                  </div>
                  <span class="rounded-full border border-slate-200 px-2 py-1 text-[11px] uppercase tracking-[0.18em] text-slate-500 dark:border-slate-700">{deployment.strategy_type}</span>
                </div>
                <div class="mt-3 text-xs text-slate-400">{deployment.traffic_split.length} variant(s)</div>
              </button>
            {/each}
          {/if}
        </div>

        <div class="space-y-6">
          <div class="grid gap-4 md:grid-cols-2">
            <label class="space-y-1 text-sm">
              <span class="font-medium text-slate-600 dark:text-slate-300">Deployment name</span>
              <input bind:value={deploymentDraft.name} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-slate-700 dark:bg-slate-900" />
            </label>
            <label class="space-y-1 text-sm">
              <span class="font-medium text-slate-600 dark:text-slate-300">Model</span>
              <select bind:value={deploymentDraft.model_id} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-slate-700 dark:bg-slate-900">
                <option value="">Select model</option>
                {#each models as model (model.id)}
                  <option value={model.id}>{model.name}</option>
                {/each}
              </select>
            </label>
            <label class="space-y-1 text-sm">
              <span class="font-medium text-slate-600 dark:text-slate-300">Status</span>
              <select bind:value={deploymentDraft.status} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-slate-700 dark:bg-slate-900">
                <option value="active">Active</option>
                <option value="paused">Paused</option>
                <option value="archived">Archived</option>
              </select>
            </label>
            <label class="space-y-1 text-sm">
              <span class="font-medium text-slate-600 dark:text-slate-300">Strategy</span>
              <select bind:value={deploymentDraft.strategy_type} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-slate-700 dark:bg-slate-900">
                <option value="single">Single model</option>
                <option value="ab_test">A/B test</option>
              </select>
            </label>
            <label class="space-y-1 text-sm md:col-span-2">
              <span class="font-medium text-slate-600 dark:text-slate-300">Endpoint path</span>
              <input bind:value={deploymentDraft.endpoint_path} class="w-full rounded-xl border border-slate-200 px-3 py-2 font-mono text-sm dark:border-slate-700 dark:bg-slate-900" />
            </label>
            <label class="space-y-1 text-sm">
              <span class="font-medium text-slate-600 dark:text-slate-300">Monitoring window</span>
              <input bind:value={deploymentDraft.monitoring_window} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-slate-700 dark:bg-slate-900" />
            </label>
            <label class="space-y-1 text-sm">
              <span class="font-medium text-slate-600 dark:text-slate-300">Baseline dataset</span>
              <select bind:value={deploymentDraft.baseline_dataset_id} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-slate-700 dark:bg-slate-900">
                <option value="">No baseline dataset</option>
                {#each datasets as dataset (dataset.id)}
                  <option value={dataset.id}>{dataset.name}</option>
                {/each}
              </select>
            </label>
            <label class="space-y-1 text-sm md:col-span-2">
              <span class="font-medium text-slate-600 dark:text-slate-300">Traffic split JSON</span>
              <textarea rows="6" bind:value={deploymentDraft.traffic_split_text} class="w-full rounded-xl border border-slate-200 px-3 py-2 font-mono text-xs dark:border-slate-700 dark:bg-slate-900"></textarea>
            </label>
          </div>

          <div class="flex gap-3">
            <button type="button" class="rounded-xl bg-slate-900 px-4 py-2 text-sm font-medium text-white hover:bg-slate-800 disabled:opacity-50 dark:bg-slate-100 dark:text-slate-950 dark:hover:bg-white" onclick={() => void saveDeployment()} disabled={busy}>
              {deploymentDraft.id ? 'Save deployment' : 'Create deployment'}
            </button>
          </div>

          <div class="grid gap-4 xl:grid-cols-2">
            <div class="rounded-2xl border border-slate-200 p-4 dark:border-slate-700">
              <div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-400">Drift Monitoring</div>
              <div class="mt-4 space-y-3">
                <label class="space-y-1 text-sm">
                  <span class="font-medium text-slate-600 dark:text-slate-300">Baseline rows</span>
                  <input bind:value={driftBaselineRows} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-slate-700 dark:bg-slate-900" />
                </label>
                <label class="space-y-1 text-sm">
                  <span class="font-medium text-slate-600 dark:text-slate-300">Observed rows</span>
                  <input bind:value={driftObservedRows} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-slate-700 dark:bg-slate-900" />
                </label>
                <label class="flex items-center gap-3 rounded-xl border border-slate-200 px-3 py-2 text-sm dark:border-slate-700">
                  <input type="checkbox" bind:checked={autoRetrain} />
                  <span class="font-medium text-slate-600 dark:text-slate-300">Auto-enqueue retraining job if drift exceeds threshold</span>
                </label>
                <button type="button" class="rounded-xl bg-amber-500 px-4 py-2 text-sm font-medium text-white hover:bg-amber-400 disabled:opacity-50" onclick={() => void evaluateDriftAction()} disabled={busy || !selectedDeploymentId}>
                  Generate drift report
                </button>
              </div>

              {#if selectedDeployment()?.drift_report}
                <div class="mt-4 rounded-xl bg-slate-50 p-3 text-sm dark:bg-slate-900">
                  <div class="font-medium text-slate-900 dark:text-slate-100">{selectedDeployment()?.drift_report?.notes}</div>
                  <div class="mt-3 space-y-2 text-slate-500">
                    {#each selectedDeployment()?.drift_report?.dataset_metrics ?? [] as metric (`dataset-${metric.name}`)}
                      <div>{metric.name}: {metric.score} / threshold {metric.threshold} ({metric.status})</div>
                    {/each}
                    {#each selectedDeployment()?.drift_report?.concept_metrics ?? [] as metric (`concept-${metric.name}`)}
                      <div>{metric.name}: {metric.score} / threshold {metric.threshold} ({metric.status})</div>
                    {/each}
                    {#if selectedDeployment()?.drift_report?.auto_retraining_job_id}
                      <div>Auto retraining job: {selectedDeployment()?.drift_report?.auto_retraining_job_id}</div>
                    {/if}
                  </div>
                </div>
              {/if}
            </div>

            <div class="rounded-2xl border border-slate-200 p-4 dark:border-slate-700">
              <div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-400">Realtime Prediction Playground</div>
              <label class="mt-4 block space-y-1 text-sm">
                <span class="font-medium text-slate-600 dark:text-slate-300">Input records JSON</span>
                <textarea rows="7" bind:value={realtimeInputText} class="w-full rounded-xl border border-slate-200 px-3 py-2 font-mono text-xs dark:border-slate-700 dark:bg-slate-900"></textarea>
              </label>
              <button type="button" class="mt-4 rounded-xl bg-sky-600 px-4 py-2 text-sm font-medium text-white hover:bg-sky-500 disabled:opacity-50" onclick={() => void predictRealtimeAction()} disabled={busy || !selectedDeploymentId}>
                Run realtime prediction
              </button>

              {#if predictionPreview}
                <div class="mt-4 overflow-x-auto rounded-xl bg-slate-50 p-3 dark:bg-slate-900">
                  <table class="min-w-full text-sm">
                    <thead>
                      <tr class="text-left text-slate-500">
                        <th class="px-2 py-2">Record</th>
                        <th class="px-2 py-2">Variant</th>
                        <th class="px-2 py-2">Label</th>
                        <th class="px-2 py-2">Score</th>
                      </tr>
                    </thead>
                    <tbody>
                      {#each predictionPreview.outputs as output (output.record_id)}
                        <tr class="border-t border-slate-200 dark:border-slate-800">
                          <td class="px-2 py-2 font-medium text-slate-900 dark:text-slate-100">{output.record_id}</td>
                          <td class="px-2 py-2 text-slate-600 dark:text-slate-300">{output.variant}</td>
                          <td class="px-2 py-2 text-slate-600 dark:text-slate-300">{output.predicted_label}</td>
                          <td class="px-2 py-2 text-slate-600 dark:text-slate-300">{output.score} ({output.confidence})</td>
                        </tr>
                      {/each}
                    </tbody>
                  </table>
                </div>
              {/if}
            </div>
          </div>

          <div class="rounded-2xl border border-slate-200 p-4 dark:border-slate-700">
            <div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-400">Batch Predictions</div>
            <div class="mt-4 grid gap-4 lg:grid-cols-[1fr,320px]">
              <div>
                <label class="space-y-1 text-sm">
                  <span class="font-medium text-slate-600 dark:text-slate-300">Batch input JSON</span>
                  <textarea rows="7" bind:value={batchInputText} class="w-full rounded-xl border border-slate-200 px-3 py-2 font-mono text-xs dark:border-slate-700 dark:bg-slate-900"></textarea>
                </label>
              </div>
              <div class="space-y-3">
                <label class="space-y-1 text-sm">
                  <span class="font-medium text-slate-600 dark:text-slate-300">Output destination</span>
                  <input bind:value={batchOutputDestination} placeholder="s3://bucket/predictions/run.parquet" class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-slate-700 dark:bg-slate-900" />
                </label>
                <button type="button" class="rounded-xl bg-emerald-600 px-4 py-2 text-sm font-medium text-white hover:bg-emerald-500 disabled:opacity-50" onclick={() => void runBatchPredictionAction()} disabled={busy || !selectedDeploymentId}>
                  Run batch prediction
                </button>
              </div>
            </div>

            <div class="mt-6 space-y-3">
              {#if batchPredictions.length === 0}
                <div class="rounded-2xl border border-dashed border-slate-300 px-4 py-10 text-center text-sm text-slate-500 dark:border-slate-700">No batch prediction jobs yet.</div>
              {:else}
                {#each batchPredictions as batchJob (batchJob.id)}
                  <div class="rounded-2xl border border-slate-200 p-4 dark:border-slate-700">
                    <div class="flex flex-wrap items-start justify-between gap-3">
                      <div>
                        <div class="font-medium text-slate-900 dark:text-slate-100">{batchJob.id}</div>
                        <div class="mt-1 text-sm text-slate-500">{batchJob.record_count} record(s)</div>
                      </div>
                      <span class="rounded-full border border-slate-200 px-3 py-1 text-[11px] uppercase tracking-[0.18em] text-slate-500 dark:border-slate-700">{batchJob.status}</span>
                    </div>
                    <div class="mt-3 text-xs text-slate-400">{batchJob.output_destination ?? 'No external destination'}</div>
                  </div>
                {/each}
              {/if}
            </div>
          </div>
        </div>
      </div>
    </section>
  </div>
{/if}