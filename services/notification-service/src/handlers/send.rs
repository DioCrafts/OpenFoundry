use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use lettre::{AsyncTransport, Message, message::Mailbox};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::{
    AppState,
    models::{
        notification::{
            NotificationDelivery, NotificationEvent, NotificationRecord, SendNotificationRequest,
        },
        subscription::NotificationPreference,
    },
};

pub async fn internal_send_notification(
    State(state): State<AppState>,
    Json(body): Json<SendNotificationRequest>,
) -> impl IntoResponse {
    match create_notification(&state, body).await {
        Ok(notification) => (StatusCode::CREATED, Json(notification)).into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": error })),
        )
            .into_response(),
    }
}

pub async fn send_notification(
    State(state): State<AppState>,
    auth_middleware::layer::AuthUser(claims): auth_middleware::layer::AuthUser,
    Json(mut body): Json<SendNotificationRequest>,
) -> impl IntoResponse {
    if body.user_id.is_none() {
        body.user_id = Some(claims.sub);
    }

    match create_notification(&state, body).await {
        Ok(notification) => (StatusCode::CREATED, Json(notification)).into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": error })),
        )
            .into_response(),
    }
}

pub async fn create_notification(
    state: &AppState,
    body: SendNotificationRequest,
) -> Result<NotificationRecord, String> {
    let channels = body.channels.unwrap_or_else(|| vec!["in_app".to_string()]);
    let metadata = body.metadata.unwrap_or_else(|| json!({}));

    let notification = sqlx::query_as::<_, NotificationRecord>(
        r#"INSERT INTO notifications (
			   id, user_id, title, body, category, severity, status, channels, metadata
		   )
		   VALUES ($1, $2, $3, $4, $5, $6, 'unread', $7, $8)
		   RETURNING *"#,
    )
    .bind(Uuid::now_v7())
    .bind(body.user_id)
    .bind(&body.title)
    .bind(&body.body)
    .bind(body.category.as_deref().unwrap_or("system"))
    .bind(body.severity.as_deref().unwrap_or("info"))
    .bind(serde_json::to_value(&channels).map_err(|error| error.to_string())?)
    .bind(&metadata)
    .fetch_one(&state.db)
    .await
    .map_err(|error| error.to_string())?;

    let preference = load_preferences(state, body.user_id).await?;

    for channel in channels {
        record_delivery(
            state,
            &notification,
            &channel,
            dispatch_channel(state, &notification, preference.as_ref(), &channel).await,
        )
        .await?;
    }

    let unread_count = unread_count(state, notification.user_id).await.unwrap_or(0);
    let _ = state.notification_bus.send(NotificationEvent {
        kind: "notification.created".to_string(),
        user_id: notification.user_id,
        notification: Some(notification.clone()),
        unread_count,
    });

    Ok(notification)
}

pub async fn unread_count(state: &AppState, user_id: Option<Uuid>) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar::<_, i64>(
        r#"SELECT COUNT(*) FROM notifications
		   WHERE status = 'unread'
			 AND (($1::UUID IS NULL AND user_id IS NULL) OR user_id = $1 OR user_id IS NULL)"#,
    )
    .bind(user_id)
    .fetch_one(&state.db)
    .await
}

pub async fn latest_notifications(
    state: &AppState,
    user_id: Uuid,
    limit: i64,
) -> Result<Vec<NotificationRecord>, sqlx::Error> {
    sqlx::query_as::<_, NotificationRecord>(
        r#"SELECT * FROM notifications
		   WHERE user_id = $1 OR user_id IS NULL
		   ORDER BY created_at DESC
		   LIMIT $2"#,
    )
    .bind(user_id)
    .bind(limit)
    .fetch_all(&state.db)
    .await
}

async fn load_preferences(
    state: &AppState,
    user_id: Option<Uuid>,
) -> Result<Option<NotificationPreference>, String> {
    let Some(user_id) = user_id else {
        return Ok(None);
    };

    sqlx::query_as::<_, NotificationPreference>(
        r#"SELECT * FROM notification_preferences WHERE user_id = $1"#,
    )
    .bind(user_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|error| error.to_string())
}

