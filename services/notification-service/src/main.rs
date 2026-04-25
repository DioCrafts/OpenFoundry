mod config;
mod handlers;
mod models;

use auth_middleware::jwt::JwtConfig;
use axum::{
    Router, middleware,
    routing::{get, patch, post},
};
use lettre::{
    AsyncSmtpTransport, Tokio1Executor, message::Mailbox,
    transport::smtp::authentication::Credentials,
};
use sqlx::postgres::PgPoolOptions;
use tracing_subscriber::EnvFilter;

#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::PgPool,
    pub jwt_config: JwtConfig,
    pub http_client: reqwest::Client,
    pub email_sender: Option<AsyncSmtpTransport<Tokio1Executor>>,
    pub email_from: Option<Mailbox>,
    pub notification_bus: tokio::sync::broadcast::Sender<models::notification::NotificationEvent>,
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

    let jwt_config = JwtConfig::new(&cfg.jwt_secret).with_env_defaults();
    let http_client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .expect("failed to build notification HTTP client");
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
    let (notification_bus, _) = tokio::sync::broadcast::channel(256);

    let state = AppState {
        db: pool,
        jwt_config: jwt_config.clone(),
        http_client,
        email_sender,
        email_from,
        notification_bus,
    };

    let public = Router::new()
        .route("/health", get(|| async { "ok" }))
        .route(
            "/internal/notifications",
            post(handlers::send::internal_send_notification),
        )
        .route(
            "/api/v1/notifications/ws",
            get(handlers::ws::notifications_ws),
        );

    let protected = Router::new()
        .route(
            "/api/v1/notifications",
            get(handlers::history::list_notifications),
        )
        .route(
            "/api/v1/notifications/send",
            post(handlers::send::send_notification),
        )
        .route(
            "/api/v1/notifications/read-all",
            post(handlers::history::mark_all_read),
        )
        .route(
            "/api/v1/notifications/{id}/read",
            patch(handlers::history::mark_read),
        )
        .route(
            "/api/v1/notifications/preferences",
            get(handlers::preferences::get_preferences)
                .put(handlers::preferences::update_preferences),
        )
        .route(
            "/api/v1/notifications/ws-ticket",
            axum::routing::post(handlers::ws::issue_ws_ticket),
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
    tracing::info!("starting notification-service on {addr}");

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
