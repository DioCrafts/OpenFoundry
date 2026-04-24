mod config;
mod domain;
mod handlers;
mod models;

use auth_middleware::jwt::JwtConfig;
use axum::{Router, extract::FromRef, middleware, routing::get};
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
        .route(
            "/api/v1/marketplace/overview",
            get(handlers::browse::get_overview),
        )
        .route(
            "/api/v1/marketplace/categories",
            get(handlers::browse::list_categories),
        )
        .route(
            "/api/v1/marketplace/listings",
            get(handlers::browse::list_listings).post(handlers::publish::publish_listing),
        )
        .route(
            "/api/v1/marketplace/listings/{id}",
            get(handlers::browse::get_listing).patch(handlers::publish::update_listing),
        )
        .route(
            "/api/v1/marketplace/listings/{id}/versions",
            get(handlers::publish::list_versions).post(handlers::publish::publish_version),
        )
        .route(
            "/api/v1/marketplace/listings/{id}/reviews",
            get(handlers::reviews::list_reviews).post(handlers::reviews::create_review),
        )
        .route(
            "/api/v1/marketplace/search",
            get(handlers::browse::search_listings),
        )
        .route(
            "/api/v1/marketplace/installs",
            get(handlers::install::list_installs).post(handlers::install::create_install),
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
    tracing::info!("starting marketplace-service on {addr}");

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("failed to bind");

    axum::serve(listener, app).await.expect("server error");
}
