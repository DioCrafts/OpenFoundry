<script lang="ts">
	import type { PackageType } from '$lib/api/marketplace';

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

	export let listingDraft: ListingDraft;
	export let versionDraft: VersionDraft;
	export let hasSelectedListing = false;
	export let busy = false;
	export let onListingDraftChange: (patch: Partial<ListingDraft>) => void;
	export let onVersionDraftChange: (patch: Partial<VersionDraft>) => void;
	export let onPublishListing: () => void;
	export let onPublishVersion: () => void;

	const packageTypes: PackageType[] = ['connector', 'transform', 'widget', 'app_template', 'ml_model', 'ai_agent'];

	function inputValue(event: Event) {
		return (event.currentTarget as HTMLInputElement).value;
	}

	function textValue(event: Event) {
		return (event.currentTarget as HTMLTextAreaElement).value;
	}
</script>

<section class="rounded-3xl border border-stone-200 bg-white p-5 shadow-sm shadow-stone-200/60">
	<div class="flex flex-col gap-4 lg:flex-row lg:items-start lg:justify-between">
		<div>
			<p class="text-xs font-semibold uppercase tracking-[0.24em] text-purple-700">Publish Wizard</p>
			<h3 class="mt-2 text-xl font-semibold text-stone-900">Listing metadata and version publication</h3>
			<p class="mt-1 text-sm text-stone-500">Create or update a registry listing, then publish a new version with dependency resolution metadata.</p>
		</div>
		<div class="flex gap-2">
			<button class="rounded-full bg-purple-600 px-4 py-2 text-sm font-semibold text-white transition hover:bg-purple-700 disabled:cursor-not-allowed disabled:bg-purple-300" onclick={onPublishListing} disabled={busy}>{listingDraft.id ? 'Update listing' : 'Create listing'}</button>
			<button class="rounded-full border border-purple-300 px-4 py-2 text-sm font-medium text-purple-700 transition hover:border-purple-400 hover:bg-purple-50 disabled:cursor-not-allowed disabled:border-purple-100 disabled:text-purple-300" onclick={onPublishVersion} disabled={busy || !hasSelectedListing}>Publish version</button>
		</div>
	</div>

	<div class="mt-5 grid gap-4 xl:grid-cols-[1fr_1fr]">
		<div class="space-y-4 rounded-2xl border border-stone-200 bg-stone-50/80 p-4">
			<p class="text-xs font-semibold uppercase tracking-[0.18em] text-stone-500">Listing draft</p>
			<div class="grid gap-4 md:grid-cols-2">
				<label class="block text-sm md:col-span-2">
					<span class="mb-2 block font-medium text-stone-700">Name</span>
					<input class="w-full rounded-2xl border border-stone-300 bg-white px-4 py-3 outline-none transition focus:border-purple-500" value={listingDraft.name} oninput={(event) => onListingDraftChange({ name: inputValue(event) })} />
				</label>
				<label class="block text-sm">
					<span class="mb-2 block font-medium text-stone-700">Slug</span>
					<input class="w-full rounded-2xl border border-stone-300 bg-white px-4 py-3 outline-none transition focus:border-purple-500" value={listingDraft.slug} oninput={(event) => onListingDraftChange({ slug: inputValue(event) })} />
				</label>
				<label class="block text-sm">
					<span class="mb-2 block font-medium text-stone-700">Publisher</span>
					<input class="w-full rounded-2xl border border-stone-300 bg-white px-4 py-3 outline-none transition focus:border-purple-500" value={listingDraft.publisher} oninput={(event) => onListingDraftChange({ publisher: inputValue(event) })} />
				</label>
				<label class="block text-sm md:col-span-2">
					<span class="mb-2 block font-medium text-stone-700">Summary</span>
					<input class="w-full rounded-2xl border border-stone-300 bg-white px-4 py-3 outline-none transition focus:border-purple-500" value={listingDraft.summary} oninput={(event) => onListingDraftChange({ summary: inputValue(event) })} />
				</label>
				<label class="block text-sm md:col-span-2">
					<span class="mb-2 block font-medium text-stone-700">Description</span>
					<textarea class="min-h-28 w-full rounded-2xl border border-stone-300 bg-white px-4 py-3 outline-none transition focus:border-purple-500" oninput={(event) => onListingDraftChange({ description: textValue(event) })}>{listingDraft.description}</textarea>
				</label>
				<label class="block text-sm">
					<span class="mb-2 block font-medium text-stone-700">Category</span>
					<input class="w-full rounded-2xl border border-stone-300 bg-white px-4 py-3 outline-none transition focus:border-purple-500" value={listingDraft.category_slug} oninput={(event) => onListingDraftChange({ category_slug: inputValue(event) })} />
				</label>
				<label class="block text-sm">
					<span class="mb-2 block font-medium text-stone-700">Package type</span>
					<select class="w-full rounded-2xl border border-stone-300 bg-white px-4 py-3 outline-none transition focus:border-purple-500" value={listingDraft.package_kind} onchange={(event) => onListingDraftChange({ package_kind: (event.currentTarget as HTMLSelectElement).value as PackageType })}>
						{#each packageTypes as packageType}
							<option value={packageType}>{packageType}</option>
						{/each}
					</select>
				</label>
				<label class="block text-sm">
					<span class="mb-2 block font-medium text-stone-700">Repository slug</span>
					<input class="w-full rounded-2xl border border-stone-300 bg-white px-4 py-3 outline-none transition focus:border-purple-500" value={listingDraft.repository_slug} oninput={(event) => onListingDraftChange({ repository_slug: inputValue(event) })} />
				</label>
				<label class="block text-sm">
					<span class="mb-2 block font-medium text-stone-700">Visibility</span>
					<input class="w-full rounded-2xl border border-stone-300 bg-white px-4 py-3 outline-none transition focus:border-purple-500" value={listingDraft.visibility} oninput={(event) => onListingDraftChange({ visibility: inputValue(event) })} />
				</label>
				<label class="block text-sm md:col-span-2">
					<span class="mb-2 block font-medium text-stone-700">Tags</span>
					<input class="w-full rounded-2xl border border-stone-300 bg-white px-4 py-3 outline-none transition focus:border-purple-500" value={listingDraft.tags_text} oninput={(event) => onListingDraftChange({ tags_text: inputValue(event) })} />
				</label>
				<label class="block text-sm md:col-span-2">
					<span class="mb-2 block font-medium text-stone-700">Capabilities</span>
					<input class="w-full rounded-2xl border border-stone-300 bg-white px-4 py-3 outline-none transition focus:border-purple-500" value={listingDraft.capabilities_text} oninput={(event) => onListingDraftChange({ capabilities_text: inputValue(event) })} />
				</label>
			</div>
		</div>

		<div class="space-y-4 rounded-2xl border border-stone-200 bg-stone-950 p-4 text-stone-100">
			<p class="text-xs font-semibold uppercase tracking-[0.18em] text-purple-300">Version draft</p>
			<label class="block text-sm">
				<span class="mb-2 block font-medium text-stone-100">Version</span>
				<input class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-purple-400" value={versionDraft.version} oninput={(event) => onVersionDraftChange({ version: inputValue(event) })} />
			</label>
			<label class="block text-sm">
				<span class="mb-2 block font-medium text-stone-100">Dependency mode</span>
				<input class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-purple-400" value={versionDraft.dependency_mode} oninput={(event) => onVersionDraftChange({ dependency_mode: inputValue(event) })} />
			</label>
			<label class="block text-sm">
				<span class="mb-2 block font-medium text-stone-100">Changelog</span>
				<textarea class="min-h-24 w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-purple-400" oninput={(event) => onVersionDraftChange({ changelog: textValue(event) })}>{versionDraft.changelog}</textarea>
			</label>
			<label class="block text-sm">
				<span class="mb-2 block font-medium text-stone-100">Dependencies JSON</span>
				<textarea class="min-h-32 w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 font-mono text-xs text-purple-100 outline-none transition focus:border-purple-400" oninput={(event) => onVersionDraftChange({ dependencies_text: textValue(event) })}>{versionDraft.dependencies_text}</textarea>
			</label>
			<label class="block text-sm">
				<span class="mb-2 block font-medium text-stone-100">Manifest JSON</span>
				<textarea class="min-h-32 w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 font-mono text-xs text-purple-100 outline-none transition focus:border-purple-400" oninput={(event) => onVersionDraftChange({ manifest_text: textValue(event) })}>{versionDraft.manifest_text}</textarea>
			</label>
		</div>
	</div>
</section>
