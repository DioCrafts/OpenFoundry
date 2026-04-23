<script lang="ts">
	import {
		createDefaultDateRange,
		resolveDateRange,
		type DashboardDatePreset,
		type DashboardDateRange,
	} from '$lib/utils/dashboards';

	interface Props {
		value: DashboardDateRange;
		disabled?: boolean;
		onChange?: (value: DashboardDateRange) => void;
	}

	const presetOptions: Array<{ value: DashboardDatePreset; label: string }> = [
		{ value: 'last_7_days', label: 'Last 7 days' },
		{ value: 'last_30_days', label: 'Last 30 days' },
		{ value: 'last_90_days', label: 'Last 90 days' },
		{ value: 'this_month', label: 'This month' },
		{ value: 'quarter_to_date', label: 'Quarter to date' },
		{ value: 'custom', label: 'Custom range' },
	];

	let {
		value,
		disabled = false,
		onChange,
	}: Props = $props();

	const resolved = $derived(resolveDateRange(value));

	function handlePresetChange(event: Event) {
		const preset = (event.currentTarget as HTMLSelectElement).value as DashboardDatePreset;

		if (preset === 'custom') {
			const fallback = createDefaultDateRange();
			onChange?.({
				mode: 'absolute',
				preset,
				from: value.from || fallback.from,
				to: value.to || fallback.to,
			});
			return;
		}

		const nextValue: DashboardDateRange = {
			mode: 'relative',
			preset,
			from: value.from,
			to: value.to,
		};

		const nextRange = resolveDateRange(nextValue);
		onChange?.({
			...nextValue,
			from: nextRange.from,
			to: nextRange.to,
		});
	}

	function handleAbsoluteChange(key: 'from' | 'to', nextValue: string) {
		onChange?.({
			mode: 'absolute',
			preset: 'custom',
			from: key === 'from' ? nextValue : value.from,
			to: key === 'to' ? nextValue : value.to,
		});
	}
</script>

<div class="flex flex-wrap items-center gap-3 rounded-xl border border-slate-200 bg-white px-3 py-2 dark:border-slate-800 dark:bg-slate-900">
	<div>
		<div class="text-[11px] font-semibold uppercase tracking-[0.24em] text-slate-500">Date Range</div>
		<div class="mt-1 text-sm font-medium text-slate-700 dark:text-slate-200">{resolved.label}</div>
	</div>

	<select
		class="rounded-lg border border-slate-300 bg-slate-50 px-3 py-2 text-sm dark:border-slate-700 dark:bg-slate-950"
		value={value.preset}
		onchange={handlePresetChange}
		disabled={disabled}
	>
		{#each presetOptions as option}
			<option value={option.value}>{option.label}</option>
		{/each}
	</select>

	{#if value.mode === 'absolute' || value.preset === 'custom'}
		<label class="flex items-center gap-2 text-sm text-slate-600 dark:text-slate-300">
			<span>From</span>
			<input
				type="date"
				class="rounded-lg border border-slate-300 bg-slate-50 px-3 py-2 dark:border-slate-700 dark:bg-slate-950"
				value={value.from}
				onchange={(event) => handleAbsoluteChange('from', (event.currentTarget as HTMLInputElement).value)}
				disabled={disabled}
			/>
		</label>

		<label class="flex items-center gap-2 text-sm text-slate-600 dark:text-slate-300">
			<span>To</span>
			<input
				type="date"
				class="rounded-lg border border-slate-300 bg-slate-50 px-3 py-2 dark:border-slate-700 dark:bg-slate-950"
				value={value.to}
				onchange={(event) => handleAbsoluteChange('to', (event.currentTarget as HTMLInputElement).value)}
				disabled={disabled}
			/>
		</label>
	{/if}
</div>
