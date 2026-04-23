<script lang="ts">
	import type { TerraformProviderSchema } from '$lib/api/developer';

	interface Props {
		schema: TerraformProviderSchema | null;
		loading?: boolean;
		error?: string;
	}

	let { schema, loading = false, error = '' }: Props = $props();

	const example = `provider "openfoundry" {
  api_url   = "https://platform.openfoundry.local"
  token     = var.openfoundry_token
  workspace = "production"
}

resource "openfoundry_repository_integration" "github_widget_kit" {
  repository_id       = "0196839d-d210-7f8c-8a1d-7ab001030001"
  provider            = "github"
  external_project    = "foundry-widget-kit"
  sync_mode           = "bidirectional_mirror"
  ci_trigger_strategy = "github_actions"
}`;
</script>

<section class="rounded-[32px] border border-slate-200 bg-white p-6 shadow-sm">
	<div class="flex flex-wrap items-start justify-between gap-4">
		<div>
			<div class="text-xs font-semibold uppercase tracking-[0.28em] text-sky-600">Terraform Provider</div>
			<h2 class="mt-2 text-2xl font-semibold text-slate-950">Infrastructure-as-code surface</h2>
			<p class="mt-2 max-w-3xl text-sm text-slate-600">
				The generated provider schema turns OpenFoundry resources into stable IaC primitives. Platform teams can version repository integrations, audit policies, and Nexus peers the same way they manage networks, databases, and queues.
			</p>
		</div>

		<div class="rounded-3xl border border-sky-200 bg-sky-50 px-4 py-3 text-sm text-sky-900">
			<div class="font-semibold">Generator</div>
			<div class="mt-1">`just terraform-schema`</div>
		</div>
	</div>

	{#if loading}
		<div class="mt-6 text-sm text-slate-500">Loading provider schema...</div>
	{:else if error}
		<div class="mt-6 text-sm text-rose-600">{error}</div>
	{:else if !schema}
		<div class="mt-6 text-sm text-slate-500">The provider schema has not been generated yet.</div>
	{:else}
		<div class="mt-6 grid gap-6 xl:grid-cols-[0.95fr,1.05fr]">
			<div class="space-y-4">
				<div class="rounded-3xl border border-slate-200 bg-slate-50 p-5">
					<div class="text-xs font-semibold uppercase tracking-[0.22em] text-slate-400">Provider configuration</div>
					<div class="mt-3 space-y-3">
						{#each Object.entries(schema.provider.configuration) as [name, description]}
							<div class="rounded-2xl border border-slate-200 bg-white p-4">
								<div class="font-semibold text-slate-950">{name}</div>
								<p class="mt-1 text-sm text-slate-600">{description}</p>
							</div>
						{/each}
					</div>
				</div>

				<div class="rounded-3xl border border-slate-200 bg-slate-50 p-5">
					<div class="text-xs font-semibold uppercase tracking-[0.22em] text-slate-400">Example</div>
					<pre class="mt-3 overflow-x-auto rounded-2xl bg-slate-950 p-4 text-xs text-slate-100">{example}</pre>
				</div>
			</div>

			<div class="space-y-4">
				<div class="rounded-3xl border border-slate-200 bg-slate-50 p-5">
					<div class="flex items-center justify-between gap-3">
						<div class="text-xs font-semibold uppercase tracking-[0.22em] text-slate-400">Resources</div>
						<div class="text-sm text-slate-500">{schema.resources.length} managed surfaces</div>
					</div>
					<div class="mt-3 space-y-3">
						{#each schema.resources as resource}
							<div class="rounded-2xl border border-slate-200 bg-white p-4">
								<div class="text-base font-semibold text-slate-950">{resource.name}</div>
								<p class="mt-1 text-sm text-slate-600">{resource.description}</p>
								<div class="mt-3 flex flex-wrap gap-2">
									{#each Object.entries(resource.attributes) as [name, description]}
										<span class="rounded-full border border-slate-200 px-3 py-1 text-xs text-slate-600" title={description}>{name}</span>
									{/each}
								</div>
							</div>
						{/each}
					</div>
				</div>

				<div class="rounded-3xl border border-slate-200 bg-slate-50 p-5">
					<div class="flex items-center justify-between gap-3">
						<div class="text-xs font-semibold uppercase tracking-[0.22em] text-slate-400">Data sources</div>
						<div class="text-sm text-slate-500">{schema.data_sources.length} exported views</div>
					</div>
					<div class="mt-3 space-y-3">
						{#each schema.data_sources as dataSource}
							<div class="rounded-2xl border border-slate-200 bg-white p-4">
								<div class="text-base font-semibold text-slate-950">{dataSource.name}</div>
								<p class="mt-1 text-sm text-slate-600">{dataSource.description}</p>
								<div class="mt-3 flex flex-wrap gap-2">
									{#each Object.entries(dataSource.attributes) as [name, description]}
										<span class="rounded-full border border-slate-200 px-3 py-1 text-xs text-slate-600" title={description}>{name}</span>
									{/each}
								</div>
							</div>
						{/each}
					</div>
				</div>
			</div>
		</div>
	{/if}
</section>