CREATE TABLE IF NOT EXISTS apps (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    slug TEXT NOT NULL UNIQUE,
    description TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'draft',
    pages JSONB NOT NULL DEFAULT '[]'::jsonb,
    theme JSONB NOT NULL DEFAULT '{}'::jsonb,
    settings JSONB NOT NULL DEFAULT '{}'::jsonb,
    template_key TEXT,
    created_by UUID,
    published_version_id UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_apps_status ON apps(status);
CREATE INDEX IF NOT EXISTS idx_apps_updated_at ON apps(updated_at DESC);

CREATE TABLE IF NOT EXISTS app_versions (
    id UUID PRIMARY KEY,
    app_id UUID NOT NULL REFERENCES apps(id) ON DELETE CASCADE,
    version_number INTEGER NOT NULL,
    status TEXT NOT NULL DEFAULT 'draft',
    app_snapshot JSONB NOT NULL,
    notes TEXT NOT NULL DEFAULT '',
    created_by UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    published_at TIMESTAMPTZ,
    UNIQUE(app_id, version_number)
);

CREATE INDEX IF NOT EXISTS idx_app_versions_app_id ON app_versions(app_id, version_number DESC);

ALTER TABLE apps
    ADD CONSTRAINT apps_published_version_id_fkey
    FOREIGN KEY (published_version_id)
    REFERENCES app_versions(id)
    ON DELETE SET NULL;

CREATE TABLE IF NOT EXISTS app_templates (
    id UUID PRIMARY KEY,
    key TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    category TEXT NOT NULL DEFAULT 'general',
    preview_image_url TEXT,
    definition JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

INSERT INTO app_templates (id, key, name, description, category, preview_image_url, definition)
VALUES
(
    '00000000-0000-0000-0000-000000000301',
    'ops-center',
    'Operations Center',
    'Command view for throughput, incidents, and regional coverage.',
    'operations',
    NULL,
    $$
    {
      "pages": [
        {
          "id": "ops-overview",
          "name": "Overview",
          "path": "/",
          "description": "Operational pulse across queues and regions.",
          "layout": { "kind": "grid", "columns": 12, "gap": "1.25rem", "max_width": "1440px" },
          "visible": true,
          "widgets": [
            {
              "id": "hero-text",
              "widget_type": "text",
              "title": "Shift briefing",
              "description": "",
              "position": { "x": 0, "y": 0, "width": 12, "height": 2 },
              "props": { "format": "markdown", "content": "# Operations Center\nMonitor throughput, incidents, and escalation status in one surface." },
              "events": [],
              "children": []
            },
            {
              "id": "throughput-chart",
              "widget_type": "chart",
              "title": "Throughput trend",
              "description": "",
              "position": { "x": 0, "y": 2, "width": 6, "height": 4 },
              "props": { "chart_type": "line", "x_field": "label", "y_field": "value" },
              "binding": {
                "source_type": "query",
                "query_text": "select 'Mon' as label, 122 as value union all select 'Tue', 118 union all select 'Wed', 139 union all select 'Thu', 151 union all select 'Fri', 146"
              },
              "events": [],
              "children": []
            },
            {
              "id": "queue-table",
              "widget_type": "table",
              "title": "Open work items",
              "description": "",
              "position": { "x": 6, "y": 2, "width": 6, "height": 4 },
              "props": { "page_size": 8, "striped": true },
              "binding": {
                "source_type": "query",
                "query_text": "select 'P1' as priority, 'Delayed shipment' as title, 'Amber' as status union all select 'P2', 'Sensor outage', 'Red' union all select 'P3', 'Route reassignment', 'Green'"
              },
              "events": [],
              "children": []
            },
            {
              "id": "coverage-map",
              "widget_type": "map",
              "title": "Regional coverage",
              "description": "",
              "position": { "x": 0, "y": 6, "width": 8, "height": 4 },
              "props": { "latitude_field": "lat", "longitude_field": "lon", "zoom": 2 },
              "binding": {
                "source_type": "query",
                "query_text": "select 40.4168 as lat, -3.7038 as lon, 'Madrid' as label union all select 51.5072, -0.1276, 'London' union all select 34.0522, -118.2437, 'Los Angeles'"
              },
              "events": [],
              "children": []
            },
            {
              "id": "escalate-button",
              "widget_type": "button",
              "title": "Escalate issue",
              "description": "",
              "position": { "x": 8, "y": 6, "width": 4, "height": 1 },
              "props": { "label": "Open workflow queue", "variant": "primary" },
              "events": [
                {
                  "id": "open-workflows",
                  "trigger": "click",
                  "action": "open_link",
                  "label": "Open workflows",
                  "config": { "url": "/workflows" }
                }
              ],
              "children": []
            }
          ]
        },
        {
          "id": "ops-drilldown",
          "name": "Drilldown",
          "path": "/drilldown",
          "description": "Route-specific investigation page.",
          "layout": { "kind": "grid", "columns": 12, "gap": "1rem", "max_width": "1440px" },
          "visible": true,
          "widgets": [
            {
              "id": "drilldown-container",
              "widget_type": "container",
              "title": "Incident brief",
              "description": "",
              "position": { "x": 0, "y": 0, "width": 12, "height": 4 },
              "props": { "title": "Incident brief", "variant": "card" },
              "events": [],
              "children": [
                {
                  "id": "incident-photo",
                  "widget_type": "image",
                  "title": "Site image",
                  "description": "",
                  "position": { "x": 0, "y": 0, "width": 4, "height": 3 },
                  "props": { "url": "https://images.unsplash.com/photo-1494412651409-8963ce7935a7?auto=format&fit=crop&w=1200&q=80", "alt": "Operations" },
                  "events": [],
                  "children": []
                },
                {
                  "id": "incident-copy",
                  "widget_type": "text",
                  "title": "Runbook",
                  "description": "",
                  "position": { "x": 4, "y": 0, "width": 8, "height": 3 },
                  "props": { "format": "markdown", "content": "### Recommended response\n1. Confirm owner\n2. Trigger fallback route\n3. Notify downstream teams" },
                  "events": [],
                  "children": []
                }
              ]
            }
          ]
        }
      ],
      "theme": {
        "name": "Operations Signal",
        "primary_color": "#155e75",
        "accent_color": "#f59e0b",
        "background_color": "#f8fafc",
        "surface_color": "#ffffff",
        "text_color": "#0f172a",
        "heading_font": "Space Grotesk",
        "body_font": "Manrope",
        "border_radius": 22,
        "logo_url": null
      },
      "settings": {
        "home_page_id": "ops-overview",
        "navigation_style": "tabs",
        "max_width": "1440px",
        "show_branding": true,
        "custom_css": null
      }
    }
    $$::jsonb
),
(
    '00000000-0000-0000-0000-000000000302',
    'case-workbench',
    'Case Workbench',
    'Operator workspace for triage, case intake, and guided review.',
    'case-management',
    NULL,
    $$
    {
      "pages": [
        {
          "id": "case-home",
          "name": "Case Home",
          "path": "/",
          "description": "Investigate and update the selected case.",
          "layout": { "kind": "grid", "columns": 12, "gap": "1rem", "max_width": "1320px" },
          "visible": true,
          "widgets": [
            {
              "id": "case-summary",
              "widget_type": "container",
              "title": "Case summary",
              "description": "",
              "position": { "x": 0, "y": 0, "width": 7, "height": 4 },
              "props": { "title": "Case summary", "variant": "card" },
              "events": [],
              "children": [
                {
                  "id": "case-text",
                  "widget_type": "text",
                  "title": "Case narrative",
                  "description": "",
                  "position": { "x": 0, "y": 0, "width": 4, "height": 2 },
                  "props": { "format": "markdown", "content": "### Investigate recent activity\nReview ownership, risk notes, and last actions before approving." },
                  "events": [],
                  "children": []
                },
                {
                  "id": "case-image",
                  "widget_type": "image",
                  "title": "Analyst note",
                  "description": "",
                  "position": { "x": 4, "y": 0, "width": 3, "height": 2 },
                  "props": { "url": "https://images.unsplash.com/photo-1520607162513-77705c0f0d4a?auto=format&fit=crop&w=1200&q=80", "alt": "Case workspace" },
                  "events": [],
                  "children": []
                }
              ]
            },
            {
              "id": "case-form",
              "widget_type": "form",
              "title": "Case update",
              "description": "",
              "position": { "x": 7, "y": 0, "width": 5, "height": 4 },
              "props": {
                "fields": [
                  { "name": "owner", "label": "Owner", "type": "text" },
                  { "name": "status", "label": "Status", "type": "select", "options": ["New", "In Review", "Approved"] },
                  { "name": "notes", "label": "Notes", "type": "textarea" }
                ],
                "submit_label": "Update case"
              },
              "events": [
                {
                  "id": "form-submit",
                  "trigger": "submit",
                  "action": "filter",
                  "label": "Refresh related cases",
                  "config": { "target": "related-cases", "field": "status" }
                }
              ],
              "children": []
            },
            {
              "id": "related-cases",
              "widget_type": "table",
              "title": "Related cases",
              "description": "",
              "position": { "x": 0, "y": 4, "width": 12, "height": 4 },
              "props": { "page_size": 6, "striped": true },
              "binding": {
                "source_type": "query",
                "query_text": "select 'Case-1042' as case_id, 'AML review' as title, 'In Review' as status union all select 'Case-1048', 'Vendor screening', 'Approved' union all select 'Case-1051', 'Travel rule follow-up', 'New'"
              },
              "events": [],
              "children": []
            },
            {
              "id": "approval-button",
              "widget_type": "button",
              "title": "Submit decision",
              "description": "",
              "position": { "x": 9, "y": 8, "width": 3, "height": 1 },
              "props": { "label": "Open approvals", "variant": "secondary" },
              "events": [
                {
                  "id": "goto-approvals",
                  "trigger": "click",
                  "action": "open_link",
                  "label": "Open approvals",
                  "config": { "url": "/workflows" }
                }
              ],
              "children": []
            }
          ]
        }
      ],
      "theme": {
        "name": "Case Review",
        "primary_color": "#7c2d12",
        "accent_color": "#0891b2",
        "background_color": "#fff7ed",
        "surface_color": "#ffffff",
        "text_color": "#1c1917",
        "heading_font": "Sora",
        "body_font": "Manrope",
        "border_radius": 18,
        "logo_url": null
      },
      "settings": {
        "home_page_id": "case-home",
        "navigation_style": "sidebar",
        "max_width": "1320px",
        "show_branding": true,
        "custom_css": null
      }
    }
    $$::jsonb
)
ON CONFLICT (key) DO NOTHING;