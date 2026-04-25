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
    pub http_client: reqwest::Client,
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
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .expect("failed to build HTTP client");
    let state = AppState {
        db: pool,
        jwt_config: jwt_config.clone(),
        http_client,
    };

    let public = Router::new().route("/health", get(|| async { "ok" }));

    let protected = Router::new()
        .route("/api/v1/ai/overview", get(handlers::chat::get_overview))
        .route(
            "/api/v1/ai/providers",
            get(handlers::chat::list_providers).post(handlers::chat::create_provider),
        )
        .route(
            "/api/v1/ai/providers/{id}",
            axum::routing::patch(handlers::chat::update_provider),
        )
        .route(
            "/api/v1/ai/prompts",
            get(handlers::prompts::list_prompts).post(handlers::prompts::create_prompt),
        )
        .route(
            "/api/v1/ai/prompts/{id}",
            axum::routing::patch(handlers::prompts::update_prompt),
        )
        .route(
            "/api/v1/ai/prompts/{id}/render",
            post(handlers::prompts::render_prompt),
        )
        .route(
            "/api/v1/ai/knowledge-bases",
            get(handlers::knowledge::list_knowledge_bases)
                .post(handlers::knowledge::create_knowledge_base),
        )
        .route(
            "/api/v1/ai/knowledge-bases/{id}",
            axum::routing::patch(handlers::knowledge::update_knowledge_base),
        )
        .route(
            "/api/v1/ai/knowledge-bases/{id}/documents",
            get(handlers::knowledge::list_documents).post(handlers::knowledge::create_document),
        )
        .route(
            "/api/v1/ai/knowledge-bases/{id}/search",
            post(handlers::knowledge::search_knowledge_base),
        )
        .route(
            "/api/v1/ai/tools",
            get(handlers::tools::list_tools).post(handlers::tools::create_tool),
        )
        .route(
            "/api/v1/ai/tools/{id}",
            axum::routing::patch(handlers::tools::update_tool),
        )
        .route(
            "/api/v1/ai/agents",
            get(handlers::agents::list_agents).post(handlers::agents::create_agent),
        )
        .route(
            "/api/v1/ai/agents/{id}",
            axum::routing::patch(handlers::agents::update_agent),
        )
        .route(
            "/api/v1/ai/agents/{id}/execute",
            post(handlers::agents::execute_agent),
        )
        .route(
            "/api/v1/ai/conversations",
            get(handlers::chat::list_conversations),
        )
        .route(
            "/api/v1/ai/conversations/{id}",
            get(handlers::chat::get_conversation),
        )
        .route(
            "/api/v1/ai/chat/completions",
            post(handlers::chat::create_chat_completion),
        )
        .route("/api/v1/ai/copilot/ask", post(handlers::chat::ask_copilot))
        .route(
            "/api/v1/ai/guardrails/evaluate",
            post(handlers::chat::evaluate_guardrails),
        )
        .route(
            "/api/v1/ai/evaluations/benchmarks",
            post(handlers::chat::benchmark_providers),
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
    tracing::info!("starting ai-service on {addr}");

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("failed to bind");

    axum::serve(listener, app).await.expect("server error");
}
