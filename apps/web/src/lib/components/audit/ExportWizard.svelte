<script lang="ts">
	import type { ComplianceReport, ComplianceStandard, GdprEraseResponse, GdprExportPayload } from '$lib/api/audit';

	type ReportDraft = {
		standard: ComplianceStandard;
		title: string;
		scope: string;
		window_start: string;
		window_end: string;
	};

	type GdprDraft = {
		subject_id: string;
		portable_format: string;
		hard_delete: boolean;
		legal_hold: boolean;
	};

	export let reports: ComplianceReport[] = [];
	export let reportDraft: ReportDraft;
	export let gdprDraft: GdprDraft;
	export let exportPayload: GdprExportPayload | null = null;
	export let eraseResponse: GdprEraseResponse | null = null;
	export let busy = false;
	export let onReportDraftChange: (patch: Partial<ReportDraft>) => void;
	export let onGdprDraftChange: (patch: Partial<GdprDraft>) => void;
	export let onGenerateReport: () => void;
	export let onExportSubject: () => void;
	export let onEraseSubject: () => void;

	const standards: ComplianceStandard[] = ['soc2', 'iso27001', 'hipaa', 'gdpr', 'itar'];

	function inputValue(event: Event) {
		return (event.currentTarget as HTMLInputElement).value;
	}

	function boolValue(event: Event) {
		return (event.currentTarget as HTMLInputElement).checked;
	}
</script>

