<script lang="ts">
	import type { AuditEvent, AuditEventStatus, AuditSeverity, ClassificationCatalogEntry, ClassificationLevel } from '$lib/api/audit';

	type EventFilterDraft = {
		source_service: string;
		subject_id: string;
		classification: string;
	};

	type EventDraft = {
		source_service: string;
		channel: string;
		actor: string;
		action: string;
		resource_type: string;
		resource_id: string;
		status: AuditEventStatus;
		severity: AuditSeverity;
		classification: ClassificationLevel;
		subject_id: string;
		ip_address: string;
		location: string;
		labels_text: string;
		metadata_text: string;
		retention_days: string;
	};

	export let events: AuditEvent[] = [];
	export let classifications: ClassificationCatalogEntry[] = [];
	export let filters: EventFilterDraft;
	export let draft: EventDraft;
	export let busy = false;
	export let onFilterChange: (patch: Partial<EventFilterDraft>) => void;
	export let onApplyFilters: () => void;
	export let onDraftChange: (patch: Partial<EventDraft>) => void;
	export let onAppendEvent: () => void;

	const statuses: AuditEventStatus[] = ['success', 'failure', 'denied'];
	const severities: AuditSeverity[] = ['low', 'medium', 'high', 'critical'];

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
			<p class="text-xs font-semibold uppercase tracking-[0.24em] text-sky-700">Audit Log Viewer</p>
			<h3 class="mt-2 text-xl font-semibold text-stone-900">Append-only events with filters and manual probes</h3>
			<p class="mt-1 text-sm text-stone-500">Filter by service, subject, or classification, and append probe events to validate the chain end to end.</p>
		</div>
		<button class="rounded-full bg-sky-500 px-4 py-2 text-sm font-semibold text-stone-950 transition hover:bg-sky-400 disabled:cursor-not-allowed disabled:bg-sky-200" onclick={onAppendEvent} disabled={busy}>Append event</button>
	</div>

	<div class="mt-5 grid gap-4 xl:grid-cols-[1.02fr_0.98fr]">
		<div class="space-y-4 rounded-2xl border border-stone-200 bg-stone-50/80 p-4">
			<div class="grid gap-4 md:grid-cols-3">
				<label class="block text-sm">
					<span class="mb-2 block font-medium text-stone-700">Source service</span>
					<input class="w-full rounded-2xl border border-stone-300 bg-white px-4 py-3 outline-none transition focus:border-sky-500" value={filters.source_service} oninput={(event) => onFilterChange({ source_service: inputValue(event) })} />
				</label>
				<label class="block text-sm">
					<span class="mb-2 block font-medium text-stone-700">Subject ID</span>
					<input class="w-full rounded-2xl border border-stone-300 bg-white px-4 py-3 outline-none transition focus:border-sky-500" value={filters.subject_id} oninput={(event) => onFilterChange({ subject_id: inputValue(event) })} />
				</label>
				<label class="block text-sm">
					<span class="mb-2 block font-medium text-stone-700">Classification</span>
					<select class="w-full rounded-2xl border border-stone-300 bg-white px-4 py-3 outline-none transition focus:border-sky-500" value={filters.classification} onchange={(event) => onFilterChange({ classification: (event.currentTarget as HTMLSelectElement).value })}>
						<option value="">All</option>
						{#each classifications as option}
							<option value={option.classification}>{option.classification}</option>
						{/each}
					</select>
				</label>
			</div>
			<button class="rounded-full border border-sky-300 px-4 py-2 text-sm font-medium text-sky-700 transition hover:border-sky-400 hover:bg-sky-50" onclick={onApplyFilters} disabled={busy}>Apply filters</button>

			<div class="space-y-3">
				{#each events as event}
					<div class="rounded-2xl border border-stone-200 bg-white px-4 py-4">
						<div class="flex items-start justify-between gap-3">
							<div>
								<p class="font-semibold text-stone-900">#{event.sequence} • {event.action}</p>
								<p class="text-sm text-stone-500">{event.source_service} • {event.actor} • {event.resource_type}:{event.resource_id}</p>
							</div>
							<span class={`rounded-full px-3 py-1 text-xs font-semibold uppercase tracking-[0.18em] ${event.severity === 'critical' ? 'bg-rose-100 text-rose-700' : 'bg-stone-100 text-stone-700'}`}>{event.severity}</span>
						</div>
						<div class="mt-3 flex flex-wrap gap-2 text-xs text-stone-600">
							<span class="rounded-full bg-stone-100 px-2 py-1">{event.status}</span>
							<span class="rounded-full bg-stone-100 px-2 py-1">{event.classification}</span>
							{#each event.labels as label}
								<span class="rounded-full bg-sky-50 px-2 py-1 text-sky-700">{label}</span>
							{/each}
						</div>
					</div>
				{/each}
			</div>
		</div>

		<div class="rounded-2xl border border-stone-200 bg-stone-950 p-4 text-stone-100">
			<p class="text-xs font-semibold uppercase tracking-[0.2em] text-sky-300">Manual event probe</p>
			<div class="mt-4 grid gap-4 md:grid-cols-2">
				<label class="block text-sm">
					<span class="mb-2 block font-medium text-stone-100">Service</span>
					<input class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-sky-400" value={draft.source_service} oninput={(event) => onDraftChange({ source_service: inputValue(event) })} />
				</label>
				<label class="block text-sm">
					<span class="mb-2 block font-medium text-stone-100">Channel</span>
					<input class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-sky-400" value={draft.channel} oninput={(event) => onDraftChange({ channel: inputValue(event) })} />
				</label>
				<label class="block text-sm">
					<span class="mb-2 block font-medium text-stone-100">Actor</span>
					<input class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-sky-400" value={draft.actor} oninput={(event) => onDraftChange({ actor: inputValue(event) })} />
				</label>
				<label class="block text-sm">
					<span class="mb-2 block font-medium text-stone-100">Action</span>
					<input class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-sky-400" value={draft.action} oninput={(event) => onDraftChange({ action: inputValue(event) })} />
				</label>
				<label class="block text-sm">
					<span class="mb-2 block font-medium text-stone-100">Resource type</span>
					<input class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-sky-400" value={draft.resource_type} oninput={(event) => onDraftChange({ resource_type: inputValue(event) })} />
				</label>
				<label class="block text-sm">
					<span class="mb-2 block font-medium text-stone-100">Resource ID</span>
					<input class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-sky-400" value={draft.resource_id} oninput={(event) => onDraftChange({ resource_id: inputValue(event) })} />
				</label>
				<label class="block text-sm">
					<span class="mb-2 block font-medium text-stone-100">Status</span>
					<select class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-sky-400" value={draft.status} onchange={(event) => onDraftChange({ status: (event.currentTarget as HTMLSelectElement).value as AuditEventStatus })}>
						{#each statuses as status}
							<option value={status}>{status}</option>
						{/each}
					</select>
				</label>
				<label class="block text-sm">
					<span class="mb-2 block font-medium text-stone-100">Severity</span>
					<select class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-sky-400" value={draft.severity} onchange={(event) => onDraftChange({ severity: (event.currentTarget as HTMLSelectElement).value as AuditSeverity })}>
						{#each severities as severity}
							<option value={severity}>{severity}</option>
						{/each}
					</select>
				</label>
				<label class="block text-sm">
					<span class="mb-2 block font-medium text-stone-100">Classification</span>
					<select class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-sky-400" value={draft.classification} onchange={(event) => onDraftChange({ classification: (event.currentTarget as HTMLSelectElement).value as ClassificationLevel })}>
						{#each classifications as option}
							<option value={option.classification}>{option.classification}</option>
						{/each}
					</select>
				</label>
				<label class="block text-sm">
					<span class="mb-2 block font-medium text-stone-100">Subject ID</span>
					<input class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-sky-400" value={draft.subject_id} oninput={(event) => onDraftChange({ subject_id: inputValue(event) })} />
				</label>
				<label class="block text-sm">
					<span class="mb-2 block font-medium text-stone-100">IP address</span>
					<input class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-sky-400" value={draft.ip_address} oninput={(event) => onDraftChange({ ip_address: inputValue(event) })} />
				</label>
				<label class="block text-sm">
					<span class="mb-2 block font-medium text-stone-100">Location</span>
					<input class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-sky-400" value={draft.location} oninput={(event) => onDraftChange({ location: inputValue(event) })} />
				</label>
				<label class="block text-sm md:col-span-2">
					<span class="mb-2 block font-medium text-stone-100">Labels</span>
					<input class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-sky-400" value={draft.labels_text} oninput={(event) => onDraftChange({ labels_text: inputValue(event) })} />
				</label>
				<label class="block text-sm md:col-span-2">
					<span class="mb-2 block font-medium text-stone-100">Metadata JSON</span>
					<textarea class="min-h-24 w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 font-mono text-xs text-sky-100 outline-none transition focus:border-sky-400" oninput={(event) => onDraftChange({ metadata_text: textValue(event) })}>{draft.metadata_text}</textarea>
				</label>
				<label class="block text-sm md:col-span-2">
					<span class="mb-2 block font-medium text-stone-100">Retention days</span>
					<input class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-sky-400" value={draft.retention_days} oninput={(event) => onDraftChange({ retention_days: inputValue(event) })} />
				</label>
			</div>
		</div>
	</div>
</section>
