INSERT INTO app_templates (id, key, name, description, category, preview_image_url, definition)
VALUES
(
    '00000000-0000-0000-0000-000000000303',
    'customer-portal',
    'Customer Portal',
    'Consumer-facing portal with scenario controls, embedded agent help, and KPI tiles.',
    'consumer',
    NULL,
    $$
    {
      "pages": [
        {
          "id": "customer-home",
          "name": "Home",
          "path": "/",
          "description": "Consumer portal with self-service and what-if controls.",
          "layout": { "kind": "grid", "columns": 12, "gap": "1rem", "max_width": "1360px" },
          "visible": true,
          "widgets": [
            {
              "id": "customer-hero",
              "widget_type": "text",
              "title": "Portal intro",
              "description": "",
              "position": { "x": 0, "y": 0, "width": 12, "height": 2 },
              "props": { "content": "# Customer Portal\nTrack demand assumptions, service commitments, and ask for guided help in one place." },
              "events": [],
              "children": []
            },
            {
              "id": "customer-scenario",
              "widget_type": "scenario",
              "title": "Scenario controls",
              "description": "",
              "position": { "x": 0, "y": 2, "width": 5, "height": 4 },
              "props": {
                "headline": "Demand and service assumptions",
                "parameters": [
                  {
                    "name": "demand_multiplier",
                    "label": "Demand multiplier",
                    "type": "number",
                    "default_value": "1.12",
                    "description": "Scale weekly volume against the baseline plan."
                  },
                  {
                    "name": "service_level",
                    "label": "Service level target",
                    "type": "number",
                    "default_value": "0.97",
                    "description": "Target on-time fulfillment for the scenario."
                  }
                ],
                "apply_label": "Apply scenario",
                "reset_label": "Reset"
              },
              "events": [
                {
                  "id": "customer-scenario-apply",
                  "trigger": "scenario_change",
                  "action": "set_parameters",
                  "label": "Apply parameters",
                  "config": {}
                }
              ],
              "children": []
            },
            {
              "id": "customer-kpi",
              "widget_type": "chart",
              "title": "Scenario impact",
              "description": "",
              "position": { "x": 5, "y": 2, "width": 7, "height": 4 },
              "props": { "chart_type": "bar", "x_field": "label", "y_field": "value" },
              "binding": {
                "source_type": "query",
                "query_text": "select 'Baseline demand' as label, 100 as value union all select 'Scenario demand', round((100 * {{demand_multiplier}})::numeric, 2) union all select 'Service target', round((100 * {{service_level}})::numeric, 2)"
              },
              "events": [],
              "children": []
            },
            {
              "id": "customer-agent",
              "widget_type": "agent",
              "title": "Guided help",
              "description": "",
              "position": { "x": 0, "y": 6, "width": 12, "height": 4 },
              "props": {
                "agent_id": "",
                "welcome_message": "Embed an OpenFoundry agent here to answer consumer questions or guide triage.",
                "placeholder": "Ask how this scenario changes the current commitment...",
                "submit_label": "Ask agent",
                "show_traces": true
              },
              "events": [],
              "children": []
            }
          ]
        }
      ],
      "theme": {
        "name": "Customer Signal",
        "primary_color": "#0f766e",
        "accent_color": "#ea580c",
        "background_color": "#f8fafc",
        "surface_color": "#ffffff",
        "text_color": "#0f172a",
        "heading_font": "Space Grotesk",
        "body_font": "Manrope",
        "border_radius": 24,
        "logo_url": null
      },
      "settings": {
        "home_page_id": "customer-home",
        "navigation_style": "tabs",
        "max_width": "1360px",
        "show_branding": true,
        "custom_css": null,
        "builder_experience": "workshop",
        "consumer_mode": {
          "enabled": true,
          "allow_guest_access": true,
          "portal_title": "Customer Portal",
          "portal_subtitle": "Self-service workspace for external users and partners.",
          "primary_cta_label": "Open orders",
          "primary_cta_url": "/datasets"
        },
        "slate": {
          "enabled": false,
          "framework": "react",
          "package_name": "@open-foundry/customer-portal",
          "entry_file": "src/App.tsx",
          "sdk_import": "@open-foundry/sdk/react"
        }
      }
    }
    $$::jsonb
),
(
    '00000000-0000-0000-0000-000000000304',
    'slate-starter',
    'Slate Starter',
    'Pro-code app shell with a React starter package generated from Workshop.',
    'pro-code',
    NULL,
    $$
    {
      "pages": [
        {
          "id": "slate-home",
          "name": "Starter",
          "path": "/",
          "description": "Seed page for a React-backed Slate app.",
          "layout": { "kind": "grid", "columns": 12, "gap": "1rem", "max_width": "1360px" },
          "visible": true,
          "widgets": [
            {
              "id": "slate-copy",
              "widget_type": "text",
              "title": "Slate starter",
              "description": "",
              "position": { "x": 0, "y": 0, "width": 12, "height": 2 },
              "props": { "content": "# Slate Starter\nGenerate a React package with `@open-foundry/sdk/react`, then keep iterating in your own repo." },
              "events": [],
              "children": []
            },
            {
              "id": "slate-table",
              "widget_type": "table",
              "title": "Starter dataset",
              "description": "",
              "position": { "x": 0, "y": 2, "width": 12, "height": 4 },
              "props": { "page_size": 6, "striped": true },
              "binding": {
                "source_type": "query",
                "query_text": "select 'sdk' as key, '@open-foundry/sdk/react' as value union all select 'framework', 'React' union all select 'entry', 'src/App.tsx'"
              },
              "events": [],
              "children": []
            }
          ]
        }
      ],
      "theme": {
        "name": "Slate Studio",
        "primary_color": "#0f172a",
        "accent_color": "#0ea5e9",
        "background_color": "#f8fafc",
        "surface_color": "#ffffff",
        "text_color": "#0f172a",
        "heading_font": "Space Grotesk",
        "body_font": "Manrope",
        "border_radius": 24,
        "logo_url": null
      },
      "settings": {
        "home_page_id": "slate-home",
        "navigation_style": "tabs",
        "max_width": "1360px",
        "show_branding": true,
        "custom_css": null,
        "builder_experience": "slate",
        "consumer_mode": {
          "enabled": false,
          "allow_guest_access": false,
          "portal_title": null,
          "portal_subtitle": null,
          "primary_cta_label": null,
          "primary_cta_url": null
        },
        "slate": {
          "enabled": true,
          "framework": "react",
          "package_name": "@open-foundry/slate-starter",
          "entry_file": "src/App.tsx",
          "sdk_import": "@open-foundry/sdk/react"
        }
      }
    }
    $$::jsonb
)
ON CONFLICT (key) DO UPDATE
SET
    name = EXCLUDED.name,
    description = EXCLUDED.description,
    category = EXCLUDED.category,
    preview_image_url = EXCLUDED.preview_image_url,
    definition = EXCLUDED.definition;
