<script lang="ts">
	import { onMount } from 'svelte';

	import LiveDataView from '$components/streaming/LiveDataView.svelte';
	import StreamList from '$components/streaming/StreamList.svelte';
	import StreamMonitor from '$components/streaming/StreamMonitor.svelte';
	import TopologyEditor from '$components/streaming/TopologyEditor.svelte';
	import WindowConfig from '$components/streaming/WindowConfig.svelte';
	import {
		createStream,
		createTopology,
		createWindow,
		getLiveTail,
		getOverview,
		getRuntime,
		listConnectors,
		listStreams,
		listTopologies,
		listWindows,
		runTopology,
		updateStream,
		updateTopology,
		updateWindow,
		type BackpressurePolicy,
		type CepDefinition,
		type LiveTailResponse,
		type StreamingOverview,
		type StreamDefinition,
		type StreamSchema,
		type TopologyDefinition,
		type TopologyEdge,
		type TopologyNode,
		type TopologyRuntimeSnapshot,
		type WindowDefinition,
		type ConnectorBinding,
		type ConnectorCatalogEntry,
		type JoinDefinition,
	} from '$lib/api/streaming';
	import { notifications } from '$stores/notifications';

	type StreamDraft = {
		id?: string;
		name: string;
		description: string;
		status: string;
		retention_hours: number;
		connector_type: string;
		endpoint: string;
		format: string;
		schema_text: string;
	};

	type WindowDraft = {
		id?: string;
		name: string;
		description: string;
		status: string;
		window_type: string;
		duration_seconds: number;
		slide_seconds: number;
		session_gap_seconds: number;
		allowed_lateness_seconds: number;
		aggregation_keys_text: string;
		measure_fields_text: string;
	};

	type TopologyDraft = {
		id?: string;
		name: string;
		description: string;
		status: string;
		state_backend: string;
		source_stream_ids_text: string;
		nodes_text: string;
		edges_text: string;
		join_definition_text: string;
		cep_definition_text: string;
		backpressure_policy_text: string;
		sink_bindings_text: string;
	};

	let overview = $state<StreamingOverview | null>(null);
	let streams = $state<StreamDefinition[]>([]);
	let windows = $state<WindowDefinition[]>([]);
	let topologies = $state<TopologyDefinition[]>([]);
	let connectors = $state<ConnectorCatalogEntry[]>([]);
	let liveTail = $state<LiveTailResponse | null>(null);
	let runtime = $state<TopologyRuntimeSnapshot | null>(null);

	let selectedTopologyId = $state('');
	let streamDraft = $state<StreamDraft>(createEmptyStreamDraft());
	let windowDraft = $state<WindowDraft>(createEmptyWindowDraft());
	let topologyDraft = $state<TopologyDraft>(createEmptyTopologyDraft());

	let loading = $state(true);
	let busyAction = $state('');
	let uiError = $state('');
	const busy = $derived(loading || busyAction.length > 0);

	onMount(() => {
		void refreshAll();
	});

	function createEmptyStreamDraft(): StreamDraft {
		return {
			name: 'Orders Ingress',
			description: 'Kafka topic carrying customer checkout events.',
			status: 'active',
			retention_hours: 72,
			connector_type: 'kafka',
			endpoint: 'kafka://stream/orders',
			format: 'json',
			schema_text: formatJson({
				fields: [
					{ name: 'event_time', data_type: 'timestamp', nullable: false, semantic_role: 'event_time' },
					{ name: 'customer_id', data_type: 'string', nullable: false, semantic_role: 'join_key' },
					{ name: 'amount', data_type: 'double', nullable: false, semantic_role: 'metric' },
				],
				primary_key: null,
				watermark_field: 'event_time',
			}),
		};
	}

	function createEmptyWindowDraft(): WindowDraft {
		return {
			name: 'Five Minute Revenue',
			description: 'Tumbling revenue aggregates keyed by customer and risk band.',
			status: 'active',
			window_type: 'tumbling',
			duration_seconds: 300,
			slide_seconds: 300,
			session_gap_seconds: 180,
			allowed_lateness_seconds: 30,
			aggregation_keys_text: 'customer_id, risk_band',
			measure_fields_text: 'amount',
		};
	}

	function createEmptyTopologyDraft(): TopologyDraft {
		return {
			name: 'Revenue Anomaly Pipeline',
			description: 'Join checkout and payment events, compute windowed aggregates, and push live anomalies.',
			status: 'active',
			state_backend: 'rocksdb',
			source_stream_ids_text: '',
			nodes_text: formatJson([
				{ id: 'src-orders', label: 'Orders', node_type: 'source', stream_id: null, window_id: null, config: { parallelism: 3 } },
				{ id: 'src-payments', label: 'Payments', node_type: 'source', stream_id: null, window_id: null, config: { parallelism: 2 } },
				{ id: 'join-risk', label: 'Join', node_type: 'join', stream_id: null, window_id: null, config: { type: 'stream-stream' } },
				{ id: 'window-revenue', label: 'Five Minute Window', node_type: 'window', stream_id: null, window_id: null, config: { emit: 'incremental' } },
				{ id: 'sink-live', label: 'Live Tail', node_type: 'sink', stream_id: null, window_id: null, config: { connector: 'websocket' } },
			]),
			edges_text: formatJson([
				{ source_node_id: 'src-orders', target_node_id: 'join-risk', label: 'orders' },
				{ source_node_id: 'src-payments', target_node_id: 'join-risk', label: 'payments' },
				{ source_node_id: 'join-risk', target_node_id: 'window-revenue', label: 'enriched-events' },
				{ source_node_id: 'window-revenue', target_node_id: 'sink-live', label: 'alerts' },
			]),
			join_definition_text: formatJson({
				join_type: 'stream-stream',
				left_stream_id: '',
				right_stream_id: '',
				table_name: 'payments_lookup',
				key_fields: ['customer_id'],
				window_seconds: 600,
			}),
			cep_definition_text: formatJson({
				pattern_name: 'payment-before-order',
				sequence: ['authorized', 'captured', 'checkout'],
				within_seconds: 900,
				output_stream: 'fraud_alerts',
			}),
			backpressure_policy_text: formatJson({
				max_in_flight: 512,
				queue_capacity: 2048,
				throttle_strategy: 'credit-based',
			}),
			sink_bindings_text: formatJson([
				{ connector_type: 'websocket', endpoint: 'ws://localhost:8080/api/v1/streaming/live-tail', format: 'json', config: { channel: 'revenue-alerts' } },
				{ connector_type: 'dataset', endpoint: 'dataset://materialized/revenue_alerts', format: 'parquet', config: { mode: 'append' } },
			]),
		};
	}

	function formatJson(value: unknown) {
		return JSON.stringify(value, null, 2);
	}

	function parseCsv(value: string) {
		return value.split(',').map((entry) => entry.trim()).filter(Boolean);
	}

	function parseJson<T>(value: string, fallback: T): T {
		if (!value.trim()) return fallback;
		try {
			return JSON.parse(value) as T;
		} catch {
			throw new Error('Invalid JSON payload');
		}
	}

	function parseOptionalJson<T>(value: string): T | null {
		if (!value.trim()) return null;
		return parseJson<T>(value, null as T);
	}

	function streamToDraft(stream: StreamDefinition): StreamDraft {
		return {
			id: stream.id,
			name: stream.name,
			description: stream.description,
			status: stream.status,
			retention_hours: stream.retention_hours,
			connector_type: stream.source_binding.connector_type,
			endpoint: stream.source_binding.endpoint,
			format: stream.source_binding.format,
			schema_text: formatJson(stream.schema),
		};
	}

	function windowToDraft(window: WindowDefinition): WindowDraft {
		return {
			id: window.id,
			name: window.name,
			description: window.description,
			status: window.status,
			window_type: window.window_type,
			duration_seconds: window.duration_seconds,
			slide_seconds: window.slide_seconds,
			session_gap_seconds: window.session_gap_seconds,
			allowed_lateness_seconds: window.allowed_lateness_seconds,
			aggregation_keys_text: window.aggregation_keys.join(', '),
			measure_fields_text: window.measure_fields.join(', '),
		};
	}

	function topologyToDraft(topology: TopologyDefinition): TopologyDraft {
		return {
			id: topology.id,
			name: topology.name,
			description: topology.description,
			status: topology.status,
			state_backend: topology.state_backend,
			source_stream_ids_text: topology.source_stream_ids.join(', '),
			nodes_text: formatJson(topology.nodes),
			edges_text: formatJson(topology.edges),
			join_definition_text: topology.join_definition ? formatJson(topology.join_definition) : '',
			cep_definition_text: topology.cep_definition ? formatJson(topology.cep_definition) : '',
			backpressure_policy_text: formatJson(topology.backpressure_policy),
			sink_bindings_text: formatJson(topology.sink_bindings),
		};
	}

	async function refreshAll() {
		loading = true;
		uiError = '';
		try {
			const [overviewResponse, streamResponse, windowResponse, topologyResponse, connectorResponse, liveTailResponse] = await Promise.all([
				getOverview(),
				listStreams(),
				listWindows(),
				listTopologies(),
				listConnectors(),
				getLiveTail(),
			]);

			overview = overviewResponse;
			streams = streamResponse.data;
			windows = windowResponse.data;
			topologies = topologyResponse.data;
			connectors = connectorResponse.data;
			liveTail = liveTailResponse;

			if (!streamDraft.id && streams[0]) streamDraft = streamToDraft(streams[0]);
			if (!windowDraft.id && windows[0]) windowDraft = windowToDraft(windows[0]);
			if (!selectedTopologyId && topologies[0]) selectedTopologyId = topologies[0].id;
			if (!topologyDraft.id && topologies[0]) topologyDraft = topologyToDraft(topologies[0]);

			if (!topologyDraft.source_stream_ids_text && streams.length > 0) {
				topologyDraft = {
					...topologyDraft,
					source_stream_ids_text: streams.slice(0, 2).map((stream) => stream.id).join(', '),
				};
			}

			if (selectedTopologyId) {
				await loadRuntime(selectedTopologyId);
			} else {
				runtime = null;
			}
		} catch (cause) {
			uiError = cause instanceof Error ? cause.message : 'Failed to load streaming data';
			notifications.error(uiError);
		} finally {
			loading = false;
		}
	}

	async function loadRuntime(topologyId: string) {
		selectedTopologyId = topologyId;
		runtime = await getRuntime(topologyId);
	}

	async function runAction(label: string, action: () => Promise<void>) {
		busyAction = label;
		uiError = '';
		try {
			await action();
		} catch (cause) {
			uiError = cause instanceof Error ? cause.message : 'Action failed';
			notifications.error(uiError);
		} finally {
			busyAction = '';
		}
	}

	async function saveStreamDraft() {
		await runAction('save-stream', async () => {
			const schema = parseJson<StreamSchema>(streamDraft.schema_text, { fields: [], primary_key: null, watermark_field: null });
			const payload = {
				name: streamDraft.name.trim(),
				description: streamDraft.description,
				status: streamDraft.status,
				retention_hours: streamDraft.retention_hours,
				schema,
				source_binding: {
					connector_type: streamDraft.connector_type,
					endpoint: streamDraft.endpoint,
					format: streamDraft.format,
					config: {},
				},
			};
			const saved = streamDraft.id
				? await updateStream(streamDraft.id, payload)
				: await createStream(payload);
			streamDraft = streamToDraft(saved);
			await refreshAll();
			notifications.success('Stream saved.');
		});
	}

	async function saveWindowDraft() {
		await runAction('save-window', async () => {
			const payload = {
				name: windowDraft.name.trim(),
				description: windowDraft.description,
				status: windowDraft.status,
				window_type: windowDraft.window_type,
				duration_seconds: windowDraft.duration_seconds,
				slide_seconds: windowDraft.slide_seconds,
				session_gap_seconds: windowDraft.session_gap_seconds,
				allowed_lateness_seconds: windowDraft.allowed_lateness_seconds,
				aggregation_keys: parseCsv(windowDraft.aggregation_keys_text),
				measure_fields: parseCsv(windowDraft.measure_fields_text),
			};
			const saved = windowDraft.id
				? await updateWindow(windowDraft.id, payload)
				: await createWindow(payload);
			windowDraft = windowToDraft(saved);
			await refreshAll();
			notifications.success('Window saved.');
		});
	}

	async function saveTopologyDraft() {
		await runAction('save-topology', async () => {
			const sourceIds = parseCsv(topologyDraft.source_stream_ids_text);
			const nodes = parseJson<TopologyNode[]>(topologyDraft.nodes_text, []);
			const edges = parseJson<TopologyEdge[]>(topologyDraft.edges_text, []);
			const joinDefinition = parseOptionalJson<JoinDefinition>(topologyDraft.join_definition_text);
			const cepDefinition = parseOptionalJson<CepDefinition>(topologyDraft.cep_definition_text);
			const backpressurePolicy = parseJson<BackpressurePolicy>(topologyDraft.backpressure_policy_text, {
				max_in_flight: 512,
				queue_capacity: 2048,
				throttle_strategy: 'credit-based',
			});
			const sinkBindings = parseJson<ConnectorBinding[]>(topologyDraft.sink_bindings_text, []);

			const saved = topologyDraft.id
				? await updateTopology(topologyDraft.id, {
					name: topologyDraft.name.trim(),
					description: topologyDraft.description,
					status: topologyDraft.status,
					state_backend: topologyDraft.state_backend,
					source_stream_ids: sourceIds,
					nodes,
					edges,
					join_definition: joinDefinition,
					cep_definition: cepDefinition,
					backpressure_policy: backpressurePolicy,
					sink_bindings: sinkBindings,
				})
				: await createTopology({
					name: topologyDraft.name.trim(),
					description: topologyDraft.description,
					status: topologyDraft.status,
					state_backend: topologyDraft.state_backend,
					source_stream_ids: sourceIds,
					nodes,
					edges,
					join_definition: joinDefinition,
					cep_definition: cepDefinition,
					backpressure_policy: backpressurePolicy,
					sink_bindings: sinkBindings,
				});

			topologyDraft = topologyToDraft(saved);
			selectedTopologyId = saved.id;
			await refreshAll();
			notifications.success('Topology saved.');
		});
	}

	async function runSelectedTopology() {
		if (!selectedTopologyId) {
			notifications.warning('Select a topology to run.');
			return;
		}
		await runAction('run-topology', async () => {
			await runTopology(selectedTopologyId);
			await refreshAll();
			notifications.success('Topology run completed.');
		});
	}
