<script lang="ts">
	import { onMount } from 'svelte';

	import BranchManager from '$components/code-repo/BranchManager.svelte';
	import CommitHistory from '$components/code-repo/CommitHistory.svelte';
	import DiffViewer from '$components/code-repo/DiffViewer.svelte';
	import FileViewer from '$components/code-repo/FileViewer.svelte';
	import MergeRequestDetail from '$components/code-repo/MergeRequestDetail.svelte';
	import MergeRequestList from '$components/code-repo/MergeRequestList.svelte';
	import RepoExplorer from '$components/code-repo/RepoExplorer.svelte';
	import {
		createBranch,
		createComment,
		createCommit,
		createMergeRequest,
		createRepository,
		getDiff,
		getMergeRequest,
		getOverview,
		listBranches,
		listCiRuns,
		listCommits,
		listFiles,
		listMergeRequests,
		listRepositories,
		searchFiles,
		triggerCiRun,
		updateMergeRequest,
		updateRepository,
		type BranchDefinition,
		type CiRun,
		type CommitDefinition,
		type MergeRequestDefinition,
		type MergeRequestDetail as MergeRequestDetailModel,
		type MergeRequestStatus,
		type PackageKind,
		type RepositoryDefinition,
		type RepositoryFile,
		type RepositoryOverview,
		type RepositoryVisibility,
		type SearchResult,
		type ReviewerState,
	} from '$lib/api/code-repos';
	import { notifications } from '$lib/stores/notifications';

	type RepositoryDraft = {
		id?: string;
		name: string;
		slug: string;
		description: string;
		owner: string;
		default_branch: string;
		visibility: RepositoryVisibility;
		object_store_backend: string;
		package_kind: PackageKind;
		tags_text: string;
		settings_text: string;
	};

	type BranchDraft = {
		name: string;
		base_branch: string;
		protected: boolean;
	};

	type CommitDraft = {
		branch_name: string;
		title: string;
		description: string;
		author_name: string;
		additions: string;
		deletions: string;
	};

	type MergeRequestDraft = {
		title: string;
		description: string;
		source_branch: string;
		target_branch: string;
		author: string;
		labels_text: string;
		reviewers_text: string;
		approvals_required: string;
		changed_files: string;
	};

	type CommentDraft = {
		author: string;
		body: string;
		file_path: string;
		line_number: string;
		resolved: boolean;
	};

	let overview = $state<RepositoryOverview | null>(null);
	let repositories = $state<RepositoryDefinition[]>([]);
	let branches = $state<BranchDefinition[]>([]);
	let commits = $state<CommitDefinition[]>([]);
	let files = $state<RepositoryFile[]>([]);
	let ciRuns = $state<CiRun[]>([]);
	let mergeRequests = $state<MergeRequestDefinition[]>([]);
	let mergeRequestDetail = $state<MergeRequestDetailModel | null>(null);
	let searchResults = $state<SearchResult[]>([]);
	let selectedRepositoryId = $state('');
	let selectedMergeRequestId = $state('');
	let selectedFilePath = $state('');
	let searchQuery = $state('widget');
	let diffBranch = $state('main');
	let diffPatch = $state('');
	let loading = $state(true);
	let busyAction = $state('');
	let uiError = $state('');
	let repositoryDraft = $state<RepositoryDraft>(createEmptyRepositoryDraft());
	let branchDraft = $state<BranchDraft>(createEmptyBranchDraft());
	let commitDraft = $state<CommitDraft>(createEmptyCommitDraft());
	let mergeRequestDraft = $state<MergeRequestDraft>(createEmptyMergeRequestDraft());
	let commentDraft = $state<CommentDraft>(createEmptyCommentDraft());

	const busy = $derived(loading || busyAction.length > 0);
	const branchOptions = $derived(branches.map((branch) => branch.name));

	onMount(() => {
		void refreshAll();
	});

	function createEmptyRepositoryDraft(): RepositoryDraft {
		return {
			name: 'Foundry Widget Kit',
			slug: 'foundry-widget-kit',
			description: 'Shared widget primitives ready for marketplace publication.',
			owner: 'Platform UI',
			default_branch: 'main',
			visibility: 'private',
			object_store_backend: 'gitoxide-pack',
			package_kind: 'widget',
			tags_text: 'widgets, ui, marketplace',
			settings_text: JSON.stringify({ default_path: 'src/lib.rs', ci_required: true }, null, 2),
		};
	}

	function createEmptyBranchDraft(defaultBranch = 'main'): BranchDraft {
		return {
			name: 'feature/new-package-flow',
			base_branch: defaultBranch,
			protected: false,
		};
	}

	function createEmptyCommitDraft(defaultBranch = 'main'): CommitDraft {
		return {
			branch_name: defaultBranch,
			title: 'Refine package manifest defaults',
			description: 'Tightens metadata and manifest defaults ahead of publication.',
			author_name: 'Platform UI',
			additions: '24',
			deletions: '6',
		};
	}

	function createEmptyMergeRequestDraft(defaultBranch = 'main'): MergeRequestDraft {
		return {
			title: 'Publish package flow improvements',
			description: 'Promotes the feature branch after CI and inline review are green.',
			source_branch: 'feature/new-package-flow',
			target_branch: defaultBranch,
			author: 'Platform UI',
			labels_text: 'preview, package',
			reviewers_text: 'Elena, Marco',
			approvals_required: '2',
			changed_files: '5',
		};
	}

	function createEmptyCommentDraft(filePath = 'src/lib.rs'): CommentDraft {
		return {
			author: 'Reviewer Bot',
			body: 'Please split the publishing helper into a smaller function before merge.',
			file_path: filePath,
			line_number: '12',
			resolved: false,
		};
	}

	function parseCsv(value: string) {
		return value.split(',').map((entry) => entry.trim()).filter(Boolean);
	}

	function parseJson<T>(value: string): T {
		return JSON.parse(value) as T;
	}

	function repositoryToDraft(repository: RepositoryDefinition): RepositoryDraft {
		return {
			id: repository.id,
			name: repository.name,
			slug: repository.slug,
			description: repository.description,
			owner: repository.owner,
			default_branch: repository.default_branch,
			visibility: repository.visibility,
			object_store_backend: repository.object_store_backend,
			package_kind: repository.package_kind,
			tags_text: repository.tags.join(', '),
			settings_text: JSON.stringify(repository.settings, null, 2),
		};
	}

	function updateRepositoryDraft(patch: Partial<RepositoryDraft>) {
		repositoryDraft = { ...repositoryDraft, ...patch };
	}

	function updateBranchDraft(patch: Partial<BranchDraft>) {
		branchDraft = { ...branchDraft, ...patch };
	}

	function updateCommitDraft(patch: Partial<CommitDraft>) {
		commitDraft = { ...commitDraft, ...patch };
	}

	function updateMergeRequestDraft(patch: Partial<MergeRequestDraft>) {
		mergeRequestDraft = { ...mergeRequestDraft, ...patch };
	}

	function updateCommentDraft(patch: Partial<CommentDraft>) {
		commentDraft = { ...commentDraft, ...patch };
	}

	async function refreshAll(preferredRepositoryId?: string, preferredMergeRequestId?: string) {
		loading = true;
		uiError = '';
		try {
			const [overviewResponse, repositoriesResponse] = await Promise.all([getOverview(), listRepositories()]);
			overview = overviewResponse;
			repositories = repositoriesResponse.items;
			const nextRepositoryId = preferredRepositoryId ?? selectedRepositoryId ?? repositories[0]?.id ?? '';
			if (nextRepositoryId) {
				await loadRepositoryContext(nextRepositoryId, preferredMergeRequestId, false);
			} else {
				branches = [];
				commits = [];
				files = [];
				ciRuns = [];
				mergeRequests = [];
				mergeRequestDetail = null;
				diffPatch = '';
			}
		} catch (error) {
			uiError = error instanceof Error ? error.message : 'Unable to load repository surfaces';
			notifications.error(uiError);
		} finally {
			loading = false;
		}
	}

	async function loadRepositoryContext(repositoryId: string, preferredMergeRequestId?: string, notify = true) {
		selectedRepositoryId = repositoryId;
		const repository = repositories.find((entry) => entry.id === repositoryId) ?? null;
		repositoryDraft = repository ? repositoryToDraft(repository) : createEmptyRepositoryDraft();
		const defaultBranch = repository?.default_branch ?? 'main';
		branchDraft = createEmptyBranchDraft(defaultBranch);
		commitDraft = createEmptyCommitDraft(defaultBranch);
		mergeRequestDraft = createEmptyMergeRequestDraft(defaultBranch);
		diffBranch = defaultBranch;

		const [branchesResponse, commitsResponse, filesResponse, ciRunsResponse, diffResponse, mergeRequestsResponse] = await Promise.all([
			listBranches(repositoryId),
			listCommits(repositoryId),
			listFiles(repositoryId),
			listCiRuns(repositoryId),
			getDiff(repositoryId, defaultBranch),
			listMergeRequests(repositoryId),
		]);

		branches = branchesResponse.items;
		commits = commitsResponse.items;
		files = filesResponse.items;
		ciRuns = ciRunsResponse.items;
		diffPatch = diffResponse.patch;
		mergeRequests = mergeRequestsResponse.items;
		selectedFilePath = files[0]?.path ?? '';
		commentDraft = createEmptyCommentDraft(selectedFilePath || 'src/lib.rs');
		searchResults = [];

		const nextMergeRequestId = preferredMergeRequestId ?? selectedMergeRequestId ?? mergeRequests[0]?.id ?? '';
		if (nextMergeRequestId) {
			await selectMergeRequest(nextMergeRequestId, false);
		} else {
			selectedMergeRequestId = '';
			mergeRequestDetail = null;
		}

		if (notify && repository) {
			notifications.info(`Loaded ${repository.name}`);
		}
	}

	async function selectRepository(repositoryId: string, notify = true) {
		if (!repositoryId) {
			selectedRepositoryId = '';
			repositoryDraft = createEmptyRepositoryDraft();
			return;
		}

		busyAction = 'repository';
		uiError = '';
		try {
			await loadRepositoryContext(repositoryId, undefined, notify);
		} catch (error) {
			uiError = error instanceof Error ? error.message : 'Unable to load repository';
			notifications.error(uiError);
		} finally {
			busyAction = '';
		}
	}

	async function refreshDiff(branchName: string) {
		if (!selectedRepositoryId) return;
		busyAction = 'diff';
		try {
			const response = await getDiff(selectedRepositoryId, branchName);
			diffBranch = response.branch_name;
			diffPatch = response.patch;
		} catch (error) {
			uiError = error instanceof Error ? error.message : 'Unable to refresh diff';
			notifications.error(uiError);
		} finally {
			busyAction = '';
		}
	}

	async function selectMergeRequest(mergeRequestId: string, notify = true) {
		busyAction = 'merge-request';
		try {
			selectedMergeRequestId = mergeRequestId;
			mergeRequestDetail = await getMergeRequest(mergeRequestId);
			commentDraft = {
				...commentDraft,
				file_path: mergeRequestDetail.comments[0]?.file_path || selectedFilePath || 'src/lib.rs',
			};
			if (notify) {
				notifications.info(`Loaded ${mergeRequestDetail.merge_request.title}`);
			}
		} catch (error) {
			uiError = error instanceof Error ? error.message : 'Unable to load merge request';
			notifications.error(uiError);
		} finally {
			busyAction = '';
		}
	}

	async function saveRepository() {
		busyAction = 'save-repository';
		uiError = '';
		try {
			const payload = {
				name: repositoryDraft.name,
				slug: repositoryDraft.slug,
				description: repositoryDraft.description,
				owner: repositoryDraft.owner,
				default_branch: repositoryDraft.default_branch,
				visibility: repositoryDraft.visibility,
				object_store_backend: repositoryDraft.object_store_backend,
				package_kind: repositoryDraft.package_kind,
				tags: parseCsv(repositoryDraft.tags_text),
				settings: parseJson<Record<string, unknown>>(repositoryDraft.settings_text),
			};
			const repository = repositoryDraft.id
				? await updateRepository(repositoryDraft.id, payload)
				: await createRepository(payload);
			await refreshAll(repository.id, selectedMergeRequestId || undefined);
			notifications.success(`${repositoryDraft.id ? 'Updated' : 'Created'} ${repository.name}`);
		} catch (error) {
			uiError = error instanceof Error ? error.message : 'Unable to save repository';
			notifications.error(uiError);
		} finally {
			busyAction = '';
		}
	}

	async function createBranchAction() {
		if (!selectedRepositoryId) {
			notifications.warning('Select a repository before creating a branch');
			return;
		}
		busyAction = 'branch';
		try {
			await createBranch(selectedRepositoryId, {
				name: branchDraft.name,
				base_branch: branchDraft.base_branch,
				protected: branchDraft.protected,
			});
			await loadRepositoryContext(selectedRepositoryId, selectedMergeRequestId || undefined, false);
			notifications.success(`Created branch ${branchDraft.name}`);
		} catch (error) {
			uiError = error instanceof Error ? error.message : 'Unable to create branch';
			notifications.error(uiError);
		} finally {
			busyAction = '';
		}
	}

	async function createCommitAction() {
		if (!selectedRepositoryId) {
			notifications.warning('Select a repository before committing');
			return;
		}
		busyAction = 'commit';
		try {
			await createCommit(selectedRepositoryId, {
				branch_name: commitDraft.branch_name,
				title: commitDraft.title,
				description: commitDraft.description,
				author_name: commitDraft.author_name,
				additions: Number(commitDraft.additions),
				deletions: Number(commitDraft.deletions),
			});
			await loadRepositoryContext(selectedRepositoryId, selectedMergeRequestId || undefined, false);
			notifications.success(`Created commit ${commitDraft.title}`);
		} catch (error) {
			uiError = error instanceof Error ? error.message : 'Unable to create commit';
			notifications.error(uiError);
		} finally {
			busyAction = '';
		}
	}

	async function triggerCiAction() {
		if (!selectedRepositoryId) {
			notifications.warning('Select a repository before triggering CI');
			return;
		}
		busyAction = 'ci';
		try {
			const run = await triggerCiRun(selectedRepositoryId, { branch_name: commitDraft.branch_name });
			ciRuns = [run, ...ciRuns];
			notifications.success(`Triggered ${run.pipeline_name}`);
		} catch (error) {
			uiError = error instanceof Error ? error.message : 'Unable to trigger CI';
			notifications.error(uiError);
		} finally {
			busyAction = '';
		}
	}

	async function runSearchAction() {
		if (!selectedRepositoryId) {
			notifications.warning('Select a repository before searching files');
			return;
		}
		busyAction = 'search';
		try {
			const response = await searchFiles(selectedRepositoryId, searchQuery);
			searchResults = response.results;
			notifications.success(`Found ${response.results.length} matches`);
		} catch (error) {
			uiError = error instanceof Error ? error.message : 'Unable to search repository files';
			notifications.error(uiError);
		} finally {
			busyAction = '';
		}
	}

	async function createMergeRequestAction() {
		if (!selectedRepositoryId) {
			notifications.warning('Select a repository before opening a merge request');
			return;
		}
		busyAction = 'create-mr';
		try {
			const reviewers: ReviewerState[] = parseCsv(mergeRequestDraft.reviewers_text).map((reviewer) => ({
				reviewer,
				approved: false,
				state: 'pending',
			}));
			const mergeRequest = await createMergeRequest({
				repository_id: selectedRepositoryId,
				title: mergeRequestDraft.title,
				description: mergeRequestDraft.description,
				source_branch: mergeRequestDraft.source_branch,
				target_branch: mergeRequestDraft.target_branch,
				author: mergeRequestDraft.author,
				labels: parseCsv(mergeRequestDraft.labels_text),
				reviewers,
				approvals_required: Number(mergeRequestDraft.approvals_required),
				changed_files: Number(mergeRequestDraft.changed_files),
			});
			await loadRepositoryContext(selectedRepositoryId, mergeRequest.id, false);
			notifications.success(`Opened ${mergeRequest.title}`);
		} catch (error) {
			uiError = error instanceof Error ? error.message : 'Unable to create merge request';
			notifications.error(uiError);
		} finally {
			busyAction = '';
		}
	}

	async function changeMergeRequestStatus(status: MergeRequestStatus) {
		if (!selectedMergeRequestId) return;
		busyAction = 'mr-status';
		try {
			await updateMergeRequest(selectedMergeRequestId, { status });
			await loadRepositoryContext(selectedRepositoryId, selectedMergeRequestId, false);
			notifications.success(`Marked merge request as ${status}`);
		} catch (error) {
			uiError = error instanceof Error ? error.message : 'Unable to update merge request';
			notifications.error(uiError);
		} finally {
			busyAction = '';
		}
	}

	async function createCommentAction() {
		if (!selectedMergeRequestId) {
			notifications.warning('Select a merge request before adding comments');
			return;
		}
		busyAction = 'comment';
		try {
			await createComment(selectedMergeRequestId, {
				author: commentDraft.author,
				body: commentDraft.body,
				file_path: commentDraft.file_path,
				line_number: commentDraft.line_number ? Number(commentDraft.line_number) : undefined,
				resolved: commentDraft.resolved,
			});
			await selectMergeRequest(selectedMergeRequestId, false);
			notifications.success('Added review comment');
		} catch (error) {
			uiError = error instanceof Error ? error.message : 'Unable to create review comment';
			notifications.error(uiError);
		} finally {
			busyAction = '';
		}
	}

	function selectFile(path: string) {
		selectedFilePath = path;
		commentDraft = { ...commentDraft, file_path: path };
	}
