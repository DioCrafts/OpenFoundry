mod config;
mod domain;
mod handlers;
mod models;

use auth_middleware::jwt::JwtConfig;
use axum::{
    extract::FromRef,
    middleware,
    routing::get,
    Router,
};
use futures::StreamExt;
use models::audit_event::AppendAuditEventRequest;
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

    if let Some(nats_url) = cfg.nats_url.as_deref() {
        tracing::info!(nats_url, "audit collector bus configured");
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

    let jwt_config = JwtConfig::new(&cfg.jwt_secret);
    let state = AppState {
        db: pool,
        jwt_config: jwt_config.clone(),
    };

    if let Some(nats_url) = cfg.nats_url.clone() {
        tokio::spawn(run_nats_collector(state.db.clone(), nats_url));
    }

    let public = Router::new().route("/health", get(|| async { "ok" }));

    let protected = Router::new()
        .route("/api/v1/audit/overview", get(handlers::events::get_overview))
        .route(
            "/api/v1/audit/events",
            get(handlers::events::list_events).post(handlers::events::append_event),
        )
        .route("/api/v1/audit/events/{id}", get(handlers::events::get_event))
        .route("/api/v1/audit/collectors", get(handlers::events::list_collectors))
        .route("/api/v1/audit/anomalies", get(handlers::events::list_anomalies))
        .route("/api/v1/audit/classifications", get(handlers::policies::list_classifications))
        .route(
            "/api/v1/audit/policies",
            get(handlers::policies::list_policies).post(handlers::policies::create_policy),
        )
        .route(
            "/api/v1/audit/policies/{id}",
            axum::routing::patch(handlers::policies::update_policy),
        )
        .route("/api/v1/audit/reports", get(handlers::reports::list_reports))
        .route("/api/v1/audit/reports/generate", axum::routing::post(handlers::reports::generate_report))
        .route("/api/v1/audit/gdpr/export", axum::routing::post(handlers::reports::export_subject_data))
        .route("/api/v1/audit/gdpr/erase", axum::routing::post(handlers::reports::erase_subject_data))
        .layer(middleware::from_fn_with_state(
            jwt_config,
            auth_middleware::auth_layer,
        ));

    let app = Router::new().merge(public).merge(protected).with_state(state);

    let addr = format!("{}:{}", cfg.host, cfg.port);
    tracing::info!("starting audit-service on {addr}");

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("failed to bind");

    axum::serve(listener, app).await.expect("server error");
}

async fn run_nats_collector(db: sqlx::PgPool, nats_url: String) {
    let js = match event_bus::connect(&nats_url).await {
        Ok(js) => js,
        Err(cause) => {
            tracing::warn!(?cause, nats_url, "failed to connect audit collector to NATS");
            return;
        }
    };

    let stream = match event_bus::subscriber::ensure_stream(
        &js,
        event_bus::topics::streams::AUDIT,
        &[event_bus::topics::subjects::AUDIT],
    )
    .await
    {
        Ok(stream) => stream,
        Err(cause) => {
            tracing::warn!(?cause, "failed to ensure audit stream for collector");
            return;
        }
    };

    let consumer = match event_bus::subscriber::create_consumer(&stream, "audit-service-collector", None).await {
        Ok(consumer) => consumer,
        Err(cause) => {
            tracing::warn!(?cause, "failed to create audit collector consumer");
            return;
        }
    };

    tracing::info!("audit-service NATS collector started");

    loop {
        match consumer.messages().await {
            Ok(mut messages) => {
                while let Some(next_message) = messages.next().await {
                    match next_message {
                        Ok(message) => {
                            let parsed = serde_json::from_slice::<event_bus::Event<AppendAuditEventRequest>>(&message.payload);
                            match parsed {
                                Ok(mut envelope) => {
                                    if let Err(cause) = handlers::events::persist_event(&db, &mut envelope.payload).await {
                                        tracing::warn!(?cause, event_type = envelope.event_type, "failed to persist collected audit event");
                                    }
                                }
                                Err(cause) => {
                                    tracing::warn!(?cause, "failed to decode collected audit event");
                                }
                            }

                            if let Err(cause) = message.ack().await {
                                tracing::warn!(?cause, "failed to ack collected audit event");
                            }
                        }
                        Err(cause) => {
                            tracing::warn!(?cause, "audit collector message stream failed");
                            break;
                        }
                    }
                }
            }
            Err(cause) => {
                tracing::warn!(?cause, "failed to pull audit collector messages");
                break;
            }
        }
    }
}