<section class="rounded-3xl border border-stone-200 bg-white p-5 shadow-sm shadow-stone-200/60">
	<div class="flex flex-col gap-4 lg:flex-row lg:items-start lg:justify-between">
		<div>
			<p class="text-xs font-semibold uppercase tracking-[0.24em] text-fuchsia-700">Export Wizard</p>
			<h3 class="mt-2 text-xl font-semibold text-stone-900">Compliance evidence and GDPR workflows</h3>
			<p class="mt-1 text-sm text-stone-500">Generate standard-specific evidence packs, export subject data, or request erasure/masking flows.</p>
		</div>
		<button class="rounded-full bg-fuchsia-600 px-4 py-2 text-sm font-semibold text-white transition hover:bg-fuchsia-700 disabled:cursor-not-allowed disabled:bg-fuchsia-300" onclick={onGenerateReport} disabled={busy}>Generate report</button>
	</div>

	<div class="mt-5 grid gap-4 xl:grid-cols-[1fr_1fr]">
		<div class="space-y-4 rounded-2xl border border-stone-200 bg-stone-50/80 p-4">
			<p class="text-xs font-semibold uppercase tracking-[0.18em] text-stone-500">Compliance report</p>
			<div class="grid gap-4 md:grid-cols-2">
				<label class="block text-sm">
					<span class="mb-2 block font-medium text-stone-700">Standard</span>
					<select class="w-full rounded-2xl border border-stone-300 bg-white px-4 py-3 outline-none transition focus:border-fuchsia-500" value={reportDraft.standard} onchange={(event) => onReportDraftChange({ standard: (event.currentTarget as HTMLSelectElement).value as ComplianceStandard })}>
						{#each standards as standard}
							<option value={standard}>{standard}</option>
						{/each}
					</select>
				</label>
				<label class="block text-sm">
					<span class="mb-2 block font-medium text-stone-700">Scope</span>
					<input class="w-full rounded-2xl border border-stone-300 bg-white px-4 py-3 outline-none transition focus:border-fuchsia-500" value={reportDraft.scope} oninput={(event) => onReportDraftChange({ scope: inputValue(event) })} />
				</label>
				<label class="block text-sm md:col-span-2">
					<span class="mb-2 block font-medium text-stone-700">Title</span>
					<input class="w-full rounded-2xl border border-stone-300 bg-white px-4 py-3 outline-none transition focus:border-fuchsia-500" value={reportDraft.title} oninput={(event) => onReportDraftChange({ title: inputValue(event) })} />
				</label>
				<label class="block text-sm">
					<span class="mb-2 block font-medium text-stone-700">Window start</span>
					<input type="datetime-local" class="w-full rounded-2xl border border-stone-300 bg-white px-4 py-3 outline-none transition focus:border-fuchsia-500" value={reportDraft.window_start} oninput={(event) => onReportDraftChange({ window_start: inputValue(event) })} />
				</label>
				<label class="block text-sm">
					<span class="mb-2 block font-medium text-stone-700">Window end</span>
					<input type="datetime-local" class="w-full rounded-2xl border border-stone-300 bg-white px-4 py-3 outline-none transition focus:border-fuchsia-500" value={reportDraft.window_end} oninput={(event) => onReportDraftChange({ window_end: inputValue(event) })} />
				</label>
			</div>

			<div class="rounded-2xl border border-stone-200 bg-white px-4 py-4">
				<p class="text-sm font-semibold text-stone-900">Recent evidence packs</p>
				<div class="mt-3 space-y-3">
					{#each reports as report}
						<div class="rounded-2xl border border-stone-200 bg-stone-50 px-4 py-4">
							<p class="font-medium text-stone-900">{report.title}</p>
							<p class="mt-1 text-sm text-stone-500">{report.standard} • {report.artifact.file_name}</p>
							<p class="mt-2 text-xs text-stone-500">{report.control_summary}</p>
						</div>
					{/each}
				</div>
			</div>
		</div>

		<div class="space-y-4 rounded-2xl border border-stone-200 bg-stone-950 p-4 text-stone-100">
			<div>
				<p class="text-xs font-semibold uppercase tracking-[0.18em] text-fuchsia-300">GDPR actions</p>
				<div class="mt-4 grid gap-4 md:grid-cols-2">
					<label class="block text-sm md:col-span-2">
						<span class="mb-2 block font-medium text-stone-100">Subject ID</span>
						<input class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-fuchsia-400" value={gdprDraft.subject_id} oninput={(event) => onGdprDraftChange({ subject_id: inputValue(event) })} />
					</label>
					<label class="block text-sm md:col-span-2">
						<span class="mb-2 block font-medium text-stone-100">Portable format</span>
						<input class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-fuchsia-400" value={gdprDraft.portable_format} oninput={(event) => onGdprDraftChange({ portable_format: inputValue(event) })} />
					</label>
					<label class="inline-flex items-center gap-3 rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 text-sm text-stone-200">
						<input type="checkbox" checked={gdprDraft.hard_delete} onchange={(event) => onGdprDraftChange({ hard_delete: boolValue(event) })} />
						<span>Hard delete</span>
					</label>
					<label class="inline-flex items-center gap-3 rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 text-sm text-stone-200">
						<input type="checkbox" checked={gdprDraft.legal_hold} onchange={(event) => onGdprDraftChange({ legal_hold: boolValue(event) })} />
						<span>Legal hold</span>
					</label>
				</div>

				<div class="mt-4 flex flex-wrap gap-2">
					<button class="rounded-full bg-cyan-500 px-4 py-2 text-sm font-semibold text-stone-950 transition hover:bg-cyan-400 disabled:cursor-not-allowed disabled:bg-cyan-200" onclick={onExportSubject} disabled={busy}>Export subject</button>
					<button class="rounded-full bg-rose-500 px-4 py-2 text-sm font-semibold text-white transition hover:bg-rose-400 disabled:cursor-not-allowed disabled:bg-rose-200" onclick={onEraseSubject} disabled={busy}>Erase subject</button>
				</div>
			</div>

			{#if exportPayload}
				<div class="rounded-2xl border border-stone-700 bg-stone-900 px-4 py-4">
					<p class="font-medium text-stone-100">Portable export</p>
					<p class="mt-2 text-sm text-stone-400">{exportPayload.subject_id} • {exportPayload.event_count} events • {exportPayload.portable_format}</p>
					<div class="mt-3 flex flex-wrap gap-2">
						{#each exportPayload.resources as resource}
							<span class="rounded-full bg-stone-800 px-2 py-1 text-xs text-stone-300">{resource}</span>
						{/each}
					</div>
				</div>
			{/if}

			{#if eraseResponse}
				<div class="rounded-2xl border border-stone-700 bg-stone-900 px-4 py-4">
					<p class="font-medium text-stone-100">Erasure response</p>
					<p class="mt-2 text-sm text-stone-400">{eraseResponse.status} • {eraseResponse.masked_event_count} events • legal hold {eraseResponse.legal_hold ? 'enabled' : 'disabled'}</p>
				</div>
			{/if}
		</div>
	</div>
</section>
