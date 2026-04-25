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
use lettre::{
    AsyncSmtpTransport, Tokio1Executor, message::Mailbox,
    transport::smtp::authentication::Credentials,
};
use sqlx::postgres::PgPoolOptions;
use std::{sync::Arc, time::Duration};
use storage_abstraction::{StorageBackend, local::LocalStorage, s3::S3Storage};
use tracing_subscriber::EnvFilter;

#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::PgPool,
    pub jwt_config: JwtConfig,
    pub http_client: reqwest::Client,
    pub dataset_service_url: String,
    pub geospatial_service_url: String,
    pub email_sender: Option<AsyncSmtpTransport<Tokio1Executor>>,
    pub email_from: Option<Mailbox>,
    pub object_store: Option<Arc<dyn StorageBackend>>,
    pub object_store_kind: String,
    pub report_delivery_timeout: Duration,
    pub report_delivery_max_retries: usize,
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
    let http_client = reqwest::Client::builder()
        .timeout(Duration::from_secs(cfg.report_delivery_timeout_secs.max(1)))
        .build()
        .expect("failed to build report HTTP client");
    let email_sender = cfg
        .smtp_host
        .as_deref()
        .map(build_smtp_transport)
        .transpose()
        .expect("failed to build SMTP transport");
    let email_from = cfg.smtp_from_address.as_deref().map(|address| {
        let parsed = address.parse().expect("invalid smtp from address");
        Mailbox::new(cfg.smtp_from_name.clone(), parsed)
    });
    let object_store = build_object_store(&cfg).expect("failed to build report object store");
    let state = AppState {
        db: pool,
        jwt_config: jwt_config.clone(),
        http_client,
        dataset_service_url: cfg.dataset_service_url.clone(),
        geospatial_service_url: cfg.geospatial_service_url.clone(),
        email_sender,
        email_from,
        object_store,
        object_store_kind: cfg.object_storage_backend.clone(),
        report_delivery_timeout: Duration::from_secs(cfg.report_delivery_timeout_secs.max(1)),
        report_delivery_max_retries: cfg.report_delivery_max_retries as usize,
    };

    let public = Router::new().route("/health", get(|| async { "ok" }));

    let protected = Router::new()
        .route(
            "/api/v1/reports/overview",
            get(handlers::crud::get_overview),
        )
        .route(
            "/api/v1/reports/catalog",
            get(handlers::generate::get_catalog),
        )
        .route(
            "/api/v1/reports/definitions",
            get(handlers::crud::list_reports).post(handlers::crud::create_report),
        )
        .route(
            "/api/v1/reports/definitions/{id}",
            axum::routing::patch(handlers::crud::update_report),
        )
        .route(
            "/api/v1/reports/definitions/{id}/generate",
            post(handlers::generate::generate_report),
        )
        .route(
            "/api/v1/reports/definitions/{id}/history",
            get(handlers::generate::list_history),
        )
        .route(
            "/api/v1/reports/schedules",
            get(handlers::schedule::get_schedule_board),
        )
        .route(
            "/api/v1/reports/executions/{id}",
            get(handlers::generate::get_execution),
        )
        .route(
            "/api/v1/reports/executions/{id}/download",
            get(handlers::download::download_execution),
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
    tracing::info!("starting report-service on {addr}");

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("failed to bind");

    axum::serve(listener, app).await.expect("server error");
}

fn build_smtp_transport(
    host: &str,
) -> Result<AsyncSmtpTransport<Tokio1Executor>, lettre::transport::smtp::Error> {
    let cfg = config::AppConfig::from_env().expect("failed to reload config for SMTP");
    let mut builder = AsyncSmtpTransport::<Tokio1Executor>::relay(host)?;

    if let Some(port) = cfg.smtp_port {
        builder = builder.port(port);
    }

    if let (Some(username), Some(password)) = (cfg.smtp_username, cfg.smtp_password) {
        builder = builder.credentials(Credentials::new(username, password));
    }

    Ok(builder.build())
}

fn build_object_store(cfg: &config::AppConfig) -> Result<Option<Arc<dyn StorageBackend>>, String> {
    match cfg.object_storage_backend.as_str() {
        "disabled" => Ok(None),
        "local" => {
            let backend =
                LocalStorage::new(&cfg.local_delivery_root).map_err(|error| error.to_string())?;
            Ok(Some(Arc::new(backend)))
        }
        "s3" => {
            let bucket = cfg
                .object_storage_bucket
                .as_deref()
                .ok_or_else(|| "OBJECT_STORAGE_BUCKET is required for s3 delivery".to_string())?;
            let region = cfg
                .object_storage_region
                .as_deref()
                .ok_or_else(|| "OBJECT_STORAGE_REGION is required for s3 delivery".to_string())?;
            let access_key = cfg.object_storage_access_key.as_deref().ok_or_else(|| {
                "OBJECT_STORAGE_ACCESS_KEY is required for s3 delivery".to_string()
            })?;
            let secret_key = cfg.object_storage_secret_key.as_deref().ok_or_else(|| {
                "OBJECT_STORAGE_SECRET_KEY is required for s3 delivery".to_string()
            })?;
            let backend = S3Storage::new(
                bucket,
                region,
                cfg.object_storage_endpoint.as_deref(),
                access_key,
                secret_key,
            )
            .map_err(|error| error.to_string())?;
            Ok(Some(Arc::new(backend)))
        }
        other => Err(format!("unsupported OBJECT_STORAGE_BACKEND '{other}'")),
    }
}
