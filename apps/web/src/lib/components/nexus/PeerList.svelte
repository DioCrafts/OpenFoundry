<script lang="ts">
	import type { PeerOrganization } from '$lib/api/nexus';

	interface PeerDraft {
		slug: string;
		display_name: string;
		organization_type: string;
		region: string;
		endpoint_url: string;
		auth_mode: string;
		trust_level: string;
		public_key_fingerprint: string;
		shared_scopes_text: string;
		admin_contacts_text: string;
	}

	export let peers: PeerOrganization[] = [];
	export let draft: PeerDraft;
	export let busy = false;
	export let onDraftChange: (patch: Partial<PeerDraft>) => void;
	export let onCreate: () => void;
	export let onAuthenticate: (peerId: string) => void;

	function inputValue(event: Event) {
		return (event.currentTarget as HTMLInputElement).value;
	}
</script>

<section class="rounded-3xl border border-stone-200 bg-white p-5 shadow-sm shadow-stone-200/60">
	<div class="flex flex-col gap-4 lg:flex-row lg:items-start lg:justify-between">
		<div>
			<p class="text-xs font-semibold uppercase tracking-[0.24em] text-emerald-700">Peer List</p>
			<h3 class="mt-2 text-xl font-semibold text-stone-900">Partner registration and authentication</h3>
			<p class="mt-1 text-sm text-stone-500">Register cross-org peers, capture trust posture, and trigger an authentication handshake.</p>
		</div>
		<button class="rounded-full bg-emerald-600 px-4 py-2 text-sm font-semibold text-white transition hover:bg-emerald-700 disabled:cursor-not-allowed disabled:bg-emerald-300" onclick={onCreate} disabled={busy}>Register peer</button>
	</div>

	<div class="mt-5 grid gap-4 xl:grid-cols-[0.9fr_1.1fr]">
		<div class="space-y-3 rounded-2xl border border-stone-200 bg-stone-50/80 p-4">
			{#each peers as peer}
				<div class="rounded-2xl border border-stone-200 bg-white px-4 py-4">
					<div class="flex items-start justify-between gap-3">
						<div>
							<p class="font-semibold text-stone-900">{peer.display_name}</p>
							<p class="text-sm text-stone-500">{peer.slug} • {peer.organization_type} • {peer.region} • {peer.auth_mode}</p>
						</div>
						<span class={`rounded-full px-3 py-1 text-xs font-semibold uppercase tracking-[0.18em] ${peer.status === 'authenticated' ? 'bg-emerald-100 text-emerald-700' : 'bg-amber-100 text-amber-700'}`}>{peer.status}</span>
					</div>
					<div class="mt-3 flex flex-wrap gap-2 text-xs text-stone-600">
						{#each peer.shared_scopes as scope}
							<span class="rounded-full bg-stone-100 px-2 py-1">{scope}</span>
						{/each}
					</div>
					<div class="mt-3 text-xs text-stone-500">
						<div>Lifecycle {peer.lifecycle_stage}</div>
						<div class="mt-1">Contacts {peer.admin_contacts.join(', ') || 'n/a'}</div>
					</div>
					<div class="mt-3 flex items-center justify-between gap-3 text-xs text-stone-500">
						<span>{peer.public_key_fingerprint}</span>
						<button class="rounded-full border border-emerald-300 px-3 py-1 font-medium text-emerald-700 transition hover:border-emerald-400 hover:bg-emerald-50 disabled:cursor-not-allowed disabled:opacity-50" onclick={() => onAuthenticate(peer.id)} disabled={busy || peer.status === 'authenticated'}>Authenticate</button>
					</div>
				</div>
			{/each}
		</div>

		<div class="rounded-2xl border border-stone-200 bg-stone-950 p-4 text-stone-100">
			<div class="grid gap-4 md:grid-cols-2">
				<label class="block text-sm">
					<span class="mb-2 block font-medium text-stone-100">Slug</span>
					<input class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-emerald-400" value={draft.slug} oninput={(event) => onDraftChange({ slug: inputValue(event) })} />
				</label>
				<label class="block text-sm">
					<span class="mb-2 block font-medium text-stone-100">Display name</span>
					<input class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-emerald-400" value={draft.display_name} oninput={(event) => onDraftChange({ display_name: inputValue(event) })} />
				</label>
				<label class="block text-sm">
					<span class="mb-2 block font-medium text-stone-100">Organization type</span>
					<input class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-emerald-400" value={draft.organization_type} oninput={(event) => onDraftChange({ organization_type: inputValue(event) })} />
				</label>
				<label class="block text-sm">
					<span class="mb-2 block font-medium text-stone-100">Region</span>
					<input class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-emerald-400" value={draft.region} oninput={(event) => onDraftChange({ region: inputValue(event) })} />
				</label>
				<label class="block text-sm">
					<span class="mb-2 block font-medium text-stone-100">Auth mode</span>
					<input class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-emerald-400" value={draft.auth_mode} oninput={(event) => onDraftChange({ auth_mode: inputValue(event) })} />
				</label>
				<label class="block text-sm md:col-span-2">
					<span class="mb-2 block font-medium text-stone-100">Endpoint URL</span>
					<input class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-emerald-400" value={draft.endpoint_url} oninput={(event) => onDraftChange({ endpoint_url: inputValue(event) })} />
				</label>
				<label class="block text-sm">
					<span class="mb-2 block font-medium text-stone-100">Trust level</span>
					<input class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-emerald-400" value={draft.trust_level} oninput={(event) => onDraftChange({ trust_level: inputValue(event) })} />
				</label>
				<label class="block text-sm">
					<span class="mb-2 block font-medium text-stone-100">Fingerprint</span>
					<input class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-emerald-400" value={draft.public_key_fingerprint} oninput={(event) => onDraftChange({ public_key_fingerprint: inputValue(event) })} />
				</label>
				<label class="block text-sm md:col-span-2">
					<span class="mb-2 block font-medium text-stone-100">Shared scopes</span>
					<input class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-emerald-400" value={draft.shared_scopes_text} oninput={(event) => onDraftChange({ shared_scopes_text: inputValue(event) })} />
				</label>
				<label class="block text-sm md:col-span-2">
					<span class="mb-2 block font-medium text-stone-100">Admin contacts</span>
					<input class="w-full rounded-2xl border border-stone-700 bg-stone-900 px-4 py-3 outline-none transition focus:border-emerald-400" value={draft.admin_contacts_text} oninput={(event) => onDraftChange({ admin_contacts_text: inputValue(event) })} />
				</label>
			</div>
		</div>
	</div>
</section>
