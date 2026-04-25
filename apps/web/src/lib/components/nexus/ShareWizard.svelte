<script lang="ts">
	import type { NexusSpace, PeerOrganization, ShareDetail, SharingContract } from '$lib/api/nexus';

	interface ShareDraft {
		contract_id: string;
		provider_peer_id: string;
		consumer_peer_id: string;
		provider_space_id: string;
		consumer_space_id: string;
		dataset_name: string;
		selector_text: string;
		provider_schema_text: string;
		consumer_schema_text: string;
		sample_rows_text: string;
		replication_mode: string;
	}

	export let shares: ShareDetail[] = [];
	export let peers: PeerOrganization[] = [];
	export let contracts: SharingContract[] = [];
	export let spaces: NexusSpace[] = [];
	export let draft: ShareDraft;
	export let busy = false;
	export let onDraftChange: (patch: Partial<ShareDraft>) => void;
	export let onCreate: () => void;

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
			<p class="text-xs font-semibold uppercase tracking-[0.24em] text-fuchsia-700">Share Wizard</p>
			<h3 class="mt-2 text-xl font-semibold text-stone-900">Selective replication and dataset exchange</h3>
			<p class="mt-1 text-sm text-stone-500">Create cross-org shares, limit rows by selector, and define provider vs consumer schemas before federation.</p>
		</div>
		<button class="rounded-full bg-fuchsia-600 px-4 py-2 text-sm font-semibold text-white transition hover:bg-fuchsia-700 disabled:cursor-not-allowed disabled:bg-fuchsia-300" onclick={onCreate} disabled={busy}>Create share</button>
	</div>

	<div class="mt-5 grid gap-4 xl:grid-cols-[1.05fr_0.95fr]">
		<div class="rounded-2xl border border-stone-200 bg-stone-950 p-4 text-stone-100">
			<div class="grid gap-4 md:grid-cols-2">
				<label class="block text-sm md:col-span-2">
					<span class="mb-2 block font-medium text-stone-100">Contract</span>
					<select class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-fuchsia-400" value={draft.contract_id} onchange={(event) => onDraftChange({ contract_id: (event.currentTarget as HTMLSelectElement).value })}>
						<option value="">Select contract</option>
						{#each contracts as contract}
							<option value={contract.id}>{contract.name}</option>
						{/each}
					</select>
				</label>
				<label class="block text-sm">
					<span class="mb-2 block font-medium text-stone-100">Provider peer</span>
					<select class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-fuchsia-400" value={draft.provider_peer_id} onchange={(event) => onDraftChange({ provider_peer_id: (event.currentTarget as HTMLSelectElement).value })}>
						<option value="">Select provider</option>
						{#each peers as peer}
							<option value={peer.id}>{peer.display_name}</option>
						{/each}
					</select>
				</label>
				<label class="block text-sm">
					<span class="mb-2 block font-medium text-stone-100">Consumer peer</span>
					<select class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-fuchsia-400" value={draft.consumer_peer_id} onchange={(event) => onDraftChange({ consumer_peer_id: (event.currentTarget as HTMLSelectElement).value })}>
						<option value="">Select consumer</option>
						{#each peers as peer}
							<option value={peer.id}>{peer.display_name}</option>
						{/each}
					</select>
				</label>
				<label class="block text-sm">
					<span class="mb-2 block font-medium text-stone-100">Provider space</span>
					<select class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-fuchsia-400" value={draft.provider_space_id} onchange={(event) => onDraftChange({ provider_space_id: (event.currentTarget as HTMLSelectElement).value })}>
						<option value="">No space</option>
						{#each spaces as space}
							<option value={space.id}>{space.display_name}</option>
						{/each}
					</select>
				</label>
				<label class="block text-sm">
					<span class="mb-2 block font-medium text-stone-100">Consumer space</span>
					<select class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-fuchsia-400" value={draft.consumer_space_id} onchange={(event) => onDraftChange({ consumer_space_id: (event.currentTarget as HTMLSelectElement).value })}>
						<option value="">No space</option>
						{#each spaces as space}
							<option value={space.id}>{space.display_name}</option>
						{/each}
					</select>
				</label>
				<label class="block text-sm md:col-span-2">
					<span class="mb-2 block font-medium text-stone-100">Dataset name</span>
					<input class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-fuchsia-400" value={draft.dataset_name} oninput={(event) => onDraftChange({ dataset_name: inputValue(event) })} />
				</label>
				<label class="block text-sm">
					<span class="mb-2 block font-medium text-stone-100">Replication mode</span>
					<input class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-fuchsia-400" value={draft.replication_mode} oninput={(event) => onDraftChange({ replication_mode: inputValue(event) })} />
				</label>
				<label class="block text-sm">
					<span class="mb-2 block font-medium text-stone-100">Selector JSON</span>
					<input class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 font-mono text-xs outline-none transition focus:border-fuchsia-400" value={draft.selector_text} oninput={(event) => onDraftChange({ selector_text: inputValue(event) })} />
				</label>
				<label class="block text-sm md:col-span-2">
					<span class="mb-2 block font-medium text-stone-100">Provider schema JSON</span>
					<textarea class="min-h-24 w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 font-mono text-xs outline-none transition focus:border-fuchsia-400" oninput={(event) => onDraftChange({ provider_schema_text: textValue(event) })}>{draft.provider_schema_text}</textarea>
				</label>
				<label class="block text-sm md:col-span-2">
					<span class="mb-2 block font-medium text-stone-100">Consumer schema JSON</span>
					<textarea class="min-h-24 w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 font-mono text-xs outline-none transition focus:border-fuchsia-400" oninput={(event) => onDraftChange({ consumer_schema_text: textValue(event) })}>{draft.consumer_schema_text}</textarea>
				</label>
				<label class="block text-sm md:col-span-2">
					<span class="mb-2 block font-medium text-stone-100">Sample rows JSON</span>
					<textarea class="min-h-28 w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 font-mono text-xs outline-none transition focus:border-fuchsia-400" oninput={(event) => onDraftChange({ sample_rows_text: textValue(event) })}>{draft.sample_rows_text}</textarea>
				</label>
			</div>
		</div>

		<div class="space-y-3 rounded-2xl border border-stone-200 bg-stone-50/80 p-4">
			{#each shares as item}
				<div class="rounded-2xl border border-stone-200 bg-white px-4 py-4">
					<div class="flex items-start justify-between gap-3">
						<div>
							<p class="font-semibold text-stone-900">{item.share.dataset_name}</p>
							<p class="text-sm text-stone-500">{item.share.replication_mode} • {item.share.status}</p>
						</div>
						<span class={`rounded-full px-3 py-1 text-xs font-semibold uppercase tracking-[0.18em] ${item.compatibility.compatible ? 'bg-emerald-100 text-emerald-700' : 'bg-amber-100 text-amber-700'}`}>{item.compatibility.compatible ? 'Compatible' : 'Review'}</span>
					</div>
					<p class="mt-3 text-xs text-stone-500">Selector {JSON.stringify(item.share.selector)}</p>
					<p class="mt-2 text-xs text-stone-500">Spaces {item.share.provider_space_id ?? 'none'} → {item.share.consumer_space_id ?? 'none'}</p>
				</div>
			{/each}
		</div>
	</div>
</section>
