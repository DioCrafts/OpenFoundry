use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::{
    handlers::sanitize_pages,
    models::{
        app::{
            App, AppSettings, ImportSlatePackageRequest, SlatePackageFile, SlatePackageResponse,
            SlateRoundTripResponse,
        },
        page::AppPage,
        theme::AppTheme,
    },
};

const WORKSHOP_MANIFEST_PATH: &str = ".openfoundry/workshop.json";

pub fn build_slate_package(app: &App) -> SlatePackageResponse {
    let framework = app.settings.slate.framework.clone();
    let package_name = if app.settings.slate.package_name.trim().is_empty() {
        format!("@open-foundry/{}", app.slug)
    } else {
        app.settings.slate.package_name.clone()
    };
    let entry_file = if app.settings.slate.entry_file.trim().is_empty() {
        "src/App.tsx".to_string()
    } else {
        app.settings.slate.entry_file.clone()
    };
    let sdk_import = if app.settings.slate.sdk_import.trim().is_empty() {
        "@open-foundry/sdk/react".to_string()
    } else {
        app.settings.slate.sdk_import.clone()
    };

    let generated_files = vec![
        SlatePackageFile {
            path: "package.json".to_string(),
            language: "json".to_string(),
            content: render_package_json(&package_name),
        },
        SlatePackageFile {
            path: "tsconfig.json".to_string(),
            language: "json".to_string(),
            content: render_tsconfig(),
        },
        SlatePackageFile {
            path: "README.md".to_string(),
            language: "markdown".to_string(),
            content: render_readme(app),
        },
        SlatePackageFile {
            path: "src/main.tsx".to_string(),
            language: "typescript".to_string(),
            content: render_main_tsx(&sdk_import),
        },
        SlatePackageFile {
            path: "src/App.tsx".to_string(),
            language: "typescript".to_string(),
            content: render_app_tsx(app, &sdk_import),
        },
        SlatePackageFile {
            path: "src/theme.ts".to_string(),
            language: "typescript".to_string(),
            content: render_theme_ts(app),
        },
        SlatePackageFile {
            path: WORKSHOP_MANIFEST_PATH.to_string(),
            language: "json".to_string(),
            content: render_workshop_manifest(app),
        },
    ];

    let files = if app.settings.slate.workspace.files.is_empty() {
        generated_files
    } else {
        with_upserted_workshop_manifest(app.settings.slate.workspace.files.clone(), app)
    };

    SlatePackageResponse {
        app_id: app.id,
        app_slug: app.slug.clone(),
        framework,
        package_name,
        entry_file,
        sdk_import,
        files,
    }
}

pub fn apply_slate_round_trip(
    app: &mut App,
    request: ImportSlatePackageRequest,
) -> Result<SlateRoundTripResponse, String> {
    let files = normalize_files(request.files)?;
    if let Some(manifest) = extract_manifest(&files)? {
        app.name = manifest.name;
        app.slug = manifest.slug;
        app.description = manifest.description;
        app.pages = manifest.pages;
        app.theme = manifest.theme;
        app.settings = manifest.settings;
        app.template_key = manifest.template_key;
    }

    app.settings.builder_experience = "slate".to_string();
    app.settings.slate.enabled = true;
    if let Some(framework) = request.framework.filter(|value| !value.trim().is_empty()) {
        app.settings.slate.framework = framework;
    }
    if let Some(package_name) = request
        .package_name
        .filter(|value| !value.trim().is_empty())
    {
        app.settings.slate.package_name = package_name;
    }
    if let Some(entry_file) = request.entry_file.filter(|value| !value.trim().is_empty()) {
        app.settings.slate.entry_file = entry_file;
    }
    if let Some(sdk_import) = request.sdk_import.filter(|value| !value.trim().is_empty()) {
        app.settings.slate.sdk_import = sdk_import;
    }

    app.settings.slate.workspace.enabled = true;
    if let Some(repository_id) = request.repository_id {
        app.settings.slate.workspace.repository_id = if repository_id.trim().is_empty() {
            None
        } else {
            Some(repository_id)
        };
    }
    if let Some(layout) = request.layout.filter(|value| !value.trim().is_empty()) {
        app.settings.slate.workspace.layout = layout;
    }
    if let Some(runtime) = request.runtime.filter(|value| !value.trim().is_empty()) {
        app.settings.slate.workspace.runtime = runtime;
    }
    if let Some(dev_command) = request.dev_command.filter(|value| !value.trim().is_empty()) {
        app.settings.slate.workspace.dev_command = dev_command;
    }
    if let Some(preview_command) = request
        .preview_command
        .filter(|value| !value.trim().is_empty())
    {
        app.settings.slate.workspace.preview_command = preview_command;
    }

    sanitize_pages(&mut app.pages, &mut app.settings);
    app.settings.slate.workspace.files = with_upserted_workshop_manifest(files, app);

    let response_app = app.clone();
    let slate_package = build_slate_package(app);
    Ok(SlateRoundTripResponse {
        app: response_app,
        slate_package,
    })
}

