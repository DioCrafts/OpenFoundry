<script lang="ts">
	import type { PeerOrganization, SharingContract } from '$lib/api/nexus';

	interface ContractDraft {
		id?: string;
		peer_id: string;
		name: string;
		description: string;
		dataset_locator: string;
		allowed_purposes_text: string;
		data_classes_text: string;
		residency_region: string;
		query_template: string;
		max_rows_per_query: string;
		replication_mode: string;
		encryption_profile: string;
		retention_days: string;
		status: string;
		expires_at: string;
	}

	export let contracts: SharingContract[] = [];
	export let peers: PeerOrganization[] = [];
	export let selectedContractId = '';
	export let draft: ContractDraft;
	export let busy = false;
	export let onSelect: (contractId: string) => void;
	export let onDraftChange: (patch: Partial<ContractDraft>) => void;
	export let onSave: () => void;
	export let onReset: () => void;

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
			<p class="text-xs font-semibold uppercase tracking-[0.24em] text-sky-700">Contract Manager</p>
			<h3 class="mt-2 text-xl font-semibold text-stone-900">Sharing contracts, residency, and access terms</h3>
			<p class="mt-1 text-sm text-stone-500">Define what is shared, for which purpose, under what residency, encryption, and retention constraints.</p>
		</div>
		<div class="flex gap-2">
			<button class="rounded-full border border-stone-300 px-4 py-2 text-sm font-medium text-stone-700 transition hover:border-stone-400 hover:bg-stone-50" onclick={onReset} disabled={busy}>New contract</button>
			<button class="rounded-full bg-sky-600 px-4 py-2 text-sm font-semibold text-white transition hover:bg-sky-700 disabled:cursor-not-allowed disabled:bg-sky-300" onclick={onSave} disabled={busy}>{draft.id ? 'Update contract' : 'Create contract'}</button>
		</div>
	</div>

	<div class="mt-5 grid gap-4 xl:grid-cols-[0.9fr_1.1fr]">
		<div class="space-y-3 rounded-2xl border border-stone-200 bg-stone-50/80 p-4">
			{#each contracts as contract}
				<button class={`w-full rounded-2xl border px-4 py-4 text-left transition ${selectedContractId === contract.id ? 'border-sky-500 bg-sky-50' : 'border-stone-200 bg-white hover:border-sky-300 hover:bg-sky-50/60'}`} onclick={() => onSelect(contract.id)}>
					<div class="flex items-start justify-between gap-3">
						<div>
							<p class="font-semibold text-stone-900">{contract.name}</p>
							<p class="text-sm text-stone-500">{contract.dataset_locator}</p>
						</div>
						<span class={`rounded-full px-3 py-1 text-xs font-semibold uppercase tracking-[0.18em] ${contract.status === 'active' ? 'bg-emerald-100 text-emerald-700' : 'bg-amber-100 text-amber-700'}`}>{contract.status}</span>
					</div>
					<div class="mt-3 flex flex-wrap gap-2 text-xs text-stone-600">
						{#each contract.allowed_purposes as purpose}
							<span class="rounded-full bg-stone-100 px-2 py-1">{purpose}</span>
						{/each}
					</div>
				</button>
			{/each}
		</div>

		<div class="rounded-2xl border border-stone-200 bg-stone-950 p-4 text-stone-100">
			<div class="grid gap-4 md:grid-cols-2">
				<label class="block text-sm md:col-span-2">
					<span class="mb-2 block font-medium text-stone-100">Peer</span>
					<select class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-sky-400" value={draft.peer_id} onchange={(event) => onDraftChange({ peer_id: (event.currentTarget as HTMLSelectElement).value })}>
						<option value="">Select peer</option>
						{#each peers as peer}
							<option value={peer.id}>{peer.display_name}</option>
						{/each}
					</select>
				</label>
				<label class="block text-sm md:col-span-2">
					<span class="mb-2 block font-medium text-stone-100">Name</span>
					<input class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-sky-400" value={draft.name} oninput={(event) => onDraftChange({ name: inputValue(event) })} />
				</label>
				<label class="block text-sm md:col-span-2">
					<span class="mb-2 block font-medium text-stone-100">Description</span>
					<textarea class="min-h-24 w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-sky-400" oninput={(event) => onDraftChange({ description: textValue(event) })}>{draft.description}</textarea>
				</label>
				<label class="block text-sm md:col-span-2">
					<span class="mb-2 block font-medium text-stone-100">Dataset locator</span>
					<input class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-sky-400" value={draft.dataset_locator} oninput={(event) => onDraftChange({ dataset_locator: inputValue(event) })} />
				</label>
				<label class="block text-sm">
					<span class="mb-2 block font-medium text-stone-100">Allowed purposes</span>
					<input class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-sky-400" value={draft.allowed_purposes_text} oninput={(event) => onDraftChange({ allowed_purposes_text: inputValue(event) })} />
				</label>
				<label class="block text-sm">
					<span class="mb-2 block font-medium text-stone-100">Data classes</span>
					<input class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-sky-400" value={draft.data_classes_text} oninput={(event) => onDraftChange({ data_classes_text: inputValue(event) })} />
				</label>
				<label class="block text-sm">
					<span class="mb-2 block font-medium text-stone-100">Residency region</span>
					<input class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-sky-400" value={draft.residency_region} oninput={(event) => onDraftChange({ residency_region: inputValue(event) })} />
				</label>
				<label class="block text-sm">
					<span class="mb-2 block font-medium text-stone-100">Status</span>
					<input class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-sky-400" value={draft.status} oninput={(event) => onDraftChange({ status: inputValue(event) })} />
				</label>
				<label class="block text-sm md:col-span-2">
					<span class="mb-2 block font-medium text-stone-100">Query template</span>
					<textarea class="min-h-24 w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 font-mono text-xs outline-none transition focus:border-sky-400" oninput={(event) => onDraftChange({ query_template: textValue(event) })}>{draft.query_template}</textarea>
				</label>
				<label class="block text-sm">
					<span class="mb-2 block font-medium text-stone-100">Max rows</span>
					<input class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-sky-400" value={draft.max_rows_per_query} oninput={(event) => onDraftChange({ max_rows_per_query: inputValue(event) })} />
				</label>
				<label class="block text-sm">
					<span class="mb-2 block font-medium text-stone-100">Replication mode</span>
					<input class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-sky-400" value={draft.replication_mode} oninput={(event) => onDraftChange({ replication_mode: inputValue(event) })} />
				</label>
				<label class="block text-sm">
					<span class="mb-2 block font-medium text-stone-100">Encryption profile</span>
					<input class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-sky-400" value={draft.encryption_profile} oninput={(event) => onDraftChange({ encryption_profile: inputValue(event) })} />
				</label>
				<label class="block text-sm">
					<span class="mb-2 block font-medium text-stone-100">Retention days</span>
					<input class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-sky-400" value={draft.retention_days} oninput={(event) => onDraftChange({ retention_days: inputValue(event) })} />
				</label>
				<label class="block text-sm md:col-span-2">
					<span class="mb-2 block font-medium text-stone-100">Expires at</span>
					<input type="datetime-local" class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-sky-400" value={draft.expires_at} oninput={(event) => onDraftChange({ expires_at: inputValue(event) })} />
				</label>
			</div>
		</div>
	</div>
</section>
