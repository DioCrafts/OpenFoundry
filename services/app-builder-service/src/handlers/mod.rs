pub mod apps;
pub mod pages;
pub mod preview;
pub mod publish;
pub mod slate;
pub mod widgets;

use axum::http::StatusCode;
use sqlx::types::Json;
use uuid::Uuid;

use crate::{
    AppState,
    models::{
        app::{App, AppRow, AppSettings, AppTemplate, AppTemplateRow, WorkshopScenarioPreset},
        page::AppPage,
        version::{AppVersion, AppVersionRow},
        widget::WidgetDefinition,
    },
};

pub type ServiceResult<T> = Result<T, (StatusCode, String)>;

pub fn bad_request(message: impl Into<String>) -> (StatusCode, String) {
    (StatusCode::BAD_REQUEST, message.into())
}

pub fn conflict(message: impl Into<String>) -> (StatusCode, String) {
    (StatusCode::CONFLICT, message.into())
}

pub fn not_found(resource: &str) -> (StatusCode, String) {
    (StatusCode::NOT_FOUND, format!("{resource} not found"))
}

pub fn internal_error(message: impl Into<String>) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, message.into())
}

pub fn db_error(error: sqlx::Error) -> (StatusCode, String) {
    if let Some(database_error) = error.as_database_error() {
        if let Some(constraint) = database_error.constraint() {
            if constraint == "apps_slug_key" {
                return conflict("app slug already exists");
            }
        }
    }

    tracing::error!("app-builder database error: {error}");
    internal_error("database operation failed")
}

pub fn slugify(input: &str) -> String {
    let mut slug = String::new();
    let mut last_dash = false;

    for character in input.chars().flat_map(char::to_lowercase) {
        if character.is_ascii_alphanumeric() {
            slug.push(character);
            last_dash = false;
        } else if !last_dash {
            slug.push('-');
            last_dash = true;
        }
    }

    let trimmed = slug.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "app".to_string()
    } else {
        trimmed
    }
}

pub fn normalize_slug(candidate: Option<&str>, fallback_name: &str) -> String {
    let raw = candidate.unwrap_or(fallback_name).trim();
    slugify(raw)
}

pub fn sanitize_pages(pages: &mut Vec<AppPage>, settings: &mut AppSettings) {
    if pages.is_empty() {
        pages.push(AppPage::default());
    }

    for (index, page) in pages.iter_mut().enumerate() {
        if page.id.trim().is_empty() {
            page.id = Uuid::now_v7().to_string();
        }

        if page.name.trim().is_empty() {
            page.name = format!("Page {}", index + 1);
        }

        if page.path.trim().is_empty() {
            page.path = if index == 0 {
                "/".to_string()
            } else {
                format!("/{}", slugify(&page.name))
            };
        }

        sanitize_widgets(&mut page.widgets);
    }

    let current_home_exists = settings
        .home_page_id
        .as_ref()
        .map(|page_id| pages.iter().any(|page| &page.id == page_id))
        .unwrap_or(false);

    if !current_home_exists {
        settings.home_page_id = pages.first().map(|page| page.id.clone());
    }

    sanitize_interactive_workshop_settings(pages, settings);
}

fn sanitize_widgets(widgets: &mut [WidgetDefinition]) {
    for widget in widgets {
        if widget.id.trim().is_empty() {
            widget.id = Uuid::now_v7().to_string();
        }

        if widget.title.trim().is_empty() {
            widget.title = "Untitled widget".to_string();
        }

        sanitize_widgets(&mut widget.children);
    }
}

fn sanitize_interactive_workshop_settings(pages: &[AppPage], settings: &mut AppSettings) {
    let widget_ids = collect_widget_ids(pages);
    settings.interactive_workshop.suggested_questions = settings
        .interactive_workshop
        .suggested_questions
        .iter()
        .map(|question| question.trim().to_string())
        .filter(|question| !question.is_empty())
        .collect();

    for (index, preset) in settings
        .interactive_workshop
        .scenario_presets
        .iter_mut()
        .enumerate()
    {
        sanitize_scenario_preset(index, preset);
    }

    if settings
        .interactive_workshop
        .primary_scenario_widget_id
        .as_ref()
        .is_some_and(|widget_id| !widget_ids.contains(widget_id))
    {
        settings.interactive_workshop.primary_scenario_widget_id = None;
    }

    if settings
        .interactive_workshop
        .primary_agent_widget_id
        .as_ref()
        .is_some_and(|widget_id| !widget_ids.contains(widget_id))
    {
        settings.interactive_workshop.primary_agent_widget_id = None;
    }
}

