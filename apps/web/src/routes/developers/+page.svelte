<script lang="ts">
	import { onMount } from 'svelte';

	import ApiExplorer from '$components/developer/ApiExplorer.svelte';
	import GitIntegrationManager from '$components/developer/GitIntegrationManager.svelte';
	import SdkToolkit from '$components/developer/SdkToolkit.svelte';
	import TerraformProviderPanel from '$components/developer/TerraformProviderPanel.svelte';
	import {
		createIntegration,
		getIntegration,
		listIntegrations,
		loadOpenApiSpec,
		loadTerraformProviderSchema,
		triggerIntegrationSync,
		updateIntegration,
		type IntegrationDetail,
		type OpenApiSpec,
		type RepositoryIntegration,
		type TerraformProviderSchema,
	} from '$lib/api/developer';
	import { listRepositories, type RepositoryDefinition } from '$lib/api/code-repos';
	import { notifications } from '$lib/stores/notifications';

	type IntegrationDraft = {
		id?: string;
		repository_id: string;
		provider: 'github' | 'gitlab';
		external_namespace: string;
		external_project: string;
		external_url: string;
		sync_mode: string;
		ci_trigger_strategy: string;
		status: string;
		default_branch: string;
		branch_mapping_text: string;
		webhook_url: string;
	};

	type SyncDraft = {
		trigger: string;
		commit_sha: string;
		branch_name: string;
	};

	let repositories = $state<RepositoryDefinition[]>([]);
	let integrations = $state<RepositoryIntegration[]>([]);
	let selectedIntegration = $state<IntegrationDetail | null>(null);
	let openApiSpec = $state<OpenApiSpec | null>(null);
	let terraformSchema = $state<TerraformProviderSchema | null>(null);
	let loading = $state(true);
	let docsLoading = $state(true);
	let busyAction = $state('');
	let integrationError = $state('');
	let docsError = $state('');
	let draft = $state<IntegrationDraft>(createEmptyIntegrationDraft());
	let syncDraft = $state<SyncDraft>(createEmptySyncDraft());

	const busy = $derived(loading || busyAction.length > 0);
	const pathCount = $derived(openApiSpec ? Object.keys(openApiSpec.paths).length : 0);
	const resourceCount = $derived(terraformSchema ? terraformSchema.resources.length : 0);
	const providerMix = $derived(Array.from(new Set(integrations.map((entry) => entry.provider))).join(' + ') || 'github + gitlab');

	onMount(() => {
		void refreshAll();
	});

	function createEmptyIntegrationDraft(repository?: RepositoryDefinition | null): IntegrationDraft {
		return {
			repository_id: repository?.id ?? '',
			provider: 'github',
			external_namespace: 'openfoundry-labs',
			external_project: repository?.slug ?? 'plugin-starter',
			external_url: repository ? `https://github.com/openfoundry-labs/${repository.slug}` : 'https://github.com/openfoundry-labs/plugin-starter',
			sync_mode: 'bidirectional_mirror',
			ci_trigger_strategy: 'github_actions',
			status: 'connected',
			default_branch: repository?.default_branch ?? 'main',
			branch_mapping_text: `${repository?.default_branch ?? 'main'} -> ${repository?.default_branch ?? 'main'}`,
			webhook_url: 'https://platform.openfoundry.local/api/v1/hooks/git',
		};
	}

	function createEmptySyncDraft(branchName = 'main'): SyncDraft {
		return {
			trigger: 'manual',
			commit_sha: '8f4c1d2b9a6e77c1',
			branch_name: branchName,
		};
	}

	function parseLines(value: string) {
		return value.split('\n').map((entry) => entry.trim()).filter(Boolean);
	}

	function draftFromIntegration(integration: RepositoryIntegration): IntegrationDraft {
		return {
			id: integration.id,
			repository_id: integration.repository_id,
			provider: integration.provider,
			external_namespace: integration.external_namespace,
			external_project: integration.external_project,
			external_url: integration.external_url,
			sync_mode: integration.sync_mode,
			ci_trigger_strategy: integration.ci_trigger_strategy,
			status: integration.status,
			default_branch: integration.default_branch,
			branch_mapping_text: integration.branch_mapping.join('\n'),
			webhook_url: integration.webhook_url,
		};
	}

	function updateDraft(patch: Partial<IntegrationDraft>) {
		const nextDraft = { ...draft, ...patch };
		if (patch.repository_id && !draft.id) {
			const repository = repositories.find((entry) => entry.id === patch.repository_id) ?? null;
			nextDraft.default_branch = repository?.default_branch ?? nextDraft.default_branch;
			nextDraft.external_project = repository?.slug ?? nextDraft.external_project;
			nextDraft.external_url = repository ? `https://github.com/openfoundry-labs/${repository.slug}` : nextDraft.external_url;
			nextDraft.branch_mapping_text = `${repository?.default_branch ?? nextDraft.default_branch} -> ${repository?.default_branch ?? nextDraft.default_branch}`;
			syncDraft = { ...syncDraft, branch_name: repository?.default_branch ?? syncDraft.branch_name };
		}
		draft = nextDraft;
	}

	function updateSyncDraft(patch: Partial<SyncDraft>) {
		syncDraft = { ...syncDraft, ...patch };
	}

	function startNewIntegration() {
		const repository = repositories[0] ?? null;
		selectedIntegration = null;
		integrationError = '';
		draft = createEmptyIntegrationDraft(repository);
		syncDraft = createEmptySyncDraft(repository?.default_branch ?? 'main');
	}

	async function refreshAll(preferredIntegrationId?: string) {
		loading = true;
		integrationError = '';
		try {
			const [repositoriesResponse, integrationsResponse] = await Promise.all([listRepositories(), listIntegrations()]);
			repositories = repositoriesResponse.items;
			integrations = integrationsResponse.items;

			if (integrations.length) {
				const nextId = preferredIntegrationId ?? selectedIntegration?.integration.id ?? integrations[0]?.id;
				if (nextId) {
					await selectIntegration(nextId, false);
				}
			} else {
				startNewIntegration();
			}
		} catch (error) {
			integrationError = error instanceof Error ? error.message : 'Unable to load developer portal data';
			notifications.error(integrationError);
		} finally {
			loading = false;
		}

		void refreshStaticAssets();
	}

	async function refreshStaticAssets() {
		docsLoading = true;
		docsError = '';
		try {
			const [spec, schema] = await Promise.all([loadOpenApiSpec(), loadTerraformProviderSchema()]);
			openApiSpec = spec;
			terraformSchema = schema;
		} catch (error) {
			docsError = error instanceof Error ? error.message : 'Unable to load generated assets';
		} finally {
			docsLoading = false;
		}
	}

	async function selectIntegration(integrationId: string, notify = true) {
		busyAction = 'loading-integration';
		integrationError = '';
		try {
			const detail = await getIntegration(integrationId);
			selectedIntegration = detail;
			draft = draftFromIntegration(detail.integration);
			syncDraft = createEmptySyncDraft(detail.integration.default_branch);
			if (notify) {
				notifications.info(`Loaded ${detail.integration.external_namespace}/${detail.integration.external_project}`);
			}
		} catch (error) {
			integrationError = error instanceof Error ? error.message : 'Unable to load integration';
			notifications.error(integrationError);
		} finally {
			busyAction = '';
		}
	}

	async function saveIntegration() {
		busyAction = draft.id ? 'updating-integration' : 'creating-integration';
		integrationError = '';
		try {
			if (draft.id) {
				const updated = await updateIntegration(draft.id, {
					external_namespace: draft.external_namespace,
					external_project: draft.external_project,
					external_url: draft.external_url,
					sync_mode: draft.sync_mode,
					ci_trigger_strategy: draft.ci_trigger_strategy,
					status: draft.status,
					default_branch: draft.default_branch,
					branch_mapping: parseLines(draft.branch_mapping_text),
					webhook_url: draft.webhook_url,
				});
				notifications.success(`Updated ${updated.external_namespace}/${updated.external_project}`);
				await refreshAll(updated.id);
			} else {
				const created = await createIntegration({
					repository_id: draft.repository_id,
					provider: draft.provider,
					external_namespace: draft.external_namespace,
					external_project: draft.external_project,
					external_url: draft.external_url,
					sync_mode: draft.sync_mode,
					ci_trigger_strategy: draft.ci_trigger_strategy,
					default_branch: draft.default_branch,
					branch_mapping: parseLines(draft.branch_mapping_text),
					webhook_url: draft.webhook_url,
				});
				notifications.success(`Created ${created.external_namespace}/${created.external_project}`);
				await refreshAll(created.id);
			}
		} catch (error) {
			integrationError = error instanceof Error ? error.message : 'Unable to save integration';
			notifications.error(integrationError);
		} finally {
			busyAction = '';
		}
	}

	async function queueSyncRun() {
		if (!selectedIntegration) {
			notifications.warning('Select an integration before queueing a sync run');
			return;
		}

		busyAction = 'queueing-sync';
		integrationError = '';
		try {
			const run = await triggerIntegrationSync(selectedIntegration.integration.id, {
				trigger: syncDraft.trigger,
				commit_sha: syncDraft.commit_sha,
				branch_name: syncDraft.branch_name,
			});
			notifications.success(`Queued ${run.trigger} sync for ${run.branch_name}`);
			await refreshAll(selectedIntegration.integration.id);
		} catch (error) {
			integrationError = error instanceof Error ? error.message : 'Unable to queue sync run';
			notifications.error(integrationError);
		} finally {
			busyAction = '';
		}
	}
