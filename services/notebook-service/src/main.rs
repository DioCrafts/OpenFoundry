mod config;
mod domain;
mod handlers;
mod models;

use auth_middleware::jwt::JwtConfig;
use axum::{
    Router, middleware,
    routing::{delete, get, post, put},
};
use domain::kernel::KernelManager;
use sqlx::postgres::PgPoolOptions;
use tracing_subscriber::EnvFilter;

#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::PgPool,
    pub jwt_config: JwtConfig,
    pub kernel_manager: KernelManager,
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
    let kernel_manager = KernelManager::new(jwt_config.clone(), cfg.query_service_url.clone());

    let state = AppState {
        db: pool,
        jwt_config: jwt_config.clone(),
        kernel_manager,
    };

    let public = Router::new().route("/health", get(|| async { "ok" }));

    let protected = Router::new()
        // Notebooks
        .route("/api/v1/notebooks", post(handlers::crud::create_notebook))
        .route("/api/v1/notebooks", get(handlers::crud::list_notebooks))
        .route("/api/v1/notebooks/{id}", get(handlers::crud::get_notebook))
        .route("/api/v1/notebooks/{id}", put(handlers::crud::update_notebook))
        .route("/api/v1/notebooks/{id}", delete(handlers::crud::delete_notebook))
        // Cells
        .route("/api/v1/notebooks/{id}/cells", post(handlers::crud::add_cell))
        .route("/api/v1/notebooks/{notebook_id}/cells/{cell_id}", put(handlers::crud::update_cell))
        .route("/api/v1/notebooks/{notebook_id}/cells/{cell_id}", delete(handlers::crud::delete_cell))
        // Execution
        .route("/api/v1/notebooks/{notebook_id}/cells/{cell_id}/execute", post(handlers::execute::execute_cell))
        .route("/api/v1/notebooks/{id}/execute", post(handlers::execute::execute_all_cells))
        // Sessions
        .route("/api/v1/notebooks/{id}/sessions", post(handlers::sessions::create_session))
        .route("/api/v1/notebooks/{id}/sessions", get(handlers::sessions::list_sessions))
        .route("/api/v1/notebooks/{notebook_id}/sessions/{session_id}", delete(handlers::sessions::stop_session))
        // Notepad
        .route(
            "/api/v1/notepad/documents",
            post(handlers::notepad::create_document).get(handlers::notepad::list_documents),
        )
        .route(
            "/api/v1/notepad/documents/{id}",
            get(handlers::notepad::get_document)
                .patch(handlers::notepad::update_document)
                .delete(handlers::notepad::delete_document),
        )
        .route(
            "/api/v1/notepad/documents/{id}/presence",
            get(handlers::notepad::list_presence).post(handlers::notepad::upsert_presence),
        )
        .route(
            "/api/v1/notepad/documents/{id}/export",
            get(handlers::notepad::export_document),
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
    tracing::info!("starting notebook-service on {addr}");

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("failed to bind");

    axum::serve(listener, app).await.expect("server error");
}
