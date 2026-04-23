<script lang="ts">
	import type { OpenApiOperation, OpenApiSpec } from '$lib/api/developer';

	interface Props {
		spec: OpenApiSpec | null;
		loading?: boolean;
		error?: string;
	}

	type ExplorerOperation = {
		key: string;
		path: string;
		method: string;
		operation: OpenApiOperation;
	};

	let { spec, loading = false, error = '' }: Props = $props();
	let search = $state('');
	let selectedKey = $state('');

	const operations = $derived(
		spec
			? Object.entries(spec.paths).flatMap(([path, methods]) =>
				Object.entries(methods).map(([method, operation]) => ({
					key: `${method}:${path}`,
					path,
					method,
					operation,
				})),
			)
			: [],
	);

	const filteredOperations = $derived(
		operations.filter((entry) => {
			const query = search.trim().toLowerCase();
			if (!query) {
				return true;
			}

			return (
				entry.path.toLowerCase().includes(query)
				|| entry.method.toLowerCase().includes(query)
				|| entry.operation.summary.toLowerCase().includes(query)
				|| entry.operation.operationId.toLowerCase().includes(query)
				|| entry.operation.tags.some((tag) => tag.toLowerCase().includes(query))
			);
		}),
	);

	$effect(() => {
		if (!filteredOperations.length) {
			selectedKey = '';
			return;
		}

		if (!filteredOperations.some((entry) => entry.key === selectedKey)) {
			selectedKey = filteredOperations[0]?.key ?? '';
		}
	});

	const selectedOperation = $derived(
		filteredOperations.find((entry) => entry.key === selectedKey)
			?? operations.find((entry) => entry.key === selectedKey)
			?? null,
	);

	function previewJson(value: unknown) {
		return JSON.stringify(value, null, 2);
	}

	function responseEntries(operation: ExplorerOperation | null) {
		return operation ? Object.entries(operation.operation.responses) : [];
	}
</script>

