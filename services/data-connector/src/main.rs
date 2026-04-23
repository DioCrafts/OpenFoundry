mod config;
mod connectors;
mod domain;
mod handlers;
mod models;

use auth_middleware::jwt::JwtConfig;
use axum::{
    middleware,
    routing::{delete, get, post},
    Router,
};
use sqlx::postgres::PgPoolOptions;
use tracing_subscriber::EnvFilter;

#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::PgPool,
    pub jwt_config: JwtConfig,
}

impl axum::extract::FromRef<AppState> for JwtConfig {
    fn from_ref(state: &AppState) -> Self {
        state.jwt_config.clone()
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

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

    let state = AppState {
        db: pool,
        jwt_config: jwt_config.clone(),
    };

    let public = Router::new()
        .route("/health", get(|| async { "ok" }));

    let protected = Router::new()
        .route("/api/v1/connections", post(handlers::connections::create_connection))
        .route("/api/v1/connections", get(handlers::connections::list_connections))
        .route("/api/v1/connections/{id}", get(handlers::connections::get_connection))
        .route("/api/v1/connections/{id}", delete(handlers::connections::delete_connection))
        .route("/api/v1/connections/{id}/test", post(handlers::connections::test_connection))
        .route("/api/v1/connections/{id}/sync", post(handlers::sync_ops::sync_connection))
        .route("/api/v1/connections/{id}/sync-jobs", get(handlers::sync_ops::list_sync_jobs))
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
