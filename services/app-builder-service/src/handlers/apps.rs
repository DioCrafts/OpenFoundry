use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use sqlx::types::Json as SqlJson;
use uuid::Uuid;

use crate::{
    AppState,
    domain::templating::instantiate_template_definition,
    handlers::{
        ServiceResult, bad_request, db_error, load_app, load_template_by_key, normalize_slug,
        persist_app, sanitize_pages,
    },
    models::app::{
        App, AppRow, AppSummary, AppTemplateRow, CreateAppRequest, ListAppTemplatesResponse,
        ListAppsQuery, ListAppsResponse, UpdateAppRequest,
    },
};

pub async fn list_apps(
    State(state): State<AppState>,
    Query(query): Query<ListAppsQuery>,
) -> ServiceResult<Json<ListAppsResponse>> {
    let page = query.page.max(1);
    let per_page = query.per_page.clamp(1, 100);
    let offset = (page - 1) * per_page;
    let search_pattern = format!("%{}%", query.search.unwrap_or_default());
    let status_filter = query.status.unwrap_or_default();

    let total: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)
		 FROM apps
		 WHERE (name ILIKE $1 OR slug ILIKE $1 OR description ILIKE $1)
		   AND ($2 = '' OR status = $2)",
    )
    .bind(&search_pattern)
    .bind(&status_filter)
    .fetch_one(&state.db)
    .await
    .map_err(db_error)?;

    let rows = sqlx::query_as::<_, AppRow>(
		"SELECT id, name, slug, description, status, pages, theme, settings, template_key, created_by, published_version_id, created_at, updated_at
		 FROM apps
		 WHERE (name ILIKE $1 OR slug ILIKE $1 OR description ILIKE $1)
		   AND ($2 = '' OR status = $2)
		 ORDER BY updated_at DESC
		 LIMIT $3 OFFSET $4",
	)
	.bind(&search_pattern)
	.bind(&status_filter)
	.bind(per_page)
	.bind(offset)
	.fetch_all(&state.db)
	.await
	.map_err(db_error)?;

    let data = rows
        .into_iter()
        .map(App::from)
        .map(|app| AppSummary::from(&app))
        .collect();

    Ok(Json(ListAppsResponse { data, total }))
}

pub async fn list_templates(
    State(state): State<AppState>,
) -> ServiceResult<Json<ListAppTemplatesResponse>> {
    let rows = sqlx::query_as::<_, AppTemplateRow>(
        "SELECT id, key, name, description, category, preview_image_url, definition, created_at
		 FROM app_templates
		 ORDER BY category, name",
    )
    .fetch_all(&state.db)
    .await
    .map_err(db_error)?;

    Ok(Json(ListAppTemplatesResponse {
        data: rows.into_iter().map(Into::into).collect(),
    }))
}

pub async fn create_app(
    State(state): State<AppState>,
    Json(request): Json<CreateAppRequest>,
) -> ServiceResult<Json<App>> {
    create_app_impl(&state, request, false).await.map(Json)
}

pub async fn create_from_template(
    State(state): State<AppState>,
    Json(request): Json<CreateAppRequest>,
) -> ServiceResult<Json<App>> {
    create_app_impl(&state, request, true).await.map(Json)
}

pub async fn get_app(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ServiceResult<Json<App>> {
    load_app(&state, id).await.map(Json)
}

pub async fn update_app(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(request): Json<UpdateAppRequest>,
) -> ServiceResult<Json<App>> {
    let mut app = load_app(&state, id).await?;

    if let Some(name) = request.name {
        let name = name.trim();
        if name.is_empty() {
            return Err(bad_request("app name cannot be empty"));
        }
        app.name = name.to_string();
    }

    if let Some(slug) = request.slug {
        app.slug = normalize_slug(Some(&slug), &app.name);
    }

    if let Some(description) = request.description {
        app.description = description;
    }

    if let Some(status) = request.status {
        app.status = status;
    }

    if let Some(pages) = request.pages {
        app.pages = pages;
    }

    if let Some(theme) = request.theme {
        app.theme = theme;
    }

    if let Some(settings) = request.settings {
        app.settings = settings;
    }

    if let Some(template_key) = request.template_key {
        if template_key.trim().is_empty() {
            app.template_key = None;
        } else {
            load_template_by_key(&state, &template_key).await?;
            app.template_key = Some(template_key);
        }
    }

    sanitize_pages(&mut app.pages, &mut app.settings);

    persist_app(&state, &app).await.map(Json)
}

pub async fn delete_app(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ServiceResult<StatusCode> {
    let result = sqlx::query("DELETE FROM apps WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await
        .map_err(db_error)?;

    if result.rows_affected() == 0 {
        return Err((StatusCode::NOT_FOUND, "app not found".to_string()));
    }

    Ok(StatusCode::NO_CONTENT)
}

async fn create_app_impl(
    state: &AppState,
    request: CreateAppRequest,
    require_template: bool,
) -> ServiceResult<App> {
    let CreateAppRequest {
        name,
        slug,
        description,
        status,
        pages,
        theme,
        settings,
        template_key,
    } = request;

    let name = name.trim().to_string();
    if name.is_empty() {
        return Err(bad_request("app name cannot be empty"));
    }

    let template = match template_key.as_deref() {
        Some(key) => Some(load_template_by_key(state, key).await?),
        None if require_template => return Err(bad_request("template_key is required")),
        None => None,
    };

    let instantiated_template = template
        .as_ref()
        .map(|template| instantiate_template_definition(&template.definition));

    let mut pages = pages
        .or_else(|| {
            instantiated_template
                .as_ref()
                .map(|definition| definition.pages.clone())
        })
        .unwrap_or_else(|| vec![crate::models::page::AppPage::default()]);
    let theme = theme
        .or_else(|| {
            instantiated_template
                .as_ref()
                .map(|definition| definition.theme.clone())
        })
        .unwrap_or_default();
    let mut settings = settings
        .or_else(|| {
            instantiated_template
                .as_ref()
                .map(|definition| definition.settings.clone())
        })
        .unwrap_or_default();
    sanitize_pages(&mut pages, &mut settings);

    let description = description
        .or_else(|| template.as_ref().map(|value| value.description.clone()))
        .unwrap_or_default();
    let status = status.unwrap_or_else(|| "draft".to_string());
    let slug = normalize_slug(slug.as_deref(), &name);
    let template_key = template.map(|value| value.key);

    let row = sqlx::query_as::<_, AppRow>(
		"INSERT INTO apps (
			id, name, slug, description, status, pages, theme, settings, template_key, created_by
		 )
		 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
		 RETURNING id, name, slug, description, status, pages, theme, settings, template_key, created_by, published_version_id, created_at, updated_at",
	)
	.bind(Uuid::now_v7())
	.bind(&name)
	.bind(&slug)
	.bind(&description)
	.bind(&status)
	.bind(SqlJson(pages))
	.bind(SqlJson(theme))
	.bind(SqlJson(settings))
	.bind(&template_key)
	.bind(Option::<Uuid>::None)
	.fetch_one(&state.db)
	.await
	.map_err(db_error)?;

    Ok(row.into())
}
