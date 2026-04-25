<script lang="ts">
	import type {
		ChatCompletionResponse,
		Conversation,
		ConversationSummary,
		KnowledgeBase,
		LlmProvider,
		ProviderBenchmarkResponse,
		PromptTemplate,
	} from '$lib/api/ai';

	interface ChatDraft {
		conversation_id: string;
		user_message: string;
		system_prompt: string;
		prompt_template_id: string;
		prompt_variables_text: string;
		knowledge_base_id: string;
		preferred_provider_id: string;
		attachments_text: string;
		max_tokens: number;
		fallback_enabled: boolean;
		require_private_network: boolean;
	}

	interface Props {
		conversations: ConversationSummary[];
		conversation: Conversation | null;
		providers: LlmProvider[];
		prompts: PromptTemplate[];
		knowledgeBases: KnowledgeBase[];
		draft: ChatDraft;
		response: ChatCompletionResponse | null;
		benchmarkResponse?: ProviderBenchmarkResponse | null;
		busy?: boolean;
		onSelectConversation?: (conversationId: string) => void;
		onDraftChange?: (draft: ChatDraft) => void;
		onSend?: () => void;
		onBenchmark?: () => void;
		onResetConversation?: () => void;
	}

	let {
		conversations,
		conversation,
		providers,
		prompts,
		knowledgeBases,
		draft,
		response,
		benchmarkResponse = null,
		busy = false,
		onSelectConversation,
		onDraftChange,
		onSend,
		onBenchmark,
		onResetConversation,
	}: Props = $props();

	let localDraft = $state<ChatDraft>({
		conversation_id: '',
		user_message: '',
		system_prompt: '',
		prompt_template_id: '',
		prompt_variables_text: '',
		knowledge_base_id: '',
		preferred_provider_id: '',
		attachments_text: '[]',
		max_tokens: 512,
		fallback_enabled: true,
		require_private_network: false,
	});

	$effect(() => {
		localDraft = { ...draft };
	});

	function updateDraft<K extends keyof ChatDraft>(key: K, value: ChatDraft[K]) {
		const nextDraft = { ...localDraft, [key]: value };
		localDraft = nextDraft;
		onDraftChange?.(nextDraft);
	}
</script>

