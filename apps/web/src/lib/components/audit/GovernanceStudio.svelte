<script lang="ts">
	import type {
		CompliancePostureOverview,
		GovernanceTemplate,
		GovernanceTemplateApplication,
		SensitiveDataScanResponse
	} from '$lib/api/audit';

	type TemplateDraft = {
		scope: string;
		updated_by: string;
		scan_text: string;
	};

	export let templates: GovernanceTemplate[] = [];
	export let applications: GovernanceTemplateApplication[] = [];
	export let posture: CompliancePostureOverview | null = null;
	export let scanResult: SensitiveDataScanResponse | null = null;
	export let draft: TemplateDraft;
	export let busy = false;
	export let onDraftChange: (patch: Partial<TemplateDraft>) => void;
	export let onApplyTemplate: (slug: string) => void;
	export let onScan: () => void;

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
			<p class="text-xs font-semibold uppercase tracking-[0.24em] text-emerald-700">Governance Studio</p>
			<h3 class="mt-2 text-xl font-semibold text-stone-900">Templates, compliance posture, and SDS remediation</h3>
			<p class="mt-1 text-sm text-stone-500">Apply governance baselines by scope, monitor coverage by standard, and run sensitive-data scans from one surface.</p>
		</div>
		<div class="rounded-2xl bg-emerald-50 px-4 py-3 text-sm text-emerald-900">
			<div class="text-xs uppercase tracking-[0.18em] text-emerald-600">Template apps</div>
			<div class="mt-2 text-2xl font-semibold">{posture?.active_template_application_count ?? applications.length}</div>
		</div>
	</div>

	<div class="mt-5 grid gap-4 xl:grid-cols-[1.1fr_0.9fr]">
		<div class="space-y-4">
			<div class="grid gap-4 md:grid-cols-2 xl:grid-cols-3">
				{#each posture?.standards ?? [] as standard}
					<div class="rounded-2xl border border-stone-200 bg-stone-50/80 px-4 py-4">
						<div class="flex items-center justify-between gap-3">
							<div class="text-sm font-semibold uppercase tracking-[0.16em] text-stone-900">{standard.standard}</div>
							<div class="rounded-full bg-stone-950 px-3 py-1 text-xs font-semibold text-stone-50">{standard.coverage_score}%</div>
						</div>
						<p class="mt-3 text-xs text-stone-500">{standard.evidence_summary}</p>
						<div class="mt-3 flex flex-wrap gap-2 text-xs text-stone-600">
							<span class="rounded-full bg-white px-2 py-1">{standard.applied_scope_count} scopes</span>
							<span class="rounded-full bg-white px-2 py-1">{standard.active_policy_count} policies</span>
							<span class="rounded-full bg-white px-2 py-1">{standard.checkpoint_prompt_count} prompts</span>
						</div>
					</div>
				{/each}
			</div>

			<div class="rounded-2xl border border-stone-200 bg-stone-50/80 p-4">
				<div class="flex items-center justify-between gap-3">
					<p class="text-sm font-semibold text-stone-900">Applied governance templates</p>
					<p class="text-xs uppercase tracking-[0.18em] text-stone-500">Auditable by scope</p>
				</div>
				<div class="mt-3 space-y-3">
					{#each applications as application}
						<div class="rounded-2xl border border-stone-200 bg-white px-4 py-4">
							<div class="flex items-start justify-between gap-3">
								<div>
									<p class="font-medium text-stone-900">{application.template_name}</p>
									<p class="text-sm text-stone-500">{application.scope} • {application.applied_by}</p>
								</div>
								<span class="rounded-full bg-emerald-100 px-3 py-1 text-xs font-semibold uppercase tracking-[0.16em] text-emerald-700">{application.default_report_standard}</span>
							</div>
							<div class="mt-3 flex flex-wrap gap-2">
								{#each application.standards as standard}
									<span class="rounded-full bg-stone-100 px-2 py-1 text-xs text-stone-700">{standard}</span>
								{/each}
							</div>
						</div>
					{/each}
				</div>
			</div>
		</div>

		<div class="space-y-4 rounded-2xl border border-stone-200 bg-stone-950 p-4 text-stone-100">
			<div>
				<p class="text-xs font-semibold uppercase tracking-[0.18em] text-emerald-300">Apply template</p>
				<div class="mt-4 grid gap-4">
					<label class="block text-sm">
						<span class="mb-2 block font-medium text-stone-100">Scope / project</span>
						<input class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-emerald-400" value={draft.scope} oninput={(event) => onDraftChange({ scope: inputValue(event) })} />
					</label>
					<label class="block text-sm">
						<span class="mb-2 block font-medium text-stone-100">Applied by</span>
						<input class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-emerald-400" value={draft.updated_by} oninput={(event) => onDraftChange({ updated_by: inputValue(event) })} />
					</label>
				</div>
			</div>

			<div class="space-y-3">
				{#each templates as template}
					<div class="rounded-2xl border border-stone-700 bg-stone-900 px-4 py-4">
						<div class="flex items-start justify-between gap-3">
							<div>
								<p class="font-medium text-stone-100">{template.name}</p>
								<p class="mt-1 text-sm text-stone-400">{template.summary}</p>
							</div>
							<button class="rounded-full bg-emerald-400 px-3 py-2 text-xs font-semibold text-stone-950 transition hover:bg-emerald-300 disabled:cursor-not-allowed disabled:bg-emerald-200" onclick={() => onApplyTemplate(template.slug)} disabled={busy}>Apply</button>
						</div>
						<div class="mt-3 flex flex-wrap gap-2">
							{#each template.standards as standard}
								<span class="rounded-full bg-stone-800 px-2 py-1 text-xs text-stone-300">{standard}</span>
							{/each}
							<span class="rounded-full bg-emerald-950 px-2 py-1 text-xs text-emerald-300">report {template.default_report_standard}</span>
						</div>
						<div class="mt-3 grid gap-3 md:grid-cols-2">
							<div>
								<p class="text-xs uppercase tracking-[0.16em] text-stone-500">Checkpoint prompts</p>
								<ul class="mt-2 space-y-1 text-xs text-stone-300">
									{#each template.checkpoint_prompts as prompt}
										<li>{prompt}</li>
									{/each}
								</ul>
							</div>
							<div>
								<p class="text-xs uppercase tracking-[0.16em] text-stone-500">SDS remediations</p>
								<ul class="mt-2 space-y-1 text-xs text-stone-300">
									{#each template.sds_remediations as remediation}
										<li>{remediation}</li>
									{/each}
								</ul>
							</div>
						</div>
					</div>
				{/each}
			</div>

			<div class="rounded-2xl border border-stone-700 bg-stone-900 px-4 py-4">
				<p class="text-xs font-semibold uppercase tracking-[0.18em] text-emerald-300">Sensitive Data Scanner</p>
				<textarea class="mt-3 min-h-28 w-full rounded-2xl border border-stone-700 bg-stone-950 px-4 py-3 text-sm outline-none transition focus:border-emerald-400" oninput={(event) => onDraftChange({ scan_text: textValue(event) })}>{draft.scan_text}</textarea>
				<div class="mt-3 flex items-center justify-between gap-3">
					<p class="text-xs text-stone-400">Run a remediation-oriented scan over sample payloads.</p>
					<button class="rounded-full border border-emerald-400 px-4 py-2 text-sm font-medium text-emerald-300 transition hover:bg-emerald-950 disabled:cursor-not-allowed disabled:border-emerald-800 disabled:text-emerald-800" onclick={onScan} disabled={busy}>Scan</button>
				</div>
				{#if scanResult}
					<div class="mt-4 rounded-2xl border border-stone-700 bg-stone-950 px-4 py-4">
						<p class="font-medium text-stone-100">Risk score {scanResult.risk_score}</p>
						<div class="mt-3 flex flex-wrap gap-2">
							{#each scanResult.findings as finding}
								<span class="rounded-full bg-rose-950 px-2 py-1 text-xs text-rose-300">{finding.kind} × {finding.match_count}</span>
							{/each}
						</div>
						<p class="mt-3 text-xs text-stone-400">{scanResult.redacted_content}</p>
					</div>
				{/if}
			</div>
		</div>
	</div>
</section>
