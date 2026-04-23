<script lang="ts">
	import type { PackageKind, RepositoryDefinition, RepositoryOverview, RepositoryVisibility } from '$lib/api/code-repos';

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

	export let overview: RepositoryOverview | null = null;
	export let repositories: RepositoryDefinition[] = [];
	export let selectedRepositoryId = '';
	export let draft: RepositoryDraft;
	export let busy = false;
	export let onSelectRepository: (repositoryId: string) => void;
	export let onDraftChange: (patch: Partial<RepositoryDraft>) => void;
	export let onSave: () => void;
	export let onReset: () => void;

	const visibilities: RepositoryVisibility[] = ['private', 'public'];
	const packageKinds: PackageKind[] = ['connector', 'transform', 'widget', 'app_template', 'ml_model', 'ai_agent'];

	function inputValue(event: Event) {
		return (event.currentTarget as HTMLInputElement).value;
	}

	function textValue(event: Event) {
		return (event.currentTarget as HTMLTextAreaElement).value;
	}

	function pretty(value: string) {
		return value.replaceAll('_', ' ');
	}
</script>

<section class="rounded-3xl border border-stone-200 bg-white p-5 shadow-sm shadow-stone-200/60">
	<div class="flex flex-col gap-4 lg:flex-row lg:items-start lg:justify-between">
		<div>
			<p class="text-xs font-semibold uppercase tracking-[0.24em] text-sky-700">Repository Control Plane</p>
			<h2 class="mt-2 text-2xl font-semibold text-stone-900">Object-backed repos, package kinds, and owner metadata</h2>
			<p class="mt-1 text-sm text-stone-600">Shape repository descriptors that power branches, merge requests, CI triggers, and marketplace publication.</p>
		</div>
		<div class="grid grid-cols-2 gap-3 sm:grid-cols-4">
			<div class="rounded-2xl bg-stone-950 px-4 py-3 text-stone-50">
				<p class="text-xs uppercase tracking-[0.18em] text-sky-300">Repos</p>
				<p class="mt-2 text-2xl font-semibold">{overview?.repository_count ?? 0}</p>
			</div>
			<div class="rounded-2xl bg-stone-100 px-4 py-3 text-stone-800">
				<p class="text-xs uppercase tracking-[0.18em] text-stone-500">Private</p>
				<p class="mt-2 text-2xl font-semibold">{overview?.private_repository_count ?? 0}</p>
			</div>
			<div class="rounded-2xl bg-sky-50 px-4 py-3 text-sky-900">
				<p class="text-xs uppercase tracking-[0.18em] text-sky-600">Open MRs</p>
				<p class="mt-2 text-2xl font-semibold">{overview?.open_merge_request_count ?? 0}</p>
			</div>
			<div class="rounded-2xl bg-amber-50 px-4 py-3 text-amber-900">
				<p class="text-xs uppercase tracking-[0.18em] text-amber-600">Kinds</p>
				<p class="mt-2 text-sm font-semibold">{overview?.package_kind_mix.join(', ') || 'n/a'}</p>
			</div>
		</div>
	</div>

	<div class="mt-5 grid gap-4 xl:grid-cols-[0.95fr_1.05fr]">
		<div class="rounded-2xl border border-stone-200 bg-stone-50/80 p-4">
			<label class="block text-xs font-semibold uppercase tracking-[0.2em] text-stone-500">
				Repositories
				<select class="mt-2 w-full rounded-2xl border border-stone-300 bg-white px-4 py-3 text-sm text-stone-800 outline-none transition focus:border-sky-500" value={selectedRepositoryId} onchange={(event) => onSelectRepository((event.currentTarget as HTMLSelectElement).value)}>
					<option value="">Create a new repository</option>
					{#each repositories as repository}
						<option value={repository.id}>{repository.name} • {repository.package_kind}</option>
					{/each}
				</select>
			</label>

			<div class="mt-4 space-y-3">
				{#each repositories as repository}
					<button class={`w-full rounded-2xl border px-4 py-3 text-left transition ${selectedRepositoryId === repository.id ? 'border-sky-500 bg-sky-50' : 'border-stone-200 bg-white hover:border-sky-300 hover:bg-sky-50/60'}`} onclick={() => onSelectRepository(repository.id)}>
						<div class="flex items-center justify-between gap-3">
							<div>
								<p class="font-semibold text-stone-900">{repository.name}</p>
								<p class="text-sm text-stone-500">{repository.owner} • {repository.slug}</p>
							</div>
							<span class="rounded-full bg-stone-950 px-3 py-1 text-xs font-semibold uppercase tracking-[0.18em] text-stone-50">{pretty(repository.package_kind)}</span>
						</div>
						<p class="mt-2 text-sm text-stone-600">{repository.description}</p>
					</button>
				{/each}
			</div>
		</div>

		<div class="space-y-4 rounded-2xl border border-stone-200 bg-stone-950 p-4 text-stone-100">
			<div class="flex items-center justify-between gap-3">
				<div>
					<p class="text-xs font-semibold uppercase tracking-[0.2em] text-sky-300">Metadata</p>
					<p class="mt-2 text-sm text-stone-300">Define the package kind, visibility, backend, and settings payload.</p>
				</div>
				<div class="flex gap-2">
					<button class="rounded-full border border-stone-600 px-4 py-2 text-sm font-medium text-stone-200 transition hover:border-stone-400 hover:bg-stone-800" onclick={onReset} disabled={busy}>New draft</button>
					<button class="rounded-full bg-sky-500 px-4 py-2 text-sm font-semibold text-stone-950 transition hover:bg-sky-400 disabled:cursor-not-allowed disabled:bg-sky-200" onclick={onSave} disabled={busy}>{draft.id ? 'Update repo' : 'Create repo'}</button>
				</div>
			</div>

			<div class="grid gap-4 md:grid-cols-2">
				<label class="block text-sm">
					<span class="mb-2 block font-medium text-stone-100">Name</span>
					<input class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-sky-400" value={draft.name} oninput={(event) => onDraftChange({ name: inputValue(event) })} />
				</label>
				<label class="block text-sm">
					<span class="mb-2 block font-medium text-stone-100">Slug</span>
					<input class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-sky-400" value={draft.slug} oninput={(event) => onDraftChange({ slug: inputValue(event) })} />
				</label>
				<label class="block text-sm md:col-span-2">
					<span class="mb-2 block font-medium text-stone-100">Description</span>
					<textarea class="min-h-24 w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-sky-400" oninput={(event) => onDraftChange({ description: textValue(event) })}>{draft.description}</textarea>
				</label>
				<label class="block text-sm">
					<span class="mb-2 block font-medium text-stone-100">Owner</span>
					<input class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-sky-400" value={draft.owner} oninput={(event) => onDraftChange({ owner: inputValue(event) })} />
				</label>
				<label class="block text-sm">
					<span class="mb-2 block font-medium text-stone-100">Default branch</span>
					<input class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-sky-400" value={draft.default_branch} oninput={(event) => onDraftChange({ default_branch: inputValue(event) })} />
				</label>
				<label class="block text-sm">
					<span class="mb-2 block font-medium text-stone-100">Visibility</span>
					<select class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-sky-400" value={draft.visibility} onchange={(event) => onDraftChange({ visibility: (event.currentTarget as HTMLSelectElement).value as RepositoryVisibility })}>
						{#each visibilities as visibility}
							<option value={visibility}>{visibility}</option>
						{/each}
					</select>
				</label>
				<label class="block text-sm">
					<span class="mb-2 block font-medium text-stone-100">Package kind</span>
					<select class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-sky-400" value={draft.package_kind} onchange={(event) => onDraftChange({ package_kind: (event.currentTarget as HTMLSelectElement).value as PackageKind })}>
						{#each packageKinds as packageKind}
							<option value={packageKind}>{pretty(packageKind)}</option>
						{/each}
					</select>
				</label>
				<label class="block text-sm md:col-span-2">
					<span class="mb-2 block font-medium text-stone-100">Object store backend</span>
					<input class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-sky-400" value={draft.object_store_backend} oninput={(event) => onDraftChange({ object_store_backend: inputValue(event) })} />
				</label>
				<label class="block text-sm md:col-span-2">
					<span class="mb-2 block font-medium text-stone-100">Tags</span>
					<input class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-sky-400" value={draft.tags_text} oninput={(event) => onDraftChange({ tags_text: inputValue(event) })} />
				</label>
				<label class="block text-sm md:col-span-2">
					<span class="mb-2 block font-medium text-stone-100">Settings JSON</span>
					<textarea class="min-h-32 w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 font-mono text-xs text-sky-100 outline-none transition focus:border-sky-400" oninput={(event) => onDraftChange({ settings_text: textValue(event) })}>{draft.settings_text}</textarea>
				</label>
			</div>
		</div>
	</div>
</section>
