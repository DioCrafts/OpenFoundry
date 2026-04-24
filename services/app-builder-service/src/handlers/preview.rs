use axum::{
    Json,
    extract::{Path, State},
};
use uuid::Uuid;

use crate::{
    AppState,
    domain::{embedding::build_embed_info, renderer},
    handlers::{ServiceResult, load_app, load_published_app},
    models::app::{AppEmbedInfo, AppPreviewResponse, PublishedAppResponse},
};

pub async fn preview_app(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ServiceResult<Json<AppPreviewResponse>> {
    let app = load_app(&state, id).await?;
    Ok(Json(renderer::build_preview_response(
        app,
        &state.public_base_url,
    )))
}

pub async fn get_published_app(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> ServiceResult<Json<PublishedAppResponse>> {
    let (app, version) = load_published_app(&state, &slug).await?;
    Ok(Json(renderer::build_published_response(
        app,
        version,
        &state.public_base_url,
    )))
}

pub async fn get_embed_info(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> ServiceResult<Json<AppEmbedInfo>> {
    let (app, _) = load_published_app(&state, &slug).await?;
    Ok(Json(build_embed_info(&state.public_base_url, &app.slug)))
}
