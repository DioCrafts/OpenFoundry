use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde_json::json;
use uuid::Uuid;

use crate::{
    AppState,
    handlers::send::unread_count,
    models::notification::{ListNotificationsQuery, NotificationEvent, NotificationRecord},
};

pub async fn list_notifications(
    State(state): State<AppState>,
    auth_middleware::layer::AuthUser(claims): auth_middleware::layer::AuthUser,
    Query(params): Query<ListNotificationsQuery>,
) -> impl IntoResponse {
    let limit = params.limit.unwrap_or(20).clamp(1, 100);

    let notifications = sqlx::query_as::<_, NotificationRecord>(
        r#"SELECT * FROM notifications
		   WHERE (user_id = $1 OR user_id IS NULL)
			 AND ($2::TEXT IS NULL OR status = $2)
		   ORDER BY created_at DESC
		   LIMIT $3"#,
    )
    .bind(claims.sub)
    .bind(&params.status)
    .bind(limit)
    .fetch_all(&state.db)
    .await;

    match notifications {
        Ok(data) => Json(json!({
            "data": data,
            "unread_count": unread_count(&state, Some(claims.sub)).await.unwrap_or(0),
        }))
        .into_response(),
        Err(error) => {
            tracing::error!("list notifications failed: {error}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn mark_read(
    State(state): State<AppState>,
    Path(notification_id): Path<Uuid>,
    auth_middleware::layer::AuthUser(claims): auth_middleware::layer::AuthUser,
) -> impl IntoResponse {
    let notification = sqlx::query_as::<_, NotificationRecord>(
        r#"UPDATE notifications
		   SET status = 'read', read_at = NOW()
		   WHERE id = $1 AND (user_id = $2 OR user_id IS NULL)
		   RETURNING *"#,
    )
    .bind(notification_id)
    .bind(claims.sub)
    .fetch_optional(&state.db)
    .await;

    match notification {
        Ok(Some(notification)) => {
            let unread = unread_count(&state, Some(claims.sub)).await.unwrap_or(0);
            let _ = state.notification_bus.send(NotificationEvent {
                kind: "notification.read".to_string(),
                user_id: Some(claims.sub),
                notification: Some(notification.clone()),
                unread_count: unread,
            });
            Json(json!({ "notification": notification, "unread_count": unread })).into_response()
        }
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(error) => {
            tracing::error!("mark notification read failed: {error}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn mark_all_read(
    State(state): State<AppState>,
    auth_middleware::layer::AuthUser(claims): auth_middleware::layer::AuthUser,
) -> impl IntoResponse {
    match sqlx::query(
        r#"UPDATE notifications
		   SET status = 'read', read_at = NOW()
		   WHERE status = 'unread' AND (user_id = $1 OR user_id IS NULL)"#,
    )
    .bind(claims.sub)
    .execute(&state.db)
    .await
    {
        Ok(_) => {
            let _ = state.notification_bus.send(NotificationEvent {
                kind: "notification.read_all".to_string(),
                user_id: Some(claims.sub),
                notification: None,
                unread_count: 0,
            });
            Json(json!({ "unread_count": 0 })).into_response()
        }
        Err(error) => {
            tracing::error!("mark all notifications read failed: {error}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