<section class="rounded-[32px] border border-slate-200 bg-white shadow-sm">
	<div class="border-b border-slate-200 px-6 py-5">
		<div class="flex flex-wrap items-start justify-between gap-4">
			<div>
				<div class="text-xs font-semibold uppercase tracking-[0.28em] text-emerald-600">REST API Docs</div>
				<h2 class="mt-2 text-2xl font-semibold text-slate-950">Proto-derived explorer</h2>
				<p class="mt-2 max-w-3xl text-sm text-slate-600">
					Every operation in this panel is generated from the workspace proto services. Use it to inspect request and response contracts before wiring SDK plugins, CLI scripts, or CI jobs.
				</p>
			</div>

			<div class="grid min-w-[240px] grid-cols-2 gap-3 text-sm">
				<div class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3">
					<div class="text-xs uppercase tracking-[0.2em] text-slate-400">Paths</div>
					<div class="mt-1 text-2xl font-semibold text-slate-950">{spec ? Object.keys(spec.paths).length : 0}</div>
				</div>
				<div class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3">
					<div class="text-xs uppercase tracking-[0.2em] text-slate-400">Schemas</div>
					<div class="mt-1 text-2xl font-semibold text-slate-950">{spec ? Object.keys(spec.components.schemas).length : 0}</div>
				</div>
			</div>
		</div>
	</div>

	{#if loading}
		<div class="px-6 py-12 text-sm text-slate-500">Loading OpenAPI document...</div>
	{:else if error}
		<div class="px-6 py-12 text-sm text-rose-600">{error}</div>
	{:else if !spec}
		<div class="px-6 py-12 text-sm text-slate-500">The generated OpenAPI document is not available yet.</div>
	{:else}
		<div class="grid gap-0 lg:grid-cols-[320px,1fr]">
			<aside class="border-b border-slate-200 px-6 py-5 lg:border-b-0 lg:border-r">
				<label class="text-xs font-semibold uppercase tracking-[0.22em] text-slate-400" for="developer-api-search">Find operation</label>
				<input
					id="developer-api-search"
					type="search"
					value={search}
					oninput={(event) => search = (event.currentTarget as HTMLInputElement).value}
					placeholder="search by path, tag, summary"
					class="mt-3 w-full rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 text-sm text-slate-700 outline-none transition focus:border-emerald-300 focus:bg-white"
				/>

				<div class="mt-4 space-y-2 overflow-y-auto lg:max-h-[620px]">
					{#each filteredOperations as entry (entry.key)}
						<button
							type="button"
							onclick={() => selectedKey = entry.key}
							class={`w-full rounded-2xl border px-4 py-3 text-left transition ${selectedKey === entry.key ? 'border-emerald-300 bg-emerald-50' : 'border-slate-200 hover:border-slate-300 hover:bg-slate-50'}`}
						>
							<div class="flex items-center gap-3 text-xs font-semibold uppercase tracking-[0.2em] text-slate-500">
								<span class={`rounded-full px-2 py-1 ${entry.method === 'get' ? 'bg-sky-100 text-sky-700' : entry.method === 'post' ? 'bg-emerald-100 text-emerald-700' : entry.method === 'patch' ? 'bg-amber-100 text-amber-700' : 'bg-rose-100 text-rose-700'}`}>{entry.method}</span>
								<span>{entry.operation.tags[0] ?? 'open_foundry'}</span>
							</div>
							<div class="mt-2 text-sm font-semibold text-slate-900">{entry.path}</div>
							<div class="mt-1 text-xs text-slate-500">{entry.operation.summary}</div>
						</button>
					{/each}

					{#if !filteredOperations.length}
						<div class="rounded-2xl border border-dashed border-slate-200 px-4 py-6 text-sm text-slate-500">
							No operations match the current filter.
						</div>
					{/if}
				</div>
			</aside>

			<section class="px-6 py-5">
				{#if selectedOperation}
					<div class="flex flex-wrap items-center gap-3">
						<span class={`rounded-full px-3 py-1 text-xs font-semibold uppercase tracking-[0.2em] ${selectedOperation.method === 'get' ? 'bg-sky-100 text-sky-700' : selectedOperation.method === 'post' ? 'bg-emerald-100 text-emerald-700' : selectedOperation.method === 'patch' ? 'bg-amber-100 text-amber-700' : 'bg-rose-100 text-rose-700'}`}>{selectedOperation.method}</span>
						<span class="rounded-full border border-slate-200 px-3 py-1 text-xs uppercase tracking-[0.2em] text-slate-500">{selectedOperation.operation.operationId}</span>
					</div>
					<h3 class="mt-4 text-2xl font-semibold text-slate-950">{selectedOperation.path}</h3>
					<p class="mt-2 text-sm text-slate-600">{selectedOperation.operation.summary}</p>

					<div class="mt-6 grid gap-4 xl:grid-cols-2">
						<div class="rounded-3xl border border-slate-200 bg-slate-50 p-5">
							<div class="text-xs font-semibold uppercase tracking-[0.22em] text-slate-400">Request body</div>
							{#if selectedOperation.operation.requestBody}
								<pre class="mt-3 overflow-x-auto rounded-2xl bg-slate-950 p-4 text-xs text-slate-100">{previewJson(selectedOperation.operation.requestBody.content['application/json']?.schema ?? {})}</pre>
							{:else}
								<div class="mt-3 rounded-2xl border border-dashed border-slate-200 px-4 py-6 text-sm text-slate-500">This operation does not declare a request body.</div>
							{/if}
						</div>

						<div class="rounded-3xl border border-slate-200 bg-slate-50 p-5">
							<div class="text-xs font-semibold uppercase tracking-[0.22em] text-slate-400">Responses</div>
							<div class="mt-3 space-y-4">
								{#each responseEntries(selectedOperation) as [status, response]}
									<div class="rounded-2xl border border-slate-200 bg-white p-4">
										<div class="flex items-center justify-between gap-3 text-sm">
											<div class="font-semibold text-slate-900">HTTP {status}</div>
											<div class="text-xs uppercase tracking-[0.18em] text-slate-400">{response.description}</div>
										</div>
										<pre class="mt-3 overflow-x-auto rounded-2xl bg-slate-950 p-4 text-xs text-slate-100">{previewJson(response.content['application/json']?.schema ?? {})}</pre>
									</div>
								{/each}
							</div>
						</div>
					</div>

					<div class="mt-6 rounded-3xl border border-slate-200 bg-gradient-to-r from-emerald-50 via-white to-sky-50 p-5">
						<div class="text-xs font-semibold uppercase tracking-[0.22em] text-slate-400">Tags</div>
						<div class="mt-3 flex flex-wrap gap-2">
							{#each selectedOperation.operation.tags as tag}
								<span class="rounded-full border border-emerald-200 bg-white px-3 py-1 text-sm text-slate-700">{tag}</span>
							{/each}
						</div>
					</div>
				{:else}
					<div class="rounded-3xl border border-dashed border-slate-200 px-5 py-8 text-sm text-slate-500">
						Select an operation to inspect its contract.
					</div>
				{/if}
			</section>
		</div>
	{/if}
</section>