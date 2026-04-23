use axum::{extract::State, Json};

use crate::{models::widget_type::WidgetCatalogItem, AppState};

pub async fn list_widget_catalog(
	State(_state): State<AppState>,
) -> Json<Vec<WidgetCatalogItem>> {
	Json(crate::models::widget_type::widget_catalog())
}
