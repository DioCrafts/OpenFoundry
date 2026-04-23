mod config;
mod domain;
mod handlers;
mod models;

use auth_middleware::jwt::JwtConfig;
use axum::{
    extract::FromRef,
    middleware,
    routing::get,
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
        .route("/api/v1/nexus/overview", get(handlers::peers::get_overview))
        .route(
            "/api/v1/nexus/peers",
            get(handlers::peers::list_peers).post(handlers::peers::create_peer),
        )
        .route(
            "/api/v1/nexus/peers/{id}",
            axum::routing::patch(handlers::peers::update_peer),
        )
        .route(
            "/api/v1/nexus/peers/{id}/authenticate",
            axum::routing::post(handlers::peers::authenticate_peer),
        )
        .route(
            "/api/v1/nexus/contracts",
            get(handlers::contracts::list_contracts).post(handlers::contracts::create_contract),
        )
        .route(
            "/api/v1/nexus/contracts/{id}",
            axum::routing::patch(handlers::contracts::update_contract),
        )
        .route(
            "/api/v1/nexus/shares",
            get(handlers::shares::list_shares).post(handlers::shares::create_share),
        )
        .route(
            "/api/v1/nexus/shares/{id}",
            get(handlers::shares::get_share).patch(handlers::shares::update_share),
        )
        .route(
            "/api/v1/nexus/federation/query",
            axum::routing::post(handlers::consume::run_federated_query),
        )
        .route(
            "/api/v1/nexus/replication/plans",
            get(handlers::consume::list_replication_plans),
        )
        .route(
            "/api/v1/nexus/schema-compatibility",
            get(handlers::consume::list_schema_compatibility),
        )
        .route(
            "/api/v1/nexus/audit-bridge",
            get(handlers::consume::get_audit_bridge),
        )
        .layer(middleware::from_fn_with_state(
            jwt_config,
            auth_middleware::auth_layer,
        ));

    let app = Router::new().merge(public).merge(protected).with_state(state);

    let addr = format!("{}:{}", cfg.host, cfg.port);
    tracing::info!("starting nexus-service on {addr}");

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("failed to bind");

    axum::serve(listener, app).await.expect("server error");
}
