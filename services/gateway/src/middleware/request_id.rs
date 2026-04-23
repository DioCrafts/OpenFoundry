use axum::{
    extract::Request,
    http::{HeaderName, HeaderValue},
    middleware::Next,
    response::Response,
};
use uuid::Uuid;

static X_REQUEST_ID: HeaderName = HeaderName::from_static("x-request-id");

/// Middleware that ensures every request has an `x-request-id` header.
/// If one isn't present, a new UUID v7 is generated.
pub async fn request_id_layer(mut req: Request, next: Next) -> Response {
    if !req.headers().contains_key(&X_REQUEST_ID) {
        let id = Uuid::now_v7().to_string();
        if let Ok(val) = HeaderValue::from_str(&id) {
            req.headers_mut().insert(X_REQUEST_ID.clone(), val);
        }
    }

    let request_id = req
        .headers()
        .get(&X_REQUEST_ID)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .to_string();

    let mut response = next.run(req).await;

    if let Ok(val) = HeaderValue::from_str(&request_id) {
        response.headers_mut().insert(X_REQUEST_ID.clone(), val);
    }

    response
}
