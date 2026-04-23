mod config;
mod domain;
mod handlers;
mod models;

use auth_middleware::jwt::JwtConfig;
use axum::{
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
    pub public_base_url: String,
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
        public_base_url: cfg.public_base_url.clone(),
    };

    let public = Router::new()
        .route("/health", get(|| async { "ok" }))
        .route(
            "/api/v1/apps/public/{slug}",
            get(handlers::preview::get_published_app),
        )
        .route(
            "/api/v1/apps/public/{slug}/embed",
            get(handlers::preview::get_embed_info),
        );

    let protected = Router::new()
        .route(
            "/api/v1/apps",
            get(handlers::apps::list_apps).post(handlers::apps::create_app),
        )
        .route(
            "/api/v1/apps/from-template",
            post(handlers::apps::create_from_template),
        )
        .route("/api/v1/apps/templates", get(handlers::apps::list_templates))
        .route("/api/v1/widgets/catalog", get(handlers::widgets::list_widget_catalog))
        .route(
            "/api/v1/apps/{id}",
            get(handlers::apps::get_app)
                .patch(handlers::apps::update_app)
                .delete(handlers::apps::delete_app),
        )
        .route(
            "/api/v1/apps/{id}/preview",
            get(handlers::preview::preview_app),
        )
        .route(
            "/api/v1/apps/{id}/pages",
            post(handlers::pages::create_page),
        )
        .route(
            "/api/v1/apps/{app_id}/pages/{page_id}",
            axum::routing::patch(handlers::pages::update_page)
                .delete(handlers::pages::delete_page),
        )
        .route(
            "/api/v1/apps/{id}/versions",
            get(handlers::publish::list_versions),
        )
        .route(
            "/api/v1/apps/{id}/publish",
            post(handlers::publish::publish_app),
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
    tracing::info!("starting app-builder-service on {addr}");

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("failed to bind");

    axum::serve(listener, app).await.expect("server error");
}
