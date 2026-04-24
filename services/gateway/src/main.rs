mod config;
mod middleware;
mod proxy;
mod routes;

use axum::{Router, middleware as axum_mw, routing::get};
use core_models::{health::HealthStatus, observability};
use tower_http::trace::TraceLayer;

#[tokio::main]
async fn main() {
    observability::init_tracing("gateway");

    let cfg = config::GatewayConfig::from_env().expect("failed to load config");

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .expect("failed to build HTTP client");
    let rate_limit_state =
        middleware::rate_limit::RateLimitState::new(&cfg.jwt_secret, cfg.rate_limit.clone());

    // Health check (unauthenticated)
    let health = Router::new().route(
        "/health",
        get(|| async { axum::Json(HealthStatus::ok("gateway")) }),
    );

    // API proxy routes
    let api = routes::v1::router(cfg.clone(), client, rate_limit_state);

    let app = Router::new()
        .merge(health)
        .merge(api)
        .layer(axum_mw::from_fn(middleware::request_id::request_id_layer))
        .layer(axum_mw::from_fn(middleware::audit::audit_layer))
        .layer(middleware::cors::cors_layer(&cfg.cors_origins))
        .layer(TraceLayer::new_for_http());

    let addr = format!("{}:{}", cfg.host, cfg.port);
    tracing::info!("starting gateway on {addr}");

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("failed to bind");

    axum::serve(listener, app).await.expect("server error");
}
