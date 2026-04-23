<script lang="ts">
	import type { AuditBridgeSummary, FederatedQueryResult, ReplicationPlan, ShareDetail } from '$lib/api/nexus';

	interface QueryDraft {
		share_id: string;
		sql: string;
		purpose: string;
		limit: string;
	}

	export let shares: ShareDetail[] = [];
	export let selectedShareId = '';
	export let selectedShare: ShareDetail | null = null;
	export let replicationPlans: ReplicationPlan[] = [];
	export let auditBridge: AuditBridgeSummary | null = null;
	export let queryDraft: QueryDraft;
	export let queryResult: FederatedQueryResult | null = null;
	export let busy = false;
	export let onSelectShare: (shareId: string) => void;
	export let onQueryDraftChange: (patch: Partial<QueryDraft>) => void;
	export let onRunQuery: () => void;

	function inputValue(event: Event) {
		return (event.currentTarget as HTMLInputElement).value;
	}

	function textValue(event: Event) {
		return (event.currentTarget as HTMLTextAreaElement).value;
	}

	$: selectedPlan = replicationPlans.find((plan) => plan.share_id === selectedShareId) ?? null;
	$: bridgeEntry = auditBridge?.entries.find((entry) => entry.share_id === selectedShareId) ?? null;
</script>

