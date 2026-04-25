<script lang="ts">
	import { goto } from '$app/navigation';
	import { executeQuery } from '$lib/api/queries';
	import type { AppDefinition, AppPage, WidgetEvent, WorkshopScenarioPreset } from '$lib/api/apps';

	import AppWidgetRenderer from './AppWidgetRenderer.svelte';

	interface Props {
		app: AppDefinition;
		mode?: 'builder' | 'published';
	}

	let { app, mode = 'published' }: Props = $props();

	let activePageId = $state('');
	let runtimeFilter = $state('');
	let banner = $state('');
	let runtimeParameters = $state<Record<string, string>>({});
	let interactivePromptSeed = $state('');
	let activePresetId = $state('');

	const visiblePages = $derived(app.pages.filter((page) => page.visible));
	const interactiveWorkshop = $derived(app.settings.interactive_workshop);
	const activePage = $derived(
		visiblePages.find((page) => page.id === activePageId)
			?? visiblePages[0]
			?? null,
	);
	const runtimeLabel = $derived(
		app.settings.builder_experience === 'slate'
			? 'Slate runtime'
			: app.settings.consumer_mode.enabled
				? 'Consumer runtime'
				: 'Workshop runtime',
	);
	const portalTitle = $derived(app.settings.consumer_mode.portal_title || app.name);
	const portalSubtitle = $derived(
		app.settings.consumer_mode.portal_subtitle
			|| 'Published experience for operators, partners, or external consumers.',
	);
	const interactiveTitle = $derived(interactiveWorkshop.title || 'Interactive Workshop');
	const interactiveSubtitle = $derived(
		interactiveWorkshop.subtitle
			|| 'Coordinate scenarios, decisions, and copilot prompts from a single app runtime.',
	);
	const interactiveBriefing = $derived(
		interactiveWorkshop.briefing_template
			? interpolateTemplate(interactiveWorkshop.briefing_template, runtimeParameters)
			: '',
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

	$effect(() => {
		app.id;
		runtimeParameters = {};
		interactivePromptSeed = '';
		activePresetId = '';
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
			return;
		}

		if (action.action === 'set_parameters') {
			const nextParameters: Record<string, string> = { ...runtimeParameters };
			for (const [key, value] of Object.entries(payload ?? {})) {
				if (value === null || value === undefined || value === '') {
					delete nextParameters[key];
				} else {
					nextParameters[key] = String(value);
				}
			}
			runtimeParameters = nextParameters;
			activePresetId = '';
			const nextKeys = Object.keys(nextParameters);
			banner = nextKeys.length
				? `Scenario applied: ${nextKeys.join(', ')}`
				: 'Scenario parameters cleared';
			return;
		}

		if (action.action === 'clear_parameters') {
			runtimeParameters = {};
			activePresetId = '';
			interactivePromptSeed = '';
			banner = 'Scenario parameters cleared';
		}
	}

	function interpolateTemplate(template: string, parameters: Record<string, string>) {
		return template.replace(/\{\{\s*([a-zA-Z0-9_.-]+)\s*\}\}/g, (_, key: string) => {
			const value = parameters[key];
			return value === undefined ? '' : value;
		});
	}

	function applyInteractivePreset(preset: WorkshopScenarioPreset) {
		runtimeParameters = { ...preset.parameters };
		activePresetId = preset.id;
		const nextPrompt = preset.prompt_template
			? interpolateTemplate(preset.prompt_template, preset.parameters)
			: '';
		interactivePromptSeed = nextPrompt;
		banner = `Scenario preset applied: ${preset.label}`;
	}

	function clearInteractivePreset() {
		runtimeParameters = {};
		activePresetId = '';
		interactivePromptSeed = '';
		banner = 'Scenario parameters cleared';
	}

	function seedInteractivePrompt(question: string) {
		interactivePromptSeed = interpolateTemplate(question, runtimeParameters);
		banner = 'Copilot prompt seeded from Workshop guide';
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
		{#if interactiveWorkshop.enabled}
			<section class="mb-5 overflow-hidden rounded-[calc(var(--app-radius)_+_4px)] border border-slate-200 bg-[radial-gradient(circle_at_top_left,_rgba(249,115,22,0.16),_transparent_24%),linear-gradient(135deg,_#fff7ed,_#ffffff_52%,_#ecfeff)] p-5">
				<div class="flex flex-wrap items-start justify-between gap-4">
					<div class="max-w-3xl">
						<div class="text-xs uppercase tracking-[0.28em] text-slate-400">Workshop interactive</div>
						<h2 class="mt-2 text-3xl font-semibold" style={`font-family:${app.theme.heading_font}, sans-serif;`}>{interactiveTitle}</h2>
						<p class="mt-3 text-sm leading-7 text-slate-600">{interactiveSubtitle}</p>
					</div>
					<div class="flex flex-wrap gap-2 text-xs">
						{#if Object.keys(runtimeParameters).length > 0}
							<span class="rounded-full border border-orange-200 bg-orange-50 px-3 py-1 text-orange-700">{Object.keys(runtimeParameters).length} active scenario signal(s)</span>
						{/if}
						{#if interactiveWorkshop.primary_agent_widget_id}
							<span class="rounded-full border border-slate-200 bg-white px-3 py-1 text-slate-600">Copilot linked</span>
						{/if}
					</div>
				</div>

				{#if interactiveWorkshop.scenario_presets.length > 0}
					<div class="mt-5">
						<div class="text-xs font-semibold uppercase tracking-[0.2em] text-slate-400">Scenario presets</div>
						<div class="mt-3 flex flex-wrap gap-2">
							{#each interactiveWorkshop.scenario_presets as preset (preset.id)}
								<button
									type="button"
									onclick={() => applyInteractivePreset(preset)}
									class={`rounded-full px-4 py-2 text-sm transition ${activePresetId === preset.id ? 'bg-[var(--app-primary)] text-white' : 'border border-slate-200 bg-white text-slate-700 hover:bg-slate-50'}`}
								>
									{preset.label}
								</button>
							{/each}
							<button type="button" onclick={clearInteractivePreset} class="rounded-full border border-slate-200 bg-white px-4 py-2 text-sm text-slate-600 hover:bg-slate-50">
								Reset
							</button>
						</div>
					</div>
				{/if}

				<div class="mt-5 grid gap-4 lg:grid-cols-[1.1fr,0.9fr]">
					<div class="rounded-2xl border border-white/70 bg-white/80 p-4 shadow-sm">
						<div class="text-xs font-semibold uppercase tracking-[0.2em] text-slate-400">Decision brief</div>
						{#if interactiveBriefing.trim()}
							<p class="mt-3 whitespace-pre-wrap text-sm leading-7 text-slate-700">{interactiveBriefing}</p>
						{:else}
							<p class="mt-3 text-sm text-slate-500">Add a briefing template in Workshop settings to summarize the current runtime assumptions.</p>
						{/if}
					</div>

					<div class="space-y-4">
						<div class="rounded-2xl border border-white/70 bg-white/80 p-4 shadow-sm">
							<div class="text-xs font-semibold uppercase tracking-[0.2em] text-slate-400">Active scenario context</div>
							{#if Object.keys(runtimeParameters).length > 0}
								<div class="mt-3 flex flex-wrap gap-2">
									{#each Object.entries(runtimeParameters) as [key, value]}
										<span class="rounded-full border border-slate-200 bg-white px-3 py-1 text-xs text-slate-600">{key}: {value}</span>
									{/each}
								</div>
							{:else}
								<p class="mt-3 text-sm text-slate-500">Apply a scenario preset or use a scenario widget to populate runtime assumptions.</p>
							{/if}
						</div>

						{#if interactiveWorkshop.suggested_questions.length > 0}
							<div class="rounded-2xl border border-white/70 bg-white/80 p-4 shadow-sm">
								<div class="text-xs font-semibold uppercase tracking-[0.2em] text-slate-400">Copilot starters</div>
								<div class="mt-3 flex flex-wrap gap-2">
									{#each interactiveWorkshop.suggested_questions as question}
										<button type="button" onclick={() => seedInteractivePrompt(question)} class="rounded-full border border-slate-200 bg-white px-3 py-2 text-left text-xs text-slate-600 hover:bg-slate-50">
											{interpolateTemplate(question, runtimeParameters)}
										</button>
									{/each}
								</div>
							</div>
						{/if}
					</div>
				</div>
			</section>
		{/if}

		{#if app.settings.consumer_mode.enabled && mode === 'published'}
			<section class="mb-5 overflow-hidden rounded-[calc(var(--app-radius)_+_4px)] border border-slate-200 bg-[radial-gradient(circle_at_top_left,_rgba(15,118,110,0.16),_transparent_28%),linear-gradient(135deg,_#ffffff,_#f8fafc_55%,_#e0f2fe)] p-5">
				<div class="flex flex-wrap items-start justify-between gap-4">
					<div class="max-w-3xl">
						<div class="text-xs uppercase tracking-[0.28em] text-slate-400">Consumer mode</div>
						<h2 class="mt-2 text-3xl font-semibold" style={`font-family:${app.theme.heading_font}, sans-serif;`}>{portalTitle}</h2>
						<p class="mt-3 text-sm leading-7 text-slate-600">{portalSubtitle}</p>
					</div>
					<div class="flex flex-wrap gap-2 text-xs">
						{#if app.settings.consumer_mode.allow_guest_access}
							<span class="rounded-full border border-emerald-200 bg-emerald-50 px-3 py-1 text-emerald-700">Guest access ready</span>
						{/if}
						{#if app.settings.consumer_mode.primary_cta_label && app.settings.consumer_mode.primary_cta_url}
							<a
								href={app.settings.consumer_mode.primary_cta_url}
								class="rounded-full bg-[var(--app-primary)] px-4 py-2 font-medium text-white"
							>
								{app.settings.consumer_mode.primary_cta_label}
							</a>
						{/if}
					</div>
				</div>
			</section>
		{/if}

		<div class="flex flex-wrap items-start justify-between gap-4 border-b border-slate-200 pb-4">
			<div>
				<div class="flex items-center gap-3">
					{#if app.theme.logo_url}
						<img src={app.theme.logo_url} alt={app.name} class="h-10 w-10 rounded-xl object-cover" />
					{/if}
					<div>
						<div class="text-xs uppercase tracking-[0.28em] text-slate-400">{runtimeLabel}</div>
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
				{#if Object.keys(runtimeParameters).length > 0}
					<span class="rounded-full bg-[var(--app-accent)]/10 px-3 py-1 text-[var(--app-accent)]">{Object.keys(runtimeParameters).length} runtime parameter(s)</span>
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
										<AppWidgetRenderer widget={widget} globalFilter={runtimeFilter} runtimeParameters={runtimeParameters} interactivePromptSeed={interactivePromptSeed} primaryInteractiveAgentWidgetId={interactiveWorkshop.primary_agent_widget_id} onAction={handleAction} />
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
									<AppWidgetRenderer widget={widget} globalFilter={runtimeFilter} runtimeParameters={runtimeParameters} interactivePromptSeed={interactivePromptSeed} primaryInteractiveAgentWidgetId={interactiveWorkshop.primary_agent_widget_id} onAction={handleAction} />
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
						<AppWidgetRenderer widget={widget} globalFilter={runtimeFilter} runtimeParameters={runtimeParameters} interactivePromptSeed={interactivePromptSeed} primaryInteractiveAgentWidgetId={interactiveWorkshop.primary_agent_widget_id} onAction={handleAction} />
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
