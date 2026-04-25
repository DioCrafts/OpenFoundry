<script lang="ts">
	import type { CellOutput as NotebookCellOutput } from '$lib/api/notebooks';

	interface Props {
		output: NotebookCellOutput | null;
	}

	interface TableContent {
		columns: Array<{ name: string; data_type: string }>;
		rows: string[][];
		total_rows: number;
		execution_time_ms: number;
	}

	interface LlmContent {
		reply: string;
		provider_name?: string;
		conversation_id?: string;
		citations?: Array<{ document_title?: string; excerpt?: string; source_uri?: string | null }>;
		usage?: {
			total_tokens?: number;
			latency_ms?: number;
			estimated_cost_usd?: number;
		};
	}

	let { output }: Props = $props();

	function getTableContent(content: unknown): TableContent | null {
		if (!content || typeof content !== 'object') {
			return null;
		}

		const candidate = content as Partial<TableContent>;
		if (!Array.isArray(candidate.columns) || !Array.isArray(candidate.rows)) {
			return null;
		}

		return candidate as TableContent;
	}

	function formatContent(content: unknown): string {
		return typeof content === 'string' ? content : JSON.stringify(content, null, 2);
	}

	function getLlmContent(content: unknown): LlmContent | null {
		if (!content || typeof content !== 'object') {
			return null;
		}

		const candidate = content as Partial<LlmContent>;
		return typeof candidate.reply === 'string' ? candidate as LlmContent : null;
	}
</script>

{#if output}
	<div class="border-t border-slate-800 bg-slate-950 text-slate-100">
		{#if output.output_type === 'table'}
			{@const table = getTableContent(output.content)}
			{#if table}
				<div class="border-b border-slate-800 px-4 py-2 text-xs text-slate-400">
					{table.total_rows} rows in {table.execution_time_ms}ms
				</div>
				<div class="max-h-72 overflow-auto">
					<table class="min-w-full text-left text-sm">
						<thead class="sticky top-0 bg-slate-900 text-xs uppercase tracking-wide text-slate-400">
							<tr>
								{#each table.columns as column}
									<th class="border-b border-slate-800 px-4 py-2 font-medium">
										{column.name}
										<span class="ml-2 text-[10px] normal-case text-slate-500">{column.data_type}</span>
									</th>
								{/each}
							</tr>
						</thead>
						<tbody>
							{#each table.rows as row, index}
								<tr class={index % 2 === 0 ? 'bg-slate-900/40' : ''}>
									{#each row as cell}
										<td class="border-b border-slate-900 px-4 py-2 font-mono text-xs text-slate-200">
											{cell}
										</td>
									{/each}
								</tr>
							{/each}
						</tbody>
					</table>
				</div>
			{:else}
				<pre class="overflow-auto p-4 font-mono text-xs text-slate-200">{formatContent(output.content)}</pre>
			{/if}
		{:else if output.output_type === 'llm'}
			{@const llm = getLlmContent(output.content)}
			{#if llm}
				<div class="space-y-4 p-4">
					<div class="flex flex-wrap items-center gap-2 text-xs text-slate-400">
						<span class="rounded-full border border-sky-700/60 bg-sky-950/40 px-2 py-1 font-semibold uppercase tracking-[0.2em] text-sky-300">LLM</span>
						{#if llm.provider_name}
							<span>{llm.provider_name}</span>
						{/if}
						{#if llm.usage?.total_tokens}
							<span>· {llm.usage.total_tokens} tokens</span>
						{/if}
						{#if llm.usage?.latency_ms}
							<span>· {llm.usage.latency_ms}ms</span>
						{/if}
						{#if llm.usage?.estimated_cost_usd !== undefined}
							<span>· ${llm.usage.estimated_cost_usd.toFixed(4)}</span>
						{/if}
					</div>
					<div class="whitespace-pre-wrap text-sm leading-7 text-slate-100">{llm.reply}</div>
					{#if llm.citations && llm.citations.length > 0}
						<div class="rounded-2xl border border-slate-800 bg-slate-900/70 p-3">
							<div class="text-xs font-semibold uppercase tracking-[0.2em] text-slate-400">Citations</div>
							<div class="mt-3 space-y-2">
								{#each llm.citations as citation}
									<div class="rounded-xl border border-slate-800 bg-slate-950/80 px-3 py-2">
										<div class="text-sm font-medium text-slate-100">{citation.document_title ?? 'Knowledge document'}</div>
										<div class="mt-1 text-xs leading-6 text-slate-400">{citation.excerpt ?? 'No excerpt available.'}</div>
									</div>
								{/each}
							</div>
						</div>
					{/if}
				</div>
			{:else}
				<pre class="overflow-auto p-4 font-mono text-xs text-slate-200">{formatContent(output.content)}</pre>
			{/if}
		{:else if output.output_type === 'error'}
			<pre class="overflow-auto p-4 font-mono text-xs text-red-300">{formatContent(output.content)}</pre>
		{:else}
			<pre class="overflow-auto p-4 font-mono text-xs text-emerald-300">{formatContent(output.content)}</pre>
		{/if}
	</div>
{/if}
