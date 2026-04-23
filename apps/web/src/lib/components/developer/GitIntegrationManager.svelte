<script lang="ts">
	import type { IntegrationDetail, RepositoryIntegration } from '$lib/api/developer';
	import type { RepositoryDefinition } from '$lib/api/code-repos';

	interface Props {
		repositories: RepositoryDefinition[];
		integrations: RepositoryIntegration[];
		selectedIntegration: IntegrationDetail | null;
		draft: {
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
		syncDraft: {
			trigger: string;
			commit_sha: string;
			branch_name: string;
		};
		busy?: boolean;
		error?: string;
		onSelectIntegration: (integrationId: string) => void;
		onDraftChange: (patch: Partial<Props['draft']>) => void;
		onSyncDraftChange: (patch: Partial<Props['syncDraft']>) => void;
		onSaveIntegration: () => void;
		onTriggerSync: () => void;
		onCreateNew: () => void;
	}

	let {
		repositories,
		integrations,
		selectedIntegration,
		draft,
		syncDraft,
		busy = false,
		error = '',
		onSelectIntegration,
		onDraftChange,
		onSyncDraftChange,
		onSaveIntegration,
		onTriggerSync,
		onCreateNew,
	}: Props = $props();

	const selectedRepository = $derived(repositories.find((entry) => entry.id === draft.repository_id) ?? null);

	function repositoryName(repositoryId: string) {
		return repositories.find((entry) => entry.id === repositoryId)?.name ?? 'Unknown repository';
	}

	function shortSha(value: string) {
		return value.slice(0, 8);
	}
</script>

<section class="rounded-[32px] border border-slate-200 bg-white shadow-sm">
	<div class="border-b border-slate-200 px-6 py-5">
		<div class="flex flex-wrap items-start justify-between gap-4">
			<div>
				<div class="text-xs font-semibold uppercase tracking-[0.28em] text-rose-600">GitHub + GitLab</div>
				<h2 class="mt-2 text-2xl font-semibold text-slate-950">Repository integrations and sync runs</h2>
				<p class="mt-2 max-w-3xl text-sm text-slate-600">
					Manage external Git mirrors, map branches, and queue CI-aware sync runs directly from the developer portal. This surface is backed by the new `code-repo-service` integration APIs.
				</p>
			</div>

			<div class="flex flex-wrap gap-3 text-sm">
				<div class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3">
					<div class="text-xs uppercase tracking-[0.18em] text-slate-400">Repositories</div>
					<div class="mt-1 text-2xl font-semibold text-slate-950">{repositories.length}</div>
				</div>
				<div class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3">
					<div class="text-xs uppercase tracking-[0.18em] text-slate-400">Integrations</div>
					<div class="mt-1 text-2xl font-semibold text-slate-950">{integrations.length}</div>
				</div>
			</div>
		</div>
	</div>

	<div class="grid gap-0 xl:grid-cols-[320px,1fr]">
		<aside class="border-b border-slate-200 px-6 py-5 xl:border-b-0 xl:border-r">
			<div class="flex items-center justify-between gap-3">
				<div class="text-xs font-semibold uppercase tracking-[0.22em] text-slate-400">Connected repositories</div>
				<button type="button" onclick={onCreateNew} class="rounded-full border border-slate-300 px-3 py-1 text-xs font-semibold text-slate-700 transition hover:border-slate-400 hover:bg-slate-50">New integration</button>
			</div>

			<div class="mt-4 space-y-3">
				{#each integrations as integration (integration.id)}
					<button
						type="button"
						onclick={() => onSelectIntegration(integration.id)}
						class={`w-full rounded-2xl border px-4 py-4 text-left transition ${selectedIntegration?.integration.id === integration.id ? 'border-rose-300 bg-rose-50' : 'border-slate-200 hover:border-slate-300 hover:bg-slate-50'}`}
					>
						<div class="flex items-center justify-between gap-3">
							<div class="text-sm font-semibold text-slate-950">{integration.external_namespace}/{integration.external_project}</div>
							<span class="rounded-full border border-slate-200 px-2 py-1 text-[11px] uppercase tracking-[0.16em] text-slate-500">{integration.provider}</span>
						</div>
						<div class="mt-1 text-sm text-slate-600">{repositoryName(integration.repository_id)}</div>
						<div class="mt-3 flex flex-wrap gap-2 text-xs text-slate-500">
							<span class="rounded-full bg-white px-2 py-1">{integration.sync_mode}</span>
							<span class="rounded-full bg-white px-2 py-1">{integration.ci_trigger_strategy}</span>
							<span class="rounded-full bg-white px-2 py-1">{integration.status}</span>
						</div>
					</button>
				{/each}

				{#if !integrations.length}
					<div class="rounded-2xl border border-dashed border-slate-200 px-4 py-6 text-sm text-slate-500">
						No repository integrations yet. Create the first one from the form.
					</div>
				{/if}
			</div>
		</aside>

		<section class="px-6 py-5">
			{#if error}
				<div class="mb-4 rounded-2xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-700">{error}</div>
			{/if}

			<div class="grid gap-6 2xl:grid-cols-[0.95fr,1.05fr]">
				<div class="space-y-4">
					<div class="rounded-3xl border border-slate-200 bg-slate-50 p-5">
						<div class="flex items-center justify-between gap-3">
							<div>
								<div class="text-xs font-semibold uppercase tracking-[0.22em] text-slate-400">Configuration</div>
								<div class="mt-1 text-lg font-semibold text-slate-950">{draft.id ? 'Edit integration' : 'Create integration'}</div>
							</div>
							{#if selectedRepository}
								<span class="rounded-full border border-slate-200 bg-white px-3 py-1 text-xs text-slate-600">default branch: {selectedRepository.default_branch}</span>
							{/if}
						</div>

						<div class="mt-4 grid gap-4 md:grid-cols-2">
							<label class="space-y-2 text-sm text-slate-700">
								<span class="font-medium">Repository</span>
								<select value={draft.repository_id} onchange={(event) => onDraftChange({ repository_id: (event.currentTarget as HTMLSelectElement).value })} disabled={Boolean(draft.id)} class="w-full rounded-2xl border border-slate-200 bg-white px-4 py-3 outline-none transition focus:border-rose-300 disabled:cursor-not-allowed disabled:bg-slate-100">
									<option value="">Select repository</option>
									{#each repositories as repository}
										<option value={repository.id}>{repository.name}</option>
									{/each}
								</select>
							</label>

							<label class="space-y-2 text-sm text-slate-700">
								<span class="font-medium">Provider</span>
								<select value={draft.provider} onchange={(event) => onDraftChange({ provider: (event.currentTarget as HTMLSelectElement).value as 'github' | 'gitlab' })} disabled={Boolean(draft.id)} class="w-full rounded-2xl border border-slate-200 bg-white px-4 py-3 outline-none transition focus:border-rose-300 disabled:cursor-not-allowed disabled:bg-slate-100">
									<option value="github">GitHub</option>
									<option value="gitlab">GitLab</option>
								</select>
							</label>

							<label class="space-y-2 text-sm text-slate-700">
								<span class="font-medium">Namespace</span>
								<input value={draft.external_namespace} oninput={(event) => onDraftChange({ external_namespace: (event.currentTarget as HTMLInputElement).value })} class="w-full rounded-2xl border border-slate-200 bg-white px-4 py-3 outline-none transition focus:border-rose-300" />
							</label>

							<label class="space-y-2 text-sm text-slate-700">
								<span class="font-medium">Project</span>
								<input value={draft.external_project} oninput={(event) => onDraftChange({ external_project: (event.currentTarget as HTMLInputElement).value })} class="w-full rounded-2xl border border-slate-200 bg-white px-4 py-3 outline-none transition focus:border-rose-300" />
							</label>

							<label class="space-y-2 text-sm text-slate-700 md:col-span-2">
								<span class="font-medium">Remote URL</span>
								<input value={draft.external_url} oninput={(event) => onDraftChange({ external_url: (event.currentTarget as HTMLInputElement).value })} class="w-full rounded-2xl border border-slate-200 bg-white px-4 py-3 outline-none transition focus:border-rose-300" />
							</label>

							<label class="space-y-2 text-sm text-slate-700">
								<span class="font-medium">Sync mode</span>
								<input value={draft.sync_mode} oninput={(event) => onDraftChange({ sync_mode: (event.currentTarget as HTMLInputElement).value })} class="w-full rounded-2xl border border-slate-200 bg-white px-4 py-3 outline-none transition focus:border-rose-300" />
							</label>

							<label class="space-y-2 text-sm text-slate-700">
								<span class="font-medium">CI trigger strategy</span>
								<input value={draft.ci_trigger_strategy} oninput={(event) => onDraftChange({ ci_trigger_strategy: (event.currentTarget as HTMLInputElement).value })} class="w-full rounded-2xl border border-slate-200 bg-white px-4 py-3 outline-none transition focus:border-rose-300" />
							</label>

							<label class="space-y-2 text-sm text-slate-700">
								<span class="font-medium">Status</span>
								<input value={draft.status} oninput={(event) => onDraftChange({ status: (event.currentTarget as HTMLInputElement).value })} class="w-full rounded-2xl border border-slate-200 bg-white px-4 py-3 outline-none transition focus:border-rose-300" />
							</label>

							<label class="space-y-2 text-sm text-slate-700">
								<span class="font-medium">Default branch</span>
								<input value={draft.default_branch} oninput={(event) => onDraftChange({ default_branch: (event.currentTarget as HTMLInputElement).value })} class="w-full rounded-2xl border border-slate-200 bg-white px-4 py-3 outline-none transition focus:border-rose-300" />
							</label>

							<label class="space-y-2 text-sm text-slate-700 md:col-span-2">
								<span class="font-medium">Webhook URL</span>
								<input value={draft.webhook_url} oninput={(event) => onDraftChange({ webhook_url: (event.currentTarget as HTMLInputElement).value })} class="w-full rounded-2xl border border-slate-200 bg-white px-4 py-3 outline-none transition focus:border-rose-300" />
							</label>

							<label class="space-y-2 text-sm text-slate-700 md:col-span-2">
								<span class="font-medium">Branch mapping</span>
								<textarea value={draft.branch_mapping_text} oninput={(event) => onDraftChange({ branch_mapping_text: (event.currentTarget as HTMLTextAreaElement).value })} rows="4" class="w-full rounded-2xl border border-slate-200 bg-white px-4 py-3 outline-none transition focus:border-rose-300"></textarea>
							</label>
						</div>

						<div class="mt-4 flex flex-wrap gap-3">
							<button type="button" onclick={onSaveIntegration} disabled={busy} class="rounded-full bg-slate-950 px-5 py-3 text-sm font-semibold text-white transition hover:bg-slate-800 disabled:cursor-not-allowed disabled:bg-slate-400">{draft.id ? 'Save integration' : 'Create integration'}</button>
							<button type="button" onclick={onCreateNew} disabled={busy} class="rounded-full border border-slate-300 px-5 py-3 text-sm font-semibold text-slate-700 transition hover:border-slate-400 hover:bg-white disabled:cursor-not-allowed">Reset form</button>
						</div>
					</div>
				</div>

				<div class="space-y-4">
					<div class="rounded-3xl border border-slate-200 bg-slate-50 p-5">
						<div class="flex items-center justify-between gap-3">
							<div>
								<div class="text-xs font-semibold uppercase tracking-[0.22em] text-slate-400">Sync queue</div>
								<div class="mt-1 text-lg font-semibold text-slate-950">{selectedIntegration ? 'Trigger external sync' : 'Select an integration'}</div>
							</div>
							{#if selectedIntegration?.integration.last_synced_at}
								<span class="rounded-full border border-slate-200 bg-white px-3 py-1 text-xs text-slate-600">last synced {new Date(selectedIntegration.integration.last_synced_at).toLocaleString()}</span>
							{/if}
						</div>

						{#if selectedIntegration}
							<div class="mt-4 grid gap-4 md:grid-cols-2">
								<label class="space-y-2 text-sm text-slate-700">
									<span class="font-medium">Trigger</span>
									<input value={syncDraft.trigger} oninput={(event) => onSyncDraftChange({ trigger: (event.currentTarget as HTMLInputElement).value })} class="w-full rounded-2xl border border-slate-200 bg-white px-4 py-3 outline-none transition focus:border-rose-300" />
								</label>

								<label class="space-y-2 text-sm text-slate-700">
									<span class="font-medium">Branch</span>
									<input value={syncDraft.branch_name} oninput={(event) => onSyncDraftChange({ branch_name: (event.currentTarget as HTMLInputElement).value })} class="w-full rounded-2xl border border-slate-200 bg-white px-4 py-3 outline-none transition focus:border-rose-300" />
								</label>

								<label class="space-y-2 text-sm text-slate-700 md:col-span-2">
									<span class="font-medium">Commit SHA</span>
									<input value={syncDraft.commit_sha} oninput={(event) => onSyncDraftChange({ commit_sha: (event.currentTarget as HTMLInputElement).value })} class="w-full rounded-2xl border border-slate-200 bg-white px-4 py-3 outline-none transition focus:border-rose-300" />
								</label>
							</div>

							<div class="mt-4">
								<button type="button" onclick={onTriggerSync} disabled={busy} class="rounded-full bg-rose-600 px-5 py-3 text-sm font-semibold text-white transition hover:bg-rose-500 disabled:cursor-not-allowed disabled:bg-rose-300">Queue sync run</button>
							</div>
						{:else}
							<div class="mt-4 rounded-2xl border border-dashed border-slate-200 px-4 py-6 text-sm text-slate-500">
								Pick an integration from the left column or create a new one.
							</div>
						{/if}
					</div>

					<div class="rounded-3xl border border-slate-200 bg-slate-50 p-5">
						<div class="flex items-center justify-between gap-3">
							<div class="text-xs font-semibold uppercase tracking-[0.22em] text-slate-400">Recent sync runs</div>
							<div class="text-sm text-slate-500">{selectedIntegration?.sync_runs.length ?? 0} runs</div>
						</div>
						<div class="mt-3 space-y-3">
							{#each selectedIntegration?.sync_runs ?? [] as run (run.id)}
								<div class="rounded-2xl border border-slate-200 bg-white p-4">
									<div class="flex flex-wrap items-center justify-between gap-3">
										<div>
											<div class="text-sm font-semibold text-slate-950">{run.summary}</div>
											<div class="mt-1 text-xs text-slate-500">branch {run.branch_name} · commit {shortSha(run.commit_sha)}</div>
										</div>
										<span class="rounded-full border border-slate-200 px-3 py-1 text-xs uppercase tracking-[0.16em] text-slate-500">{run.status}</span>
									</div>
									<div class="mt-3 flex flex-wrap gap-2">
										{#each run.checks as check}
											<span class="rounded-full bg-slate-100 px-2 py-1 text-xs text-slate-600">{check}</span>
										{/each}
									</div>
								</div>
							{/each}

							{#if !(selectedIntegration?.sync_runs.length)}
								<div class="rounded-2xl border border-dashed border-slate-200 px-4 py-6 text-sm text-slate-500">
									No sync runs recorded for the selected integration yet.
								</div>
							{/if}
						</div>
					</div>
				</div>
			</div>
		</section>
	</div>
</section>