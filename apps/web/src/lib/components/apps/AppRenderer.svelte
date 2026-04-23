<script lang="ts">
	import { goto } from '$app/navigation';
	import { executeQuery } from '$lib/api/queries';
	import type { AppDefinition, AppPage, WidgetEvent } from '$lib/api/apps';

	import AppWidgetRenderer from './AppWidgetRenderer.svelte';

	interface Props {
		app: AppDefinition;
		mode?: 'builder' | 'published';
	}

	let { app, mode = 'published' }: Props = $props();

	let activePageId = $state('');
	let runtimeFilter = $state('');
	let banner = $state('');

	const visiblePages = $derived(app.pages.filter((page) => page.visible));
	const activePage = $derived(
		visiblePages.find((page) => page.id === activePageId)
			?? visiblePages[0]
			?? null,
	);

	const themeStyle = $derived([
		`--app-primary:${app.theme.primary_color}`,
		`--app-accent:${app.theme.accent_color}`,
		`--app-background:${app.theme.background_color}`,
		`--app-surface:${app.theme.surface_color}`,
		`--app-text:${app.theme.text_color}`,
		`--app-radius:${app.theme.border_radius}px`,
		`--app-heading-font:${app.theme.heading_font}`,
		`--app-body-font:${app.theme.body_font}`,
	].join(';'));

	$effect(() => {
		const homePageId = app.settings.home_page_id ?? app.pages[0]?.id ?? '';
		if (!activePageId || !app.pages.some((page) => page.id === activePageId)) {
			activePageId = homePageId;
		}
	});

	async function handleAction(action: WidgetEvent, payload?: Record<string, unknown>) {
		const config = action.config ?? {};

		if (action.action === 'navigate') {
			const target = String(config.page_id ?? config.page_path ?? config.path ?? '');
			const page = app.pages.find((candidate) => candidate.id === target || candidate.path === target);
			if (page) {
				activePageId = page.id;
				banner = `Navigated to ${page.name}`;
				return;
			}

			if (mode === 'published' && target.startsWith('/')) {
				await goto(target);
			}
			return;
		}

		if (action.action === 'open_link') {
			const url = String(config.url ?? '');
			if (!url) return;

			if (mode === 'builder') {
				banner = `Preview would open ${url}`;
				return;
			}

			if (url.startsWith('/')) {
				await goto(url);
			} else {
				window.open(url, '_blank', 'noopener,noreferrer');
			}
			return;
		}

		if (action.action === 'filter') {
			const explicit = typeof config.value === 'string' ? config.value : null;
			const field = typeof config.field === 'string' ? config.field : null;
			const nextFilter = explicit ?? (field && payload ? String(payload[field] ?? '') : '');
			runtimeFilter = nextFilter;
			banner = nextFilter ? `Filter applied: ${nextFilter}` : 'Filter cleared';
			return;
		}

		if (action.action === 'execute_query') {
			const sql = typeof config.sql === 'string' ? config.sql : '';
			if (!sql) {
				banner = 'No SQL configured for this action';
				return;
			}

			try {
				const result = await executeQuery(sql, 20);
				banner = `Action query executed: ${result.total_rows} row(s)`;
			} catch (error) {
				banner = error instanceof Error ? error.message : 'Action query failed';
			}
		}
	}

	function canvasStyle(page: AppPage | null) {
		if (!page) return '';
		return [
			`grid-template-columns: repeat(${page.layout.columns}, minmax(0, 1fr))`,
			`gap: ${page.layout.gap}`,
			'grid-auto-rows: minmax(88px, auto)',
			`max-width: ${app.settings.max_width || page.layout.max_width}`,
		].join(';');
	}
</script>