</script>

<svelte:head>
	<title>OpenFoundry Developers</title>
	<meta name="description" content="Developer portal for plugin SDKs, CLI automation, generated API docs, Terraform provider assets, and Git integrations." />
</svelte:head>

<div class="space-y-8 pb-8">
	<section class="overflow-hidden rounded-[40px] border border-slate-200 bg-[radial-gradient(circle_at_top_left,_rgba(16,185,129,0.18),_transparent_28%),linear-gradient(135deg,_#f8fafc_0%,_#ffffff_48%,_#eef6ff_100%)] p-8 shadow-sm">
		<div class="grid gap-8 xl:grid-cols-[1.15fr,0.85fr] xl:items-end">
			<div>
				<div class="text-xs font-semibold uppercase tracking-[0.34em] text-emerald-700">Milestone 5.2</div>
				<h1 class="mt-3 max-w-4xl text-4xl font-semibold tracking-tight text-slate-950">Developer ecosystem for SDK plugins, automation, and external platform delivery</h1>
				<p class="mt-4 max-w-3xl text-base leading-7 text-slate-600">
					This portal bundles the Rust + WASM plugin SDK, the `of` CLI, proto-derived REST docs, Terraform provider metadata, and GitHub or GitLab sync management into one operator surface.
				</p>
			</div>

			<div class="grid gap-3 sm:grid-cols-2">
				<div class="rounded-3xl border border-white/70 bg-white/80 px-5 py-4 backdrop-blur">
					<div class="text-xs uppercase tracking-[0.18em] text-slate-400">Repositories</div>
					<div class="mt-1 text-3xl font-semibold text-slate-950">{repositories.length}</div>
					<div class="mt-1 text-sm text-slate-500">scaffold and sync targets</div>
				</div>
				<div class="rounded-3xl border border-white/70 bg-white/80 px-5 py-4 backdrop-blur">
					<div class="text-xs uppercase tracking-[0.18em] text-slate-400">Git providers</div>
					<div class="mt-1 text-3xl font-semibold text-slate-950">{providerMix}</div>
					<div class="mt-1 text-sm text-slate-500">integration coverage</div>
				</div>
				<div class="rounded-3xl border border-white/70 bg-white/80 px-5 py-4 backdrop-blur">
					<div class="text-xs uppercase tracking-[0.18em] text-slate-400">REST paths</div>
					<div class="mt-1 text-3xl font-semibold text-slate-950">{pathCount}</div>
					<div class="mt-1 text-sm text-slate-500">generated from proto</div>
				</div>
				<div class="rounded-3xl border border-white/70 bg-white/80 px-5 py-4 backdrop-blur">
					<div class="text-xs uppercase tracking-[0.18em] text-slate-400">Terraform resources</div>
					<div class="mt-1 text-3xl font-semibold text-slate-950">{resourceCount}</div>
					<div class="mt-1 text-sm text-slate-500">IaC primitives</div>
				</div>
			</div>
		</div>
	</section>

	<SdkToolkit />

	<ApiExplorer spec={openApiSpec} loading={docsLoading} error={docsError} />

	<TerraformProviderPanel schema={terraformSchema} loading={docsLoading} error={docsError} />

	<GitIntegrationManager
		repositories={repositories}
		integrations={integrations}
		selectedIntegration={selectedIntegration}
		draft={draft}
		syncDraft={syncDraft}
		busy={busy}
		error={integrationError}
		onSelectIntegration={selectIntegration}
		onDraftChange={updateDraft}
		onSyncDraftChange={updateSyncDraft}
		onSaveIntegration={saveIntegration}
		onTriggerSync={queueSyncRun}
		onCreateNew={startNewIntegration}
	/>
</div>