</script>

<svelte:head>
	<title>Streaming | OpenFoundry</title>
</svelte:head>

<div class="space-y-6">
	<section class="overflow-hidden rounded-[36px] border border-slate-200 bg-[radial-gradient(circle_at_top_left,_rgba(56,189,248,0.24),_transparent_34%),linear-gradient(135deg,#0f172a_0%,#1d4ed8_32%,#ecfeff_100%)] p-6 text-white shadow-sm dark:border-slate-800">
		<div class="grid gap-6 xl:grid-cols-[minmax(0,1.08fr)_minmax(0,0.92fr)]">
			<div>
				<div class="text-[11px] font-semibold uppercase tracking-[0.34em] text-cyan-100">Phase 4.2</div>
				<h1 class="mt-3 max-w-3xl text-4xl font-semibold leading-tight">Real-time streaming with DAG topologies, windows, joins, CEP, and live tails</h1>
				<p class="mt-4 max-w-2xl text-sm leading-7 text-slate-100/85">Define named streams, wire DAG processors, simulate RocksDB-backed state, and monitor live event flow with backpressure awareness across connectors and sinks.</p>
			</div>
			<div class="rounded-[28px] border border-white/15 bg-white/10 p-5 backdrop-blur">
				<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-cyan-100">Control Loop</div>
				<div class="mt-3 grid gap-3 text-sm text-slate-100/90">
					<div class="rounded-2xl border border-white/10 bg-white/5 px-4 py-3">1. Register streams and windows.</div>
					<div class="rounded-2xl border border-white/10 bg-white/5 px-4 py-3">2. Compose join, CEP, and sink topologies.</div>
					<div class="rounded-2xl border border-white/10 bg-white/5 px-4 py-3">3. Run the topology and inspect state, backpressure, and live tails.</div>
				</div>
			</div>
		</div>
	</section>

	{#if uiError}
		<div class="rounded-2xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-700 dark:border-rose-900/70 dark:bg-rose-950/40 dark:text-rose-200">{uiError}</div>
	{/if}

	{#if loading}
		<div class="rounded-[28px] border border-slate-200 bg-white px-6 py-10 text-center text-sm text-slate-500 shadow-sm dark:border-slate-800 dark:bg-slate-950 dark:text-slate-400">Loading streaming surfaces...</div>
	{:else}
		<div class="grid gap-6 xl:grid-cols-[minmax(0,1.02fr)_minmax(0,0.98fr)]">
			<StreamList
				streams={streams}
				draft={streamDraft}
				busy={busy}
				onSelect={(streamId) => {
					const stream = streams.find((item) => item.id === streamId);
					if (stream) streamDraft = streamToDraft(stream);
				}}
				onDraftChange={(draft) => streamDraft = draft}
				onSave={saveStreamDraft}
				onReset={() => streamDraft = createEmptyStreamDraft()}
			/>
			<WindowConfig
				windows={windows}
				draft={windowDraft}
				busy={busy}
				onSelect={(windowId) => {
					const window = windows.find((item) => item.id === windowId);
					if (window) windowDraft = windowToDraft(window);
				}}
				onDraftChange={(draft) => windowDraft = draft}
				onSave={saveWindowDraft}
				onReset={() => windowDraft = createEmptyWindowDraft()}
			/>
		</div>

		<TopologyEditor
			topologies={topologies}
			streams={streams}
			windows={windows}
			draft={topologyDraft}
			busy={busy}
			onSelect={(topologyId) => {
				const topology = topologies.find((item) => item.id === topologyId);
				if (topology) {
					topologyDraft = topologyToDraft(topology);
					void loadRuntime(topologyId);
				}
			}}
			onDraftChange={(draft) => topologyDraft = draft}
			onSave={saveTopologyDraft}
			onReset={() => topologyDraft = createEmptyTopologyDraft()}
		/>

		<StreamMonitor
			overview={overview}
			topologies={topologies}
			selectedTopologyId={selectedTopologyId}
			runtime={runtime}
			busy={busy}
			onSelectTopology={(topologyId) => {
				const topology = topologies.find((item) => item.id === topologyId);
				if (topology) topologyDraft = topologyToDraft(topology);
				void loadRuntime(topologyId);
			}}
			onRun={runSelectedTopology}
		/>

		<LiveDataView connectors={connectors} liveTail={liveTail} runtime={runtime} />
	{/if}
</div>