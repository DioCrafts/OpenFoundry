<script lang="ts">
	const archetypes = [
		{
			name: 'Connector plugin',
			description: 'Wrap an external system, validate credentials, and expose sync-ready metadata through the Rust + WASM SDK.',
			command: 'cargo run -p of-cli -- project init payment-connector --template connector --output plugins',
		},
		{
			name: 'Transform plugin',
			description: 'Package deterministic row or document transforms that can run inside pipeline or notebook execution surfaces.',
			command: 'cargo run -p of-cli -- project init pii-redactor --template transform --output plugins',
		},
		{
			name: 'Widget plugin',
			description: 'Ship a presentation layer component with manifest metadata ready for app builder and marketplace flows.',
			command: 'cargo run -p of-cli -- project init telemetry-widget --template widget --output plugins',
		},
	];

	const commandDeck = [
		'cargo run -p of-cli -- deploy plan gateway --environment staging',
		'cargo run -p of-cli -- script render "deploy {{service}} to {{env}}" --var service=gateway --var env=prod',
		'cargo run -p of-cli -- docs generate-openapi --output apps/web/static/generated/openapi/openfoundry.json',
		'cargo run -p of-cli -- docs generate-sdk-typescript --input apps/web/static/generated/openapi/openfoundry.json --output sdks/typescript/openfoundry-sdk',
		'cargo run -p of-cli -- docs generate-sdk-python --input apps/web/static/generated/openapi/openfoundry.json --output sdks/python/openfoundry-sdk',
		'cargo run -p of-cli -- docs generate-sdk-java --input apps/web/static/generated/openapi/openfoundry.json --output sdks/java/openfoundry-sdk',
		'cargo run -p of-cli -- terraform schema --output apps/web/static/generated/terraform/openfoundry-provider.json',
	];

	const cookbooks = [
		{
			title: 'Mirror GitHub packages into Code Repos',
			focus: 'Set up a bidirectional sync, map release branches, and wire GitHub Actions triggers to repository integrations.',
		},
		{
			title: 'Generate API contracts for SDK consumers',
			focus: 'Regenerate the OpenAPI document and the official TypeScript, Python, and Java SDKs from the same checked-in contract without touching handwritten docs.',
		},
		{
			title: 'Codify audit and Nexus surfaces with Terraform',
			focus: 'Manage audit policies, repository integrations, and cross-org peers alongside environment provisioning.',
		},
		{
			title: 'Bootstrap a WASM widget package',
			focus: 'Scaffold a new widget crate, fill in the manifest, and prepare the distribution assets consumed by app builder.',
		},
		{
			title: 'Ship a Slate React starter',
			focus: 'Use `@open-foundry/sdk/react` hooks plus the generated Slate package to move from Workshop into a pro-code React app quickly.',
		},
	];
</script>

<section class="space-y-6 rounded-[32px] border border-slate-200 bg-white p-6 shadow-sm">
	<div class="flex flex-wrap items-start justify-between gap-4">
		<div>
			<div class="text-xs font-semibold uppercase tracking-[0.28em] text-amber-600">SDK + CLI</div>
			<h2 class="mt-2 text-2xl font-semibold text-slate-950">Build plugins and automate delivery</h2>
			<p class="mt-2 max-w-3xl text-sm text-slate-600">
				The `plugin-sdk` crate standardizes Rust + WASM manifests for connectors, transforms, and widgets. The `of` CLI scaffolds projects, renders deployment scripts, emits proto-derived docs, generates the official TypeScript, Python, and Java SDKs, and now includes React-friendly helpers for Slate-style apps on top of the TypeScript SDK.
			</p>
		</div>

		<div class="rounded-3xl border border-amber-200 bg-amber-50 px-4 py-3 text-sm text-amber-900">
			<div class="font-semibold">Included in Milestone 5.2</div>
			<div class="mt-1 text-amber-800">SDK crate, official TypeScript/Python/Java SDKs, React hooks for Slate apps, CLI commands, generated docs, Terraform schema, and portal surfaces.</div>
		</div>
	</div>

	<div class="grid gap-4 xl:grid-cols-3">
		{#each archetypes as archetype}
			<div class="rounded-3xl border border-slate-200 bg-gradient-to-b from-white to-slate-50 p-5">
				<div class="text-lg font-semibold text-slate-950">{archetype.name}</div>
				<p class="mt-2 text-sm text-slate-600">{archetype.description}</p>
				<pre class="mt-4 overflow-x-auto rounded-2xl bg-slate-950 p-4 text-xs text-slate-100">{archetype.command}</pre>
			</div>
		{/each}
	</div>

	<div class="grid gap-6 xl:grid-cols-[1.1fr,0.9fr]">
		<div class="rounded-3xl border border-slate-200 bg-slate-50 p-5">
			<div class="text-xs font-semibold uppercase tracking-[0.22em] text-slate-400">CLI cookbook</div>
			<div class="mt-3 space-y-3">
				{#each commandDeck as command}
					<pre class="overflow-x-auto rounded-2xl bg-slate-950 p-4 text-xs text-slate-100">{command}</pre>
				{/each}
			</div>
		</div>

		<div class="rounded-3xl border border-slate-200 bg-slate-50 p-5">
			<div class="text-xs font-semibold uppercase tracking-[0.22em] text-slate-400">Cookbooks</div>
			<div class="mt-3 space-y-3">
				{#each cookbooks as cookbook}
					<div class="rounded-2xl border border-slate-200 bg-white p-4">
						<div class="text-base font-semibold text-slate-950">{cookbook.title}</div>
						<p class="mt-1 text-sm text-slate-600">{cookbook.focus}</p>
					</div>
				{/each}
			</div>
		</div>
	</div>
</section>
