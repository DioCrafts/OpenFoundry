use axum::{
    extract::Request,
    http::{StatusCode, header::AUTHORIZATION},
    middleware::Next,
    response::{IntoResponse, Response},
};

use crate::{claims::Claims, jwt};

/// Axum middleware layer that extracts and validates JWT from the Authorization header.
/// On success, inserts `Claims` into request extensions for downstream handlers.
///
/// Usage:
/// ```ignore
/// let app = Router::new()
///     .route("/protected", get(handler))
///     .layer(axum::middleware::from_fn_with_state(jwt_config.clone(), auth_layer));
/// ```
pub async fn auth_layer(
    axum::extract::State(config): axum::extract::State<jwt::JwtConfig>,
    mut req: Request,
    next: Next,
) -> Response {
    let token = req
        .headers()
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));

    let Some(token) = token else {
        return (StatusCode::UNAUTHORIZED, "missing Bearer token").into_response();
    };

    match jwt::decode_token(&config, token) {
        Ok(claims) => {
            req.extensions_mut().insert(claims);
            next.run(req).await
        }
        Err(jwt::JwtError::Expired) => (StatusCode::UNAUTHORIZED, "token expired").into_response(),
        Err(e) => {
            tracing::warn!("JWT validation failed: {e}");
            (StatusCode::UNAUTHORIZED, "invalid token").into_response()
        }
    }
}

/// Axum extractor — extracts `Claims` from request extensions.
/// Use in handlers after the auth layer:
/// ```ignore
/// async fn handler(claims: AuthUser) -> impl IntoResponse { ... }
/// ```
#[derive(Debug, Clone)]
pub struct AuthUser(pub Claims);

impl<S: Send + Sync> axum::extract::FromRequestParts<S> for AuthUser {
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<Claims>()
            .cloned()
            .map(AuthUser)
            .ok_or((StatusCode::UNAUTHORIZED, "not authenticated"))
    }
}
