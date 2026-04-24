use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use serde_json::json;

use crate::{
    AppState,
    models::subscription::{NotificationPreference, UpdateNotificationPreferenceRequest},
};

pub async fn get_preferences(
    State(state): State<AppState>,
    auth_middleware::layer::AuthUser(claims): auth_middleware::layer::AuthUser,
) -> impl IntoResponse {
    match load_or_default_preferences(&state, claims.sub).await {
        Ok(preferences) => Json(preferences).into_response(),
        Err(error) => {
            tracing::error!("get notification preferences failed: {error}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn update_preferences(
    State(state): State<AppState>,
    auth_middleware::layer::AuthUser(claims): auth_middleware::layer::AuthUser,
    Json(body): Json<UpdateNotificationPreferenceRequest>,
) -> impl IntoResponse {
    let current = match load_or_default_preferences(&state, claims.sub).await {
        Ok(preferences) => preferences,
        Err(error) => {
            tracing::error!("load current notification preferences failed: {error}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let updated = sqlx::query_as::<_, NotificationPreference>(
		r#"INSERT INTO notification_preferences (
			   user_id, in_app_enabled, email_enabled, email_address, slack_webhook_url, teams_webhook_url, digest_frequency, quiet_hours
		   )
		   VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
		   ON CONFLICT (user_id)
		   DO UPDATE SET
			   in_app_enabled = EXCLUDED.in_app_enabled,
			   email_enabled = EXCLUDED.email_enabled,
			   email_address = EXCLUDED.email_address,
			   slack_webhook_url = EXCLUDED.slack_webhook_url,
			   teams_webhook_url = EXCLUDED.teams_webhook_url,
			   digest_frequency = EXCLUDED.digest_frequency,
			   quiet_hours = EXCLUDED.quiet_hours,
			   updated_at = NOW()
		   RETURNING *"#,
	)
	.bind(claims.sub)
	.bind(body.in_app_enabled.unwrap_or(current.in_app_enabled))
	.bind(body.email_enabled.unwrap_or(current.email_enabled))
	.bind(body.email_address.or(current.email_address))
	.bind(body.slack_webhook_url.or(current.slack_webhook_url))
	.bind(body.teams_webhook_url.or(current.teams_webhook_url))
	.bind(body.digest_frequency.unwrap_or(current.digest_frequency))
	.bind(body.quiet_hours.unwrap_or(current.quiet_hours))
	.fetch_one(&state.db)
	.await;

    match updated {
        Ok(preferences) => Json(preferences).into_response(),
        Err(error) => {
            tracing::error!("update notification preferences failed: {error}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": error.to_string() })),
            )
                .into_response()
        }
    }
}

pub async fn load_or_default_preferences(
    state: &AppState,
    user_id: uuid::Uuid,
) -> Result<NotificationPreference, sqlx::Error> {
    let existing = sqlx::query_as::<_, NotificationPreference>(
        r#"SELECT * FROM notification_preferences WHERE user_id = $1"#,
    )
    .bind(user_id)
    .fetch_optional(&state.db)
    .await?;

    if let Some(existing) = existing {
        Ok(existing)
    } else {
        Ok(NotificationPreference {
            user_id,
            in_app_enabled: true,
            email_enabled: false,
            email_address: None,
            slack_webhook_url: None,
            teams_webhook_url: None,
            digest_frequency: "instant".to_string(),
            quiet_hours: json!({}),
            updated_at: chrono::Utc::now(),
        })
    }
}
