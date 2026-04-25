import { describe, expect, it } from 'vitest';

import type { WidgetCatalogItem } from '$lib/api/apps';
import {
  createDefaultSettings,
  createDefaultTheme,
  createEmptyAppDraft,
  createWidgetFromCatalog,
  normalizePageLayout,
  seedSlateWorkspace,
} from './apps';

describe('app builder utilities', () => {
  it('creates the default theme and settings expected by new drafts', () => {
    const theme = createDefaultTheme();
    const settings = createDefaultSettings('page-1');

    expect(theme).toMatchObject({
      name: 'Signal',
      primary_color: '#0f766e',
      accent_color: '#f97316',
      heading_font: 'Space Grotesk',
      body_font: 'Manrope',
    });
    expect(settings).toEqual({
      home_page_id: 'page-1',
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
      interactive_workshop: {
        enabled: false,
        title: 'Interactive Workshop',
        subtitle: 'Coordinate scenario presets, decision briefs, and copilots from one runtime surface.',
        primary_scenario_widget_id: null,
        primary_agent_widget_id: null,
        briefing_template: 'Current scenario context:\n{{demand_multiplier}} demand multiplier\n{{service_level}} service level\nUse these assumptions to brief the operator.',
        suggested_questions: [
          'What changed versus the baseline scenario?',
          'Which mitigations should the team prioritize first?',
        ],
        scenario_presets: [],
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
    });
  });

  it('creates widgets from the catalog with cloned props and a default binding', () => {
    const item: WidgetCatalogItem = {
      widget_type: 'chart.line',
      label: 'Revenue',
      description: 'Tracks monthly revenue',
      category: 'analytics',
      default_props: {
        axes: {
          x: 'month',
          y: 'revenue',
        },
      },
      default_size: {
        width: 6,
        height: 4,
      },
      supported_bindings: ['query'],
      supports_children: false,
    };

    const widget = createWidgetFromCatalog(item);
    (item.default_props.axes as { x: string; y: string }).x = 'quarter';

    expect(widget.widget_type).toBe('chart.line');
    expect(widget.position).toEqual({ x: 0, y: 0, width: 6, height: 4 });
    expect(widget.props).toEqual({
      axes: {
        x: 'month',
        y: 'revenue',
      },
    });
    expect(widget.binding).toMatchObject({
      source_type: 'query',
      source_id: null,
      query_text: null,
      limit: 25,
    });
  });

  it('creates new drafts with a home page and normalizes widget row positions', () => {
    const draft = createEmptyAppDraft();
    const page = draft.pages[0];

    expect(page).toBeDefined();
    expect(draft.settings.home_page_id).toBe(page.id);
    expect(draft.slug).toBe('new-app');

    const normalized = normalizePageLayout([
      {
        ...page,
        widgets: [
          {
            id: 'widget-1',
            widget_type: 'stat',
            title: 'One',
            description: '',
            position: { x: 0, y: 99, width: 3, height: 2 },
            props: {},
            binding: null,
            events: [],
            children: [],
          },
          {
            id: 'widget-2',
            widget_type: 'stat',
            title: 'Two',
            description: '',
            position: { x: 3, y: 99, width: 3, height: 2 },
            props: {},
            binding: null,
            events: [],
            children: [],
          },
        ],
      },
    ]);

    expect(normalized[0].widgets.map((widget) => widget.position.y)).toEqual([0, 2]);
  });

  it('clones Slate workspace files for round-trip editing', () => {
    const source = [
      {
        path: 'src/App.tsx',
        language: 'typescript',
        content: 'export default function App() { return null; }',
      },
    ];
    const files = seedSlateWorkspace(source);

    files[0].content = 'changed';

    expect(source[0].content).toContain('return null');
  });
});
