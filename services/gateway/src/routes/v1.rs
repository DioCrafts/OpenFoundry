use axum::{Router, routing::any};
use reqwest::Client;

use crate::config::GatewayConfig;
use crate::proxy::service_router::proxy_handler;

/// Build the /api/v1/* routes that proxy to backend services.
pub fn router(config: GatewayConfig, client: Client) -> Router {
    Router::new()
        .route("/api/v1/{*rest}", any(proxy_handler))
        .with_state((config, client))
}
