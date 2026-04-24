use axum::{
    Json,
    extract::{Path, State},
};
use sqlx::types::Json as SqlJson;
use uuid::Uuid;

use crate::{
    AppState,
    handlers::{ServiceResult, db_error, load_app},
    models::version::{AppVersion, AppVersionRow, ListAppVersionsResponse, PublishAppRequest},
};

pub async fn list_versions(
    State(state): State<AppState>,
    Path(app_id): Path<Uuid>,
) -> ServiceResult<Json<ListAppVersionsResponse>> {
    load_app(&state, app_id).await?;

    let rows = sqlx::query_as::<_, AppVersionRow>(
		"SELECT id, app_id, version_number, status, app_snapshot, notes, created_by, created_at, published_at
		 FROM app_versions
		 WHERE app_id = $1
		 ORDER BY version_number DESC",
	)
	.bind(app_id)
	.fetch_all(&state.db)
	.await
	.map_err(db_error)?;

    Ok(Json(ListAppVersionsResponse {
        data: rows.into_iter().map(Into::into).collect(),
    }))
}

pub async fn publish_app(
    State(state): State<AppState>,
    Path(app_id): Path<Uuid>,
    Json(request): Json<PublishAppRequest>,
) -> ServiceResult<Json<AppVersion>> {
    let app = load_app(&state, app_id).await?;
    let snapshot = app.snapshot();
    let version_id = Uuid::now_v7();
    let notes = request.notes.unwrap_or_default();

    let mut transaction = state.db.begin().await.map_err(db_error)?;

    let version_number: i32 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(version_number), 0) + 1
		 FROM app_versions
		 WHERE app_id = $1",
    )
    .bind(app_id)
    .fetch_one(&mut *transaction)
    .await
    .map_err(db_error)?;

    let version = sqlx::query_as::<_, AppVersionRow>(
		"INSERT INTO app_versions (
			id, app_id, version_number, status, app_snapshot, notes, created_by, published_at
		 )
		 VALUES ($1, $2, $3, $4, $5, $6, $7, NOW())
		 RETURNING id, app_id, version_number, status, app_snapshot, notes, created_by, created_at, published_at",
	)
	.bind(version_id)
	.bind(app_id)
	.bind(version_number)
	.bind("published")
	.bind(SqlJson(snapshot))
	.bind(notes)
	.bind(Option::<Uuid>::None)
	.fetch_one(&mut *transaction)
	.await
	.map_err(db_error)?;

    sqlx::query(
        "UPDATE apps
		 SET published_version_id = $2,
			 status = 'published',
			 updated_at = NOW()
		 WHERE id = $1",
    )
    .bind(app_id)
    .bind(version_id)
    .execute(&mut *transaction)
    .await
    .map_err(db_error)?;

    transaction.commit().await.map_err(db_error)?;

    Ok(Json(version.into()))
}
