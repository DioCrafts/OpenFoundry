<script lang="ts">
	interface Props {
		kernel?: string;
		status?: string | null;
	}

	let {
		kernel = 'python',
		status = null,
	}: Props = $props();

	function tone(currentStatus: string | null) {
		switch (currentStatus) {
			case 'busy':
				return 'bg-amber-100 text-amber-700 dark:bg-amber-950 dark:text-amber-300';
			case 'idle':
				return 'bg-emerald-100 text-emerald-700 dark:bg-emerald-950 dark:text-emerald-300';
			case 'dead':
				return 'bg-rose-100 text-rose-700 dark:bg-rose-950 dark:text-rose-300';
			default:
				return 'bg-slate-100 text-slate-600 dark:bg-slate-800 dark:text-slate-300';
		}
	}
</script>

<span class={`inline-flex items-center gap-2 rounded-full px-2.5 py-1 text-xs font-medium ${tone(status)}`}>
	<span class="h-2 w-2 rounded-full bg-current opacity-70"></span>
	{kernel} {status ?? 'offline'}
</span>
