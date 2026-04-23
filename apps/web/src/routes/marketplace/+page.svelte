<script lang="ts">
	import { onMount } from 'svelte';

	import ListingDetail from '$components/marketplace/ListingDetail.svelte';
	import MarketplaceBrowser from '$components/marketplace/MarketplaceBrowser.svelte';
	import MyPackages from '$components/marketplace/MyPackages.svelte';
	import PublishWizard from '$components/marketplace/PublishWizard.svelte';
	import {
		createInstall,
		createListing,
		createReview,
		getListing,
		getOverview,
		listCategories,
		listInstalls,
		listListings,
		publishVersion,
		searchListings,
		updateListing,
		type CategoryDefinition,
		type DependencyRequirement,
		type InstallRecord,
		type ListingDefinition,
		type ListingDetail as ListingDetailModel,
		type MarketplaceOverview,
		type PackageType,
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
		changelog: string;
		dependency_mode: string;
		dependencies_text: string;
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
	};

	let overview = $state<MarketplaceOverview | null>(null);
	let categories = $state<CategoryDefinition[]>([]);
	let listings = $state<ListingDefinition[]>([]);
	let installs = $state<InstallRecord[]>([]);
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
			changelog: 'Ships the initial marketplace package metadata and route presets.',
			dependency_mode: 'strict',
			dependencies_text: JSON.stringify([
				{ package_slug: 'map-style-base', version_req: '~1.1', required: true },
			], null, 2),
			manifest_text: JSON.stringify({ entrypoint: 'widget.json', runtime: 'svelte' }, null, 2),
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
		};
	}

	function parseCsv(value: string) {
		return value.split(',').map((entry) => entry.trim()).filter(Boolean);
	}

	function parseJson<T>(value: string): T {
		return JSON.parse(value) as T;
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

	async function refreshAll(preferredListingId?: string) {
		loading = true;
		uiError = '';
		try {
			const [overviewResponse, categoriesResponse, listingsResponse, installsResponse] = await Promise.all([
				getOverview(),
				listCategories(),
				listListings(),
				listInstalls(),
			]);

			overview = overviewResponse;
			categories = categoriesResponse.items;
			listings = listingsResponse.items;
			installs = installsResponse.items;
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
				changelog: versionDraft.changelog,
				dependency_mode: versionDraft.dependency_mode,
				dependencies: parseJson<DependencyRequirement[]>(versionDraft.dependencies_text),
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
</script>

<div class="space-y-6">
	<section class="overflow-hidden rounded-[2rem] bg-gradient-to-br from-orange-100 via-white to-emerald-100 px-6 py-6 shadow-xl shadow-orange-200/50">
		<div class="flex flex-col gap-6 lg:flex-row lg:items-end lg:justify-between">
			<div class="max-w-3xl">
				<p class="text-xs font-semibold uppercase tracking-[0.28em] text-orange-700">Milestone 4.4</p>
				<h1 class="mt-3 text-3xl font-semibold tracking-tight text-stone-900">Private package registry, discovery, reviews, and one-click installs</h1>
				<p class="mt-3 text-sm leading-6 text-stone-600">Browse internal packages backed by code repositories, inspect dependency plans, install into workspaces, and publish new versions from the same interface.</p>
			</div>
			<div class="grid grid-cols-2 gap-3 sm:grid-cols-4">
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
		<ListingDetail detail={listingDetail} {busy} {reviewDraft} {installDraft} onReviewDraftChange={updateReviewDraft} onInstallDraftChange={updateInstallDraft} onCreateReview={createReviewAction} onInstall={installAction} />
		<PublishWizard listingDraft={listingDraft} versionDraft={versionDraft} hasSelectedListing={Boolean(selectedListingId)} {busy} onListingDraftChange={updateListingDraft} onVersionDraftChange={updateVersionDraft} onPublishListing={publishListingAction} onPublishVersion={publishVersionAction} />
	</div>

	<MyPackages {installs} />
</div>