async fn dispatch_channel(
    state: &AppState,
    notification: &NotificationRecord,
    preference: Option<&NotificationPreference>,
    channel: &str,
) -> DeliveryResult {
    match channel {
        "in_app" => DeliveryResult::sent("delivered to in-app center"),
        "email" => {
            if preference.map(|pref| pref.email_enabled).unwrap_or(false) {
                if let Some(address) = preference.and_then(|pref| pref.email_address.as_deref()) {
                    send_email(state, notification, address).await
                } else {
                    DeliveryResult::skipped("email address not configured")
                }
            } else {
                DeliveryResult::skipped("email channel disabled")
            }
        }
        "slack" => {
            if let Some(url) = preference.and_then(|pref| pref.slack_webhook_url.as_deref()) {
                post_webhook(
                    state,
                    url,
                    json!({ "text": format!("{}\n{}", notification.title, notification.body) }),
                )
                .await
            } else {
                DeliveryResult::skipped("slack webhook not configured")
            }
        }
        "teams" => {
            if let Some(url) = preference.and_then(|pref| pref.teams_webhook_url.as_deref()) {
                post_webhook(
                    state,
                    url,
                    json!({ "text": format!("{}\n{}", notification.title, notification.body) }),
                )
                .await
            } else {
                DeliveryResult::skipped("teams webhook not configured")
            }
        }
        other => DeliveryResult::skipped(format!("unknown channel '{other}'")),
    }
}

async fn send_email(
    state: &AppState,
    notification: &NotificationRecord,
    address: &str,
) -> DeliveryResult {
    let Some(sender) = state.email_sender.as_ref() else {
        return DeliveryResult::skipped("SMTP adapter not configured");
    };
    let Some(from) = state.email_from.as_ref() else {
        return DeliveryResult::skipped("SMTP from address not configured");
    };

    let to: Mailbox = match address.parse() {
        Ok(mailbox) => mailbox,
        Err(error) => return DeliveryResult::failed(error.to_string()),
    };

    let message = match Message::builder()
        .from(from.clone())
        .to(to)
        .subject(notification.title.clone())
        .body(notification.body.clone())
    {
        Ok(message) => message,
        Err(error) => return DeliveryResult::failed(error.to_string()),
    };

    match sender.send(message).await {
        Ok(_) => DeliveryResult::sent(format!("email delivered to {address}")),
        Err(error) => DeliveryResult::failed(error.to_string()),
    }
}

async fn post_webhook(state: &AppState, url: &str, payload: Value) -> DeliveryResult {
    match state.http_client.post(url).json(&payload).send().await {
        Ok(response) if response.status().is_success() => DeliveryResult::sent(format!(
            "webhook delivered with status {}",
            response.status()
        )),
        Ok(response) => {
            DeliveryResult::failed(format!("webhook returned status {}", response.status()))
        }
        Err(error) => DeliveryResult::failed(error.to_string()),
    }
}

async fn record_delivery(
    state: &AppState,
    notification: &NotificationRecord,
    channel: &str,
    result: DeliveryResult,
) -> Result<NotificationDelivery, String> {
    sqlx::query_as::<_, NotificationDelivery>(
        r#"INSERT INTO notification_deliveries (id, notification_id, channel, status, response)
		   VALUES ($1, $2, $3, $4, $5)
		   RETURNING *"#,
    )
    .bind(Uuid::now_v7())
    .bind(notification.id)
    .bind(channel)
    .bind(result.status)
    .bind(result.response)
    .fetch_one(&state.db)
    .await
    .map_err(|error| error.to_string())
}

struct DeliveryResult {
    status: &'static str,
    response: Option<String>,
}

impl DeliveryResult {
    fn sent(response: impl Into<String>) -> Self {
        Self {
            status: "sent",
            response: Some(response.into()),
        }
    }

    fn skipped(response: impl Into<String>) -> Self {
        Self {
            status: "skipped",
            response: Some(response.into()),
        }
    }

    fn failed(response: impl Into<String>) -> Self {
        Self {
            status: "failed",
            response: Some(response.into()),
        }
    }
}
