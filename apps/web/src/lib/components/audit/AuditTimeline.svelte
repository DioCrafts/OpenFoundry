<script lang="ts">
	import type { AuditEvent } from '$lib/api/audit';

	export let events: AuditEvent[] = [];

	$: timeline = [...events].reverse();
</script>

<section class="rounded-3xl border border-stone-200 bg-white p-5 shadow-sm shadow-stone-200/60">
	<div>
		<p class="text-xs font-semibold uppercase tracking-[0.24em] text-violet-700">Audit Timeline</p>
		<h3 class="mt-2 text-xl font-semibold text-stone-900">Hash chain and sequence integrity</h3>
		<p class="mt-1 text-sm text-stone-500">Visualize the append-only chain from oldest to newest entry hash.</p>
	</div>

	<div class="mt-5 space-y-4">
		{#each timeline as event}
			<div class="rounded-2xl border border-stone-200 bg-stone-50 px-4 py-4">
				<div class="flex items-start justify-between gap-3">
					<div>
						<p class="font-semibold text-stone-900">Sequence {event.sequence}</p>
						<p class="text-sm text-stone-500">{event.action} • {new Date(event.occurred_at).toLocaleString()}</p>
					</div>
					<span class="rounded-full bg-violet-100 px-3 py-1 text-xs font-semibold uppercase tracking-[0.18em] text-violet-700">{event.channel}</span>
				</div>
				<div class="mt-3 grid gap-3 md:grid-cols-2">
					<div class="rounded-2xl bg-white px-3 py-3">
						<p class="text-xs uppercase tracking-[0.18em] text-stone-400">Previous hash</p>
						<p class="mt-2 font-mono text-xs text-stone-700">{event.previous_hash}</p>
					</div>
					<div class="rounded-2xl bg-stone-950 px-3 py-3 text-stone-50">
						<p class="text-xs uppercase tracking-[0.18em] text-violet-300">Entry hash</p>
						<p class="mt-2 font-mono text-xs">{event.entry_hash}</p>
					</div>
				</div>
			</div>
		{/each}
	</div>
</section>