<div class="min-h-[320px] rounded-[calc(var(--app-radius)_+_8px)] border border-slate-200 bg-[var(--app-background)] p-5 shadow-sm" style={themeStyle}>
	<div class="rounded-[var(--app-radius)] bg-[var(--app-surface)] p-5 text-[var(--app-text)] shadow-sm">
		<div class="flex flex-wrap items-start justify-between gap-4 border-b border-slate-200 pb-4">
			<div>
				<div class="flex items-center gap-3">
					{#if app.theme.logo_url}
						<img src={app.theme.logo_url} alt={app.name} class="h-10 w-10 rounded-xl object-cover" />
					{/if}
					<div>
						<div class="text-xs uppercase tracking-[0.28em] text-slate-400">Workshop runtime</div>
						<h2 class="mt-1 text-3xl font-semibold" style={`font-family:${app.theme.heading_font}, sans-serif;`}>{app.name}</h2>
					</div>
				</div>
				<p class="mt-3 max-w-3xl text-sm text-slate-500">{app.description}</p>
			</div>

			<div class="flex flex-wrap items-center gap-2 text-xs">
				<span class="rounded-full border border-slate-200 px-3 py-1">{visiblePages.length} pages</span>
				<span class="rounded-full border border-slate-200 px-3 py-1">{app.status}</span>
				{#if runtimeFilter}
					<span class="rounded-full bg-[var(--app-primary)]/10 px-3 py-1 text-[var(--app-primary)]">Filter: {runtimeFilter}</span>
				{/if}
			</div>
		</div>

		{#if banner}
			<div class="mt-4 rounded-2xl border border-slate-200 bg-slate-50 px-4 py-3 text-sm text-slate-600">{banner}</div>
		{/if}

		{#if visiblePages.length > 1}
			{#if app.settings.navigation_style === 'sidebar'}
				<div class="mt-5 grid gap-5 lg:grid-cols-[220px,1fr]">
					<aside class="rounded-2xl border border-slate-200 bg-slate-50 p-3">
						<div class="text-xs uppercase tracking-[0.24em] text-slate-400">Pages</div>
						<div class="mt-3 space-y-2">
							{#each visiblePages as page}
								<button
									type="button"
									onclick={() => activePageId = page.id}
									class={`w-full rounded-xl px-3 py-2 text-left text-sm ${activePage?.id === page.id ? 'bg-[var(--app-primary)] text-white' : 'hover:bg-white'}`}
								>
									{page.name}
								</button>
							{/each}
						</div>
					</aside>

					<section>
						{#if activePage}
							<div class="grid" style={canvasStyle(activePage)}>
								{#each activePage.widgets as widget (widget.id)}
									<div style={`grid-column:${Math.max(1, widget.position.x + 1)} / span ${Math.max(1, widget.position.width)}; grid-row:${Math.max(1, widget.position.y + 1)} / span ${Math.max(1, widget.position.height)};`}>
										<AppWidgetRenderer widget={widget} globalFilter={runtimeFilter} onAction={handleAction} />
									</div>
								{/each}
							</div>
						{/if}
					</section>
				</div>
			{:else}
				<div class="mt-5 space-y-5">
					<div class="flex flex-wrap gap-2">
						{#each visiblePages as page}
							<button
								type="button"
								onclick={() => activePageId = page.id}
								class={`rounded-full px-4 py-2 text-sm ${activePage?.id === page.id ? 'bg-[var(--app-primary)] text-white' : 'border border-slate-200 hover:bg-slate-50'}`}
							>
								{page.name}
							</button>
						{/each}
					</div>

					{#if activePage}
						<div class="grid" style={canvasStyle(activePage)}>
							{#each activePage.widgets as widget (widget.id)}
								<div style={`grid-column:${Math.max(1, widget.position.x + 1)} / span ${Math.max(1, widget.position.width)}; grid-row:${Math.max(1, widget.position.y + 1)} / span ${Math.max(1, widget.position.height)};`}>
									<AppWidgetRenderer widget={widget} globalFilter={runtimeFilter} onAction={handleAction} />
								</div>
							{/each}
						</div>
					{/if}
				</div>
			{/if}
		{:else if activePage}
			<div class="mt-5 grid" style={canvasStyle(activePage)}>
				{#each activePage.widgets as widget (widget.id)}
					<div style={`grid-column:${Math.max(1, widget.position.x + 1)} / span ${Math.max(1, widget.position.width)}; grid-row:${Math.max(1, widget.position.y + 1)} / span ${Math.max(1, widget.position.height)};`}>
						<AppWidgetRenderer widget={widget} globalFilter={runtimeFilter} onAction={handleAction} />
					</div>
				{/each}
			</div>
		{/if}

		{#if app.settings.show_branding}
			<div class="mt-6 flex items-center justify-between border-t border-slate-200 pt-4 text-xs text-slate-400">
				<span>Powered by OpenFoundry Workshop</span>
				<span>{app.slug}</span>
			</div>
		{/if}
	</div>
</div>