</script>

<div class="space-y-6">
	<section class="overflow-hidden rounded-[2rem] bg-gradient-to-br from-sky-950 via-stone-950 to-fuchsia-950 px-6 py-6 text-stone-50 shadow-xl shadow-sky-950/20">
		<div class="flex flex-col gap-6 lg:flex-row lg:items-end lg:justify-between">
			<div class="max-w-3xl">
				<p class="text-xs font-semibold uppercase tracking-[0.28em] text-sky-300">Milestone 4.4</p>
				<h1 class="mt-3 text-3xl font-semibold tracking-tight">Code repos, merge workflows, CI triggers, and indexed package search</h1>
				<p class="mt-3 text-sm leading-6 text-stone-300">Operate repository metadata, branch and commit flows, diff inspection, and review threads from a single surface that mirrors the new code-repo backend.</p>
			</div>
			<div class="grid grid-cols-2 gap-3 sm:grid-cols-4">
				<div class="rounded-2xl bg-white/10 px-4 py-3 backdrop-blur">
					<p class="text-xs uppercase tracking-[0.18em] text-sky-200">Repos</p>
					<p class="mt-2 text-2xl font-semibold">{overview?.repository_count ?? 0}</p>
				</div>
				<div class="rounded-2xl bg-white/10 px-4 py-3 backdrop-blur">
					<p class="text-xs uppercase tracking-[0.18em] text-sky-200">Open MRs</p>
					<p class="mt-2 text-2xl font-semibold">{overview?.open_merge_request_count ?? 0}</p>
				</div>
				<div class="rounded-2xl bg-white/10 px-4 py-3 backdrop-blur">
					<p class="text-xs uppercase tracking-[0.18em] text-sky-200">Files</p>
					<p class="mt-2 text-2xl font-semibold">{files.length}</p>
				</div>
				<div class="rounded-2xl bg-white/10 px-4 py-3 backdrop-blur">
					<p class="text-xs uppercase tracking-[0.18em] text-sky-200">CI Runs</p>
					<p class="mt-2 text-2xl font-semibold">{ciRuns.length}</p>
				</div>
			</div>
		</div>
	</section>

	{#if uiError}
		<div class="rounded-2xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-700">{uiError}</div>
	{/if}

	<RepoExplorer
		{overview}
		{repositories}
		{selectedRepositoryId}
		draft={repositoryDraft}
		{busy}
		onSelectRepository={selectRepository}
		onDraftChange={updateRepositoryDraft}
		onSave={saveRepository}
		onReset={() => {
			selectedRepositoryId = '';
			repositoryDraft = createEmptyRepositoryDraft();
		}}
	/>

	<div class="grid gap-6 xl:grid-cols-[1.02fr_0.98fr]">
		<FileViewer
			{files}
			{selectedFilePath}
			{searchQuery}
			{searchResults}
			{busy}
			onSelectFile={selectFile}
			onSearchQueryChange={(query) => (searchQuery = query)}
			onRunSearch={runSearchAction}
		/>
		<DiffViewer availableBranches={branchOptions} branchName={diffBranch} patch={diffPatch} {busy} onSelectBranch={refreshDiff} />
	</div>

	<div class="grid gap-6 xl:grid-cols-[0.92fr_1.08fr]">
		<BranchManager branches={branches} draft={branchDraft} {busy} onDraftChange={updateBranchDraft} onCreateBranch={createBranchAction} />
		<CommitHistory commits={commits} ciRuns={ciRuns} draft={commitDraft} {busy} onDraftChange={updateCommitDraft} onCreateCommit={createCommitAction} onTriggerCi={triggerCiAction} />
	</div>

	<div class="grid gap-6 xl:grid-cols-[0.92fr_1.08fr]">
		<MergeRequestList
			mergeRequests={mergeRequests}
			{selectedMergeRequestId}
			branchOptions={branchOptions}
			draft={mergeRequestDraft}
			{busy}
			onSelectMergeRequest={selectMergeRequest}
			onDraftChange={updateMergeRequestDraft}
			onCreateMergeRequest={createMergeRequestAction}
		/>
		<MergeRequestDetail detail={mergeRequestDetail} draft={commentDraft} {busy} onDraftChange={updateCommentDraft} onCreateComment={createCommentAction} onStatusChange={changeMergeRequestStatus} />
	</div>
</div>