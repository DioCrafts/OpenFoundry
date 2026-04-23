<script lang="ts">
	import { onMount } from 'svelte';

	import AppRenderer from '$lib/components/apps/AppRenderer.svelte';
	import {
		createApp,
		createAppFromTemplate,
		deleteApp,
		getApp,
		listApps,
		listAppTemplates,
		listAppVersions,
		listWidgetCatalog,
		previewApp,
		publishApp,
		updateApp,
		type AppDefinition,
		type AppPage,
		type AppSummary,
		type AppTemplate,
		type AppVersion,
		type AppWidget,
		type WidgetBinding,
		type WidgetCatalogItem,
		type WidgetEvent,
	} from '$lib/api/apps';
	import { listDatasets, type Dataset } from '$lib/api/datasets';
	import { listObjectTypes, type ObjectType } from '$lib/api/ontology';
	import { notifications } from '$stores/notifications';
	import { cloneValue, createEmptyAppDraft, createPage, createWidgetFromCatalog } from '$lib/utils/apps';

	type BuilderState = {
		loading: boolean;
		saving: boolean;
		publishing: boolean;
		error: string;
	};

	let apps = $state<AppSummary[]>([]);
	let templates = $state<AppTemplate[]>([]);
	let widgetCatalog = $state<WidgetCatalogItem[]>([]);
	let datasets = $state<Dataset[]>([]);
	let objectTypes = $state<ObjectType[]>([]);
	let versions = $state<AppVersion[]>([]);
	let search = $state('');
	let selectedAppId = $state('');
	let selectedPageId = $state('');
	let selectedWidgetId = $state('');
	let publishNotes = $state('');
	let previewEmbed = $state('');
	let previewUrl = $state('');
	let draggedWidgetType = $state('');
	let draggedWidgetId = $state('');
	let draft = $state<AppDefinition>(createEmptyAppDraft());
	let builderState = $state<BuilderState>({
		loading: true,
		saving: false,
		publishing: false,
		error: '',
	});

	function currentPage() {
		return draft.pages.find((page) => page.id === selectedPageId) ?? draft.pages[0];
	}

	function selectedWidget() {
		return currentPage()?.widgets.find((widget) => widget.id === selectedWidgetId);
	}

	function normalizeWidgets(widgets: AppWidget[]) {
		return widgets.map((widget, index) => ({
			...widget,
			position: {
				...widget.position,
				y: index * 2,
			},
		}));
	}

	function syncSelection() {
		const page = currentPage();
		if (page && page.id !== selectedPageId) {
			selectedPageId = page.id;
		}

		if (!page) {
			selectedWidgetId = '';
			return;
		}

		if (!page.widgets.some((widget) => widget.id === selectedWidgetId)) {
			selectedWidgetId = page.widgets[0]?.id ?? '';
		}
	}

	function resetDraft(nextDraft: AppDefinition) {
		draft = cloneValue(nextDraft);
		selectedPageId = nextDraft.pages[0]?.id ?? '';
		selectedWidgetId = nextDraft.pages[0]?.widgets[0]?.id ?? '';
		publishNotes = '';
		syncSelection();
	}

	async function loadRegistry() {
		const [appResponse, templateResponse, catalogResponse, datasetResponse, typeResponse] = await Promise.all([
			listApps({ search: search || undefined, per_page: 50 }),
			listAppTemplates(),
			listWidgetCatalog(),
			listDatasets({ per_page: 100 }),
			listObjectTypes({ per_page: 100 }).catch(() => ({ data: [] as ObjectType[], total: 0, page: 1, per_page: 100 })),
		]);

		apps = appResponse.data;
		templates = templateResponse.data;
		widgetCatalog = catalogResponse;
		datasets = datasetResponse.data;
		objectTypes = typeResponse.data;
	}

	async function loadPreviewState(appId: string) {
		const [previewResponse, versionsResponse] = await Promise.all([
			previewApp(appId).catch(() => null),
			listAppVersions(appId).catch(() => ({ data: [] as AppVersion[] })),
		]);

		previewEmbed = previewResponse?.embed.iframe_html ?? '';
		previewUrl = previewResponse?.embed.url ?? '';
		versions = versionsResponse.data;
	}

	async function selectApp(id: string) {
		selectedAppId = id;
		const app = await getApp(id);
		resetDraft(app);
		await loadPreviewState(id);
	}

	async function load() {
		builderState.loading = true;
		builderState.error = '';
		try {
			await loadRegistry();
			if (selectedAppId) {
				await selectApp(selectedAppId);
			} else if (apps.length > 0) {
				await selectApp(apps[0].id);
			} else {
				newApp();
			}
		} catch (cause) {
			builderState.error = cause instanceof Error ? cause.message : 'Failed to load apps';
		} finally {
			builderState.loading = false;
		}
	}

	function newApp() {
		selectedAppId = '';
		previewEmbed = '';
		previewUrl = '';
		versions = [];
		resetDraft(createEmptyAppDraft());
	}

	function updatePages(updater: (pages: AppPage[]) => AppPage[]) {
		draft = {
			...draft,
			pages: updater(cloneValue(draft.pages)),
		};
		syncSelection();
	}

	function updateCurrentPage(updater: (page: AppPage) => AppPage) {
		updatePages((pages) => pages.map((page) => page.id === selectedPageId ? updater(page) : page));
	}

	function updateSelectedWidget(updater: (widget: AppWidget) => AppWidget) {
		updateCurrentPage((page) => ({
			...page,
			widgets: page.widgets.map((widget) => widget.id === selectedWidgetId ? updater(widget) : widget),
		}));
	}

	function ensureBinding(widget: AppWidget): WidgetBinding {
		return widget.binding ?? {
			source_type: 'static',
			source_id: null,
			query_text: null,
			path: null,
			fields: [],
			parameters: {},
			limit: 25,
		};
	}

	function savePayload() {
		return {
			name: draft.name,
			slug: draft.slug,
			description: draft.description,
			status: draft.status,
			pages: draft.pages.map((page) => ({
				...page,
				widgets: normalizeWidgets(page.widgets),
			})),
			theme: draft.theme,
			settings: draft.settings,
			template_key: draft.template_key ?? undefined,
		};
	}

	async function saveCurrentApp() {
		builderState.saving = true;
		builderState.error = '';
		try {
			const app = draft.id
				? await updateApp(draft.id, savePayload())
				: await createApp(savePayload());

			notifications.success(`App ${draft.id ? 'updated' : 'created'}`);
			await loadRegistry();
			await selectApp(app.id);
		} catch (cause) {
			builderState.error = cause instanceof Error ? cause.message : 'Failed to save app';
		} finally {
			builderState.saving = false;
		}
	}

	async function createFromTemplate(template: AppTemplate) {
		builderState.saving = true;
		builderState.error = '';
		try {
			const app = await createAppFromTemplate({
				name: `${template.name} ${apps.length + 1}`,
				description: template.description,
				status: 'draft',
				template_key: template.key,
			});

			notifications.success(`App created from ${template.name}`);
			await loadRegistry();
			await selectApp(app.id);
		} catch (cause) {
			builderState.error = cause instanceof Error ? cause.message : 'Failed to create from template';
		} finally {
			builderState.saving = false;
		}
	}

	async function publishCurrentApp() {
		builderState.publishing = true;
		builderState.error = '';
		try {
			if (!draft.id) {
				await saveCurrentApp();
			}

			if (!draft.id) {
				throw new Error('Save the app before publishing');
			}

			await publishApp(draft.id, publishNotes ? { notes: publishNotes } : {});
			notifications.success('App published');
			await selectApp(draft.id);
		} catch (cause) {
			builderState.error = cause instanceof Error ? cause.message : 'Failed to publish app';
		} finally {
			builderState.publishing = false;
		}
	}

	async function removeCurrentApp() {
		if (!draft.id || !confirm('Delete this app?')) {
			return;
		}

		await deleteApp(draft.id);
		notifications.success('App deleted');
		newApp();
		await loadRegistry();
		if (apps.length > 0) {
			await selectApp(apps[0].id);
		}
	}

	function addNewPage() {
		const page = createPage(`Page ${draft.pages.length + 1}`, `/page-${draft.pages.length + 1}`);
		updatePages((pages) => [...pages, page]);
		selectedPageId = page.id;
	}

	function removeSelectedPage() {
		if (draft.pages.length <= 1) {
			notifications.warning('Apps need at least one page');
			return;
		}

		const remaining = draft.pages.filter((page) => page.id !== selectedPageId);
		draft = {
			...draft,
			pages: remaining,
			settings: {
				...draft.settings,
				home_page_id: remaining[0]?.id ?? null,
			},
		};
		selectedPageId = remaining[0]?.id ?? '';
		selectedWidgetId = remaining[0]?.widgets[0]?.id ?? '';
	}

	function addWidget(widgetType: string) {
		const catalogItem = widgetCatalog.find((item) => item.widget_type === widgetType);
		if (!catalogItem) return;

		const widget = createWidgetFromCatalog(catalogItem);
		updateCurrentPage((page) => ({
			...page,
			widgets: normalizeWidgets([...page.widgets, widget]),
		}));
		selectedWidgetId = widget.id;
	}

	function insertWidgetBefore(widgetType: string, targetWidgetId: string) {
		const catalogItem = widgetCatalog.find((item) => item.widget_type === widgetType);
		if (!catalogItem) return;

		const widget = createWidgetFromCatalog(catalogItem);
		updateCurrentPage((page) => {
			const widgets = [...page.widgets];
			const targetIndex = widgets.findIndex((candidate) => candidate.id === targetWidgetId);
			widgets.splice(targetIndex >= 0 ? targetIndex : widgets.length, 0, widget);
			return { ...page, widgets: normalizeWidgets(widgets) };
		});
		selectedWidgetId = widget.id;
	}

	function removeSelectedWidget() {
		if (!selectedWidgetId) return;
		updateCurrentPage((page) => ({
			...page,
			widgets: normalizeWidgets(page.widgets.filter((widget) => widget.id !== selectedWidgetId)),
		}));
		selectedWidgetId = currentPage()?.widgets[0]?.id ?? '';
	}

	function reorderWidgets(sourceWidgetId: string, targetWidgetId: string) {
		if (sourceWidgetId === targetWidgetId) return;
		updateCurrentPage((page) => {
			const widgets = [...page.widgets];
			const sourceIndex = widgets.findIndex((widget) => widget.id === sourceWidgetId);
			const targetIndex = widgets.findIndex((widget) => widget.id === targetWidgetId);
			if (sourceIndex < 0 || targetIndex < 0) {
				return page;
			}
			const [moved] = widgets.splice(sourceIndex, 1);
			widgets.splice(targetIndex, 0, moved);
			return { ...page, widgets: normalizeWidgets(widgets) };
		});
		selectedWidgetId = sourceWidgetId;
	}

	function startPaletteDrag(widgetType: string) {
		draggedWidgetType = widgetType;
		draggedWidgetId = '';
	}

	function startWidgetDrag(widgetId: string) {
		draggedWidgetId = widgetId;
		draggedWidgetType = '';
	}

	function clearDragState() {
		draggedWidgetId = '';
		draggedWidgetType = '';
	}

	function handleCanvasDrop() {
		if (draggedWidgetType) {
			addWidget(draggedWidgetType);
		}
		clearDragState();
	}

	function handleWidgetDrop(targetWidgetId: string) {
		if (draggedWidgetType) {
			insertWidgetBefore(draggedWidgetType, targetWidgetId);
		} else if (draggedWidgetId) {
			reorderWidgets(draggedWidgetId, targetWidgetId);
		}
		clearDragState();
	}

	function updateSelectedWidgetBinding(updater: (binding: WidgetBinding) => WidgetBinding) {
		updateSelectedWidget((widget) => ({
			...widget,
			binding: updater(ensureBinding(widget)),
		}));
	}

	function updateSelectedWidgetProps(key: string, value: unknown) {
		updateSelectedWidget((widget) => ({
			...widget,
			props: {
				...widget.props,
				[key]: value,
			},
		}));
	}

	function addEvent() {
		updateSelectedWidget((widget) => ({
			...widget,
			events: [
				...widget.events,
				{
					id: crypto.randomUUID(),
					trigger: widget.widget_type === 'form' ? 'submit' : 'click',
					action: 'navigate',
					label: '',
					config: { path: '/' },
				},
			],
		}));
	}

	function updateEvent(eventId: string, key: keyof WidgetEvent, value: unknown) {
		updateSelectedWidget((widget) => ({
			...widget,
			events: widget.events.map((event) => event.id === eventId ? { ...event, [key]: value } : event),
		}));
	}

	function updateEventConfig(eventId: string, key: string, value: unknown) {
		updateSelectedWidget((widget) => ({
			...widget,
			events: widget.events.map((event) => event.id === eventId ? {
				...event,
				config: {
					...event.config,
					[key]: value,
				},
			} : event),
		}));
	}

	function removeEvent(eventId: string) {
		updateSelectedWidget((widget) => ({
			...widget,
			events: widget.events.filter((event) => event.id !== eventId),
		}));
	}

	function serializeFormFields(widget: AppWidget | undefined) {
		if (!widget || !Array.isArray(widget.props.fields)) return '';
		return widget.props.fields
			.map((field) => {
				if (!field || typeof field !== 'object') return '';
				const candidate = field as Record<string, unknown>;
				return [candidate.name, candidate.label, candidate.type, Array.isArray(candidate.options) ? candidate.options.join(',') : '']
					.map((value) => typeof value === 'string' ? value : '')
					.join('|');
			})
			.filter(Boolean)
			.join('\n');
	}

	function parseFormFields(text: string) {
		return text
			.split('\n')
			.map((line) => line.trim())
			.filter(Boolean)
			.map((line) => {
				const [name, label, type, options] = line.split('|').map((part) => part.trim());
				return {
					name: name || 'field',
					label: label || name || 'Field',
					type: type || 'text',
					options: options ? options.split(',').map((value) => value.trim()).filter(Boolean) : undefined,
				};
			});
	}

	function widgetBindingType(widget: AppWidget | undefined) {
		return widget?.binding?.source_type ?? 'static';
	}

	$effect(() => {
		draft.pages.length;
		syncSelection();
	});

	onMount(() => {
		void load();
	});
