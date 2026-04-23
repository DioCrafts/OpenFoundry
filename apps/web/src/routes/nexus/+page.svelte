<script lang="ts">
	import { onMount } from 'svelte';

	import ContractManager from '$components/nexus/ContractManager.svelte';
	import PeerList from '$components/nexus/PeerList.svelte';
	import ShareWizard from '$components/nexus/ShareWizard.svelte';
	import SharedDataBrowser from '$components/nexus/SharedDataBrowser.svelte';
	import SharingDashboard from '$components/nexus/SharingDashboard.svelte';
	import {
		authenticatePeer,
		createContract,
		createPeer,
		createShare,
		getAuditBridge,
		getOverview,
		listContracts,
		listPeers,
		listReplicationPlans,
		listShares,
		runFederatedQuery,
		updateContract,
		type AuditBridgeSummary,
		type FederatedQueryResult,
		type NexusOverview,
		type PeerOrganization,
		type ReplicationPlan,
		type ShareDetail,
		type SharingContract,
	} from '$lib/api/nexus';
	import { notifications } from '$lib/stores/notifications';

	type PeerDraft = {
		slug: string;
		display_name: string;
		region: string;
		endpoint_url: string;
		auth_mode: string;
		trust_level: string;
		public_key_fingerprint: string;
		shared_scopes_text: string;
	};

	type ContractDraft = {
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
	};

	type ShareDraft = {
		contract_id: string;
		provider_peer_id: string;
		consumer_peer_id: string;
		dataset_name: string;
		selector_text: string;
		provider_schema_text: string;
		consumer_schema_text: string;
		sample_rows_text: string;
		replication_mode: string;
	};

	type QueryDraft = {
		share_id: string;
		sql: string;
		purpose: string;
		limit: string;
	};

	let overview = $state<NexusOverview | null>(null);
	let peers = $state<PeerOrganization[]>([]);
	let contracts = $state<SharingContract[]>([]);
	let shares = $state<ShareDetail[]>([]);
	let replicationPlans = $state<ReplicationPlan[]>([]);
	let auditBridge = $state<AuditBridgeSummary | null>(null);
	let queryResult = $state<FederatedQueryResult | null>(null);
	let selectedContractId = $state('');
	let selectedShareId = $state('');
	let loading = $state(true);
	let busyAction = $state('');
	let uiError = $state('');
	let peerDraft = $state<PeerDraft>(createEmptyPeerDraft());
	let contractDraft = $state<ContractDraft>(createEmptyContractDraft());
	let shareDraft = $state<ShareDraft>(createEmptyShareDraft());
	let queryDraft = $state<QueryDraft>(createEmptyQueryDraft());

	const busy = $derived(loading || busyAction.length > 0);
	const selectedShare = $derived(shares.find((share) => share.share.id === selectedShareId) ?? null);

	onMount(() => {
		void refreshAll();
	});

	function createEmptyPeerDraft(): PeerDraft {
		return {
			slug: 'partner-new',
			display_name: 'New Partner Org',
			region: 'eu-central-1',
			endpoint_url: 'https://partner.example.com/nexus',
			auth_mode: 'mtls+jwt',
			trust_level: 'partner',
			public_key_fingerprint: 'SHA256:NEW:PARTNER:FPR',
			shared_scopes_text: 'catalog, audit',
		};
	}

	function toLocalDateTime(date: Date) {
		const pad = (value: number) => String(value).padStart(2, '0');
		return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}T${pad(date.getHours())}:${pad(date.getMinutes())}`;
	}

	function createEmptyContractDraft(): ContractDraft {
		const expiresAt = new Date(Date.now() + 1000 * 60 * 60 * 24 * 180);
		return {
			peer_id: '',
			name: 'Cross-org Access Contract',
			description: 'Purpose-bound data sharing agreement with residency and encryption terms.',
			dataset_locator: 'partner://dataset/path',
			allowed_purposes_text: 'analytics, support',
			data_classes_text: 'confidential',
			residency_region: 'eu',
			query_template: 'SELECT * FROM shared_dataset LIMIT 100',
			max_rows_per_query: '1000',
			replication_mode: 'query_only',
			encryption_profile: 'mutual-tls+envelope',
			retention_days: '180',
			status: 'active',
			expires_at: toLocalDateTime(expiresAt),
		};
	}

	function createEmptyShareDraft(): ShareDraft {
		return {
			contract_id: '',
			provider_peer_id: '',
			consumer_peer_id: '',
			dataset_name: 'shared_dataset_preview',
			selector_text: JSON.stringify({ partition: '2026-Q2' }),
			provider_schema_text: JSON.stringify({ id: 'string', metric: 'number', region: 'string' }, null, 2),
			consumer_schema_text: JSON.stringify({ id: 'string', metric: 'number', region: 'string' }, null, 2),
			sample_rows_text: JSON.stringify([{ id: 'row-1', metric: 42, region: 'eu' }], null, 2),
			replication_mode: 'incremental_replication',
		};
	}

	function createEmptyQueryDraft(): QueryDraft {
		return {
			share_id: '',
			sql: 'SELECT * FROM shared_dataset LIMIT 50',
			purpose: 'analytics',
			limit: '50',
		};
	}

	function parseCsv(value: string) {
		return value.split(',').map((entry) => entry.trim()).filter(Boolean);
	}

	function parseJson<T>(value: string) {
		return JSON.parse(value) as T;
	}

	function contractToDraft(contract: SharingContract): ContractDraft {
		return {
			id: contract.id,
			peer_id: contract.peer_id,
			name: contract.name,
			description: contract.description,
			dataset_locator: contract.dataset_locator,
			allowed_purposes_text: contract.allowed_purposes.join(', '),
			data_classes_text: contract.data_classes.join(', '),
			residency_region: contract.residency_region,
			query_template: contract.query_template,
			max_rows_per_query: String(contract.max_rows_per_query),
			replication_mode: contract.replication_mode,
			encryption_profile: contract.encryption_profile,
			retention_days: String(contract.retention_days),
			status: contract.status,
			expires_at: toLocalDateTime(new Date(contract.expires_at)),
		};
	}

	function shareToQueryDraft(share: ShareDetail): QueryDraft {
		return {
			share_id: share.share.id,
			sql: share.access_grant?.query_template ?? `SELECT * FROM ${share.share.dataset_name} LIMIT 50`,
			purpose: share.access_grant?.allowed_purposes[0] ?? 'analytics',
			limit: String(share.access_grant?.max_rows_per_query ?? 50),
		};
	}

	function updatePeerDraft(patch: Partial<PeerDraft>) {
		peerDraft = { ...peerDraft, ...patch };
	}

	function updateContractDraft(patch: Partial<ContractDraft>) {
		contractDraft = { ...contractDraft, ...patch };
	}

	function updateShareDraft(patch: Partial<ShareDraft>) {
		shareDraft = { ...shareDraft, ...patch };
	}

	function updateQueryDraft(patch: Partial<QueryDraft>) {
		queryDraft = { ...queryDraft, ...patch };
	}

	function selectContract(contractId: string) {
		selectedContractId = contractId;
		const contract = contracts.find((entry) => entry.id === contractId);
		contractDraft = contract ? contractToDraft(contract) : createEmptyContractDraft();
	}

	function selectShare(shareId: string) {
		selectedShareId = shareId;
		const share = shares.find((entry) => entry.share.id === shareId);
		if (share) {
			queryDraft = shareToQueryDraft(share);
		}
	}

	async function refreshAll() {
		loading = true;
		uiError = '';
		try {
			const [overviewResponse, peersResponse, contractsResponse, sharesResponse, replicationResponse, auditBridgeResponse] = await Promise.all([
				getOverview(),
				listPeers(),
				listContracts(),
				listShares(),
				listReplicationPlans(),
				getAuditBridge(),
			]);

			overview = overviewResponse;
			peers = peersResponse.items;
			contracts = contractsResponse.items;
			shares = sharesResponse.items;
			replicationPlans = replicationResponse.items;
			auditBridge = auditBridgeResponse;

			if (!selectedContractId && contractsResponse.items[0]) {
				selectedContractId = contractsResponse.items[0].id;
				contractDraft = contractToDraft(contractsResponse.items[0]);
			} else if (selectedContractId) {
				const current = contractsResponse.items.find((contract) => contract.id === selectedContractId);
				if (current) contractDraft = contractToDraft(current);
			}

			if (!selectedShareId && sharesResponse.items[0]) {
				selectedShareId = sharesResponse.items[0].share.id;
				queryDraft = shareToQueryDraft(sharesResponse.items[0]);
			} else if (selectedShareId) {
				const current = sharesResponse.items.find((share) => share.share.id === selectedShareId);
				if (current) queryDraft = shareToQueryDraft(current);
			}

			if (!shareDraft.contract_id && contractsResponse.items[0]) {
				shareDraft = { ...shareDraft, contract_id: contractsResponse.items[0].id };
			}
			if (!shareDraft.provider_peer_id && peersResponse.items[0]) {
				shareDraft = { ...shareDraft, provider_peer_id: peersResponse.items[0].id };
			}
			if (!shareDraft.consumer_peer_id && peersResponse.items[1]) {
				shareDraft = { ...shareDraft, consumer_peer_id: peersResponse.items[1].id };
			}
			if (!contractDraft.peer_id && peersResponse.items[0]) {
				contractDraft = { ...contractDraft, peer_id: peersResponse.items[0].id };
			}
		} catch (error) {
			uiError = error instanceof Error ? error.message : 'Unable to load nexus surfaces';
			notifications.error(uiError);
		} finally {
			loading = false;
		}
	}

	async function createPeerAction() {
		busyAction = 'create-peer';
		try {
			const peer = await createPeer({
				slug: peerDraft.slug,
				display_name: peerDraft.display_name,
				region: peerDraft.region,
				endpoint_url: peerDraft.endpoint_url,
				auth_mode: peerDraft.auth_mode,
				trust_level: peerDraft.trust_level,
				public_key_fingerprint: peerDraft.public_key_fingerprint,
				shared_scopes: parseCsv(peerDraft.shared_scopes_text),
			});
			peerDraft = createEmptyPeerDraft();
			await refreshAll();
			notifications.success(`Registered ${peer.display_name}`);
		} catch (error) {
			uiError = error instanceof Error ? error.message : 'Unable to register peer';
			notifications.error(uiError);
		} finally {
			busyAction = '';
		}
	}

	async function authenticatePeerAction(peerId: string) {
		busyAction = 'authenticate-peer';
		try {
			const peer = await authenticatePeer(peerId);
			await refreshAll();
			notifications.success(`Authenticated ${peer.display_name}`);
		} catch (error) {
			uiError = error instanceof Error ? error.message : 'Unable to authenticate peer';
			notifications.error(uiError);
		} finally {
			busyAction = '';
		}
	}

	async function saveContractAction() {
		busyAction = 'save-contract';
		try {
			const payload = {
				peer_id: contractDraft.peer_id,
				name: contractDraft.name,
				description: contractDraft.description,
				dataset_locator: contractDraft.dataset_locator,
				allowed_purposes: parseCsv(contractDraft.allowed_purposes_text),
				data_classes: parseCsv(contractDraft.data_classes_text),
				residency_region: contractDraft.residency_region,
				query_template: contractDraft.query_template,
				max_rows_per_query: Number(contractDraft.max_rows_per_query),
				replication_mode: contractDraft.replication_mode,
				encryption_profile: contractDraft.encryption_profile,
				retention_days: Number(contractDraft.retention_days),
				status: contractDraft.status,
				expires_at: new Date(contractDraft.expires_at).toISOString(),
			};
			const contract = contractDraft.id
				? await updateContract(contractDraft.id, payload)
				: await createContract(payload);
			selectedContractId = contract.id;
			await refreshAll();
			notifications.success(`${contractDraft.id ? 'Updated' : 'Created'} ${contract.name}`);
		} catch (error) {
			uiError = error instanceof Error ? error.message : 'Unable to save contract';
			notifications.error(uiError);
		} finally {
			busyAction = '';
		}
	}

	async function createShareAction() {
		busyAction = 'create-share';
		try {
			const detail = await createShare({
				contract_id: shareDraft.contract_id,
				provider_peer_id: shareDraft.provider_peer_id,
				consumer_peer_id: shareDraft.consumer_peer_id,
				dataset_name: shareDraft.dataset_name,
				selector: parseJson<Record<string, unknown>>(shareDraft.selector_text),
				provider_schema: parseJson<Record<string, unknown>>(shareDraft.provider_schema_text),
				consumer_schema: parseJson<Record<string, unknown>>(shareDraft.consumer_schema_text),
				sample_rows: parseJson<Record<string, unknown>[]>(shareDraft.sample_rows_text),
				replication_mode: shareDraft.replication_mode,
			});
			selectedShareId = detail.share.id;
			queryDraft = shareToQueryDraft(detail);
			await refreshAll();
			notifications.success(`Created share ${detail.share.dataset_name}`);
		} catch (error) {
			uiError = error instanceof Error ? error.message : 'Unable to create share';
			notifications.error(uiError);
		} finally {
			busyAction = '';
		}
	}

	async function runQueryAction() {
		busyAction = 'run-query';
		try {
			queryResult = await runFederatedQuery({
				share_id: queryDraft.share_id,
				sql: queryDraft.sql,
				purpose: queryDraft.purpose,
				limit: Number(queryDraft.limit),
			});
			notifications.success(`Loaded ${queryResult.rows.length} federated rows`);
		} catch (error) {
			uiError = error instanceof Error ? error.message : 'Unable to run federated query';
			notifications.error(uiError);
		} finally {
			busyAction = '';
		}
	}
</script>

<div class="space-y-6">
	<section class="overflow-hidden rounded-[2rem] bg-gradient-to-br from-cyan-950 via-stone-950 to-fuchsia-950 px-6 py-6 text-stone-50 shadow-xl shadow-cyan-950/20">
		<div class="flex flex-col gap-6 lg:flex-row lg:items-end lg:justify-between">
			<div class="max-w-3xl">
				<p class="text-xs font-semibold uppercase tracking-[0.28em] text-cyan-300">Milestone 5.1</p>
				<h1 class="mt-3 text-3xl font-semibold tracking-tight">Nexus for cross-org sharing, federated access, and trust-bound replication</h1>
				<p class="mt-3 text-sm leading-6 text-stone-300">Operate partner onboarding, sharing contracts, schema checks, encrypted replication posture, and federated previews from one surface.</p>
			</div>
			<div class="rounded-2xl bg-white/10 px-4 py-4 backdrop-blur">
				<p class="text-xs uppercase tracking-[0.18em] text-cyan-200">Audit bridge</p>
				<p class="mt-2 text-sm font-semibold">{overview?.audit_bridge_status ?? 'pending'}</p>
				<p class="mt-1 text-xs text-stone-300">Latest sync {overview?.latest_sync_at ? new Date(overview.latest_sync_at).toLocaleString() : 'n/a'}</p>
			</div>
		</div>
	</section>

	{#if uiError}
		<div class="rounded-2xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-700">{uiError}</div>
	{/if}

	<SharingDashboard {overview} {auditBridge} {replicationPlans} />

	<div class="grid gap-6 xl:grid-cols-[0.95fr_1.05fr]">
		<PeerList {peers} draft={peerDraft} {busy} onDraftChange={updatePeerDraft} onCreate={createPeerAction} onAuthenticate={authenticatePeerAction} />
		<ContractManager {contracts} {peers} {selectedContractId} draft={contractDraft} {busy} onSelect={selectContract} onDraftChange={updateContractDraft} onSave={saveContractAction} onReset={() => {
			selectedContractId = '';
			contractDraft = createEmptyContractDraft();
		}} />
	</div>

	<div class="grid gap-6 xl:grid-cols-[0.98fr_1.02fr]">
		<ShareWizard {shares} {peers} {contracts} draft={shareDraft} {busy} onDraftChange={updateShareDraft} onCreate={createShareAction} />
		<SharedDataBrowser {shares} {selectedShareId} {selectedShare} {replicationPlans} {auditBridge} queryDraft={queryDraft} {queryResult} {busy} onSelectShare={selectShare} onQueryDraftChange={updateQueryDraft} onRunQuery={runQueryAction} />
	</div>
</div>