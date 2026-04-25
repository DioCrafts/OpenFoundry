<script lang="ts">
	import { onMount } from 'svelte';

	import DeliveryStudio from '$components/marketplace/DeliveryStudio.svelte';
	import ListingDetail from '$components/marketplace/ListingDetail.svelte';
	import MarketplaceBrowser from '$components/marketplace/MarketplaceBrowser.svelte';
	import MyPackages from '$components/marketplace/MyPackages.svelte';
	import PublishWizard from '$components/marketplace/PublishWizard.svelte';
	import {
		createEnrollmentBranch,
		createFleet,
		createInstall,
		createListing,
		createReview,
		getListing,
		getOverview,
		listEnrollmentBranches,
		listCategories,
		listFleets,
		listInstalls,
		listListings,
		publishVersion,
		searchListings,
		syncFleet,
		updateListing,
		type CategoryDefinition,
		type DependencyRequirement,
		type EnrollmentBranchRecord,
		type InstallRecord,
		type ListingDefinition,
		type ListingDetail as ListingDetailModel,
		type MaintenanceWindow,
		type MarketplaceOverview,
		type PackagedResource,
		type PackageType,
		type ProductFleetRecord,
	} from '$lib/api/marketplace';
	import { notifications } from '$lib/stores/notifications';

	type ListingDraft = {
		id?: string;
		name: string;
		slug: string;
		summary: string;
		description: string;
		publisher: string;
		category_slug: string;
		package_kind: PackageType;
		repository_slug: string;
		visibility: string;
		tags_text: string;
		capabilities_text: string;
	};

	type VersionDraft = {
		version: string;
		release_channel: string;
		changelog: string;
		dependency_mode: string;
		dependencies_text: string;
		packaged_resources_text: string;
		manifest_text: string;
	};

	type ReviewDraft = {
		author: string;
		rating: string;
		headline: string;
		body: string;
		recommended: boolean;
	};

	type InstallDraft = {
		version: string;
		workspace_name: string;
		release_channel: string;
		fleet_id: string;
		enrollment_branch: string;
	};

	type FleetDraft = {
		name: string;
		environment: string;
		workspace_targets_text: string;
		release_channel: string;
		auto_upgrade_enabled: boolean;
		maintenance_days_text: string;
		start_hour_utc: string;
		duration_minutes: string;
		branch_strategy: string;
		rollout_strategy: string;
	};

	type BranchDraft = {
		fleet_id: string;
		name: string;
		repository_branch: string;
		notes: string;
	};

	let overview = $state<MarketplaceOverview | null>(null);
	let categories = $state<CategoryDefinition[]>([]);
	let listings = $state<ListingDefinition[]>([]);
	let installs = $state<InstallRecord[]>([]);
	let fleets = $state<ProductFleetRecord[]>([]);
	let enrollmentBranches = $state<EnrollmentBranchRecord[]>([]);
	let listingDetail = $state<ListingDetailModel | null>(null);
	let scoreById = $state<Record<string, number>>({});
	let selectedListingId = $state('');
	let selectedCategory = $state('all');
	let searchQuery = $state('widget');
	let loading = $state(true);
	let busyAction = $state('');
	let uiError = $state('');
	let listingDraft = $state<ListingDraft>(createEmptyListingDraft());
	let versionDraft = $state<VersionDraft>(createEmptyVersionDraft());
	let reviewDraft = $state<ReviewDraft>(createEmptyReviewDraft());
	let installDraft = $state<InstallDraft>(createEmptyInstallDraft());
	let fleetDraft = $state<FleetDraft>(createEmptyFleetDraft());
	let branchDraft = $state<BranchDraft>(createEmptyBranchDraft());

	const busy = $derived(loading || busyAction.length > 0);

	onMount(() => {
		void refreshAll();
	});

	function createEmptyListingDraft(): ListingDraft {
		return {
			name: 'Geo Insight Widget',
			slug: 'geo-insight-widget',
			summary: 'Map widget with clustering and route overlays for dashboards.',
			description: 'Provides a marketplace-ready geospatial widget powered by MapLibre previews.',
			publisher: 'Platform UI',
			category_slug: 'widgets',
			package_kind: 'widget',
			repository_slug: 'foundry-widget-kit',
			visibility: 'private',
			tags_text: 'maps, dashboard, geospatial',
			capabilities_text: 'maplibre, clusters, routes',
		};
	}

	function createEmptyVersionDraft(): VersionDraft {
		return {
			version: '1.0.0',
			release_channel: 'stable',
			changelog: 'Ships the initial marketplace package metadata and route presets.',
			dependency_mode: 'strict',
			dependencies_text: JSON.stringify([
				{ package_slug: 'map-style-base', version_req: '~1.1', required: true },
			], null, 2),
			packaged_resources_text: JSON.stringify([
				{ kind: 'widget', name: 'Geo Insight Widget', resource_ref: 'widgets/geo-insight', required: true },
				{ kind: 'dashboard', name: 'Geo Ops Dashboard', resource_ref: 'dashboards/geo-ops', required: false },
			], null, 2),
			manifest_text: JSON.stringify({ entrypoint: 'widget.json', runtime: 'svelte', rollout_hint: 'rolling' }, null, 2),
		};
	}

	function createEmptyReviewDraft(): ReviewDraft {
		return {
			author: 'OpenFoundry User',
			rating: '5',
			headline: 'Great internal package',
			body: 'The install flow was fast and the dependency plan was easy to understand.',
			recommended: true,
		};
	}

	function createEmptyInstallDraft(): InstallDraft {
		return {
			version: '',
			workspace_name: 'OpenFoundry Workspace',
			release_channel: 'stable',
			fleet_id: '',
			enrollment_branch: '',
		};
	}

	function createEmptyFleetDraft(): FleetDraft {
		return {
			name: 'Operations rollout fleet',
			environment: 'production',
			workspace_targets_text: 'OpenFoundry Workspace',
			release_channel: 'stable',
			auto_upgrade_enabled: true,
			maintenance_days_text: 'sun',
			start_hour_utc: '2',
			duration_minutes: '180',
			branch_strategy: 'isolated_branch_per_feature',
			rollout_strategy: 'rolling',
		};
	}

	function createEmptyBranchDraft(): BranchDraft {
		return {
			fleet_id: '',
			name: 'feature/ops-drilldown',
			repository_branch: '',
			notes: 'Sandbox branch for enrollment-level changes before promotion.',
		};
	}

	function parseCsv(value: string) {
		return value.split(',').map((entry) => entry.trim()).filter(Boolean);
	}

	function parseJson<T>(value: string): T {
		return JSON.parse(value) as T;
	}

	function fleetMaintenanceWindowFromDraft(): MaintenanceWindow {
		return {
			timezone: 'UTC',
			days: parseCsv(fleetDraft.maintenance_days_text),
			start_hour_utc: Number(fleetDraft.start_hour_utc || '2'),
			duration_minutes: Number(fleetDraft.duration_minutes || '120'),
		};
	}

	function listingToDraft(listing: ListingDefinition): ListingDraft {
		return {
			id: listing.id,
			name: listing.name,
			slug: listing.slug,
			summary: listing.summary,
			description: listing.description,
			publisher: listing.publisher,
			category_slug: listing.category_slug,
			package_kind: listing.package_kind,
			repository_slug: listing.repository_slug,
			visibility: listing.visibility,
			tags_text: listing.tags.join(', '),
			capabilities_text: listing.capabilities.join(', '),
		};
	}

	function updateListingDraft(patch: Partial<ListingDraft>) {
		listingDraft = { ...listingDraft, ...patch };
	}

	function updateVersionDraft(patch: Partial<VersionDraft>) {
		versionDraft = { ...versionDraft, ...patch };
	}

	function updateReviewDraft(patch: Partial<ReviewDraft>) {
		reviewDraft = { ...reviewDraft, ...patch };
	}

	function updateInstallDraft(patch: Partial<InstallDraft>) {
		installDraft = { ...installDraft, ...patch };
	}

	function updateFleetDraft(patch: Partial<FleetDraft>) {
		fleetDraft = { ...fleetDraft, ...patch };
	}

	function updateBranchDraft(patch: Partial<BranchDraft>) {
		branchDraft = { ...branchDraft, ...patch };
	}

	async function refreshAll(preferredListingId?: string) {
		loading = true;
		uiError = '';
		try {
			const [overviewResponse, categoriesResponse, listingsResponse, installsResponse, fleetsResponse, branchesResponse] = await Promise.all([
				getOverview(),
				listCategories(),
				listListings(),
				listInstalls(),
				listFleets(),
				listEnrollmentBranches(),
			]);

			overview = overviewResponse;
			categories = categoriesResponse.items;
			listings = listingsResponse.items;
			installs = installsResponse.items;
			fleets = fleetsResponse.items;
			enrollmentBranches = branchesResponse.items;
			scoreById = {};

			const nextListingId = preferredListingId ?? selectedListingId ?? listings[0]?.id ?? '';
			if (nextListingId) {
				await selectListing(nextListingId, false);
			} else {
				listingDetail = null;
			}
		} catch (error) {
			uiError = error instanceof Error ? error.message : 'Unable to load marketplace surfaces';
			notifications.error(uiError);
		} finally {
			loading = false;
		}
	}

	async function selectListing(listingId: string, notify = true) {
		busyAction = 'listing';
		uiError = '';
		try {
			selectedListingId = listingId;
			listingDetail = await getListing(listingId);
			listingDraft = listingToDraft(listingDetail.listing);
			installDraft = {
				...installDraft,
				version: listingDetail.latest_version?.version ?? listingDetail.versions[0]?.version ?? '',
				release_channel: listingDetail.latest_version?.release_channel ?? listingDetail.versions[0]?.release_channel ?? 'stable',
				fleet_id: fleets.find((fleet) => fleet.listing_id === listingId)?.id ?? installDraft.fleet_id,
			};
			if (notify) {
				notifications.info(`Loaded ${listingDetail.listing.name}`);
			}
		} catch (error) {
			uiError = error instanceof Error ? error.message : 'Unable to load listing';
			notifications.error(uiError);
		} finally {
			busyAction = '';
		}
	}

	async function runSearch() {
		busyAction = 'search';
		uiError = '';
		try {
			if (searchQuery.trim() || selectedCategory !== 'all') {
				const response = await searchListings(searchQuery, selectedCategory === 'all' ? undefined : selectedCategory);
				listings = response.results.map(([listing]) => listing);
				scoreById = Object.fromEntries(response.results.map(([listing, score]) => [listing.id, score]));
			} else {
				const response = await listListings();
				listings = response.items;
				scoreById = {};
			}

			if (listings[0]) {
				await selectListing(listings[0].id, false);
			}
			notifications.success(`Loaded ${listings.length} listings`);
		} catch (error) {
			uiError = error instanceof Error ? error.message : 'Unable to search listings';
			notifications.error(uiError);
		} finally {
			busyAction = '';
		}
	}

	async function publishListingAction() {
		busyAction = 'publish-listing';
		uiError = '';
		try {
			const payload = {
				name: listingDraft.name,
				slug: listingDraft.slug,
				summary: listingDraft.summary,
				description: listingDraft.description,
				publisher: listingDraft.publisher,
				category_slug: listingDraft.category_slug,
				package_kind: listingDraft.package_kind,
				repository_slug: listingDraft.repository_slug,
				visibility: listingDraft.visibility,
				tags: parseCsv(listingDraft.tags_text),
				capabilities: parseCsv(listingDraft.capabilities_text),
			};
			const listing = listingDraft.id
				? await updateListing(listingDraft.id, payload)
				: await createListing(payload);
			await refreshAll(listing.id);
			notifications.success(`${listingDraft.id ? 'Updated' : 'Created'} ${listing.name}`);
		} catch (error) {
			uiError = error instanceof Error ? error.message : 'Unable to publish listing';
			notifications.error(uiError);
		} finally {
			busyAction = '';
		}
	}

	async function publishVersionAction() {
		if (!selectedListingId) {
			notifications.warning('Select a listing before publishing a version');
			return;
		}
		busyAction = 'publish-version';
		uiError = '';
		try {
			await publishVersion(selectedListingId, {
				version: versionDraft.version,
				release_channel: versionDraft.release_channel,
				changelog: versionDraft.changelog,
				dependency_mode: versionDraft.dependency_mode,
				dependencies: parseJson<DependencyRequirement[]>(versionDraft.dependencies_text),
				packaged_resources: parseJson<PackagedResource[]>(versionDraft.packaged_resources_text),
				manifest: parseJson<Record<string, unknown>>(versionDraft.manifest_text),
			});
			await selectListing(selectedListingId, false);
			notifications.success(`Published ${versionDraft.version}`);
		} catch (error) {
			uiError = error instanceof Error ? error.message : 'Unable to publish version';
			notifications.error(uiError);
		} finally {
			busyAction = '';
		}
	}

	async function createReviewAction() {
		if (!selectedListingId) {
			notifications.warning('Select a listing before publishing a review');
			return;
		}
		busyAction = 'review';
		uiError = '';
		try {
			await createReview(selectedListingId, {
				author: reviewDraft.author,
				rating: Number(reviewDraft.rating),
				headline: reviewDraft.headline,
				body: reviewDraft.body,
				recommended: reviewDraft.recommended,
			});
			await refreshAll(selectedListingId);
			notifications.success('Published review');
		} catch (error) {
			uiError = error instanceof Error ? error.message : 'Unable to publish review';
			notifications.error(uiError);
		} finally {
			busyAction = '';
		}
	}

	async function installAction() {
		if (!selectedListingId) {
			notifications.warning('Select a listing before installing');
			return;
		}
		busyAction = 'install';
		uiError = '';
		try {
			await createInstall({
				listing_id: selectedListingId,
				version: installDraft.version,
				workspace_name: installDraft.workspace_name,
				release_channel: installDraft.release_channel,
				fleet_id: installDraft.fleet_id || null,
				enrollment_branch: installDraft.enrollment_branch || null,
			});
			await refreshAll(selectedListingId);
			notifications.success(`Installed ${listingDetail?.listing.name ?? 'package'}`);
		} catch (error) {
			uiError = error instanceof Error ? error.message : 'Unable to install package';
			notifications.error(uiError);
		} finally {
			busyAction = '';
		}
	}

	async function createFleetAction() {
		if (!selectedListingId) {
			notifications.warning('Select a listing before creating a fleet');
			return;
		}
		busyAction = 'create-fleet';
		uiError = '';
		try {
			const fleet = await createFleet({
				listing_id: selectedListingId,
				name: fleetDraft.name,
				environment: fleetDraft.environment,
				workspace_targets: parseCsv(fleetDraft.workspace_targets_text),
				release_channel: fleetDraft.release_channel,
				auto_upgrade_enabled: fleetDraft.auto_upgrade_enabled,
				maintenance_window: fleetMaintenanceWindowFromDraft(),
				branch_strategy: fleetDraft.branch_strategy,
				rollout_strategy: fleetDraft.rollout_strategy,
			});
			branchDraft = { ...branchDraft, fleet_id: fleet.id };
			await refreshAll(selectedListingId);
			notifications.success(`Created fleet ${fleet.name}`);
		} catch (error) {
			uiError = error instanceof Error ? error.message : 'Unable to create fleet';
			notifications.error(uiError);
		} finally {
			busyAction = '';
		}
	}

	async function createBranchAction() {
		if (!branchDraft.fleet_id) {
			notifications.warning('Select a fleet before creating a branch');
			return;
		}
		busyAction = 'create-branch';
		uiError = '';
		try {
			const branch = await createEnrollmentBranch({
				fleet_id: branchDraft.fleet_id,
				name: branchDraft.name,
				repository_branch: branchDraft.repository_branch || null,
				notes: branchDraft.notes,
			});
			installDraft = {
				...installDraft,
				fleet_id: branch.fleet_id,
				enrollment_branch: branch.name,
			};
			await refreshAll(selectedListingId || undefined);
			notifications.success(`Created enrollment branch ${branch.name}`);
		} catch (error) {
			uiError = error instanceof Error ? error.message : 'Unable to create enrollment branch';
			notifications.error(uiError);
		} finally {
			busyAction = '';
		}
	}

	async function syncFleetAction(fleetId: string) {
		busyAction = 'sync-fleet';
		uiError = '';
		try {
			const result = await syncFleet(fleetId);
			await refreshAll(selectedListingId || undefined);
			if (result.blocked_reason) {
				notifications.warning(result.blocked_reason);
			} else {
				notifications.success(`Synced ${result.upgraded_workspaces.length} workspace(s)`);
			}
		} catch (error) {
			uiError = error instanceof Error ? error.message : 'Unable to sync fleet';
			notifications.error(uiError);
		} finally {
			busyAction = '';
		}
	}
