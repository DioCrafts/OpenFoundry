use axum::extract::ws::{Message, WebSocket};
use axum::{
    Json,
    extract::{Query, State, WebSocketUpgrade},
    http::StatusCode,
    response::IntoResponse,
};
use chrono::Utc;
use uuid::Uuid;

use crate::{
    AppState,
    handlers::send::{latest_notifications, unread_count},
    models::notification::{NotificationEvent, WebSocketQuery, WebSocketTicketResponse},
};

const WS_TICKET_TTL_SECS: i64 = 90;

pub async fn issue_ws_ticket(
    State(state): State<AppState>,
    auth_middleware::layer::AuthUser(claims): auth_middleware::layer::AuthUser,
) -> impl IntoResponse {
    let now = Utc::now().timestamp();
    let ticket_claims = auth_middleware::Claims {
        sub: claims.sub,
        iat: now,
        exp: now + WS_TICKET_TTL_SECS,
        iss: state.jwt_config.issuer().map(str::to_string),
        aud: state.jwt_config.audience().map(str::to_string),
        jti: Uuid::now_v7(),
        email: claims.email,
        name: claims.name,
        roles: vec![],
        permissions: vec![],
        org_id: claims.org_id,
        attributes: serde_json::json!({}),
        auth_methods: vec!["ws_ticket".to_string()],
        token_use: Some("ws_ticket".to_string()),
        api_key_id: None,
        session_kind: claims.session_kind,
        session_scope: claims.session_scope,
    };

    match auth_middleware::jwt::encode_token(&state.jwt_config, &ticket_claims) {
        Ok(ticket) => Json(WebSocketTicketResponse {
            ticket,
            expires_in: WS_TICKET_TTL_SECS,
        })
        .into_response(),
        Err(error) => {
            tracing::error!("failed to issue websocket ticket: {error}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn notifications_ws(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Query(params): Query<WebSocketQuery>,
) -> impl IntoResponse {
    let claims = match auth_middleware::jwt::decode_token(&state.jwt_config, &params.ticket) {
        Ok(claims) if claims.token_use.as_deref() == Some("ws_ticket") => claims,
        Err(_) => return StatusCode::UNAUTHORIZED.into_response(),
        _ => return StatusCode::UNAUTHORIZED.into_response(),
    };

    ws.on_upgrade(move |socket| websocket_loop(socket, state, claims.sub))
}

async fn websocket_loop(mut socket: WebSocket, state: AppState, user_id: uuid::Uuid) {
    let notifications = latest_notifications(&state, user_id, 20)
        .await
        .unwrap_or_default();
    let unread = unread_count(&state, Some(user_id)).await.unwrap_or(0);
    let snapshot = serde_json::json!({
        "kind": "snapshot",
        "data": notifications,
        "unread_count": unread,
    });

    if socket
        .send(Message::Text(snapshot.to_string().into()))
        .await
        .is_err()
    {
        return;
    }

    let mut rx = state.notification_bus.subscribe();
    while let Ok(event) = rx.recv().await {
        if !targets_user(&event, user_id) {
            continue;
        }

        let payload = match serde_json::to_string(&event) {
            Ok(payload) => payload,
            Err(_) => continue,
        };

        if socket.send(Message::Text(payload.into())).await.is_err() {
            break;
        }
    }
}

fn targets_user(event: &NotificationEvent, user_id: uuid::Uuid) -> bool {
    event
        .user_id
        .map(|target| target == user_id)
        .unwrap_or(true)
}
