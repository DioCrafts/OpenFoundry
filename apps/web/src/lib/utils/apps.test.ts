import { describe, expect, it } from 'vitest';

import type { WidgetCatalogItem } from '$lib/api/apps';
import {
  createDefaultSettings,
  createDefaultTheme,
  createEmptyAppDraft,
  createWidgetFromCatalog,
  normalizePageLayout,
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
});