fn sanitize_scenario_preset(index: usize, preset: &mut WorkshopScenarioPreset) {
    if preset.id.trim().is_empty() {
        preset.id = Uuid::now_v7().to_string();
    }

    if preset.label.trim().is_empty() {
        preset.label = format!("Scenario preset {}", index + 1);
    } else {
        preset.label = preset.label.trim().to_string();
    }

    preset.description = preset
        .description
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    preset.prompt_template = preset
        .prompt_template
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    preset.parameters = preset
        .parameters
        .iter()
        .filter_map(|(key, value)| {
            let key = key.trim();
            let value = value.trim();
            if key.is_empty() || value.is_empty() {
                None
            } else {
                Some((key.to_string(), value.to_string()))
            }
        })
        .collect();
}

fn collect_widget_ids(pages: &[AppPage]) -> std::collections::BTreeSet<String> {
    let mut ids = std::collections::BTreeSet::new();
    for page in pages {
        collect_widget_ids_recursive(&page.widgets, &mut ids);
    }
    ids
}

fn collect_widget_ids_recursive(
    widgets: &[WidgetDefinition],
    ids: &mut std::collections::BTreeSet<String>,
) {
    for widget in widgets {
        ids.insert(widget.id.clone());
        collect_widget_ids_recursive(&widget.children, ids);
    }
}

pub async fn load_app(state: &AppState, app_id: Uuid) -> ServiceResult<App> {
    let row = sqlx::query_as::<_, AppRow>(
		"SELECT id, name, slug, description, status, pages, theme, settings, template_key, created_by, published_version_id, created_at, updated_at
		 FROM apps
		 WHERE id = $1",
	)
	.bind(app_id)
	.fetch_optional(&state.db)
	.await
	.map_err(db_error)?;

    row.map(Into::into).ok_or_else(|| not_found("app"))
}

pub async fn load_template_by_key(state: &AppState, key: &str) -> ServiceResult<AppTemplate> {
    let row = sqlx::query_as::<_, AppTemplateRow>(
        "SELECT id, key, name, description, category, preview_image_url, definition, created_at
		 FROM app_templates
		 WHERE key = $1",
    )
    .bind(key)
    .fetch_optional(&state.db)
    .await
    .map_err(db_error)?;

    row.map(Into::into).ok_or_else(|| not_found("app template"))
}

pub async fn load_published_app(state: &AppState, slug: &str) -> ServiceResult<(App, AppVersion)> {
    let app_row = sqlx::query_as::<_, AppRow>(
		"SELECT id, name, slug, description, status, pages, theme, settings, template_key, created_by, published_version_id, created_at, updated_at
		 FROM apps
		 WHERE slug = $1 AND published_version_id IS NOT NULL",
	)
	.bind(slug)
	.fetch_optional(&state.db)
	.await
	.map_err(db_error)?;

    let app: App = app_row
        .map(Into::into)
        .ok_or_else(|| not_found("published app"))?;
    let version_id = app
        .published_version_id
        .ok_or_else(|| not_found("published app version"))?;

    let version_row = sqlx::query_as::<_, AppVersionRow>(
		"SELECT id, app_id, version_number, status, app_snapshot, notes, created_by, created_at, published_at
		 FROM app_versions
		 WHERE id = $1",
	)
	.bind(version_id)
	.fetch_optional(&state.db)
	.await
	.map_err(db_error)?;

    let version = version_row
        .map(Into::into)
        .ok_or_else(|| not_found("published app version"))?;

    Ok((app, version))
}

pub async fn persist_app(state: &AppState, app: &App) -> ServiceResult<App> {
    let row = sqlx::query_as::<_, AppRow>(
		"UPDATE apps
		 SET name = $2,
			 slug = $3,
			 description = $4,
			 status = $5,
			 pages = $6,
			 theme = $7,
			 settings = $8,
			 template_key = $9,
			 published_version_id = $10,
			 updated_at = NOW()
		 WHERE id = $1
		 RETURNING id, name, slug, description, status, pages, theme, settings, template_key, created_by, published_version_id, created_at, updated_at",
	)
	.bind(app.id)
	.bind(&app.name)
	.bind(&app.slug)
	.bind(&app.description)
	.bind(&app.status)
	.bind(Json(app.pages.clone()))
	.bind(Json(app.theme.clone()))
	.bind(Json(app.settings.clone()))
	.bind(&app.template_key)
	.bind(app.published_version_id)
	.fetch_one(&state.db)
	.await
	.map_err(db_error)?;

    Ok(row.into())
}