</script>

<div class="space-y-6">
	<section class="overflow-hidden rounded-[2rem] bg-gradient-to-br from-orange-100 via-white to-emerald-100 px-6 py-6 shadow-xl shadow-orange-200/50">
		<div class="flex flex-col gap-6 lg:flex-row lg:items-end lg:justify-between">
			<div class="max-w-3xl">
				<p class="text-xs font-semibold uppercase tracking-[0.28em] text-orange-700">Milestone 4.4</p>
				<h1 class="mt-3 text-3xl font-semibold tracking-tight text-stone-900">Private package registry, fleets, release channels, and one-click installs</h1>
				<p class="mt-3 text-sm leading-6 text-stone-600">Browse internal packages backed by code repositories, package product resources, manage rollout fleets, and open enrollment branches from the same interface.</p>
			</div>
			<div class="grid grid-cols-2 gap-3 sm:grid-cols-5">
				<div class="rounded-2xl bg-white/80 px-4 py-3 backdrop-blur">
					<p class="text-xs uppercase tracking-[0.18em] text-orange-600">Listings</p>
					<p class="mt-2 text-2xl font-semibold text-stone-900">{overview?.listing_count ?? 0}</p>
				</div>
				<div class="rounded-2xl bg-white/80 px-4 py-3 backdrop-blur">
					<p class="text-xs uppercase tracking-[0.18em] text-orange-600">Categories</p>
					<p class="mt-2 text-2xl font-semibold text-stone-900">{overview?.category_count ?? 0}</p>
				</div>
				<div class="rounded-2xl bg-white/80 px-4 py-3 backdrop-blur">
					<p class="text-xs uppercase tracking-[0.18em] text-orange-600">Installs</p>
					<p class="mt-2 text-2xl font-semibold text-stone-900">{installs.length}</p>
				</div>
				<div class="rounded-2xl bg-white/80 px-4 py-3 backdrop-blur">
					<p class="text-xs uppercase tracking-[0.18em] text-orange-600">Selected</p>
					<p class="mt-2 text-sm font-semibold text-stone-900">{listingDetail?.listing.name ?? 'None'}</p>
				</div>
				<div class="rounded-2xl bg-white/80 px-4 py-3 backdrop-blur">
					<p class="text-xs uppercase tracking-[0.18em] text-orange-600">Fleets</p>
					<p class="mt-2 text-2xl font-semibold text-stone-900">{fleets.length}</p>
				</div>
			</div>
		</div>
	</section>

	{#if uiError}
		<div class="rounded-2xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-700">{uiError}</div>
	{/if}

	<MarketplaceBrowser
		{overview}
		{categories}
		{listings}
		{selectedListingId}
		{searchQuery}
		{selectedCategory}
		{scoreById}
		{busy}
		onSearchQueryChange={(query: string) => (searchQuery = query)}
		onCategoryChange={(category: string) => (selectedCategory = category)}
		onSearch={runSearch}
		onSelectListing={selectListing}
	/>

	<div class="grid gap-6 xl:grid-cols-[1.05fr_0.95fr]">
		<ListingDetail detail={listingDetail} fleets={fleets} {busy} {reviewDraft} {installDraft} onReviewDraftChange={updateReviewDraft} onInstallDraftChange={updateInstallDraft} onCreateReview={createReviewAction} onInstall={installAction} />
		<PublishWizard listingDraft={listingDraft} versionDraft={versionDraft} hasSelectedListing={Boolean(selectedListingId)} {busy} onListingDraftChange={updateListingDraft} onVersionDraftChange={updateVersionDraft} onPublishListing={publishListingAction} onPublishVersion={publishVersionAction} />
	</div>

	<DeliveryStudio
		{fleets}
		branches={enrollmentBranches}
		{selectedListingId}
		{busy}
		{fleetDraft}
		{branchDraft}
		onFleetDraftChange={updateFleetDraft}
		onBranchDraftChange={updateBranchDraft}
		onCreateFleet={createFleetAction}
		onCreateBranch={createBranchAction}
		onSyncFleet={syncFleetAction}
	/>

	<MyPackages {installs} />
</div>
