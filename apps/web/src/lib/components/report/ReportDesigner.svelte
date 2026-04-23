<script lang="ts">
	import type { GeneratorKind, ReportDefinition } from '$lib/api/reports';

	type ReportDraft = {
		id?: string;
		name: string;
		description: string;
		owner: string;
		generator_kind: GeneratorKind;
		dataset_name: string;
		active: boolean;
		tags_text: string;
		schedule_text: string;
		template_text: string;
		recipients_text: string;
	};

	export let reports: ReportDefinition[] = [];
	export let selectedReportId = '';
	export let draft: ReportDraft;
	export let busy = false;
	export let onSelect: (reportId: string) => void;
	export let onDraftChange: (patch: Partial<ReportDraft>) => void;
	export let onSave: () => void;
	export let onReset: () => void;

	const generators: GeneratorKind[] = ['pdf', 'excel', 'csv', 'html', 'pptx'];

	function inputValue(event: Event) {
		return (event.currentTarget as HTMLInputElement).value;
	}

	function textValue(event: Event) {
		return (event.currentTarget as HTMLTextAreaElement).value;
	}

	function boolValue(event: Event) {
		return (event.currentTarget as HTMLInputElement).checked;
	}

	function capitalize(value: string) {
		return value.toUpperCase();
	}
</script>

<section class="rounded-3xl border border-stone-200 bg-white p-5 shadow-sm shadow-stone-200/60">
	<div class="flex flex-col gap-4 lg:flex-row lg:items-start lg:justify-between">
		<div>
			<p class="text-xs font-semibold uppercase tracking-[0.24em] text-amber-700">Report Designer</p>
			<h2 class="mt-2 text-2xl font-semibold text-stone-900">Definitions, template payloads, and scheduling</h2>
			<p class="mt-1 max-w-2xl text-sm text-stone-600">
				Select an existing report or shape a new one with generator, template, schedule, and delivery bindings.
			</p>
		</div>
		<div class="flex gap-2">
			<button class="rounded-full border border-stone-300 px-4 py-2 text-sm font-medium text-stone-700 transition hover:border-stone-400 hover:bg-stone-50" onclick={onReset} disabled={busy}>
				New draft
			</button>
			<button class="rounded-full bg-amber-600 px-4 py-2 text-sm font-semibold text-white transition hover:bg-amber-700 disabled:cursor-not-allowed disabled:bg-amber-300" onclick={onSave} disabled={busy}>
				{draft.id ? 'Update report' : 'Create report'}
			</button>
		</div>
	</div>

	<div class="mt-5 grid gap-4 xl:grid-cols-[1.15fr_0.85fr]">
		<div class="space-y-4 rounded-2xl border border-stone-200 bg-stone-50/80 p-4">
			<label class="block text-xs font-semibold uppercase tracking-[0.2em] text-stone-500">
				Existing definitions
				<select
					class="mt-2 w-full rounded-2xl border border-stone-300 bg-white px-4 py-3 text-sm text-stone-800 outline-none transition focus:border-amber-500"
					value={selectedReportId}
					onchange={(event) => onSelect((event.currentTarget as HTMLSelectElement).value)}
				>
					<option value="">Create a new definition</option>
					{#each reports as report}
						<option value={report.id}>{report.name}</option>
					{/each}
				</select>
			</label>

			<div class="grid gap-4 md:grid-cols-2">
				<label class="block text-sm text-stone-700">
					<span class="mb-2 block font-medium">Name</span>
					<input class="w-full rounded-2xl border border-stone-300 bg-white px-4 py-3 outline-none transition focus:border-amber-500" value={draft.name} oninput={(event) => onDraftChange({ name: inputValue(event) })} />
				</label>
				<label class="block text-sm text-stone-700">
					<span class="mb-2 block font-medium">Owner</span>
					<input class="w-full rounded-2xl border border-stone-300 bg-white px-4 py-3 outline-none transition focus:border-amber-500" value={draft.owner} oninput={(event) => onDraftChange({ owner: inputValue(event) })} />
				</label>
				<label class="block text-sm text-stone-700 md:col-span-2">
					<span class="mb-2 block font-medium">Description</span>
					<textarea class="min-h-24 w-full rounded-2xl border border-stone-300 bg-white px-4 py-3 outline-none transition focus:border-amber-500" oninput={(event) => onDraftChange({ description: textValue(event) })}>{draft.description}</textarea>
				</label>
				<label class="block text-sm text-stone-700">
					<span class="mb-2 block font-medium">Generator</span>
					<select class="w-full rounded-2xl border border-stone-300 bg-white px-4 py-3 outline-none transition focus:border-amber-500" value={draft.generator_kind} onchange={(event) => onDraftChange({ generator_kind: (event.currentTarget as HTMLSelectElement).value as GeneratorKind })}>
						{#each generators as generator}
							<option value={generator}>{capitalize(generator)}</option>
						{/each}
					</select>
				</label>
				<label class="block text-sm text-stone-700">
					<span class="mb-2 block font-medium">Dataset</span>
					<input class="w-full rounded-2xl border border-stone-300 bg-white px-4 py-3 outline-none transition focus:border-amber-500" value={draft.dataset_name} oninput={(event) => onDraftChange({ dataset_name: inputValue(event) })} />
				</label>
				<label class="block text-sm text-stone-700 md:col-span-2">
					<span class="mb-2 block font-medium">Tags</span>
					<input class="w-full rounded-2xl border border-stone-300 bg-white px-4 py-3 outline-none transition focus:border-amber-500" value={draft.tags_text} oninput={(event) => onDraftChange({ tags_text: inputValue(event) })} placeholder="executive, weekly, revenue" />
				</label>
				<label class="inline-flex items-center gap-3 rounded-2xl border border-stone-300 bg-white px-4 py-3 text-sm text-stone-700">
					<input type="checkbox" checked={draft.active} onchange={(event) => onDraftChange({ active: boolValue(event) })} />
					<span>Definition active</span>
				</label>
			</div>
		</div>

		<div class="space-y-4 rounded-2xl border border-stone-200 bg-stone-950 p-4 text-stone-100">
			<div>
				<p class="text-xs font-semibold uppercase tracking-[0.2em] text-amber-300">Structured payloads</p>
				<p class="mt-2 text-sm text-stone-300">Use JSON to control template sections, schedule cadence, and distribution recipients.</p>
			</div>
			<label class="block text-sm">
				<span class="mb-2 block font-medium text-stone-100">Template JSON</span>
				<textarea class="min-h-40 w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 font-mono text-xs text-amber-100 outline-none transition focus:border-amber-400" oninput={(event) => onDraftChange({ template_text: textValue(event) })}>{draft.template_text}</textarea>
			</label>
			<label class="block text-sm">
				<span class="mb-2 block font-medium text-stone-100">Schedule JSON</span>
				<textarea class="min-h-28 w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 font-mono text-xs text-amber-100 outline-none transition focus:border-amber-400" oninput={(event) => onDraftChange({ schedule_text: textValue(event) })}>{draft.schedule_text}</textarea>
			</label>
			<label class="block text-sm">
				<span class="mb-2 block font-medium text-stone-100">Recipients JSON</span>
				<textarea class="min-h-28 w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 font-mono text-xs text-amber-100 outline-none transition focus:border-amber-400" oninput={(event) => onDraftChange({ recipients_text: textValue(event) })}>{draft.recipients_text}</textarea>
			</label>
		</div>
	</div>
</section>
