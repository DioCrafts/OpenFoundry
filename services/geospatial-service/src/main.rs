mod config;
mod domain;
mod handlers;
mod models;

use auth_middleware::jwt::JwtConfig;
use axum::{
    Router,
    extract::FromRef,
    middleware,
    routing::{get, post},
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

    let jwt_config = JwtConfig::new(&cfg.jwt_secret).with_env_defaults();
    let state = AppState {
        db: pool,
        jwt_config: jwt_config.clone(),
    };

    let public = Router::new().route("/health", get(|| async { "ok" }));

    let protected = Router::new()
        .route(
            "/api/v1/geospatial/overview",
            get(handlers::layers::get_overview),
        )
        .route(
            "/api/v1/geospatial/layers",
            get(handlers::layers::list_layers).post(handlers::layers::create_layer),
        )
        .route(
            "/api/v1/geospatial/layers/{id}",
            axum::routing::patch(handlers::layers::update_layer),
        )
        .route(
            "/api/v1/geospatial/query",
            post(handlers::features::query_features),
        )
        .route(
            "/api/v1/geospatial/clusters",
            post(handlers::features::cluster_features),
        )
        .route(
            "/api/v1/geospatial/routes",
            post(handlers::features::route_features),
        )
        .route(
            "/api/v1/geospatial/geocode",
            post(handlers::geocode::forward_geocode),
        )
        .route(
            "/api/v1/geospatial/reverse-geocode",
            post(handlers::geocode::reverse_geocode),
        )
        .route(
            "/api/v1/geospatial/tiles/{id}",
            get(handlers::tiles::get_vector_tile),
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
    tracing::info!("starting geospatial-service on {addr}");

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("failed to bind");

    axum::serve(listener, app).await.expect("server error");
}
