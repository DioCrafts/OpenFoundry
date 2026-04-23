<script lang="ts">
	import type { DownloadPayload, ReportExecution } from '$lib/api/reports';

	export let execution: ReportExecution | null = null;
	export let download: DownloadPayload | null = null;
</script>

<section class="rounded-3xl border border-stone-200 bg-white p-5 shadow-sm shadow-stone-200/60">
	<div class="flex items-start justify-between gap-4">
		<div>
			<p class="text-xs font-semibold uppercase tracking-[0.24em] text-amber-700">Preview</p>
			<h3 class="mt-2 text-xl font-semibold text-stone-900">Execution preview and generated artifact</h3>
		</div>
		{#if download}
			<a class="rounded-full border border-stone-300 px-4 py-2 text-sm font-medium text-stone-700 transition hover:border-stone-400 hover:bg-stone-50" href={download.storage_url} target="_blank" rel="noreferrer">
				Open artifact
			</a>
		{/if}
	</div>

	{#if execution}
		<div class="mt-5 grid gap-4 xl:grid-cols-[1.15fr_0.85fr]">
			<div class="space-y-4 rounded-2xl border border-stone-200 bg-stone-50 p-4">
				<div>
					<p class="text-lg font-semibold text-stone-900">{execution.preview.headline}</p>
					<p class="mt-1 text-sm text-stone-500">{execution.preview.generated_for} • {execution.preview.engine}</p>
				</div>
				<div class="grid gap-3 md:grid-cols-3">
					{#each execution.preview.highlights as highlight}
						<div class="rounded-2xl border border-stone-200 bg-white p-4">
							<p class="text-xs font-semibold uppercase tracking-[0.18em] text-stone-500">{highlight.label}</p>
							<p class="mt-3 text-2xl font-semibold text-stone-900">{highlight.value}</p>
							<p class="mt-1 text-sm text-emerald-700">{highlight.delta}</p>
						</div>
					{/each}
				</div>

				<div class="space-y-3">
					{#each execution.preview.sections as section}
						<div class="rounded-2xl border border-stone-200 bg-white p-4">
							<div class="flex items-start justify-between gap-3">
								<div>
									<p class="font-semibold text-stone-900">{section.title}</p>
									<p class="text-sm text-stone-500">{section.kind}</p>
								</div>
								<p class="max-w-xs text-right text-sm text-stone-500">{section.summary}</p>
							</div>
							<pre class="mt-3 overflow-x-auto rounded-2xl bg-stone-950 p-3 text-xs text-amber-100">{JSON.stringify(section.rows, null, 2)}</pre>
						</div>
					{/each}
				</div>
			</div>

			<div class="space-y-4 rounded-2xl border border-stone-200 bg-stone-950 p-4 text-stone-100">
				<div>
					<p class="text-xs font-semibold uppercase tracking-[0.18em] text-amber-300">Artifact</p>
					<p class="mt-2 text-lg font-semibold">{execution.artifact.file_name}</p>
					<p class="mt-1 text-sm text-stone-300">{execution.artifact.mime_type} • {execution.artifact.size_bytes.toLocaleString()} bytes</p>
				</div>
				<div class="grid gap-3 md:grid-cols-2 xl:grid-cols-1">
					<div class="rounded-2xl border border-stone-800 bg-stone-900 p-4">
						<p class="text-xs font-semibold uppercase tracking-[0.18em] text-stone-400">Generation metrics</p>
						<p class="mt-3 text-sm text-stone-200">{execution.metrics.row_count.toLocaleString()} rows</p>
						<p class="text-sm text-stone-200">{execution.metrics.section_count} sections</p>
						<p class="text-sm text-stone-200">{execution.metrics.duration_ms} ms</p>
					</div>
					<div class="rounded-2xl border border-stone-800 bg-stone-900 p-4">
						<p class="text-xs font-semibold uppercase tracking-[0.18em] text-stone-400">Distribution</p>
						<div class="mt-3 space-y-2 text-sm text-stone-200">
							{#each execution.distributions as delivery}
								<div>
									<p class="font-medium">{delivery.channel} → {delivery.target}</p>
									<p class="text-stone-400">{delivery.detail}</p>
								</div>
							{/each}
						</div>
					</div>
				</div>
				{#if download}
					<div class="rounded-2xl border border-stone-800 bg-stone-900 p-4 text-sm text-stone-300">
						<p class="font-semibold text-stone-100">Download payload</p>
						<p class="mt-2">{download.preview_excerpt}</p>
						<p class="mt-2 break-all text-xs text-stone-400">{download.storage_url}</p>
					</div>
				{/if}
			</div>
		</div>
	{:else}
		<div class="mt-5 rounded-2xl border border-dashed border-stone-300 bg-stone-50 p-8 text-center text-sm text-stone-500">
			Run a report or pick a previous execution to populate the preview.
		</div>
	{/if}
</section>
