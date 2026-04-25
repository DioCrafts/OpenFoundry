<script lang="ts">
	import InstallDialog from '$components/marketplace/InstallDialog.svelte';
	import type { ListingDetail as ListingDetailModel, ProductFleetRecord } from '$lib/api/marketplace';

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

	export let detail: ListingDetailModel | null = null;
	export let reviewDraft: ReviewDraft;
	export let installDraft: InstallDraft;
	export let fleets: ProductFleetRecord[] = [];
	export let busy = false;
	export let onReviewDraftChange: (patch: Partial<ReviewDraft>) => void;
	export let onInstallDraftChange: (patch: Partial<InstallDraft>) => void;
	export let onCreateReview: () => void;
	export let onInstall: () => void;

	function inputValue(event: Event) {
		return (event.currentTarget as HTMLInputElement).value;
	}

	function textValue(event: Event) {
		return (event.currentTarget as HTMLTextAreaElement).value;
	}

	function boolValue(event: Event) {
		return (event.currentTarget as HTMLInputElement).checked;
	}
</script>

<section class="rounded-3xl border border-stone-200 bg-white p-5 shadow-sm shadow-stone-200/60">
	<div>
		<p class="text-xs font-semibold uppercase tracking-[0.24em] text-cyan-700">Listing Detail</p>
		<h3 class="mt-2 text-xl font-semibold text-stone-900">Package metadata, versions, dependency plans, and reviews</h3>
		<p class="mt-1 text-sm text-stone-500">Inspect the currently selected listing, publish-ready metadata, and installation surface.</p>
	</div>

	{#if detail}
		<div class="mt-5 grid gap-4 xl:grid-cols-[1.02fr_0.98fr]">
			<div class="space-y-4 rounded-2xl border border-stone-200 bg-stone-50/80 p-4">
				<div class="rounded-2xl border border-stone-200 bg-white px-4 py-4">
					<div class="flex items-start justify-between gap-3">
						<div>
							<p class="text-lg font-semibold text-stone-900">{detail.listing.name}</p>
							<p class="text-sm text-stone-500">{detail.listing.publisher} • {detail.listing.package_kind} • {detail.listing.repository_slug}</p>
						</div>
						<span class="rounded-full bg-cyan-100 px-3 py-1 text-xs font-semibold uppercase tracking-[0.18em] text-cyan-700">{detail.listing.average_rating.toFixed(1)} rating</span>
					</div>
					<p class="mt-3 text-sm text-stone-700">{detail.listing.description}</p>
					<div class="mt-4 flex flex-wrap gap-2">
						{#each detail.listing.capabilities as capability}
							<span class="rounded-full bg-stone-100 px-2 py-1 text-xs text-stone-600">{capability}</span>
						{/each}
					</div>
				</div>

				<div class="rounded-2xl border border-stone-200 bg-white px-4 py-4">
					<div class="flex items-center justify-between gap-3">
						<p class="font-semibold text-stone-900">Published versions</p>
						<p class="text-xs uppercase tracking-[0.18em] text-stone-500">{detail.versions.length} versions</p>
					</div>
					<div class="mt-3 space-y-3">
						{#each detail.versions as version}
							<div class="rounded-2xl border border-stone-200 bg-stone-50 px-4 py-4">
								<div class="flex items-center justify-between gap-3">
									<p class="font-medium text-stone-900">{version.version}</p>
									<div class="flex flex-wrap gap-2">
										<span class="rounded-full bg-cyan-100 px-3 py-1 text-xs font-semibold uppercase tracking-[0.18em] text-cyan-700">{version.release_channel}</span>
										<span class="rounded-full bg-white px-3 py-1 text-xs font-semibold uppercase tracking-[0.18em] text-stone-600">{version.dependency_mode}</span>
									</div>
								</div>
								<p class="mt-2 text-sm text-stone-600">{version.changelog}</p>
								<div class="mt-3 flex flex-wrap gap-2">
									{#each version.dependencies as dependency}
										<span class="rounded-full bg-white px-2 py-1 text-xs text-stone-600">{dependency.package_slug} {dependency.version_req}</span>
									{/each}
								</div>
								{#if version.packaged_resources.length > 0}
									<div class="mt-3 rounded-2xl border border-stone-200 bg-white px-3 py-3">
										<p class="text-xs font-semibold uppercase tracking-[0.18em] text-stone-500">Packaged resources</p>
										<div class="mt-2 flex flex-wrap gap-2">
											{#each version.packaged_resources as resource}
												<span class="rounded-full bg-stone-100 px-2 py-1 text-xs text-stone-700">{resource.kind} · {resource.name}</span>
											{/each}
										</div>
									</div>
								{/if}
							</div>
						{/each}
					</div>
				</div>

				<div class="rounded-2xl border border-stone-200 bg-white px-4 py-4">
					<p class="font-semibold text-stone-900">Reviews</p>
					<div class="mt-3 space-y-3">
						{#each detail.reviews as review}
							<div class="rounded-2xl border border-stone-200 bg-stone-50 px-4 py-4">
								<div class="flex items-center justify-between gap-3">
									<div>
										<p class="font-medium text-stone-900">{review.headline}</p>
										<p class="text-sm text-stone-500">{review.author}</p>
									</div>
									<span class="rounded-full bg-amber-100 px-3 py-1 text-xs font-semibold uppercase tracking-[0.18em] text-amber-700">{review.rating}/5</span>
								</div>
								<p class="mt-3 text-sm text-stone-600">{review.body}</p>
							</div>
						{/each}
					</div>
				</div>
			</div>

			<div class="space-y-4 rounded-2xl border border-stone-200 bg-stone-950 p-4 text-stone-100">
				<InstallDialog
					versions={detail.versions.map((version) => version.version)}
					version={installDraft.version}
					workspaceName={installDraft.workspace_name}
					releaseChannel={installDraft.release_channel}
					fleetId={installDraft.fleet_id}
					enrollmentBranch={installDraft.enrollment_branch}
					fleets={fleets.filter((fleet) => fleet.listing_id === detail.listing.id)}
					{busy}
					onVersionChange={(version) => onInstallDraftChange({ version })}
					onWorkspaceNameChange={(workspace_name) => onInstallDraftChange({ workspace_name })}
					onReleaseChannelChange={(release_channel) => onInstallDraftChange({ release_channel })}
					onFleetChange={(fleet_id) => onInstallDraftChange({ fleet_id })}
					onEnrollmentBranchChange={(enrollment_branch) => onInstallDraftChange({ enrollment_branch })}
					onInstall={onInstall}
				/>

				<div>
					<p class="text-xs font-semibold uppercase tracking-[0.2em] text-cyan-300">Add review</p>
					<div class="mt-4 grid gap-4 md:grid-cols-2">
						<label class="block text-sm">
							<span class="mb-2 block font-medium text-stone-100">Author</span>
							<input class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-cyan-400" value={reviewDraft.author} oninput={(event) => onReviewDraftChange({ author: inputValue(event) })} />
						</label>
						<label class="block text-sm">
							<span class="mb-2 block font-medium text-stone-100">Rating</span>
							<input class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-cyan-400" value={reviewDraft.rating} oninput={(event) => onReviewDraftChange({ rating: inputValue(event) })} />
						</label>
						<label class="block text-sm md:col-span-2">
							<span class="mb-2 block font-medium text-stone-100">Headline</span>
							<input class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-cyan-400" value={reviewDraft.headline} oninput={(event) => onReviewDraftChange({ headline: inputValue(event) })} />
						</label>
						<label class="block text-sm md:col-span-2">
							<span class="mb-2 block font-medium text-stone-100">Body</span>
							<textarea class="min-h-28 w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-cyan-400" oninput={(event) => onReviewDraftChange({ body: textValue(event) })}>{reviewDraft.body}</textarea>
						</label>
						<label class="inline-flex items-center gap-3 rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 text-sm text-stone-200 md:col-span-2">
							<input type="checkbox" checked={reviewDraft.recommended} onchange={(event) => onReviewDraftChange({ recommended: boolValue(event) })} />
							<span>Recommend this package</span>
						</label>
					</div>
					<button class="mt-4 rounded-full bg-cyan-500 px-4 py-2 text-sm font-semibold text-stone-950 transition hover:bg-cyan-400 disabled:cursor-not-allowed disabled:bg-cyan-200" onclick={onCreateReview} disabled={busy}>Publish review</button>
				</div>
			</div>
		</div>
	{:else}
		<div class="mt-5 rounded-2xl border border-dashed border-stone-300 bg-stone-50 px-4 py-8 text-center text-sm text-stone-500">Select a listing to inspect versions, dependencies, and install options.</div>
	{/if}
</section>
