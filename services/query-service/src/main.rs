mod config;
mod domain;
mod handlers;
mod models;

use auth_middleware::jwt::JwtConfig;
use axum::{
    Router, middleware,
    routing::{delete, get, post},
};
use core_models::{health::HealthStatus, observability};
use query_engine::context::QueryContext;
use sqlx::postgres::PgPoolOptions;

#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::PgPool,
    pub jwt_config: JwtConfig,
    pub query_ctx: std::sync::Arc<QueryContext>,
    pub distributed_query_workers: usize,
}

impl axum::extract::FromRef<AppState> for JwtConfig {
    fn from_ref(state: &AppState) -> Self {
        state.jwt_config.clone()
    }
}

#[tokio::main]
async fn main() {
    observability::init_tracing("query-service");

    let cfg = config::AppConfig::from_env().expect("failed to load config");
    if let Some(data_dir) = cfg.data_dir.as_deref() {
        tracing::info!(data_dir, "query-service local data directory configured");
    }

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

    let query_ctx = std::sync::Arc::new(QueryContext::new());

    let state = AppState {
        db: pool,
        jwt_config: jwt_config.clone(),
        query_ctx,
        distributed_query_workers: cfg.distributed_query_workers.max(1),
    };

    let public = Router::new().route(
        "/health",
        get(|| async { axum::Json(HealthStatus::ok("query-service")) }),
    );

    let protected = Router::new()
        .route(
            "/api/v1/queries/execute",
            post(handlers::execute::execute_query),
        )
        .route(
            "/api/v1/queries/explain",
            post(handlers::explain::explain_query),
        )
        .route(
            "/api/v1/queries/saved",
            post(handlers::saved::create_saved_query),
        )
        .route(
            "/api/v1/queries/saved",
            get(handlers::saved::list_saved_queries),
        )
        .route(
            "/api/v1/queries/saved/{id}",
            delete(handlers::saved::delete_saved_query),
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
    tracing::info!("starting query-service on {addr}");

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("failed to bind");

    axum::serve(listener, app).await.expect("server error");
}
