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
            "/api/v1/code-repos/overview",
            get(handlers::repos::get_overview),
        )
        .route(
            "/api/v1/code-repos/repositories",
            get(handlers::repos::list_repositories).post(handlers::repos::create_repository),
        )
        .route(
            "/api/v1/code-repos/repositories/{id}",
            axum::routing::patch(handlers::repos::update_repository),
        )
        .route(
            "/api/v1/code-repos/repositories/{id}/branches",
            get(handlers::branches::list_branches).post(handlers::branches::create_branch),
        )
        .route(
            "/api/v1/code-repos/repositories/{id}/commits",
            get(handlers::commits::list_commits).post(handlers::commits::create_commit),
        )
        .route(
            "/api/v1/code-repos/repositories/{id}/files",
            get(handlers::files::list_files),
        )
        .route(
            "/api/v1/code-repos/repositories/{id}/diff",
            get(handlers::diff::get_repository_diff),
        )
        .route(
            "/api/v1/code-repos/repositories/{id}/search",
            get(handlers::files::search_files),
        )
        .route(
            "/api/v1/code-repos/repositories/{id}/ci",
            get(handlers::commits::list_ci_runs).post(handlers::commits::trigger_ci_run),
        )
        .route(
            "/api/v1/code-repos/integrations",
            get(handlers::integrations::list_integrations)
                .post(handlers::integrations::create_integration),
        )
        .route(
            "/api/v1/code-repos/integrations/{id}",
            get(handlers::integrations::get_integration)
                .patch(handlers::integrations::update_integration),
        )
        .route(
            "/api/v1/code-repos/integrations/{id}/sync",
            get(handlers::integrations::list_sync_runs).post(handlers::integrations::trigger_sync),
        )
        .route(
            "/api/v1/code-repos/merge-requests",
            get(handlers::merge_requests::list_merge_requests)
                .post(handlers::merge_requests::create_merge_request),
        )
        .route(
            "/api/v1/code-repos/merge-requests/{id}",
            get(handlers::merge_requests::get_merge_request)
                .patch(handlers::merge_requests::update_merge_request),
        )
        .route(
            "/api/v1/code-repos/merge-requests/{id}/comments",
            get(handlers::merge_requests::list_comments)
                .post(handlers::merge_requests::create_comment),
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
    tracing::info!("starting code-repo-service on {addr}");

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("failed to bind");

    axum::serve(listener, app).await.expect("server error");
}
