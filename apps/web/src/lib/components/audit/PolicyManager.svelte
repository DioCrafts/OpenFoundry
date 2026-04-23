<script lang="ts">
	import type { AuditPolicy, ClassificationCatalogEntry, ClassificationLevel } from '$lib/api/audit';

	type PolicyDraft = {
		id?: string;
		name: string;
		description: string;
		scope: string;
		classification: ClassificationLevel;
		retention_days: string;
		legal_hold: boolean;
		purge_mode: string;
		active: boolean;
		rules_text: string;
		updated_by: string;
	};

	export let policies: AuditPolicy[] = [];
	export let classifications: ClassificationCatalogEntry[] = [];
	export let selectedPolicyId = '';
	export let draft: PolicyDraft;
	export let busy = false;
	export let onSelectPolicy: (policyId: string) => void;
	export let onDraftChange: (patch: Partial<PolicyDraft>) => void;
	export let onSave: () => void;
	export let onReset: () => void;

	function inputValue(event: Event) {
		return (event.currentTarget as HTMLInputElement).value;
	}

	function textValue(event: Event) {
		return (event.currentTarget as HTMLTextAreaElement).value;
	}

	function boolValue(event: Event) {
		return (event.currentTarget as HTMLInputElement).checked;
	}
</script>

<section class="rounded-3xl border border-stone-200 bg-white p-5 shadow-sm shadow-stone-200/60">
	<div class="flex flex-col gap-4 lg:flex-row lg:items-start lg:justify-between">
		<div>
			<p class="text-xs font-semibold uppercase tracking-[0.24em] text-amber-700">Policy Manager</p>
			<h3 class="mt-2 text-xl font-semibold text-stone-900">Retention TTL, legal hold, and purge modes</h3>
			<p class="mt-1 text-sm text-stone-500">Manage audit retention policies by scope and sensitivity class.</p>
		</div>
		<div class="flex gap-2">
			<button class="rounded-full border border-stone-300 px-4 py-2 text-sm font-medium text-stone-700 transition hover:border-stone-400 hover:bg-stone-50" onclick={onReset} disabled={busy}>New policy</button>
			<button class="rounded-full bg-amber-600 px-4 py-2 text-sm font-semibold text-white transition hover:bg-amber-700 disabled:cursor-not-allowed disabled:bg-amber-300" onclick={onSave} disabled={busy}>{draft.id ? 'Update policy' : 'Create policy'}</button>
		</div>
	</div>

	<div class="mt-5 grid gap-4 xl:grid-cols-[0.9fr_1.1fr]">
		<div class="space-y-3 rounded-2xl border border-stone-200 bg-stone-50/80 p-4">
			{#each policies as policy}
				<button class={`w-full rounded-2xl border px-4 py-4 text-left transition ${selectedPolicyId === policy.id ? 'border-amber-500 bg-amber-50' : 'border-stone-200 bg-white hover:border-amber-300 hover:bg-amber-50/60'}`} onclick={() => onSelectPolicy(policy.id)}>
					<div class="flex items-start justify-between gap-3">
						<div>
							<p class="font-semibold text-stone-900">{policy.name}</p>
							<p class="text-sm text-stone-500">{policy.scope} • {policy.updated_by}</p>
						</div>
						<span class={`rounded-full px-3 py-1 text-xs font-semibold uppercase tracking-[0.18em] ${policy.active ? 'bg-emerald-100 text-emerald-700' : 'bg-stone-100 text-stone-700'}`}>{policy.active ? 'Active' : 'Paused'}</span>
					</div>
					<div class="mt-3 flex flex-wrap gap-2 text-xs text-stone-600">
						<span class="rounded-full bg-stone-100 px-2 py-1">{policy.classification}</span>
						<span class="rounded-full bg-stone-100 px-2 py-1">{policy.retention_days} days</span>
						<span class="rounded-full bg-stone-100 px-2 py-1">{policy.purge_mode}</span>
					</div>
				</button>
			{/each}
		</div>

		<div class="rounded-2xl border border-stone-200 bg-stone-950 p-4 text-stone-100">
			<div class="grid gap-4 md:grid-cols-2">
				<label class="block text-sm md:col-span-2">
					<span class="mb-2 block font-medium text-stone-100">Name</span>
					<input class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-amber-400" value={draft.name} oninput={(event) => onDraftChange({ name: inputValue(event) })} />
				</label>
				<label class="block text-sm md:col-span-2">
					<span class="mb-2 block font-medium text-stone-100">Description</span>
					<textarea class="min-h-24 w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-amber-400" oninput={(event) => onDraftChange({ description: textValue(event) })}>{draft.description}</textarea>
				</label>
				<label class="block text-sm">
					<span class="mb-2 block font-medium text-stone-100">Scope</span>
					<input class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-amber-400" value={draft.scope} oninput={(event) => onDraftChange({ scope: inputValue(event) })} />
				</label>
				<label class="block text-sm">
					<span class="mb-2 block font-medium text-stone-100">Classification</span>
					<select class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-amber-400" value={draft.classification} onchange={(event) => onDraftChange({ classification: (event.currentTarget as HTMLSelectElement).value as ClassificationLevel })}>
						{#each classifications as option}
							<option value={option.classification}>{option.classification}</option>
						{/each}
					</select>
				</label>
				<label class="block text-sm">
					<span class="mb-2 block font-medium text-stone-100">Retention days</span>
					<input class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-amber-400" value={draft.retention_days} oninput={(event) => onDraftChange({ retention_days: inputValue(event) })} />
				</label>
				<label class="block text-sm">
					<span class="mb-2 block font-medium text-stone-100">Purge mode</span>
					<input class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-amber-400" value={draft.purge_mode} oninput={(event) => onDraftChange({ purge_mode: inputValue(event) })} />
				</label>
				<label class="inline-flex items-center gap-3 rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 text-sm text-stone-200">
					<input type="checkbox" checked={draft.legal_hold} onchange={(event) => onDraftChange({ legal_hold: boolValue(event) })} />
					<span>Legal hold</span>
				</label>
				<label class="inline-flex items-center gap-3 rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 text-sm text-stone-200">
					<input type="checkbox" checked={draft.active} onchange={(event) => onDraftChange({ active: boolValue(event) })} />
					<span>Policy active</span>
				</label>
				<label class="block text-sm md:col-span-2">
					<span class="mb-2 block font-medium text-stone-100">Rules</span>
					<textarea class="min-h-24 w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-amber-400" oninput={(event) => onDraftChange({ rules_text: textValue(event) })}>{draft.rules_text}</textarea>
				</label>
				<label class="block text-sm md:col-span-2">
					<span class="mb-2 block font-medium text-stone-100">Updated by</span>
					<input class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-amber-400" value={draft.updated_by} oninput={(event) => onDraftChange({ updated_by: inputValue(event) })} />
				</label>
			</div>
		</div>
	</div>
</section>
