mod config;
mod connectors;
mod domain;
mod handlers;
mod models;

use auth_middleware::jwt::JwtConfig;
use axum::{
    Router, middleware,
    routing::{delete, get, post},
};
use core_models::{health::HealthStatus, observability};
use sqlx::postgres::PgPoolOptions;
use std::time::Duration;

#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::PgPool,
    pub jwt_config: JwtConfig,
    pub http_client: reqwest::Client,
    pub dataset_service_url: String,
}

impl axum::extract::FromRef<AppState> for JwtConfig {
    fn from_ref(state: &AppState) -> Self {
        state.jwt_config.clone()
    }
}

#[tokio::main]
async fn main() {
    observability::init_tracing("data-connector");

    let cfg = config::AppConfig::from_env().expect("failed to load config");

    let pool = PgPoolOptions::new()
        .max_connections(20)
        .connect(&cfg.database_url)
        .await
        .expect("failed to connect to database");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("failed to run migrations");

    let jwt_config = JwtConfig::new(&cfg.jwt_secret);
    let http_client = reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .expect("failed to build HTTP client");

    let state = AppState {
        db: pool,
        jwt_config: jwt_config.clone(),
        http_client,
        dataset_service_url: cfg.dataset_service_url.clone(),
    };

    let scheduler_state = state.clone();
    tokio::spawn(async move {
        crate::domain::scheduler::run_scheduler(
            scheduler_state,
            Duration::from_secs(cfg.sync_poll_interval_secs.max(1)),
        )
        .await;
    });

    let public = Router::new().route(
        "/health",
        get(|| async { axum::Json(HealthStatus::ok("data-connector")) }),
    );

    let protected = Router::new()
        .route(
            "/api/v1/connections",
            post(handlers::connections::create_connection),
        )
        .route(
            "/api/v1/connections",
            get(handlers::connections::list_connections),
        )
        .route(
            "/api/v1/connections/{id}",
            get(handlers::connections::get_connection),
        )
        .route(
            "/api/v1/connections/{id}",
            delete(handlers::connections::delete_connection),
        )
        .route(
            "/api/v1/connections/{id}/test",
            post(handlers::connections::test_connection),
        )
        .route(
            "/api/v1/connections/{id}/sync",
            post(handlers::sync_ops::sync_connection),
        )
        .route(
            "/api/v1/connections/{id}/sync-jobs",
            get(handlers::sync_ops::list_sync_jobs),
        )
        .layer(middleware::from_fn_with_state(
            jwt_config,
            auth_middleware::auth_layer,
        ));

    let app = Router::new()
        .merge(public)
        .merge(protected)
        .with_state(state);

    let addr = format!("{}:{}", cfg.host, cfg.port);
    tracing::info!("starting data-connector on {addr}");

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("failed to bind");

    axum::serve(listener, app).await.expect("server error");
}
