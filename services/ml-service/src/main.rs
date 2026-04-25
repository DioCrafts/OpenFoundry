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
        .route("/api/v1/ml/overview", get(handlers::overview::get_overview))
        .route(
            "/api/v1/ml/experiments",
            get(handlers::experiments::list_experiments)
                .post(handlers::experiments::create_experiment),
        )
        .route(
            "/api/v1/ml/experiments/{id}",
            axum::routing::patch(handlers::experiments::update_experiment),
        )
        .route(
            "/api/v1/ml/experiments/{id}/asset-lineage",
            get(handlers::experiments::get_experiment_asset_lineage),
        )
        .route(
            "/api/v1/ml/experiments/{id}/runs",
            get(handlers::experiments::list_runs).post(handlers::experiments::create_run),
        )
        .route(
            "/api/v1/ml/runs/{id}",
            axum::routing::patch(handlers::experiments::update_run),
        )
        .route(
            "/api/v1/ml/runs/compare",
            post(handlers::experiments::compare_runs),
        )
        .route(
            "/api/v1/ml/models",
            get(handlers::models::list_models).post(handlers::models::create_model),
        )
        .route(
            "/api/v1/ml/models/{id}",
            axum::routing::patch(handlers::models::update_model),
        )
        .route(
            "/api/v1/ml/models/{id}/versions",
            get(handlers::models::list_model_versions).post(handlers::models::create_model_version),
        )
        .route(
            "/api/v1/ml/model-versions/{id}/transition",
            post(handlers::models::transition_model_version),
        )
        .route(
            "/api/v1/ml/features",
            get(handlers::features::list_features).post(handlers::features::create_feature),
        )
        .route(
            "/api/v1/ml/features/{id}",
            axum::routing::patch(handlers::features::update_feature),
        )
        .route(
            "/api/v1/ml/features/{id}/materialize",
            post(handlers::features::materialize_feature),
        )
        .route(
            "/api/v1/ml/features/{id}/online",
            get(handlers::features::get_online_feature_snapshot),
        )
        .route(
            "/api/v1/ml/training-jobs",
            get(handlers::training::list_training_jobs)
                .post(handlers::training::create_training_job),
        )
        .route(
            "/api/v1/ml/deployments",
            get(handlers::deployments::list_deployments)
                .post(handlers::deployments::create_deployment),
        )
        .route(
            "/api/v1/ml/deployments/{id}",
            axum::routing::patch(handlers::deployments::update_deployment),
        )
        .route(
            "/api/v1/ml/deployments/{id}/drift",
            post(handlers::deployments::generate_drift_report),
        )
        .route(
            "/api/v1/ml/deployments/{id}/predict",
            post(handlers::predictions::realtime_predict),
        )
        .route(
            "/api/v1/ml/batch-predictions",
            get(handlers::predictions::list_batch_predictions)
                .post(handlers::predictions::create_batch_prediction),
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
    tracing::info!("starting ml-service on {addr}");

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("failed to bind");

    axum::serve(listener, app).await.expect("server error");
}
