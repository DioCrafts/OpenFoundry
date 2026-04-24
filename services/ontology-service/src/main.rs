mod config;
mod domain;
mod handlers;
mod models;

use auth_middleware::jwt::JwtConfig;
use axum::{
    Router, middleware,
    routing::{delete, get, post, put},
};
use sqlx::postgres::PgPoolOptions;
use tracing_subscriber::EnvFilter;

#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::PgPool,
    pub jwt_config: JwtConfig,
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
        .expect("failed to build ontology HTTP client");

    let state = AppState {
        db: pool,
        jwt_config: jwt_config.clone(),
        http_client,
    };

    let public = Router::new().route("/health", get(|| async { "ok" }));

    let protected = Router::new()
        // Object types
        .route("/api/v1/ontology/types", post(handlers::types::create_object_type))
        .route("/api/v1/ontology/types", get(handlers::types::list_object_types))
        .route("/api/v1/ontology/types/{id}", get(handlers::types::get_object_type))
        .route("/api/v1/ontology/types/{id}", put(handlers::types::update_object_type))
        .route("/api/v1/ontology/types/{id}", delete(handlers::types::delete_object_type))
        // Action types
        .route("/api/v1/ontology/actions", post(handlers::actions::create_action_type))
        .route("/api/v1/ontology/actions", get(handlers::actions::list_action_types))
        .route("/api/v1/ontology/actions/{id}", get(handlers::actions::get_action_type))
        .route("/api/v1/ontology/actions/{id}", put(handlers::actions::update_action_type))
        .route("/api/v1/ontology/actions/{id}", delete(handlers::actions::delete_action_type))
        .route("/api/v1/ontology/actions/{id}/validate", post(handlers::actions::validate_action))
        .route("/api/v1/ontology/actions/{id}/execute", post(handlers::actions::execute_action))
        // Object instances
        .route("/api/v1/ontology/types/{type_id}/objects", post(handlers::objects::create_object))
        .route("/api/v1/ontology/types/{type_id}/objects", get(handlers::objects::list_objects))
        .route("/api/v1/ontology/types/{type_id}/objects/{obj_id}", get(handlers::objects::get_object))
        .route("/api/v1/ontology/types/{type_id}/objects/{obj_id}", delete(handlers::objects::delete_object))

        // Link types
        .route("/api/v1/ontology/links", post(handlers::links::create_link_type))
        .route("/api/v1/ontology/links", get(handlers::links::list_link_types))
        .route("/api/v1/ontology/links/{id}", delete(handlers::links::delete_link_type))
        // Link instances
        .route("/api/v1/ontology/links/{link_type_id}/instances", post(handlers::links::create_link))
        .route("/api/v1/ontology/links/{link_type_id}/instances", get(handlers::links::list_links))
        .route("/api/v1/ontology/links/{link_type_id}/instances/{link_id}", delete(handlers::links::delete_link))
        .layer(middleware::from_fn_with_state(
            jwt_config,
            auth_middleware::auth_layer,
        ));

    let app = Router::new()
        .merge(public)
        .merge(protected)
        .with_state(state);

    let addr = format!("{}:{}", cfg.host, cfg.port);
    tracing::info!("starting ontology-service on {addr}");

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("failed to bind");

    axum::serve(listener, app).await.expect("server error");
}
