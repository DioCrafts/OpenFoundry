use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};

use crate::claims::Claims;

/// Well-known role names.
pub mod roles {
    pub const ADMIN: &str = "admin";
    pub const EDITOR: &str = "editor";
    pub const VIEWER: &str = "viewer";
    pub const SERVICE: &str = "service";
}

/// Axum middleware that requires the user to have at least one of the given roles.
/// The `Claims` must already be inserted into extensions by the auth layer.
pub fn require_roles(
    required: &'static [&'static str],
) -> impl Fn(Request, Next) -> std::pin::Pin<Box<dyn std::future::Future<Output = Response> + Send>>
       + Clone
       + Send {
    move |req: Request, next: Next| {
        let required = required;
        Box::pin(async move {
            let claims = req.extensions().get::<Claims>();
            match claims {
                Some(c) if c.has_any_role(required) => next.run(req).await,
                Some(_) => (
                    StatusCode::FORBIDDEN,
                    format!("requires one of: {}", required.join(", ")),
                )
                    .into_response(),
                None => (StatusCode::UNAUTHORIZED, "missing authentication").into_response(),
            }
        })
    }
}

/// Axum middleware that requires the `admin` role.
pub fn require_admin(
) -> impl Fn(Request, Next) -> std::pin::Pin<Box<dyn std::future::Future<Output = Response> + Send>>
       + Clone
       + Send {
    require_roles(&[roles::ADMIN])
}

/// Axum middleware that requires the user to have at least one permission key.
pub fn require_permissions(
    required: &'static [&'static str],
) -> impl Fn(Request, Next) -> std::pin::Pin<Box<dyn std::future::Future<Output = Response> + Send>>
       + Clone
       + Send {
    move |req: Request, next: Next| {
        let required = required;
        Box::pin(async move {
            let claims = req.extensions().get::<Claims>();
            match claims {
                Some(c) if required.iter().any(|permission| c.has_permission_key(permission)) => {
                    next.run(req).await
                }
                Some(_) => (
                    StatusCode::FORBIDDEN,
                    format!("requires one of: {}", required.join(", ")),
                )
                    .into_response(),
                None => (StatusCode::UNAUTHORIZED, "missing authentication").into_response(),
            }
        })
    }
}
