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
        .route("/api/v1/fusion/overview", get(handlers::jobs::get_overview))
        .route(
            "/api/v1/fusion/rules",
            get(handlers::rules::list_rules).post(handlers::rules::create_rule),
        )
        .route(
            "/api/v1/fusion/rules/{id}",
            axum::routing::patch(handlers::rules::update_rule),
        )
        .route(
            "/api/v1/fusion/merge-strategies",
            get(handlers::rules::list_merge_strategies)
                .post(handlers::rules::create_merge_strategy),
        )
        .route(
            "/api/v1/fusion/merge-strategies/{id}",
            axum::routing::patch(handlers::rules::update_merge_strategy),
        )
        .route(
            "/api/v1/fusion/jobs",
            get(handlers::jobs::list_jobs).post(handlers::jobs::create_job),
        )
        .route(
            "/api/v1/fusion/jobs/{id}/run",
            post(handlers::jobs::run_job),
        )
        .route(
            "/api/v1/fusion/clusters",
            get(handlers::clusters::list_clusters),
        )
        .route(
            "/api/v1/fusion/clusters/{id}",
            get(handlers::clusters::get_cluster),
        )
        .route(
            "/api/v1/fusion/clusters/{id}/review",
            post(handlers::clusters::submit_review),
        )
        .route(
            "/api/v1/fusion/review-queue",
            get(handlers::clusters::list_review_queue),
        )
        .route(
            "/api/v1/fusion/golden-records",
            get(handlers::clusters::list_golden_records),
        )
        .layer(middleware::from_fn_with_state(
            jwt_config,
            auth_middleware::auth_layer,
        ));

    let app = Router::new().merge(public).merge(protected).with_state(state);

    let addr = format!("{}:{}", cfg.host, cfg.port);
    tracing::info!("starting fusion-service on {addr}");

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("failed to bind");

    axum::serve(listener, app).await.expect("server error");
}
