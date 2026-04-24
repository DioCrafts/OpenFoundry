mod config;
mod domain;
mod handlers;
mod models;

use auth_middleware::jwt::JwtConfig;
use axum::{
    Router, middleware,
    routing::{get, post},
};
use sqlx::postgres::PgPoolOptions;
use tracing_subscriber::EnvFilter;

#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::PgPool,
    pub jwt_config: JwtConfig,
    pub notification_service_url: String,
    pub http_client: reqwest::Client,
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
    let http_client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .expect("failed to build workflow HTTP client");

    let state = AppState {
        db: pool,
        jwt_config: jwt_config.clone(),
        notification_service_url: cfg.notification_service_url.clone(),
        http_client,
    };

    let cron_state = state.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
        loop {
            interval.tick().await;
            if let Err(error) = domain::executor::run_due_cron_workflows(&cron_state).await {
                tracing::warn!("cron evaluation failed: {error}");
            }
        }
    });

    let public = Router::new()
        .route("/health", get(|| async { "ok" }))
        .route(
            "/api/v1/workflows/webhooks/{id}",
            post(handlers::execute::trigger_webhook),
        );

    let protected = Router::new()
        .route(
            "/api/v1/workflows",
            get(handlers::crud::list_workflows).post(handlers::crud::create_workflow),
        )
        .route(
            "/api/v1/workflows/approvals",
            get(handlers::approvals::list_approvals),
        )
        .route(
            "/api/v1/workflows/approvals/{id}/decision",
            post(handlers::approvals::decide_approval),
        )
        .route(
            "/api/v1/workflows/events/{event_name}",
            post(handlers::execute::trigger_event),
        )
        .route(
            "/api/v1/workflows/triggers/cron/run-due",
            post(handlers::execute::run_due_cron_workflows),
        )
        .route(
            "/api/v1/workflows/{id}",
            get(handlers::crud::get_workflow)
                .patch(handlers::crud::update_workflow)
                .delete(handlers::crud::delete_workflow),
        )
        .route(
            "/api/v1/workflows/{id}/runs",
            get(handlers::runs::list_runs),
        )
        .route(
            "/api/v1/workflows/{id}/runs/manual",
            post(handlers::execute::start_manual_run),
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
    tracing::info!("starting workflow-service on {addr}");

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("failed to bind");

    axum::serve(listener, app).await.expect("server error");
}
