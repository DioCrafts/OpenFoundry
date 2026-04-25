use auth_middleware::JwtConfig;
use axum::{Router, middleware as axum_mw, routing::any};
use reqwest::Client;

use crate::config::GatewayConfig;
use crate::middleware::rate_limit::RateLimitState;
use crate::proxy::service_router::proxy_handler;

/// Build the /api/v1/* routes that proxy to backend services.
pub fn router(
    config: GatewayConfig,
    client: Client,
    jwt_config: JwtConfig,
    rate_limit_state: RateLimitState,
) -> Router {
    Router::new()
        .route("/api/v1/{*rest}", any(proxy_handler))
        .route_layer(axum_mw::from_fn_with_state(
            rate_limit_state,
            crate::middleware::rate_limit::rate_limit_layer,
        ))
        .with_state((config, client, jwt_config))
}
