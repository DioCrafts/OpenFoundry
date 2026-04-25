mod config;
mod domain;
mod handlers;
mod models;

use auth_middleware::jwt::JwtConfig;
use axum::{
    Router, middleware,
    routing::{delete, get, patch, post},
};
use core_models::{health::HealthStatus, observability};
use sqlx::postgres::PgPoolOptions;
use storage_abstraction::StorageBackend;

/// Shared application state passed to all handlers.
#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::PgPool,
    pub jwt_config: JwtConfig,
    pub storage: std::sync::Arc<dyn StorageBackend>,
}

impl axum::extract::FromRef<AppState> for JwtConfig {
    fn from_ref(state: &AppState) -> Self {
        state.jwt_config.clone()
    }
}

#[tokio::main]
async fn main() {
    observability::init_tracing("dataset-service");

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

    let storage: std::sync::Arc<dyn StorageBackend> = match cfg.storage_backend.as_str() {
        "local" => {
            let root = cfg
                .local_storage_root
                .as_deref()
                .unwrap_or("/tmp/of-datasets");
            std::sync::Arc::new(
                storage_abstraction::local::LocalStorage::new(root)
                    .expect("failed to init local storage"),
            )
        }
        _ => std::sync::Arc::new(
            storage_abstraction::s3::S3Storage::new(
                &cfg.storage_bucket,
                cfg.s3_region.as_deref().unwrap_or("us-east-1"),
                cfg.s3_endpoint.as_deref(),
                cfg.s3_access_key.as_deref().unwrap_or("minioadmin"),
                cfg.s3_secret_key.as_deref().unwrap_or("minioadmin"),
            )
            .expect("failed to init S3 storage"),
        ),
    };

    let state = AppState {
        db: pool,
        jwt_config: jwt_config.clone(),
        storage,
    };

    let public = Router::new()
        .route(
            "/health",
            get(|| async { axum::Json(HealthStatus::ok("dataset-service")) }),
        )
        .route(
            "/internal/datasets/{id}/metadata",
            get(handlers::internal::get_dataset_metadata),
        );

    let protected = Router::new()
        .route("/api/v1/datasets", post(handlers::crud::create_dataset))
        .route("/api/v1/datasets", get(handlers::crud::list_datasets))
        .route(
            "/api/v1/datasets/catalog/facets",
            get(handlers::catalog::get_catalog_facets),
        )
        .route("/api/v1/datasets/{id}", get(handlers::crud::get_dataset))
        .route(
            "/api/v1/datasets/{id}",
            patch(handlers::crud::update_dataset),
        )
        .route(
            "/api/v1/datasets/{id}",
            delete(handlers::crud::delete_dataset),
        )
        .route(
            "/api/v1/datasets/{id}/upload",
            post(handlers::upload::upload_data),
        )
        .route(
            "/api/v1/datasets/{id}/preview",
            get(handlers::preview::preview_data),
        )
        .route(
            "/api/v1/datasets/{id}/schema",
            get(handlers::preview::get_schema),
        )
        .route(
            "/api/v1/datasets/{id}/versions",
            get(handlers::versions::list_versions),
        )
        .route(
            "/api/v1/datasets/{id}/transactions",
            get(handlers::transactions::list_transactions),
        )
        .route(
            "/api/v1/datasets/{id}/files",
            get(handlers::export::list_files),
        )
        .route(
            "/api/v1/datasets/{id}/filesystem",
            get(handlers::export::list_files),
        )
        .route(
            "/api/v2/filesystem/datasets/{id}",
            get(handlers::export::list_files),
        )
        .route(
            "/api/v1/datasets/{id}/views",
            get(handlers::views::list_views),
        )
        .route(
            "/api/v1/datasets/{id}/views",
            post(handlers::views::create_view),
        )
        .route(
            "/api/v1/datasets/{id}/views/{view_id}",
            get(handlers::views::get_view),
        )
        .route(
            "/api/v1/datasets/{id}/views/{view_id}/preview",
            get(handlers::views::preview_view),
        )
        .route(
            "/api/v1/datasets/{id}/views/{view_id}/refresh",
            post(handlers::views::refresh_view),
        )
        .route(
            "/api/v1/datasets/{id}/branches",
            get(handlers::branches::list_branches),
        )
        .route(
            "/api/v1/datasets/{id}/branches",
            post(handlers::branches::create_branch),
        )
        .route(
            "/api/v1/datasets/{id}/branches/{branch_name}/checkout",
            post(handlers::branches::checkout_branch),
        )
        .route(
            "/api/v1/datasets/{id}/branches/{branch_name}/merge",
            post(handlers::branches::merge_branch),
        )
        .route(
            "/api/v1/datasets/{id}/branches/{branch_name}/promote",
            post(handlers::branches::promote_branch),
        )
        .route(
            "/api/v1/datasets/{id}/quality",
            get(handlers::quality::get_dataset_quality),
        )
        .route(
            "/api/v1/datasets/{id}/quality/profile",
            post(handlers::quality::refresh_dataset_quality),
        )
        .route(
            "/api/v1/datasets/{id}/quality/rules",
            post(handlers::quality::create_quality_rule),
        )
        .route(
            "/api/v1/datasets/{id}/quality/rules/{rule_id}",
            patch(handlers::quality::update_quality_rule),
        )
        .route(
            "/api/v1/datasets/{id}/quality/rules/{rule_id}",
            delete(handlers::quality::delete_quality_rule),
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
    tracing::info!("starting dataset-service on {addr}");

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("failed to bind");

    axum::serve(listener, app).await.expect("server error");
}
