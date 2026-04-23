mod config;
mod domain;
mod handlers;
mod models;

use auth_middleware::jwt::JwtConfig;
use axum::{
    extract::FromRef,
    middleware,
    routing::{get, post},
    Router,
};
use sqlx::postgres::PgPoolOptions;
use tracing_subscriber::EnvFilter;

#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::PgPool,
    pub jwt_config: JwtConfig,
}

impl FromRef<AppState> for JwtConfig {
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

    let public = Router::new().route("/health", get(|| async { "ok" }));

    let protected = Router::new()
        .route("/api/v1/reports/overview", get(handlers::crud::get_overview))
        .route(
            "/api/v1/reports/catalog",
            get(handlers::generate::get_catalog),
        )
        .route(
            "/api/v1/reports/definitions",
            get(handlers::crud::list_reports).post(handlers::crud::create_report),
        )
        .route(
            "/api/v1/reports/definitions/{id}",
            axum::routing::patch(handlers::crud::update_report),
        )
        .route(
            "/api/v1/reports/definitions/{id}/generate",
            post(handlers::generate::generate_report),
        )
        .route(
            "/api/v1/reports/definitions/{id}/history",
            get(handlers::generate::list_history),
        )
        .route(
            "/api/v1/reports/schedules",
            get(handlers::schedule::get_schedule_board),
        )
        .route(
            "/api/v1/reports/executions/{id}",
            get(handlers::generate::get_execution),
        )
        .route(
            "/api/v1/reports/executions/{id}/download",
            get(handlers::download::download_execution),
        )
        .layer(middleware::from_fn_with_state(
            jwt_config,
            auth_middleware::auth_layer,
        ));

    let app = Router::new().merge(public).merge(protected).with_state(state);

    let addr = format!("{}:{}", cfg.host, cfg.port);
    tracing::info!("starting report-service on {addr}");

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("failed to bind");

    axum::serve(listener, app).await.expect("server error");
}
