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
		{:else if output.output_type === 'error'}
			<pre class="overflow-auto p-4 font-mono text-xs text-red-300">{formatContent(output.content)}</pre>
		{:else}
			<pre class="overflow-auto p-4 font-mono text-xs text-emerald-300">{formatContent(output.content)}</pre>
		{/if}
	</div>
{/if}
