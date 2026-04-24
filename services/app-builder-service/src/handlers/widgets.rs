use axum::{Json, extract::State};

use crate::{AppState, models::widget_type::WidgetCatalogItem};

pub async fn list_widget_catalog(State(_state): State<AppState>) -> Json<Vec<WidgetCatalogItem>> {
    Json(crate::models::widget_type::widget_catalog())
}
