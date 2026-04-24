<script lang="ts">
	import type { RepositoryFile, SearchResult } from '$lib/api/code-repos';

	export let files: RepositoryFile[] = [];
	export let selectedFilePath = '';
	export let searchQuery = '';
	export let searchResults: SearchResult[] = [];
	export let busy = false;
	export let onSelectFile: (path: string) => void;
	export let onSearchQueryChange: (query: string) => void;
	export let onRunSearch: () => void;

		let selectedFile: RepositoryFile | null;

		$: selectedFile = files.find((file) => file.path === selectedFilePath) ?? files[0] ?? null;

	function inputValue(event: Event) {
		return (event.currentTarget as HTMLInputElement).value;
	}
</script>

<section class="rounded-3xl border border-stone-200 bg-white p-5 shadow-sm shadow-stone-200/60">
	<div class="flex flex-col gap-4 lg:flex-row lg:items-start lg:justify-between">
		<div>
			<p class="text-xs font-semibold uppercase tracking-[0.24em] text-amber-700">File Browser</p>
			<h3 class="mt-2 text-xl font-semibold text-stone-900">Repository tree and Tantivy-style search results</h3>
			<p class="mt-1 text-sm text-stone-500">Switch across tracked files, inspect content, and search through indexed snippets.</p>
		</div>
		<div class="flex w-full max-w-md gap-2">
			<input class="w-full rounded-full border border-stone-300 bg-white px-4 py-2 text-sm outline-none transition focus:border-amber-500" value={searchQuery} oninput={(event) => onSearchQueryChange(inputValue(event))} placeholder="Search package, widget, connector..." />
			<button class="rounded-full bg-amber-500 px-4 py-2 text-sm font-semibold text-stone-950 transition hover:bg-amber-400 disabled:cursor-not-allowed disabled:bg-amber-200" onclick={onRunSearch} disabled={busy}>Search</button>
		</div>
	</div>

	<div class="mt-5 grid gap-4 xl:grid-cols-[0.8fr_1.2fr]">
		<div class="space-y-3 rounded-2xl border border-stone-200 bg-stone-50/80 p-4">
			{#each files as file}
				<button class={`w-full rounded-2xl border px-4 py-3 text-left transition ${selectedFilePath === file.path ? 'border-amber-500 bg-amber-50' : 'border-stone-200 bg-white hover:border-amber-300 hover:bg-amber-50/60'}`} onclick={() => onSelectFile(file.path)}>
					<p class="font-medium text-stone-900">{file.path}</p>
					<p class="mt-1 text-xs text-stone-500">{file.language} • {file.branch_name} • {file.size_bytes} bytes</p>
				</button>
			{/each}
		</div>

		<div class="space-y-4">
			<div class="rounded-2xl border border-stone-200 bg-stone-950 p-4 text-stone-100">
				<div class="flex items-center justify-between gap-3">
					<div>
						<p class="font-semibold">{selectedFile?.path ?? 'No file selected'}</p>
						<p class="text-xs text-stone-400">{selectedFile?.language ?? 'n/a'} • commit {selectedFile?.last_commit_sha ?? 'n/a'}</p>
					</div>
				</div>
				<pre class="mt-4 overflow-x-auto rounded-2xl border border-stone-800 bg-stone-900 p-4 text-xs text-amber-100">{selectedFile?.content ?? 'Select a file to inspect its content.'}</pre>
			</div>

			<div class="rounded-2xl border border-stone-200 bg-white p-4">
				<div class="flex items-center justify-between gap-3">
					<p class="text-sm font-semibold text-stone-900">Search results</p>
					<p class="text-xs uppercase tracking-[0.18em] text-stone-500">{searchResults.length} matches</p>
				</div>
				<div class="mt-3 space-y-3">
					{#each searchResults as result}
						<div class="rounded-2xl border border-stone-200 bg-stone-50 px-4 py-3">
							<div class="flex items-center justify-between gap-3">
								<p class="font-medium text-stone-900">{result.path}</p>
								<span class="rounded-full bg-white px-2 py-1 text-xs font-semibold text-stone-600">score {result.score.toFixed(2)}</span>
							</div>
							<p class="mt-2 text-xs text-stone-500">{result.branch_name}</p>
							<p class="mt-2 text-sm text-stone-700">{result.snippet}</p>
						</div>
					{/each}
					{#if searchResults.length === 0}
						<p class="rounded-2xl border border-dashed border-stone-200 px-4 py-6 text-sm text-stone-500">Run a query to surface indexed snippets across repository files.</p>
					{/if}
				</div>
			</div>
		</div>
	</div>
</section>
