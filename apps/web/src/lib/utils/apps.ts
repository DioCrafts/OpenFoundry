import type {
	AppDefinition,
	AppPage,
	AppSettings,
	AppTheme,
	AppWidget,
	SlatePackageFile,
	WidgetCatalogItem,
} from '$lib/api/apps';

function createId() {
  if (typeof crypto !== 'undefined' && 'randomUUID' in crypto) {
    return crypto.randomUUID();
  }

  return `app_${Date.now()}_${Math.floor(Math.random() * 10000)}`;
}

export function cloneValue<T>(value: T): T {
  if (typeof structuredClone === 'function') {
    return structuredClone(value);
  }

  return JSON.parse(JSON.stringify(value)) as T;
}

export function createDefaultTheme(): AppTheme {
  return {
    name: 'Signal',
    primary_color: '#0f766e',
    accent_color: '#f97316',
    background_color: '#f8fafc',
    surface_color: '#ffffff',
    text_color: '#0f172a',
    heading_font: 'Space Grotesk',
    body_font: 'Manrope',
    border_radius: 20,
    logo_url: null,
  };
}

export function createDefaultSettings(homePageId: string | null = null): AppSettings {
	return {
		home_page_id: homePageId,
		navigation_style: 'tabs',
		max_width: '1280px',
		show_branding: true,
		custom_css: null,
		builder_experience: 'workshop',
		consumer_mode: {
			enabled: false,
			allow_guest_access: false,
			portal_title: null,
			portal_subtitle: null,
			primary_cta_label: null,
			primary_cta_url: null,
		},
		slate: {
			enabled: false,
			framework: 'react',
			package_name: '@open-foundry/slate-app',
			entry_file: 'src/App.tsx',
			sdk_import: '@open-foundry/sdk/react',
			workspace: {
				enabled: false,
				repository_id: null,
				layout: 'split',
				runtime: 'typescript-react',
				dev_command: 'pnpm dev',
				preview_command: 'pnpm build',
				files: [],
			},
			quiver_embed: {
				enabled: false,
				primary_type_id: null,
				secondary_type_id: null,
				join_field: null,
				secondary_join_field: null,
				date_field: null,
				metric_field: null,
				group_field: null,
				selected_group: null,
			},
		},
	};
}

export function createPage(name = 'Overview', path = '/'): AppPage {
  return {
    id: createId(),
    name,
    path,
    description: '',
    layout: {
      kind: 'grid',
      columns: 12,
      gap: '1.25rem',
      max_width: '1280px',
    },
    widgets: [],
    visible: true,
  };
}

export function createWidgetFromCatalog(item: WidgetCatalogItem): AppWidget {
	const widgetId = createId();
	const widget: AppWidget = {
		id: widgetId,
		widget_type: item.widget_type,
		title: item.label,
		description: item.description,
		position: {
			x: 0,
			y: 0,
			width: item.default_size.width,
			height: item.default_size.height,
		},
		props: cloneValue(item.default_props ?? {}),
		binding: item.supported_bindings.length > 0
			? {
					source_type: item.supported_bindings[0],
					source_id: null,
					query_text: null,
					path: null,
					fields: [],
					parameters: {},
					limit: 25,
				}
			: null,
		events: [],
		children: [],
	};

	if (item.widget_type === 'scenario') {
		widget.events = [
			{
				id: createId(),
				trigger: 'scenario_change',
				action: 'set_parameters',
				label: 'Apply runtime parameters',
				config: { source: 'scenario-widget' },
			},
		];
	}

	return widget;
}

export function createEmptyAppDraft(): AppDefinition {
  const page = createPage();
  const now = new Date().toISOString();
  return {
    id: '',
    name: 'New App',
    slug: 'new-app',
    description: 'Operational app built with OpenFoundry Workshop.',
    status: 'draft',
    pages: [page],
    theme: createDefaultTheme(),
    settings: createDefaultSettings(page.id),
    template_key: null,
    created_by: null,
    published_version_id: null,
    created_at: now,
    updated_at: now,
  };
}

export function normalizePageLayout(pages: AppPage[]) {
  return pages.map((page) => ({
    ...page,
    widgets: page.widgets.map((widget, index) => ({
      ...widget,
      position: {
        ...widget.position,
        y: index * 2,
      },
    })),
  }));
}

export function seedSlateWorkspace(files: SlatePackageFile[]) {
	return cloneValue(files);
}