</script>

<svelte:head>
	<title>OpenFoundry — App Builder</title>
</svelte:head>

<div class="space-y-6">
	<div class="flex flex-wrap items-center justify-between gap-4">
		<div>
			<h1 class="text-3xl font-semibold tracking-tight text-slate-950 dark:text-slate-50">Workshop App Builder</h1>
			<p class="mt-2 max-w-3xl text-sm text-slate-500 dark:text-slate-400">
				Build internal apps with page layouts, widget catalog, dataset/query/ontology bindings, event handlers, theming, templates, and published runtime previews.
			</p>
		</div>

		<div class="flex flex-wrap gap-2">
			<button type="button" class="rounded-xl border border-slate-200 px-4 py-2 text-sm hover:bg-slate-50 dark:border-slate-700 dark:hover:bg-slate-800" onclick={newApp}>New app</button>
			<button type="button" class="rounded-xl bg-slate-900 px-4 py-2 text-sm text-white dark:bg-slate-100 dark:text-slate-950" disabled={builderState.saving} onclick={() => void saveCurrentApp()}>
				{builderState.saving ? 'Saving...' : draft.id ? 'Save changes' : 'Create app'}
			</button>
			<button type="button" class="rounded-xl bg-emerald-600 px-4 py-2 text-sm text-white disabled:opacity-50" disabled={builderState.publishing} onclick={() => void publishCurrentApp()}>
				{builderState.publishing ? 'Publishing...' : 'Publish app'}
			</button>
			{#if draft.slug}
				<a href={`/apps/runtime/${draft.slug}`} class="rounded-xl border border-slate-200 px-4 py-2 text-sm hover:bg-slate-50 dark:border-slate-700 dark:hover:bg-slate-800">Open runtime</a>
			{/if}
		</div>
	</div>

	{#if builderState.error}
		<div class="rounded-2xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-700 dark:border-rose-900/40 dark:bg-rose-950/30 dark:text-rose-300">{builderState.error}</div>
	{/if}

	<div class="grid gap-6 xl:grid-cols-[320px,1fr,360px]">
		<section class="space-y-5 rounded-[1.75rem] border border-slate-200 bg-white p-5 shadow-sm dark:border-slate-800 dark:bg-slate-950">
			<div>
				<div class="text-xs uppercase tracking-[0.24em] text-slate-400">App registry</div>
				<input
					type="text"
					value={search}
					oninput={(event) => { search = (event.currentTarget as HTMLInputElement).value; void loadRegistry(); }}
					placeholder="Search apps..."
					class="mt-3 w-full rounded-xl border border-slate-200 px-3 py-2 text-sm dark:border-slate-700 dark:bg-slate-900"
				/>
			</div>

			{#if builderState.loading}
				<div class="rounded-2xl border border-dashed border-slate-300 px-4 py-10 text-center text-sm text-slate-500 dark:border-slate-700">Loading apps...</div>
			{:else if apps.length === 0}
				<div class="rounded-2xl border border-dashed border-slate-300 px-4 py-10 text-center text-sm text-slate-500 dark:border-slate-700">Create the first app or start from a template.</div>
			{:else}
				<div class="space-y-3">
					{#each apps as app (app.id)}
						<button
							type="button"
							onclick={() => void selectApp(app.id)}
							class={`w-full rounded-2xl border p-4 text-left transition ${selectedAppId === app.id ? 'border-emerald-500 bg-emerald-50 dark:bg-emerald-950/20' : 'border-slate-200 hover:bg-slate-50 dark:border-slate-700 dark:hover:bg-slate-900'}`}
						>
							<div class="flex items-center justify-between gap-3">
								<div>
									<div class="font-medium text-slate-900 dark:text-slate-100">{app.name}</div>
									<div class="mt-1 text-sm text-slate-500">{app.description || 'No description'}</div>
								</div>
								<span class="rounded-full border border-slate-200 px-3 py-1 text-[11px] uppercase tracking-[0.18em] text-slate-500 dark:border-slate-700">{app.status}</span>
							</div>
							<div class="mt-3 flex flex-wrap gap-2 text-xs text-slate-400">
								<span>{app.page_count} pages</span>
								<span>{app.widget_count} widgets</span>
								{#if app.template_key}<span>{app.template_key}</span>{/if}
							</div>
						</button>
					{/each}
				</div>
			{/if}

			<div>
				<div class="mb-3 text-xs uppercase tracking-[0.24em] text-slate-400">Templates</div>
				<div class="space-y-3">
					{#each templates as template (template.id)}
						<div class="rounded-2xl border border-slate-200 p-4 dark:border-slate-700">
							<div class="flex items-start justify-between gap-3">
								<div>
									<div class="font-medium text-slate-900 dark:text-slate-100">{template.name}</div>
									<div class="mt-1 text-sm text-slate-500">{template.description}</div>
								</div>
								<span class="rounded-full bg-slate-100 px-2 py-1 text-[11px] uppercase tracking-[0.18em] text-slate-500 dark:bg-slate-900">{template.category}</span>
							</div>
							<button type="button" class="mt-3 rounded-xl border border-slate-200 px-3 py-2 text-sm hover:bg-slate-50 dark:border-slate-700 dark:hover:bg-slate-900" onclick={() => void createFromTemplate(template)}>
								Use template
							</button>
						</div>
					{/each}
				</div>
			</div>

			<div>
				<div class="mb-3 text-xs uppercase tracking-[0.24em] text-slate-400">Widget palette</div>
				<div class="grid gap-2 sm:grid-cols-2">
					{#each widgetCatalog as item (item.widget_type)}
						<button
							type="button"
							draggable="true"
							ondragstart={() => startPaletteDrag(item.widget_type)}
							onclick={() => addWidget(item.widget_type)}
							class="rounded-2xl border border-slate-200 px-3 py-3 text-left text-sm hover:bg-slate-50 dark:border-slate-700 dark:hover:bg-slate-900"
						>
							<div class="font-medium text-slate-900 dark:text-slate-100">{item.label}</div>
							<div class="mt-1 text-xs text-slate-500">{item.description}</div>
						</button>
					{/each}
				</div>
				<p class="mt-3 text-xs text-slate-400">Drag a widget onto the canvas or click to append it to the current page.</p>
			</div>
		</section>

		<section class="space-y-5 rounded-[1.75rem] border border-slate-200 bg-white p-5 shadow-sm dark:border-slate-800 dark:bg-slate-950">
			<div class="grid gap-4 lg:grid-cols-[1fr,220px]">
				<div class="space-y-4">
					<input
						type="text"
						value={draft.name}
						oninput={(event) => draft = { ...draft, name: (event.currentTarget as HTMLInputElement).value }}
						class="w-full rounded-2xl border border-slate-200 px-4 py-3 text-2xl font-semibold dark:border-slate-700 dark:bg-slate-900"
					/>
					<textarea
						rows="3"
						value={draft.description}
						oninput={(event) => draft = { ...draft, description: (event.currentTarget as HTMLTextAreaElement).value }}
						class="w-full rounded-2xl border border-slate-200 px-4 py-3 text-sm dark:border-slate-700 dark:bg-slate-900"
					></textarea>
				</div>

				<div class="space-y-3 rounded-2xl border border-slate-200 bg-slate-50 p-4 dark:border-slate-700 dark:bg-slate-900">
					<div>
						<label for="app-slug" class="mb-1 block text-xs uppercase tracking-[0.22em] text-slate-400">Slug</label>
						<input id="app-slug" type="text" value={draft.slug} oninput={(event) => draft = { ...draft, slug: (event.currentTarget as HTMLInputElement).value }} class="w-full rounded-xl border border-slate-200 px-3 py-2 text-sm dark:border-slate-700 dark:bg-slate-950" />
					</div>
					<div>
						<label for="app-status" class="mb-1 block text-xs uppercase tracking-[0.22em] text-slate-400">Status</label>
						<select id="app-status" value={draft.status} oninput={(event) => draft = { ...draft, status: (event.currentTarget as HTMLSelectElement).value }} class="w-full rounded-xl border border-slate-200 px-3 py-2 text-sm dark:border-slate-700 dark:bg-slate-950">
							<option value="draft">Draft</option>
							<option value="published">Published</option>
							<option value="archived">Archived</option>
						</select>
					</div>
					<div>
						<label for="app-publish-notes" class="mb-1 block text-xs uppercase tracking-[0.22em] text-slate-400">Publish notes</label>
						<textarea id="app-publish-notes" rows="2" value={publishNotes} oninput={(event) => publishNotes = (event.currentTarget as HTMLTextAreaElement).value} class="w-full rounded-xl border border-slate-200 px-3 py-2 text-sm dark:border-slate-700 dark:bg-slate-950"></textarea>
					</div>
				</div>
			</div>

			<div class="rounded-[1.5rem] border border-slate-200 p-4 dark:border-slate-800">
				<div class="flex flex-wrap items-center justify-between gap-3">
					<div>
						<div class="text-xs uppercase tracking-[0.22em] text-slate-400">Pages</div>
						<div class="mt-1 text-sm text-slate-500">Select a page, then drag widgets into its canvas.</div>
					</div>
					<div class="flex gap-2">
						<button type="button" class="rounded-xl border border-slate-200 px-3 py-2 text-sm hover:bg-slate-50 dark:border-slate-700 dark:hover:bg-slate-900" onclick={addNewPage}>Add page</button>
						<button type="button" class="rounded-xl border border-rose-200 px-3 py-2 text-sm text-rose-600 hover:bg-rose-50 dark:border-rose-900/40 dark:hover:bg-rose-950/20" onclick={removeSelectedPage}>Remove page</button>
					</div>
				</div>

				<div class="mt-4 flex flex-wrap gap-2">
					{#each draft.pages as page (page.id)}
						<button type="button" class={`rounded-full px-4 py-2 text-sm ${selectedPageId === page.id ? 'bg-emerald-600 text-white' : 'border border-slate-200 hover:bg-slate-50 dark:border-slate-700 dark:hover:bg-slate-900'}`} onclick={() => selectedPageId = page.id}>
							{page.name}
						</button>
					{/each}
				</div>

				<div
					class="mt-4 min-h-[280px] rounded-[1.5rem] border border-dashed border-slate-300 bg-slate-50/80 p-4 dark:border-slate-700 dark:bg-slate-900/40"
					role="region"
					aria-label="Widget canvas"
					ondragover={(event) => event.preventDefault()}
					ondrop={handleCanvasDrop}
				>
					{#if currentPage()?.widgets.length}
						<div class="grid gap-3">
							{#each currentPage()?.widgets ?? [] as widget (widget.id)}
								<button
									type="button"
									draggable="true"
									ondragstart={() => startWidgetDrag(widget.id)}
									ondragover={(event) => event.preventDefault()}
									ondrop={() => handleWidgetDrop(widget.id)}
									onclick={() => selectedWidgetId = widget.id}
									class={`rounded-2xl border p-4 text-left transition ${selectedWidgetId === widget.id ? 'border-emerald-500 bg-white shadow-sm dark:bg-slate-950' : 'border-slate-200 bg-white hover:bg-slate-50 dark:border-slate-700 dark:bg-slate-950 dark:hover:bg-slate-900'}`}
								>
									<div class="flex flex-wrap items-center justify-between gap-3">
										<div>
											<div class="flex items-center gap-2">
												<span class="rounded-full bg-slate-100 px-2 py-0.5 text-[11px] uppercase tracking-[0.18em] text-slate-500 dark:bg-slate-900">{widget.widget_type}</span>
												<span class="font-medium text-slate-900 dark:text-slate-100">{widget.title}</span>
											</div>
											<div class="mt-1 text-sm text-slate-500">{widget.description || 'No description'}</div>
										</div>
										<div class="text-xs text-slate-400">
											{`x: ${widget.position.x}, y: ${widget.position.y}, w: ${widget.position.width}, h: ${widget.position.height}`}
										</div>
									</div>
								</button>
							{/each}
						</div>
					{:else}
						<div class="flex min-h-[220px] items-center justify-center text-sm text-slate-500 dark:text-slate-400">Drop widgets here to start designing this page.</div>
					{/if}
				</div>
			</div>

			<div class="space-y-4 rounded-[1.5rem] border border-slate-200 p-4 dark:border-slate-800">
				<div class="flex items-center justify-between gap-3">
					<div>
						<div class="text-xs uppercase tracking-[0.22em] text-slate-400">Live preview</div>
						<div class="mt-1 text-sm text-slate-500">WYSIWYG runtime preview using the current draft.</div>
					</div>
					{#if previewUrl}
						<a href={previewUrl} class="text-sm text-emerald-600 hover:underline">Embed URL</a>
					{/if}
				</div>

				<AppRenderer app={draft} mode="builder" />

				{#if previewEmbed}
					<div class="rounded-2xl border border-slate-200 bg-slate-50 p-4 text-xs text-slate-500 dark:border-slate-700 dark:bg-slate-900">
						<div class="mb-2 font-semibold uppercase tracking-[0.2em] text-slate-400">Embed snippet</div>
						<pre class="overflow-auto whitespace-pre-wrap">{previewEmbed}</pre>
					</div>
				{/if}
			</div>
		</section>

		<section class="space-y-5 rounded-[1.75rem] border border-slate-200 bg-white p-5 shadow-sm dark:border-slate-800 dark:bg-slate-950">
			<div class="rounded-2xl border border-slate-200 p-4 dark:border-slate-700">
				<div class="text-xs uppercase tracking-[0.22em] text-slate-400">App theming</div>
				<div class="mt-4 grid gap-3 sm:grid-cols-2">
					<label class="text-sm">
						<span class="mb-1 block text-slate-500">Theme name</span>
						<input type="text" value={draft.theme.name} oninput={(event) => draft = { ...draft, theme: { ...draft.theme, name: (event.currentTarget as HTMLInputElement).value } }} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-slate-700 dark:bg-slate-900" />
					</label>
					<label class="text-sm">
						<span class="mb-1 block text-slate-500">Logo URL</span>
						<input type="text" value={draft.theme.logo_url ?? ''} oninput={(event) => draft = { ...draft, theme: { ...draft.theme, logo_url: (event.currentTarget as HTMLInputElement).value || null } }} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-slate-700 dark:bg-slate-900" />
					</label>
					<label class="text-sm">
						<span class="mb-1 block text-slate-500">Primary color</span>
						<input type="color" value={draft.theme.primary_color} oninput={(event) => draft = { ...draft, theme: { ...draft.theme, primary_color: (event.currentTarget as HTMLInputElement).value } }} class="h-10 w-full rounded-xl border border-slate-200 px-1 py-1 dark:border-slate-700 dark:bg-slate-900" />
					</label>
					<label class="text-sm">
						<span class="mb-1 block text-slate-500">Accent color</span>
						<input type="color" value={draft.theme.accent_color} oninput={(event) => draft = { ...draft, theme: { ...draft.theme, accent_color: (event.currentTarget as HTMLInputElement).value } }} class="h-10 w-full rounded-xl border border-slate-200 px-1 py-1 dark:border-slate-700 dark:bg-slate-900" />
					</label>
					<label class="text-sm">
						<span class="mb-1 block text-slate-500">Background color</span>
						<input type="color" value={draft.theme.background_color} oninput={(event) => draft = { ...draft, theme: { ...draft.theme, background_color: (event.currentTarget as HTMLInputElement).value } }} class="h-10 w-full rounded-xl border border-slate-200 px-1 py-1 dark:border-slate-700 dark:bg-slate-900" />
					</label>
					<label class="text-sm">
						<span class="mb-1 block text-slate-500">Surface color</span>
						<input type="color" value={draft.theme.surface_color} oninput={(event) => draft = { ...draft, theme: { ...draft.theme, surface_color: (event.currentTarget as HTMLInputElement).value } }} class="h-10 w-full rounded-xl border border-slate-200 px-1 py-1 dark:border-slate-700 dark:bg-slate-900" />
					</label>
					<label class="text-sm">
						<span class="mb-1 block text-slate-500">Text color</span>
						<input type="color" value={draft.theme.text_color} oninput={(event) => draft = { ...draft, theme: { ...draft.theme, text_color: (event.currentTarget as HTMLInputElement).value } }} class="h-10 w-full rounded-xl border border-slate-200 px-1 py-1 dark:border-slate-700 dark:bg-slate-900" />
					</label>
					<label class="text-sm">
						<span class="mb-1 block text-slate-500">Heading font</span>
						<input type="text" value={draft.theme.heading_font} oninput={(event) => draft = { ...draft, theme: { ...draft.theme, heading_font: (event.currentTarget as HTMLInputElement).value } }} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-slate-700 dark:bg-slate-900" />
					</label>
					<label class="text-sm">
						<span class="mb-1 block text-slate-500">Body font</span>
						<input type="text" value={draft.theme.body_font} oninput={(event) => draft = { ...draft, theme: { ...draft.theme, body_font: (event.currentTarget as HTMLInputElement).value } }} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-slate-700 dark:bg-slate-900" />
					</label>
					<label class="text-sm">
						<span class="mb-1 block text-slate-500">Radius</span>
						<input type="number" min="0" value={draft.theme.border_radius} oninput={(event) => draft = { ...draft, theme: { ...draft.theme, border_radius: Number((event.currentTarget as HTMLInputElement).value) || 0 } }} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-slate-700 dark:bg-slate-900" />
					</label>
				</div>
			</div>

			<div class="rounded-2xl border border-slate-200 p-4 dark:border-slate-700">
				<div class="text-xs uppercase tracking-[0.22em] text-slate-400">Page settings</div>
				{#if currentPage()}
					<div class="mt-4 space-y-3">
						<label class="text-sm">
							<span class="mb-1 block text-slate-500">Page name</span>
							<input type="text" value={currentPage()?.name ?? ''} oninput={(event) => updateCurrentPage((page) => ({ ...page, name: (event.currentTarget as HTMLInputElement).value }))} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-slate-700 dark:bg-slate-900" />
						</label>
						<label class="text-sm">
							<span class="mb-1 block text-slate-500">Page path</span>
							<input type="text" value={currentPage()?.path ?? ''} oninput={(event) => updateCurrentPage((page) => ({ ...page, path: (event.currentTarget as HTMLInputElement).value }))} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-slate-700 dark:bg-slate-900" />
						</label>
						<label class="text-sm">
							<span class="mb-1 block text-slate-500">Description</span>
							<textarea rows="3" value={currentPage()?.description ?? ''} oninput={(event) => updateCurrentPage((page) => ({ ...page, description: (event.currentTarget as HTMLTextAreaElement).value }))} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-slate-700 dark:bg-slate-900"></textarea>
						</label>
						<div class="grid gap-3 sm:grid-cols-2">
							<label class="text-sm">
								<span class="mb-1 block text-slate-500">Navigation style</span>
								<select value={draft.settings.navigation_style} oninput={(event) => draft = { ...draft, settings: { ...draft.settings, navigation_style: (event.currentTarget as HTMLSelectElement).value } }} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-slate-700 dark:bg-slate-900">
									<option value="tabs">Tabs</option>
									<option value="sidebar">Sidebar</option>
								</select>
							</label>
							<label class="flex items-center justify-between rounded-xl border border-slate-200 px-3 py-2 text-sm dark:border-slate-700">
								<span class="text-slate-500">Show branding</span>
								<input type="checkbox" checked={draft.settings.show_branding} oninput={(event) => draft = { ...draft, settings: { ...draft.settings, show_branding: (event.currentTarget as HTMLInputElement).checked } }} />
							</label>
						</div>
					</div>
				{/if}
			</div>

			<div class="rounded-2xl border border-slate-200 p-4 dark:border-slate-700">
				<div class="mb-3 flex items-center justify-between gap-3">
					<div class="text-xs uppercase tracking-[0.22em] text-slate-400">Property inspector</div>
					<button type="button" class="rounded-xl border border-rose-200 px-3 py-2 text-xs text-rose-600 hover:bg-rose-50 dark:border-rose-900/40 dark:hover:bg-rose-950/20" onclick={removeSelectedWidget} disabled={!selectedWidgetId}>Remove widget</button>
				</div>

				{#if selectedWidget()}
					<div class="space-y-3">
						<label class="text-sm">
							<span class="mb-1 block text-slate-500">Title</span>
							<input type="text" value={selectedWidget()?.title ?? ''} oninput={(event) => updateSelectedWidget((widget) => ({ ...widget, title: (event.currentTarget as HTMLInputElement).value }))} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-slate-700 dark:bg-slate-900" />
						</label>
						<label class="text-sm">
							<span class="mb-1 block text-slate-500">Description</span>
							<textarea rows="2" value={selectedWidget()?.description ?? ''} oninput={(event) => updateSelectedWidget((widget) => ({ ...widget, description: (event.currentTarget as HTMLTextAreaElement).value }))} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-slate-700 dark:bg-slate-900"></textarea>
						</label>

						<div class="grid gap-3 sm:grid-cols-2">
							<label class="text-sm">
								<span class="mb-1 block text-slate-500">X</span>
								<input type="number" value={selectedWidget()?.position.x ?? 0} oninput={(event) => updateSelectedWidget((widget) => ({ ...widget, position: { ...widget.position, x: Number((event.currentTarget as HTMLInputElement).value) || 0 } }))} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-slate-700 dark:bg-slate-900" />
							</label>
							<label class="text-sm">
								<span class="mb-1 block text-slate-500">Width</span>
								<input type="number" min="1" max="12" value={selectedWidget()?.position.width ?? 4} oninput={(event) => updateSelectedWidget((widget) => ({ ...widget, position: { ...widget.position, width: Number((event.currentTarget as HTMLInputElement).value) || 1 } }))} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-slate-700 dark:bg-slate-900" />
							</label>
							<label class="text-sm">
								<span class="mb-1 block text-slate-500">Y</span>
								<input type="number" value={selectedWidget()?.position.y ?? 0} oninput={(event) => updateSelectedWidget((widget) => ({ ...widget, position: { ...widget.position, y: Number((event.currentTarget as HTMLInputElement).value) || 0 } }))} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-slate-700 dark:bg-slate-900" />
							</label>
							<label class="text-sm">
								<span class="mb-1 block text-slate-500">Height</span>
								<input type="number" min="1" value={selectedWidget()?.position.height ?? 3} oninput={(event) => updateSelectedWidget((widget) => ({ ...widget, position: { ...widget.position, height: Number((event.currentTarget as HTMLInputElement).value) || 1 } }))} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-slate-700 dark:bg-slate-900" />
							</label>
						</div>

						<div class="rounded-2xl border border-slate-200 p-3 dark:border-slate-700">
							<div class="text-xs uppercase tracking-[0.22em] text-slate-400">Binding</div>
							<div class="mt-3 space-y-3">
								<label class="text-sm">
									<span class="mb-1 block text-slate-500">Source type</span>
									<select value={widgetBindingType(selectedWidget())} oninput={(event) => updateSelectedWidgetBinding((binding) => ({ ...binding, source_type: (event.currentTarget as HTMLSelectElement).value }))} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-slate-700 dark:bg-slate-900">
										<option value="static">Static</option>
										<option value="dataset">Dataset</option>
										<option value="query">Query</option>
										<option value="ontology">Ontology</option>
									</select>
								</label>

								{#if widgetBindingType(selectedWidget()) === 'dataset'}
									<label class="text-sm">
										<span class="mb-1 block text-slate-500">Dataset</span>
										<select value={selectedWidget()?.binding?.source_id ?? ''} oninput={(event) => updateSelectedWidgetBinding((binding) => ({ ...binding, source_id: (event.currentTarget as HTMLSelectElement).value }))} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-slate-700 dark:bg-slate-900">
											<option value="">Select dataset</option>
											{#each datasets as dataset (dataset.id)}
												<option value={dataset.id}>{dataset.name}</option>
											{/each}
										</select>
									</label>
								{:else if widgetBindingType(selectedWidget()) === 'ontology'}
									<label class="text-sm">
										<span class="mb-1 block text-slate-500">Object type</span>
										<select value={selectedWidget()?.binding?.source_id ?? ''} oninput={(event) => updateSelectedWidgetBinding((binding) => ({ ...binding, source_id: (event.currentTarget as HTMLSelectElement).value }))} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-slate-700 dark:bg-slate-900">
											<option value="">Select object type</option>
											{#each objectTypes as objectType (objectType.id)}
												<option value={objectType.id}>{objectType.display_name}</option>
											{/each}
										</select>
									</label>
								{:else if widgetBindingType(selectedWidget()) === 'query'}
									<label class="text-sm">
										<span class="mb-1 block text-slate-500">SQL query</span>
										<textarea rows="5" value={selectedWidget()?.binding?.query_text ?? ''} oninput={(event) => updateSelectedWidgetBinding((binding) => ({ ...binding, query_text: (event.currentTarget as HTMLTextAreaElement).value }))} class="w-full rounded-xl border border-slate-200 px-3 py-2 font-mono text-xs dark:border-slate-700 dark:bg-slate-900"></textarea>
									</label>
								{/if}

								<label class="text-sm">
									<span class="mb-1 block text-slate-500">Limit</span>
									<input type="number" min="1" value={selectedWidget()?.binding?.limit ?? 25} oninput={(event) => updateSelectedWidgetBinding((binding) => ({ ...binding, limit: Number((event.currentTarget as HTMLInputElement).value) || 25 }))} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-slate-700 dark:bg-slate-900" />
								</label>
							</div>
						</div>

						{#if selectedWidget()?.widget_type === 'text'}
							<label class="text-sm">
								<span class="mb-1 block text-slate-500">Content</span>
								<textarea rows="6" value={String(selectedWidget()?.props.content ?? '')} oninput={(event) => updateSelectedWidgetProps('content', (event.currentTarget as HTMLTextAreaElement).value)} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-slate-700 dark:bg-slate-900"></textarea>
							</label>
						{:else if selectedWidget()?.widget_type === 'image'}
							<div class="grid gap-3 sm:grid-cols-2">
								<label class="text-sm">
									<span class="mb-1 block text-slate-500">Image URL</span>
									<input type="text" value={String(selectedWidget()?.props.url ?? '')} oninput={(event) => updateSelectedWidgetProps('url', (event.currentTarget as HTMLInputElement).value)} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-slate-700 dark:bg-slate-900" />
								</label>
								<label class="text-sm">
									<span class="mb-1 block text-slate-500">Alt text</span>
									<input type="text" value={String(selectedWidget()?.props.alt ?? '')} oninput={(event) => updateSelectedWidgetProps('alt', (event.currentTarget as HTMLInputElement).value)} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-slate-700 dark:bg-slate-900" />
								</label>
							</div>
						{:else if selectedWidget()?.widget_type === 'button'}
							<div class="grid gap-3 sm:grid-cols-2">
								<label class="text-sm">
									<span class="mb-1 block text-slate-500">Button label</span>
									<input type="text" value={String(selectedWidget()?.props.label ?? '')} oninput={(event) => updateSelectedWidgetProps('label', (event.currentTarget as HTMLInputElement).value)} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-slate-700 dark:bg-slate-900" />
								</label>
								<label class="text-sm">
									<span class="mb-1 block text-slate-500">Variant</span>
									<select value={String(selectedWidget()?.props.variant ?? 'primary')} oninput={(event) => updateSelectedWidgetProps('variant', (event.currentTarget as HTMLSelectElement).value)} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-slate-700 dark:bg-slate-900">
										<option value="primary">Primary</option>
										<option value="secondary">Secondary</option>
									</select>
								</label>
							</div>
						{:else if selectedWidget()?.widget_type === 'chart'}
							<div class="grid gap-3 sm:grid-cols-2">
								<label class="text-sm">
									<span class="mb-1 block text-slate-500">Chart type</span>
									<select value={String(selectedWidget()?.props.chart_type ?? 'line')} oninput={(event) => updateSelectedWidgetProps('chart_type', (event.currentTarget as HTMLSelectElement).value)} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-slate-700 dark:bg-slate-900">
										<option value="line">Line</option>
										<option value="bar">Bar</option>
										<option value="area">Area</option>
										<option value="pie">Pie</option>
										<option value="scatter">Scatter</option>
									</select>
								</label>
								<label class="text-sm">
									<span class="mb-1 block text-slate-500">X field</span>
									<input type="text" value={String(selectedWidget()?.props.x_field ?? '')} oninput={(event) => updateSelectedWidgetProps('x_field', (event.currentTarget as HTMLInputElement).value)} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-slate-700 dark:bg-slate-900" />
								</label>
								<label class="text-sm">
									<span class="mb-1 block text-slate-500">Y field</span>
									<input type="text" value={String(selectedWidget()?.props.y_field ?? '')} oninput={(event) => updateSelectedWidgetProps('y_field', (event.currentTarget as HTMLInputElement).value)} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-slate-700 dark:bg-slate-900" />
								</label>
								<label class="text-sm">
									<span class="mb-1 block text-slate-500">Series fields</span>
									<input type="text" value={Array.isArray(selectedWidget()?.props.series_fields) ? (selectedWidget()?.props.series_fields as string[]).join(', ') : ''} oninput={(event) => updateSelectedWidgetProps('series_fields', (event.currentTarget as HTMLInputElement).value.split(',').map((value) => value.trim()).filter(Boolean))} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-slate-700 dark:bg-slate-900" />
								</label>
							</div>
						{:else if selectedWidget()?.widget_type === 'map'}
							<div class="grid gap-3 sm:grid-cols-3">
								<label class="text-sm">
									<span class="mb-1 block text-slate-500">Latitude field</span>
									<input type="text" value={String(selectedWidget()?.props.latitude_field ?? 'lat')} oninput={(event) => updateSelectedWidgetProps('latitude_field', (event.currentTarget as HTMLInputElement).value)} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-slate-700 dark:bg-slate-900" />
								</label>
								<label class="text-sm">
									<span class="mb-1 block text-slate-500">Longitude field</span>
									<input type="text" value={String(selectedWidget()?.props.longitude_field ?? 'lon')} oninput={(event) => updateSelectedWidgetProps('longitude_field', (event.currentTarget as HTMLInputElement).value)} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-slate-700 dark:bg-slate-900" />
								</label>
								<label class="text-sm">
									<span class="mb-1 block text-slate-500">Label field</span>
									<input type="text" value={String(selectedWidget()?.props.label_field ?? '')} oninput={(event) => updateSelectedWidgetProps('label_field', (event.currentTarget as HTMLInputElement).value)} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-slate-700 dark:bg-slate-900" />
								</label>
							</div>
						{:else if selectedWidget()?.widget_type === 'table'}
							<div class="grid gap-3 sm:grid-cols-3">
								<label class="text-sm">
									<span class="mb-1 block text-slate-500">Page size</span>
									<input type="number" min="1" value={Number(selectedWidget()?.props.page_size ?? 10)} oninput={(event) => updateSelectedWidgetProps('page_size', Number((event.currentTarget as HTMLInputElement).value) || 10)} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-slate-700 dark:bg-slate-900" />
								</label>
								<label class="text-sm">
									<span class="mb-1 block text-slate-500">Default sort</span>
									<input type="text" value={String(selectedWidget()?.props.default_sort_column ?? '')} oninput={(event) => updateSelectedWidgetProps('default_sort_column', (event.currentTarget as HTMLInputElement).value)} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-slate-700 dark:bg-slate-900" />
								</label>
								<label class="text-sm">
									<span class="mb-1 block text-slate-500">Direction</span>
									<select value={String(selectedWidget()?.props.default_sort_direction ?? 'asc')} oninput={(event) => updateSelectedWidgetProps('default_sort_direction', (event.currentTarget as HTMLSelectElement).value)} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-slate-700 dark:bg-slate-900">
										<option value="asc">Ascending</option>
										<option value="desc">Descending</option>
									</select>
								</label>
							</div>
						{:else if selectedWidget()?.widget_type === 'form'}
							<label class="text-sm">
								<span class="mb-1 block text-slate-500">Fields</span>
								<textarea rows="5" value={serializeFormFields(selectedWidget())} oninput={(event) => updateSelectedWidgetProps('fields', parseFormFields((event.currentTarget as HTMLTextAreaElement).value))} class="w-full rounded-xl border border-slate-200 px-3 py-2 font-mono text-xs dark:border-slate-700 dark:bg-slate-900"></textarea>
								<span class="mt-1 block text-xs text-slate-400">Format: `name|Label|type|option1,option2`</span>
							</label>
						{:else if selectedWidget()?.widget_type === 'container'}
							<label class="text-sm">
								<span class="mb-1 block text-slate-500">Container title</span>
								<input type="text" value={String(selectedWidget()?.props.title ?? '')} oninput={(event) => updateSelectedWidgetProps('title', (event.currentTarget as HTMLInputElement).value)} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-slate-700 dark:bg-slate-900" />
							</label>
						{/if}

						<div class="rounded-2xl border border-slate-200 p-3 dark:border-slate-700">
							<div class="mb-3 flex items-center justify-between gap-3">
								<div class="text-xs uppercase tracking-[0.22em] text-slate-400">Event handlers</div>
								<button type="button" class="rounded-xl border border-slate-200 px-3 py-2 text-xs hover:bg-slate-50 dark:border-slate-700 dark:hover:bg-slate-900" onclick={addEvent}>Add event</button>
							</div>

							{#if selectedWidget()?.events.length}
								<div class="space-y-3">
									{#each selectedWidget()?.events ?? [] as event (event.id)}
										<div class="rounded-2xl border border-slate-200 p-3 dark:border-slate-700">
											<div class="grid gap-3 sm:grid-cols-2">
												<label class="text-sm">
													<span class="mb-1 block text-slate-500">Trigger</span>
													<select value={event.trigger} oninput={(e) => updateEvent(event.id, 'trigger', (e.currentTarget as HTMLSelectElement).value)} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-slate-700 dark:bg-slate-900">
														<option value="click">Click</option>
														<option value="submit">Submit</option>
														<option value="change">Change</option>
													</select>
												</label>
												<label class="text-sm">
													<span class="mb-1 block text-slate-500">Action</span>
													<select value={event.action} oninput={(e) => updateEvent(event.id, 'action', (e.currentTarget as HTMLSelectElement).value)} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-slate-700 dark:bg-slate-900">
														<option value="navigate">Navigate</option>
														<option value="filter">Filter</option>
														<option value="open_link">Open link</option>
														<option value="execute_query">Execute query</option>
													</select>
												</label>
												<label class="text-sm sm:col-span-2">
													<span class="mb-1 block text-slate-500">Label</span>
													<input type="text" value={event.label ?? ''} oninput={(e) => updateEvent(event.id, 'label', (e.currentTarget as HTMLInputElement).value)} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-slate-700 dark:bg-slate-900" />
												</label>

												{#if event.action === 'navigate'}
													<label class="text-sm sm:col-span-2">
														<span class="mb-1 block text-slate-500">Target path or page id</span>
														<input type="text" value={String(event.config.path ?? event.config.page_id ?? '')} oninput={(e) => updateEventConfig(event.id, 'path', (e.currentTarget as HTMLInputElement).value)} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-slate-700 dark:bg-slate-900" />
													</label>
												{:else if event.action === 'filter'}
													<label class="text-sm">
														<span class="mb-1 block text-slate-500">Value</span>
														<input type="text" value={String(event.config.value ?? '')} oninput={(e) => updateEventConfig(event.id, 'value', (e.currentTarget as HTMLInputElement).value)} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-slate-700 dark:bg-slate-900" />
													</label>
													<label class="text-sm">
														<span class="mb-1 block text-slate-500">Payload field</span>
														<input type="text" value={String(event.config.field ?? '')} oninput={(e) => updateEventConfig(event.id, 'field', (e.currentTarget as HTMLInputElement).value)} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-slate-700 dark:bg-slate-900" />
													</label>
												{:else if event.action === 'open_link'}
													<label class="text-sm sm:col-span-2">
														<span class="mb-1 block text-slate-500">URL</span>
														<input type="text" value={String(event.config.url ?? '')} oninput={(e) => updateEventConfig(event.id, 'url', (e.currentTarget as HTMLInputElement).value)} class="w-full rounded-xl border border-slate-200 px-3 py-2 dark:border-slate-700 dark:bg-slate-900" />
													</label>
												{:else if event.action === 'execute_query'}
													<label class="text-sm sm:col-span-2">
														<span class="mb-1 block text-slate-500">SQL</span>
														<textarea rows="4" value={String(event.config.sql ?? '')} oninput={(e) => updateEventConfig(event.id, 'sql', (e.currentTarget as HTMLTextAreaElement).value)} class="w-full rounded-xl border border-slate-200 px-3 py-2 font-mono text-xs dark:border-slate-700 dark:bg-slate-900"></textarea>
													</label>
												{/if}
											</div>

											<div class="mt-3 flex justify-end">
												<button type="button" class="rounded-xl border border-rose-200 px-3 py-2 text-xs text-rose-600 hover:bg-rose-50 dark:border-rose-900/40 dark:hover:bg-rose-950/20" onclick={() => removeEvent(event.id)}>Remove</button>
											</div>
										</div>
									{/each}
								</div>
							{:else}
								<div class="text-sm text-slate-500">No event handlers configured yet.</div>
							{/if}
						</div>
					</div>
				{:else}
					<div class="text-sm text-slate-500">Select a widget from the canvas to edit its properties, bindings, and events.</div>
				{/if}
			</div>

			<div class="rounded-2xl border border-slate-200 p-4 dark:border-slate-700">
				<div class="text-xs uppercase tracking-[0.22em] text-slate-400">Publish history</div>
				<div class="mt-3 space-y-3">
					{#if versions.length === 0}
						<div class="text-sm text-slate-500">No published versions yet.</div>
					{:else}
						{#each versions as version (version.id)}
							<div class="rounded-2xl border border-slate-200 p-3 text-sm dark:border-slate-700">
								<div class="flex items-center justify-between gap-3">
									<div class="font-medium text-slate-900 dark:text-slate-100">v{version.version_number}</div>
									<span class="rounded-full bg-slate-100 px-2 py-1 text-[11px] uppercase tracking-[0.18em] text-slate-500 dark:bg-slate-900">{version.status}</span>
								</div>
								<div class="mt-2 text-slate-500">{version.notes || 'No notes'}</div>
								<div class="mt-2 text-xs text-slate-400">{version.published_at ?? version.created_at}</div>
							</div>
						{/each}
					{/if}
				</div>
			</div>

			<div class="flex gap-2">
				<button type="button" class="flex-1 rounded-xl border border-rose-200 px-4 py-2 text-sm text-rose-600 hover:bg-rose-50 dark:border-rose-900/40 dark:hover:bg-rose-950/20" onclick={() => void removeCurrentApp()} disabled={!draft.id}>Delete app</button>
			</div>
		</section>
	</div>
</div>