<section class="rounded-[32px] border border-slate-200 bg-white p-5 shadow-sm dark:border-slate-800 dark:bg-slate-950">
	<div class="flex flex-wrap items-start justify-between gap-3">
		<div>
			<div class="text-[11px] font-semibold uppercase tracking-[0.28em] text-slate-500">Chat Workspace</div>
			<h2 class="mt-2 text-xl font-semibold text-slate-900 dark:text-slate-100">Conversations with prompt, provider, and retrieval controls</h2>
		</div>
		<div class="flex gap-2">
			<button class="rounded-full border border-slate-300 px-3 py-1.5 text-sm text-slate-700 hover:bg-slate-100 dark:border-slate-700 dark:text-slate-200 dark:hover:bg-slate-900" onclick={() => onResetConversation?.()} disabled={busy}>New conversation</button>
			<button class="rounded-full border border-cyan-300 px-3 py-1.5 text-sm text-cyan-700 hover:bg-cyan-50 dark:border-cyan-800 dark:text-cyan-300 dark:hover:bg-cyan-950/40" onclick={() => onBenchmark?.()} disabled={busy}>Benchmark</button>
			<button class="rounded-full bg-slate-950 px-3 py-1.5 text-sm font-medium text-white hover:bg-slate-800 disabled:opacity-60 dark:bg-slate-100 dark:text-slate-950" onclick={() => onSend?.()} disabled={busy}>Send</button>
		</div>
	</div>

	<div class="mt-5 grid gap-5 xl:grid-cols-[minmax(0,0.75fr)_minmax(0,1.25fr)]">
		<div class="space-y-3">
			{#if conversations.length === 0}
				<div class="rounded-2xl border border-dashed border-slate-300 px-4 py-5 text-sm text-slate-500 dark:border-slate-700 dark:text-slate-400">No conversations yet.</div>
			{:else}
				{#each conversations as item}
					<button class={`w-full rounded-2xl border px-4 py-3 text-left transition ${conversation?.id === item.id ? 'border-cyan-400 bg-cyan-50 dark:border-cyan-700 dark:bg-cyan-950/30' : 'border-slate-200 bg-slate-50 hover:border-slate-300 dark:border-slate-800 dark:bg-slate-900 dark:hover:border-slate-700'}`} onclick={() => onSelectConversation?.(item.id)} type="button">
						<div class="text-sm font-semibold text-slate-900 dark:text-slate-100">{item.title}</div>
						<div class="mt-1 text-xs text-slate-500">{item.message_count} messages • {item.last_cache_hit ? 'cache hit' : 'fresh'}</div>
						<p class="mt-2 text-sm text-slate-600 dark:text-slate-300">{item.last_message_preview}</p>
					</button>
				{/each}
			{/if}
		</div>

		<div class="grid gap-4">
			<div class="grid gap-4 lg:grid-cols-2 xl:grid-cols-4">
				<select class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 text-sm dark:border-slate-800 dark:bg-slate-900" value={localDraft.prompt_template_id} onchange={(event) => updateDraft('prompt_template_id', (event.currentTarget as HTMLSelectElement).value)}>
					<option value="">No prompt template</option>
					{#each prompts as prompt}
						<option value={prompt.id}>{prompt.name}</option>
					{/each}
				</select>
				<select class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 text-sm dark:border-slate-800 dark:bg-slate-900" value={localDraft.knowledge_base_id} onchange={(event) => updateDraft('knowledge_base_id', (event.currentTarget as HTMLSelectElement).value)}>
					<option value="">No knowledge base</option>
					{#each knowledgeBases as knowledgeBase}
						<option value={knowledgeBase.id}>{knowledgeBase.name}</option>
					{/each}
				</select>
				<select class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 text-sm dark:border-slate-800 dark:bg-slate-900" value={localDraft.preferred_provider_id} onchange={(event) => updateDraft('preferred_provider_id', (event.currentTarget as HTMLSelectElement).value)}>
					<option value="">Automatic routing</option>
					{#each providers as provider}
						<option value={provider.id}>{provider.name}</option>
					{/each}
				</select>
				<input class="rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 text-sm dark:border-slate-800 dark:bg-slate-900" type="number" value={String(localDraft.max_tokens)} oninput={(event) => updateDraft('max_tokens', Number((event.currentTarget as HTMLInputElement).value) || 512)} />
			</div>

			<textarea class="h-20 rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 text-sm dark:border-slate-800 dark:bg-slate-900" oninput={(event) => updateDraft('system_prompt', (event.currentTarget as HTMLTextAreaElement).value)}>{localDraft.system_prompt}</textarea>
			<textarea class="h-20 rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 text-sm dark:border-slate-800 dark:bg-slate-900" oninput={(event) => updateDraft('prompt_variables_text', (event.currentTarget as HTMLTextAreaElement).value)}>{localDraft.prompt_variables_text}</textarea>
			<textarea class="h-28 rounded-2xl border border-slate-200 bg-white px-4 py-3 text-sm dark:border-slate-800 dark:bg-slate-950" oninput={(event) => updateDraft('user_message', (event.currentTarget as HTMLTextAreaElement).value)}>{localDraft.user_message}</textarea>
			<textarea class="h-24 rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 text-sm dark:border-slate-800 dark:bg-slate-900" oninput={(event) => updateDraft('attachments_text', (event.currentTarget as HTMLTextAreaElement).value)}>{localDraft.attachments_text}</textarea>
			<div class="grid gap-3 lg:grid-cols-2">
				<label class="flex items-center gap-3 rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 text-sm text-slate-700 dark:border-slate-800 dark:bg-slate-900 dark:text-slate-300">
					<input type="checkbox" checked={localDraft.fallback_enabled} onchange={() => updateDraft('fallback_enabled', !localDraft.fallback_enabled)} />
					<span>Enable fallback routing</span>
				</label>
				<label class="flex items-center gap-3 rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 text-sm text-slate-700 dark:border-slate-800 dark:bg-slate-900 dark:text-slate-300">
					<input type="checkbox" checked={localDraft.require_private_network} onchange={() => updateDraft('require_private_network', !localDraft.require_private_network)} />
					<span>Private network only</span>
				</label>
			</div>

			<div class="grid gap-4 lg:grid-cols-[minmax(0,1.2fr)_minmax(0,0.8fr)]">
				<div class="rounded-[24px] border border-slate-200 bg-slate-50 p-4 dark:border-slate-800 dark:bg-slate-900">
					<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Transcript</div>
					<div class="mt-3 max-h-[28rem] space-y-3 overflow-y-auto">
						{#if conversation?.messages.length}
							{#each conversation.messages as message}
								<div class={`rounded-2xl px-4 py-3 ${message.role === 'assistant' ? 'bg-cyan-50 text-slate-700 dark:bg-cyan-950/20 dark:text-slate-200' : 'bg-white text-slate-700 dark:bg-slate-950 dark:text-slate-200'}`}>
									<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">{message.role}</div>
									<p class="mt-2 whitespace-pre-wrap text-sm leading-6">{message.content}</p>
									{#if message.attachments.length > 0}
										<div class="mt-3 flex flex-wrap gap-2 text-xs text-slate-500 dark:text-slate-400">
											{#each message.attachments as attachment}
												<span class="rounded-full bg-white/80 px-2 py-1 dark:bg-slate-900">{attachment.kind}{attachment.name ? ` · ${attachment.name}` : ''}</span>
											{/each}
										</div>
									{/if}
									{#if message.citations.length > 0}
										<div class="mt-3 flex flex-wrap gap-2 text-xs text-cyan-700 dark:text-cyan-300">
											{#each message.citations as citation}
												<span class="rounded-full bg-white/80 px-2 py-1 dark:bg-slate-900">{citation.document_title}</span>
											{/each}
										</div>
									{/if}
								</div>
							{/each}
						{:else}
							<p class="text-sm text-slate-500">Start a conversation to see the transcript.</p>
						{/if}
					</div>
				</div>

				<div class="space-y-4 rounded-[24px] border border-slate-200 bg-gradient-to-br from-slate-50 to-cyan-50 p-4 dark:border-slate-800 dark:from-slate-900 dark:to-slate-950">
					<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Latest Response</div>
					{#if response}
						<div class="space-y-3 text-sm text-slate-700 dark:text-slate-200">
							<div class="rounded-2xl border border-slate-200 bg-white px-3 py-2 dark:border-slate-800 dark:bg-slate-950">Provider: {response.provider_name}</div>
							<div class="rounded-2xl border border-slate-200 bg-white px-3 py-2 dark:border-slate-800 dark:bg-slate-950">Cache: {response.cache.hit ? 'hit' : 'miss'} • {response.usage.total_tokens} tokens</div>
							<div class="rounded-2xl border border-slate-200 bg-white px-3 py-2 dark:border-slate-800 dark:bg-slate-950">Cost: ${response.usage.estimated_cost_usd.toFixed(4)} • {response.usage.latency_ms} ms • {response.usage.network_scope}</div>
							<div class="rounded-2xl border border-slate-200 bg-white px-3 py-2 dark:border-slate-800 dark:bg-slate-950">Routing: {response.routing.used_private_network ? 'private' : 'standard'} • {response.routing.required_modalities.join(', ')}</div>
							<div class="rounded-2xl border border-slate-200 bg-white px-3 py-3 dark:border-slate-800 dark:bg-slate-950">
								<div class="text-xs font-semibold uppercase tracking-[0.24em] text-slate-500">Guardrails</div>
								<p class="mt-2">{response.guardrail.blocked ? 'Blocked' : 'Passed'} • {response.guardrail.flags.length} flags</p>
							</div>
						</div>
					{:else}
						<p class="text-sm text-slate-500">Response metadata appears here after sending.</p>
					{/if}
					{#if benchmarkResponse}
						<div class="rounded-2xl border border-cyan-200 bg-white px-3 py-3 dark:border-cyan-900 dark:bg-slate-950">
							<div class="text-xs font-semibold uppercase tracking-[0.24em] text-slate-500">Benchmark</div>
							<div class="mt-2 space-y-2 text-xs text-slate-600 dark:text-slate-300">
								{#each benchmarkResponse.results as result}
									<div class="rounded-2xl border border-slate-200 px-3 py-2 dark:border-slate-800">
										<div class="font-semibold text-slate-900 dark:text-slate-100">{result.provider_name} • {(result.score.overall * 100).toFixed(0)}%</div>
										<div class="mt-1">{result.error ?? `${result.latency_ms} ms • $${result.estimated_cost_usd.toFixed(4)} • ${result.network_scope}`}</div>
									</div>
								{/each}
							</div>
						</div>
					{/if}
				</div>
			</div>
		</div>
	</div>
</section>