<section class="rounded-3xl border border-stone-200 bg-white p-5 shadow-sm shadow-stone-200/60">
	<div>
		<p class="text-xs font-semibold uppercase tracking-[0.24em] text-violet-700">Shared Data Browser</p>
		<h3 class="mt-2 text-xl font-semibold text-stone-900">Federated query, schema review, and encryption posture</h3>
		<p class="mt-1 text-sm text-stone-500">Browse shared datasets, inspect compatibility and transport posture, and run federated preview queries without copying the source.</p>
	</div>

	<div class="mt-5 grid gap-4 xl:grid-cols-[0.82fr_1.18fr]">
		<div class="space-y-3 rounded-2xl border border-stone-200 bg-stone-50/80 p-4">
			{#each shares as item}
				<button class={`w-full rounded-2xl border px-4 py-4 text-left transition ${selectedShareId === item.share.id ? 'border-violet-500 bg-violet-50' : 'border-stone-200 bg-white hover:border-violet-300 hover:bg-violet-50/60'}`} onclick={() => onSelectShare(item.share.id)}>
					<div class="flex items-start justify-between gap-3">
						<div>
							<p class="font-semibold text-stone-900">{item.share.dataset_name}</p>
							<p class="text-sm text-stone-500">{item.share.replication_mode} • {item.access_grant?.query_template ?? 'No grant'}</p>
						</div>
						<span class={`rounded-full px-3 py-1 text-xs font-semibold uppercase tracking-[0.18em] ${item.encryption.encrypted_in_transit && item.encryption.encrypted_at_rest ? 'bg-cyan-100 text-cyan-700' : 'bg-rose-100 text-rose-700'}`}>{item.encryption.profile}</span>
					</div>
				</button>
			{/each}
		</div>

		<div class="space-y-4">
			{#if selectedShare}
				<div class="rounded-2xl border border-stone-200 bg-stone-950 p-4 text-stone-100">
					<div class="grid gap-4 md:grid-cols-2">
						<div class="rounded-2xl border border-stone-700 bg-stone-900 px-4 py-4">
							<p class="text-xs uppercase tracking-[0.18em] text-violet-300">Schema compatibility</p>
							<p class="mt-2 text-sm text-stone-200">{selectedShare.compatibility.summary}</p>
							<div class="mt-3 flex flex-wrap gap-2">
								{#each selectedShare.compatibility.missing_fields as field}
									<span class="rounded-full bg-amber-300 px-2 py-1 text-xs text-amber-950">Missing {field}</span>
								{/each}
								{#each selectedShare.compatibility.type_mismatches as mismatch}
									<span class="rounded-full bg-rose-300 px-2 py-1 text-xs text-rose-950">{mismatch}</span>
								{/each}
							</div>
						</div>

						<div class="rounded-2xl border border-stone-700 bg-stone-900 px-4 py-4">
							<p class="text-xs uppercase tracking-[0.18em] text-cyan-300">Encryption posture</p>
							<p class="mt-2 text-sm text-stone-200">{selectedShare.encryption.transport_cipher} • {selectedShare.encryption.at_rest_cipher}</p>
							<p class="mt-2 text-xs text-stone-400">{selectedShare.encryption.key_version} • {selectedShare.encryption.recommendation}</p>
						</div>
					</div>

					<div class="mt-4 grid gap-4 md:grid-cols-2">
						<div class="rounded-2xl border border-stone-700 bg-stone-900 px-4 py-4">
							<p class="text-xs uppercase tracking-[0.18em] text-stone-400">Replication plan</p>
							<p class="mt-2 text-sm text-stone-200">{selectedPlan?.status ?? 'pending'} • backlog {selectedPlan?.backlog_rows ?? 0}</p>
							<p class="mt-2 text-xs text-stone-400">Filter {selectedPlan ? JSON.stringify(selectedPlan.selective_filter) : 'n/a'}</p>
						</div>
						<div class="rounded-2xl border border-stone-700 bg-stone-900 px-4 py-4">
							<p class="text-xs uppercase tracking-[0.18em] text-stone-400">Audit bridge</p>
							<p class="mt-2 font-mono text-xs text-stone-300">{bridgeEntry?.audit_cursor ?? 'cursor/pending'}</p>
							<p class="mt-2 text-xs text-stone-400">{bridgeEntry?.contract_name ?? 'No audit evidence linked yet'}</p>
						</div>
					</div>
				</div>

				<div class="rounded-2xl border border-stone-200 bg-stone-50/80 p-4">
					<div class="flex items-center justify-between gap-3">
						<p class="text-sm font-semibold text-stone-900">Federated query preview</p>
						<button class="rounded-full bg-violet-600 px-4 py-2 text-sm font-semibold text-white transition hover:bg-violet-700 disabled:cursor-not-allowed disabled:bg-violet-300" onclick={onRunQuery} disabled={busy}>Run query</button>
					</div>

					<div class="mt-4 grid gap-4 md:grid-cols-2">
						<label class="block text-sm md:col-span-2">
							<span class="mb-2 block font-medium text-stone-700">SQL</span>
							<textarea class="min-h-24 w-full rounded-2xl border border-stone-300 bg-white px-4 py-3 font-mono text-xs outline-none transition focus:border-violet-500" oninput={(event) => onQueryDraftChange({ sql: textValue(event) })}>{queryDraft.sql}</textarea>
						</label>
						<label class="block text-sm">
							<span class="mb-2 block font-medium text-stone-700">Purpose</span>
							<input class="w-full rounded-2xl border border-stone-300 bg-white px-4 py-3 outline-none transition focus:border-violet-500" value={queryDraft.purpose} oninput={(event) => onQueryDraftChange({ purpose: inputValue(event) })} />
						</label>
						<label class="block text-sm">
							<span class="mb-2 block font-medium text-stone-700">Limit</span>
							<input class="w-full rounded-2xl border border-stone-300 bg-white px-4 py-3 outline-none transition focus:border-violet-500" value={queryDraft.limit} oninput={(event) => onQueryDraftChange({ limit: inputValue(event) })} />
						</label>
					</div>

					{#if queryResult}
						<div class="mt-4 overflow-hidden rounded-2xl border border-stone-200 bg-white">
							<div class="border-b border-stone-200 px-4 py-3 text-sm text-stone-600">{queryResult.source_peer} • {queryResult.dataset_name}</div>
							<div class="overflow-x-auto">
								<table class="min-w-full text-left text-sm">
									<thead class="bg-stone-50 text-stone-500">
										<tr>
											{#each queryResult.columns as column}
												<th class="px-4 py-3 font-medium">{column}</th>
											{/each}
										</tr>
									</thead>
									<tbody>
										{#each queryResult.rows as row}
											<tr class="border-t border-stone-100">
												{#each queryResult.columns as column}
													<td class="px-4 py-3 text-stone-700">{String(row[column] ?? '')}</td>
												{/each}
											</tr>
										{/each}
									</tbody>
								</table>
							</div>
						</div>
					{/if}
				</div>
			{/if}
		</div>
	</div>
</section>
