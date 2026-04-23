use axum::http::{HeaderValue, Method};
use tower_http::cors::CorsLayer;

pub fn cors_layer(origins: &[String]) -> CorsLayer {
    let layer = CorsLayer::new()
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers(tower_http::cors::Any)
        .allow_credentials(true)
        .max_age(std::time::Duration::from_secs(3600));

    if origins.is_empty() {
        layer.allow_origin(tower_http::cors::Any)
    } else {
        let origins: Vec<HeaderValue> = origins
            .iter()
            .filter_map(|o| o.parse().ok())
            .collect();
        layer.allow_origin(origins)
    }
}