fn render_package_json(package_name: &str) -> String {
    serde_json::to_string_pretty(&serde_json::json!({
        "name": package_name,
        "version": "0.1.0",
        "private": true,
        "type": "module",
        "scripts": {
            "dev": "vite",
            "build": "tsc -p . && vite build",
            "check": "tsc -p . --noEmit"
        },
        "dependencies": {
            "@open-foundry/sdk": "^0.1.0",
            "react": "^18.3.0",
            "react-dom": "^18.3.0"
        },
        "devDependencies": {
            "@types/react": "^18.3.0",
            "@types/react-dom": "^18.3.0",
            "typescript": "^5.6.0",
            "vite": "^5.4.0"
        }
    }))
    .unwrap_or_else(|_| "{}".to_string())
}

fn render_tsconfig() -> String {
    serde_json::to_string_pretty(&serde_json::json!({
        "compilerOptions": {
            "target": "ES2022",
            "module": "ES2022",
            "moduleResolution": "Bundler",
            "jsx": "react-jsx",
            "strict": true,
            "lib": ["ES2022", "DOM"],
            "types": ["vite/client"]
        },
        "include": ["src/**/*.ts", "src/**/*.tsx"]
    }))
    .unwrap_or_else(|_| "{}".to_string())
}

fn render_readme(app: &App) -> String {
    format!(
        "# {name}\n\nGenerated from OpenFoundry Slate export for `{slug}`.\n\n## Included\n\n- React starter wired for `@open-foundry/sdk`\n- Provider + hooks via `@open-foundry/sdk/react`\n- Theme tokens derived from the Workshop app\n- Starter dataset/admin surfaces ready to adapt\n- `.openfoundry/workshop.json` manifest for Workshop <-> Slate round-trip\n",
        name = app.name,
        slug = app.slug
    )
}

fn render_main_tsx(sdk_import: &str) -> String {
    let template = [
        "import React from 'react';",
        "import ReactDOM from 'react-dom/client';",
        "import App from './App';",
        "import { OpenFoundryProvider } from '__SDK_IMPORT__';",
        "",
        "const baseUrl = import.meta.env.VITE_OPENFOUNDRY_BASE_URL ?? 'http://127.0.0.1:8080';",
        "const headers = import.meta.env.VITE_OPENFOUNDRY_TOKEN",
        "  ? { authorization: `Bearer ${import.meta.env.VITE_OPENFOUNDRY_TOKEN}` }",
        "  : undefined;",
        "",
        "ReactDOM.createRoot(document.getElementById('root')!).render(",
        "  <React.StrictMode>",
        "    <OpenFoundryProvider options={{ baseUrl, headers }}>",
        "      <App />",
        "    </OpenFoundryProvider>",
        "  </React.StrictMode>,",
        ");",
    ]
    .join("\n");

    template.replace("__SDK_IMPORT__", sdk_import)
}

fn render_workshop_manifest(app: &App) -> String {
    serde_json::to_string_pretty(&WorkshopRoundTripManifest::from(app))
        .unwrap_or_else(|_| "{}".to_string())
}

fn with_upserted_workshop_manifest(
    mut files: Vec<SlatePackageFile>,
    app: &App,
) -> Vec<SlatePackageFile> {
    let manifest = SlatePackageFile {
        path: WORKSHOP_MANIFEST_PATH.to_string(),
        language: "json".to_string(),
        content: render_workshop_manifest(app),
    };

    if let Some(existing) = files
        .iter_mut()
        .find(|file| file.path == WORKSHOP_MANIFEST_PATH)
    {
        *existing = manifest;
    } else {
        files.push(manifest);
    }

    files
}

fn normalize_files(files: Vec<SlatePackageFile>) -> Result<Vec<SlatePackageFile>, String> {
    let mut normalized = Vec::new();
    let mut seen_paths = BTreeSet::new();

    for mut file in files {
        file.path = file.path.trim().to_string();
        if file.path.is_empty() {
            return Err("Slate package files must include a path".to_string());
        }
        if !seen_paths.insert(file.path.clone()) {
            return Err(format!("Duplicate Slate package path: {}", file.path));
        }
        if file.language.trim().is_empty() {
            file.language = infer_language(&file.path).to_string();
        }
        normalized.push(file);
    }

    if normalized.is_empty() {
        return Err("Slate round-trip import requires at least one file".to_string());
    }

    Ok(normalized)
}

