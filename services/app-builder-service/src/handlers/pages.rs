use axum::{
    Json,
    extract::{Path, State},
};
use uuid::Uuid;

use crate::{
    AppState,
    handlers::{ServiceResult, bad_request, load_app, persist_app, sanitize_pages},
    models::{app::App, page::AppPage},
};

pub async fn create_page(
    State(state): State<AppState>,
    Path(app_id): Path<Uuid>,
    Json(page): Json<AppPage>,
) -> ServiceResult<Json<App>> {
    let mut app = load_app(&state, app_id).await?;
    app.pages.push(page);
    sanitize_pages(&mut app.pages, &mut app.settings);
    persist_app(&state, &app).await.map(Json)
}

pub async fn update_page(
    State(state): State<AppState>,
    Path((app_id, page_id)): Path<(Uuid, String)>,
    Json(mut page): Json<AppPage>,
) -> ServiceResult<Json<App>> {
    let mut app = load_app(&state, app_id).await?;
    let Some(index) = app
        .pages
        .iter()
        .position(|candidate| candidate.id == page_id)
    else {
        return Err((
            axum::http::StatusCode::NOT_FOUND,
            "page not found".to_string(),
        ));
    };

    page.id = page_id;
    app.pages[index] = page;
    sanitize_pages(&mut app.pages, &mut app.settings);
    persist_app(&state, &app).await.map(Json)
}

pub async fn delete_page(
    State(state): State<AppState>,
    Path((app_id, page_id)): Path<(Uuid, String)>,
) -> ServiceResult<Json<App>> {
    let mut app = load_app(&state, app_id).await?;
    if app.pages.len() <= 1 {
        return Err(bad_request("apps require at least one page"));
    }

    let previous_len = app.pages.len();
    app.pages.retain(|page| page.id != page_id);
    if app.pages.len() == previous_len {
        return Err((
            axum::http::StatusCode::NOT_FOUND,
            "page not found".to_string(),
        ));
    }

    sanitize_pages(&mut app.pages, &mut app.settings);
    persist_app(&state, &app).await.map(Json)
}
