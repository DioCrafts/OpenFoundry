<script lang="ts">
	import KernelStatus from './KernelStatus.svelte';

	type Kernel = 'python' | 'sql';

	interface Props {
		value: Kernel;
		status?: string | null;
		disabled?: boolean;
		onChange?: (kernel: Kernel) => void;
		onStart?: () => void;
		onStop?: () => void;
	}

	let {
		value,
		status = null,
		disabled = false,
		onChange,
		onStart,
		onStop,
	}: Props = $props();

	function handleChange(event: Event) {
		onChange?.((event.currentTarget as HTMLSelectElement).value as Kernel);
	}
</script>

<div class="flex flex-wrap items-center gap-3 rounded-xl border border-slate-200 bg-white px-3 py-2 shadow-sm dark:border-slate-800 dark:bg-slate-900">
	<div class="text-xs font-semibold uppercase tracking-[0.2em] text-slate-500">
		Kernel
	</div>

	<select
		class="rounded-lg border border-slate-300 bg-slate-50 px-3 py-1.5 text-sm dark:border-slate-700 dark:bg-slate-800"
		value={value}
		onchange={handleChange}
		disabled={disabled}
	>
		<option value="python">Python</option>
		<option value="sql">SQL</option>
	</select>

	<KernelStatus kernel={value} status={status} />

	{#if !status || status === 'dead'}
		<button class="rounded-lg bg-slate-900 px-3 py-1.5 text-sm text-white dark:bg-slate-100 dark:text-slate-950" onclick={() => onStart?.()}>
			Start Session
		</button>
	{:else}
		<button class="rounded-lg border border-slate-300 px-3 py-1.5 text-sm dark:border-slate-700" onclick={() => onStop?.()}>
			Stop Session
		</button>
	{/if}
</div>