fn extract_manifest(
    files: &[SlatePackageFile],
) -> Result<Option<WorkshopRoundTripManifest>, String> {
    let Some(file) = files
        .iter()
        .find(|candidate| candidate.path == WORKSHOP_MANIFEST_PATH)
    else {
        return Ok(None);
    };

    serde_json::from_str::<WorkshopRoundTripManifest>(&file.content)
        .map(Some)
        .map_err(|error| format!("Failed to parse Workshop manifest: {error}"))
}

fn infer_language(path: &str) -> &'static str {
    match path.rsplit('.').next().unwrap_or_default() {
        "json" => "json",
        "md" => "markdown",
        "ts" | "tsx" => "typescript",
        "js" | "jsx" => "javascript",
        "py" => "python",
        "toml" => "toml",
        _ => "text",
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WorkshopRoundTripManifest {
    name: String,
    slug: String,
    description: String,
    pages: Vec<AppPage>,
    theme: AppTheme,
    settings: AppSettings,
    template_key: Option<String>,
}

impl From<&App> for WorkshopRoundTripManifest {
    fn from(app: &App) -> Self {
        Self {
            name: app.name.clone(),
            slug: app.slug.clone(),
            description: app.description.clone(),
            pages: app.pages.clone(),
            theme: app.theme.clone(),
            settings: app.settings.clone(),
            template_key: app.template_key.clone(),
        }
    }
}

fn render_app_tsx(app: &App, sdk_import: &str) -> String {
    let component_name = component_name(&app.name);
    let portal_title = app
        .settings
        .consumer_mode
        .portal_title
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(&app.name);
    let portal_subtitle = app
        .settings
        .consumer_mode
        .portal_subtitle
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(
            "Compose a custom app shell while keeping OpenFoundry as the operational backend.",
        );

    let template = r#"import React from 'react';
import { useOpenFoundry, useOpenFoundryQuery } from '__SDK_IMPORT__';
import { appTheme } from './theme';

export default function __COMPONENT_NAME__() {
  const client = useOpenFoundry();
  const datasets = useOpenFoundryQuery(() => client.datasetDatasetListdatasets(), [client]);
  const adminUsers = useOpenFoundryQuery(() => client.getAdminUsersV2(), [client]);
  const latestDataset = datasets.data?.datasets?.[0];

  return (
    <main style={{ ...appTheme, minHeight: '100vh', padding: '40px', boxSizing: 'border-box' }}>
      <section style={{ background: '#ffffff', borderRadius: 28, padding: 32, boxShadow: '0 16px 48px rgba(15, 23, 42, 0.08)' }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', gap: 24, flexWrap: 'wrap' }}>
          <div>
            <div style={{ fontSize: 12, letterSpacing: '0.24em', textTransform: 'uppercase', opacity: 0.55 }}>Slate starter</div>
            <h1 style={{ margin: '12px 0 0', fontSize: 40 }}>__PORTAL_TITLE__</h1>
            <p style={{ maxWidth: 720, lineHeight: 1.7, opacity: 0.72 }}>__PORTAL_SUBTITLE__</p>
          </div>
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(2, minmax(0, 1fr))', gap: 12, minWidth: 280 }}>
            <MetricCard label="Datasets" value={String(datasets.data?.datasets?.length ?? 0)} />
            <MetricCard label="Admin users" value={String(adminUsers.data?.data?.length ?? 0)} />
            <MetricCard label="SDK" value="@open-foundry/sdk" />
            <MetricCard label="Runtime" value="__EXPERIENCE__" />
          </div>
        </div>

        <section style={{ marginTop: 28, display: 'grid', gridTemplateColumns: '1.2fr 0.8fr', gap: 20 }}>
          <div style={{ border: '1px solid rgba(148, 163, 184, 0.24)', borderRadius: 24, padding: 24 }}>
            <h2 style={{ marginTop: 0 }}>Platform-backed React surface</h2>
            <p style={{ opacity: 0.72 }}>Use the provider + hooks to keep auth, fetch state, and mutations close to the component tree.</p>
            <pre style={{ margin: 0, overflowX: 'auto', background: '#0f172a', color: '#e2e8f0', borderRadius: 18, padding: 18 }}>{`const client = useOpenFoundry();\nconst datasets = useOpenFoundryQuery(() => client.datasetDatasetListdatasets(), [client]);`}</pre>
          </div>
          <div style={{ border: '1px solid rgba(148, 163, 184, 0.24)', borderRadius: 24, padding: 24 }}>
            <h2 style={{ marginTop: 0 }}>Developer workspace</h2>
            <p style={{ opacity: 0.72 }}>Replace this starter with the actual Slate experience for `__APP_SLUG__`, then keep the Workshop manifest in sync for round-trips.</p>
            <div style={{ marginTop: 16, fontSize: 14, lineHeight: 1.6 }}>
              <div><strong>Latest dataset:</strong> {latestDataset?.name ?? 'none yet'}</div>
              <div><strong>Published from:</strong> __APP_NAME__</div>
              <div><strong>Experience:</strong> __EXPERIENCE__</div>
              <div><strong>Manifest:</strong> .openfoundry/workshop.json</div>
            </div>
          </div>
        </section>
      </section>
    </main>
  );
}

function MetricCard(props: { label: string; value: string }) {
  return (
    <div style={{ borderRadius: 20, background: '#f8fafc', padding: 16 }}>
      <div style={{ fontSize: 11, letterSpacing: '0.18em', textTransform: 'uppercase', opacity: 0.55 }}>{props.label}</div>
      <div style={{ marginTop: 8, fontSize: 28, fontWeight: 700 }}>{props.value}</div>
    </div>
  );
}
"#;

    template
        .replace("__SDK_IMPORT__", sdk_import)
        .replace("__COMPONENT_NAME__", &component_name)
        .replace("__PORTAL_TITLE__", portal_title)
        .replace("__PORTAL_SUBTITLE__", portal_subtitle)
        .replace("__EXPERIENCE__", &app.settings.builder_experience)
        .replace("__APP_SLUG__", &app.slug)
        .replace("__APP_NAME__", &app.name)
}

fn render_theme_ts(app: &App) -> String {
    format!(
        "export const appTheme = {{\n\
  background: '{background}',\n\
  color: '{text}',\n\
  fontFamily: '{body}, sans-serif',\n\
  ['--app-primary' as const]: '{primary}',\n\
  ['--app-accent' as const]: '{accent}',\n\
  ['--app-heading-font' as const]: '{heading}',\n\
}} as const;\n",
        background = app.theme.background_color,
        text = app.theme.text_color,
        body = app.theme.body_font,
        primary = app.theme.primary_color,
        accent = app.theme.accent_color,
        heading = app.theme.heading_font
    )
}

fn component_name(name: &str) -> String {
    let mut component = String::new();
    for part in name
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty())
    {
        let mut chars = part.chars();
        if let Some(first) = chars.next() {
            component.extend(first.to_uppercase());
            component.extend(chars.flat_map(char::to_lowercase));
        }
    }

    if component.is_empty() {
        "SlateApp".to_string()
    } else {
        component
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{app::AppSettings, page::AppPage, theme::AppTheme};
    use chrono::Utc;
    use uuid::Uuid;

    fn sample_app() -> App {
        App {
            id: Uuid::now_v7(),
            name: "Customer Signal".to_string(),
            slug: "customer-signal".to_string(),
            description: "Consumer-facing signal portal".to_string(),
            status: "draft".to_string(),
            pages: vec![AppPage::default()],
            theme: AppTheme::default(),
            settings: AppSettings::default(),
            template_key: None,
            created_by: None,
            published_version_id: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn builds_react_slate_package_with_expected_files() {
        let package = build_slate_package(&sample_app());

        assert_eq!(package.framework, "react");
        assert!(package.files.iter().any(|file| file.path == "src/App.tsx"));
        assert!(package.files.iter().any(|file| file.path == "package.json"));
        assert!(
            package
                .files
                .iter()
                .any(|file| file.path == WORKSHOP_MANIFEST_PATH)
        );
    }

    #[test]
    fn imports_round_trip_package_into_workspace_files() {
        let mut app = sample_app();
        let response = apply_slate_round_trip(
            &mut app,
            ImportSlatePackageRequest {
                package_name: Some("@open-foundry/customer-signal".to_string()),
                entry_file: Some("src/App.tsx".to_string()),
                sdk_import: Some("@open-foundry/sdk/react".to_string()),
                framework: Some("react".to_string()),
                repository_id: Some("repo-123".to_string()),
                layout: Some("split".to_string()),
                runtime: Some("typescript-react".to_string()),
                dev_command: Some("pnpm dev".to_string()),
                preview_command: Some("pnpm build".to_string()),
                files: vec![SlatePackageFile {
                    path: "src/App.tsx".to_string(),
                    language: "typescript".to_string(),
                    content: "export default function App() { return null; }\n".to_string(),
                }],
            },
        )
        .expect("round-trip response");

        assert!(response.app.settings.slate.workspace.enabled);
        assert_eq!(
            response
                .app
                .settings
                .slate
                .workspace
                .repository_id
                .as_deref(),
            Some("repo-123")
        );
        assert!(
            response
                .app
                .settings
                .slate
                .workspace
                .files
                .iter()
                .any(|file| file.path == "src/App.tsx")
        );
        assert!(
            response
                .slate_package
                .files
                .iter()
                .any(|file| file.path == WORKSHOP_MANIFEST_PATH)
        );
    }
}
