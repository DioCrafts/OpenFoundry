<script lang="ts">
	import type { NexusSpace, PeerOrganization } from '$lib/api/nexus';

	interface SpaceDraft {
		slug: string;
		display_name: string;
		description: string;
		space_kind: string;
		owner_peer_id: string;
		region: string;
		member_peer_ids: string[];
		governance_tags_text: string;
		status: string;
	}

	export let spaces: NexusSpace[] = [];
	export let peers: PeerOrganization[] = [];
	export let draft: SpaceDraft;
	export let busy = false;
	export let onDraftChange: (patch: Partial<SpaceDraft>) => void;
	export let onCreate: () => void;

	function inputValue(event: Event) {
		return (event.currentTarget as HTMLInputElement).value;
	}

	function textValue(event: Event) {
		return (event.currentTarget as HTMLTextAreaElement).value;
	}

	function selectMembers(event: Event) {
		const selected = Array.from((event.currentTarget as HTMLSelectElement).selectedOptions)
			.map((option) => option.value)
			.filter(Boolean);
		onDraftChange({ member_peer_ids: selected });
	}
</script>

<section class="rounded-3xl border border-stone-200 bg-white p-5 shadow-sm shadow-stone-200/60">
	<div class="flex flex-col gap-4 lg:flex-row lg:items-start lg:justify-between">
		<div>
			<p class="text-xs font-semibold uppercase tracking-[0.24em] text-sky-700">Space Manager</p>
			<h3 class="mt-2 text-xl font-semibold text-stone-900">Private and shared spaces</h3>
			<p class="mt-1 text-sm text-stone-500">Modela boundaries operativos para ecosistemas multi-org, ownership, miembros y governance tags.</p>
		</div>
		<button class="rounded-full bg-sky-600 px-4 py-2 text-sm font-semibold text-white transition hover:bg-sky-700 disabled:cursor-not-allowed disabled:bg-sky-300" onclick={onCreate} disabled={busy}>Create space</button>
	</div>

	<div class="mt-5 grid gap-4 xl:grid-cols-[0.95fr_1.05fr]">
		<div class="space-y-3 rounded-2xl border border-stone-200 bg-stone-50/80 p-4">
			{#each spaces as space}
				<div class="rounded-2xl border border-stone-200 bg-white px-4 py-4">
					<div class="flex items-start justify-between gap-3">
						<div>
							<p class="font-semibold text-stone-900">{space.display_name}</p>
							<p class="text-sm text-stone-500">{space.slug} • {space.space_kind} • {space.region}</p>
						</div>
						<span class={`rounded-full px-3 py-1 text-xs font-semibold uppercase tracking-[0.18em] ${space.status === 'active' ? 'bg-emerald-100 text-emerald-700' : 'bg-amber-100 text-amber-700'}`}>{space.status}</span>
					</div>
					<p class="mt-3 text-sm text-stone-600">{space.description}</p>
					<div class="mt-3 flex flex-wrap gap-2">
						{#each space.governance_tags as tag}
							<span class="rounded-full bg-sky-100 px-2 py-1 text-xs text-sky-800">{tag}</span>
						{/each}
					</div>
					<p class="mt-3 text-xs text-stone-500">Members {space.member_peer_ids.length}</p>
				</div>
			{/each}
		</div>

		<div class="rounded-2xl border border-stone-200 bg-stone-950 p-4 text-stone-100">
			<div class="grid gap-4 md:grid-cols-2">
				<label class="block text-sm">
					<span class="mb-2 block font-medium text-stone-100">Slug</span>
					<input class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-sky-400" value={draft.slug} oninput={(event) => onDraftChange({ slug: inputValue(event) })} />
				</label>
				<label class="block text-sm">
					<span class="mb-2 block font-medium text-stone-100">Display name</span>
					<input class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-sky-400" value={draft.display_name} oninput={(event) => onDraftChange({ display_name: inputValue(event) })} />
				</label>
				<label class="block text-sm md:col-span-2">
					<span class="mb-2 block font-medium text-stone-100">Description</span>
					<textarea class="min-h-24 w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-sky-400" oninput={(event) => onDraftChange({ description: textValue(event) })}>{draft.description}</textarea>
				</label>
				<label class="block text-sm">
					<span class="mb-2 block font-medium text-stone-100">Kind</span>
					<select class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-sky-400" value={draft.space_kind} onchange={(event) => onDraftChange({ space_kind: (event.currentTarget as HTMLSelectElement).value })}>
						<option value="private">private</option>
						<option value="shared">shared</option>
					</select>
				</label>
				<label class="block text-sm">
					<span class="mb-2 block font-medium text-stone-100">Region</span>
					<input class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-sky-400" value={draft.region} oninput={(event) => onDraftChange({ region: inputValue(event) })} />
				</label>
				<label class="block text-sm">
					<span class="mb-2 block font-medium text-stone-100">Owner peer</span>
					<select class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-sky-400" value={draft.owner_peer_id} onchange={(event) => onDraftChange({ owner_peer_id: (event.currentTarget as HTMLSelectElement).value })}>
						<option value="">Local host</option>
						{#each peers as peer}
							<option value={peer.id}>{peer.display_name}</option>
						{/each}
					</select>
				</label>
				<label class="block text-sm">
					<span class="mb-2 block font-medium text-stone-100">Status</span>
					<select class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-sky-400" value={draft.status} onchange={(event) => onDraftChange({ status: (event.currentTarget as HTMLSelectElement).value })}>
						<option value="draft">draft</option>
						<option value="active">active</option>
						<option value="paused">paused</option>
					</select>
				</label>
				<label class="block text-sm md:col-span-2">
					<span class="mb-2 block font-medium text-stone-100">Members</span>
					<select multiple class="min-h-32 w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-sky-400" onchange={selectMembers}>
						{#each peers as peer}
							<option value={peer.id} selected={draft.member_peer_ids.includes(peer.id)}>{peer.display_name}</option>
						{/each}
					</select>
				</label>
				<label class="block text-sm md:col-span-2">
					<span class="mb-2 block font-medium text-stone-100">Governance tags</span>
					<input class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-sky-400" value={draft.governance_tags_text} oninput={(event) => onDraftChange({ governance_tags_text: inputValue(event) })} />
				</label>
			</div>
		</div>
	</div>
</section>
