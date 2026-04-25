mod config;
mod domain;
mod handlers;
mod models;

use auth_middleware::jwt::JwtConfig;
use axum::{
    Router, middleware,
    routing::{delete, get, patch, post, put},
};
use sqlx::postgres::PgPoolOptions;
use tracing_subscriber::EnvFilter;

/// Shared application state passed to all handlers.
#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::PgPool,
    pub jwt_config: JwtConfig,
    pub public_web_origin: String,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cfg = config::AppConfig::from_env().expect("failed to load config");
    if let Some(nats_url) = cfg.nats_url.as_deref() {
        tracing::info!(nats_url, "auth-service NATS integration configured");
    }
    if let Some(redis_url) = cfg.redis_url.as_deref() {
        tracing::info!(redis_url, "auth-service Redis integration configured");
    }

    let pool = PgPoolOptions::new()
        .max_connections(20)
        .connect(&cfg.database_url)
        .await
        .expect("failed to connect to database");

    // Run migrations
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("failed to run migrations");

    let jwt_config = JwtConfig::new(&cfg.jwt_secret)
        .with_access_ttl(cfg.jwt_access_ttl_secs)
        .with_refresh_ttl(cfg.jwt_refresh_ttl_secs);

    let state = AppState {
        db: pool,
        jwt_config: jwt_config.clone(),
        public_web_origin: cfg.public_web_origin.clone(),
    };

    // Public routes (no auth required)
    let public = Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/api/v1/auth/register", post(handlers::register::register))
        .route("/api/v1/auth/login", post(handlers::login::login))
        .route("/api/v1/auth/refresh", post(handlers::token::refresh))
        .route(
            "/api/v1/auth/mfa/complete",
            post(handlers::mfa::complete_login),
        )
        .route(
            "/api/v1/auth/sso/providers/public",
            get(handlers::sso::list_public_providers),
        )
        .route(
            "/api/v1/auth/sso/providers/{slug}/start",
            get(handlers::sso::start_login),
        )
        .route(
            "/api/v1/auth/sso/callback",
            post(handlers::sso::complete_login),
        );

    // Protected routes (auth required)
    let protected = Router::new()
        .route(
            "/api/v1/control-panel",
            get(handlers::control_panel::get_control_panel)
                .put(handlers::control_panel::update_control_panel),
        )
        .route(
            "/api/v1/control-panel/upgrade-readiness",
            get(handlers::control_panel::get_upgrade_readiness),
        )
        .route(
            "/api/v2/admin/control-panel",
            get(handlers::control_panel::get_control_panel)
                .put(handlers::control_panel::update_control_panel),
        )
        .route(
            "/api/v2/admin/control-panel/upgrade-readiness",
            get(handlers::control_panel::get_upgrade_readiness),
        )
        .route("/api/v1/users/me", get(handlers::user_mgmt::me))
        .route("/api/v2/admin/users/me", get(handlers::user_mgmt::me))
        .route("/api/v1/users", get(handlers::user_mgmt::list_users))
        .route("/api/v2/admin/users", get(handlers::user_mgmt::list_users))
        .route(
            "/api/v1/users/{id}",
            patch(handlers::user_mgmt::update_user).delete(handlers::user_mgmt::deactivate_user),
        )
        .route(
            "/api/v2/admin/users/{id}",
            patch(handlers::user_mgmt::update_user).delete(handlers::user_mgmt::deactivate_user),
        )
        .route(
            "/api/v1/users/{id}/roles",
            post(handlers::role_mgmt::assign_role),
        )
        .route(
            "/api/v1/users/{id}/roles/{role_id}",
            delete(handlers::role_mgmt::remove_role),
        )
        .route(
            "/api/v1/users/{id}/groups",
            post(handlers::group_mgmt::add_user_to_group),
        )
        .route(
            "/api/v1/users/{id}/groups/{group_id}",
            delete(handlers::group_mgmt::remove_user_from_group),
        )
        .route(
            "/api/v1/roles",
            get(handlers::role_mgmt::list_roles).post(handlers::role_mgmt::create_role),
        )
        .route(
            "/api/v2/admin/roles",
            get(handlers::role_mgmt::list_roles).post(handlers::role_mgmt::create_role),
        )
        .route("/api/v1/roles/{id}", put(handlers::role_mgmt::update_role))
        .route("/api/v2/admin/roles/{id}", put(handlers::role_mgmt::update_role))
        .route(
            "/api/v1/permissions",
            get(handlers::permission_mgmt::list_permissions)
                .post(handlers::permission_mgmt::create_permission),
        )
        .route(
            "/api/v2/admin/permissions",
            get(handlers::permission_mgmt::list_permissions)
                .post(handlers::permission_mgmt::create_permission),
        )
        .route(
            "/api/v1/groups",
            get(handlers::group_mgmt::list_groups).post(handlers::group_mgmt::create_group),
        )
        .route(
            "/api/v2/admin/groups",
            get(handlers::group_mgmt::list_groups).post(handlers::group_mgmt::create_group),
        )
        .route(
            "/api/v1/groups/{id}",
            put(handlers::group_mgmt::update_group),
        )
        .route(
            "/api/v2/admin/groups/{id}",
            put(handlers::group_mgmt::update_group),
        )
        .route(
            "/api/v1/policies",
            get(handlers::policy_mgmt::list_policies).post(handlers::policy_mgmt::create_policy),
        )
        .route(
            "/api/v2/admin/policies",
            get(handlers::policy_mgmt::list_policies).post(handlers::policy_mgmt::create_policy),
        )
        .route(
            "/api/v1/policies/evaluate",
            post(handlers::policy_mgmt::evaluate_policy),
        )
        .route(
            "/api/v2/admin/policies/evaluate",
            post(handlers::policy_mgmt::evaluate_policy),
        )
        .route(
            "/api/v1/policies/{id}",
            put(handlers::policy_mgmt::update_policy).delete(handlers::policy_mgmt::delete_policy),
        )
        .route(
            "/api/v2/admin/policies/{id}",
            patch(handlers::policy_mgmt::update_policy).delete(handlers::policy_mgmt::delete_policy),
        )
        .route(
            "/api/v1/api-keys",
            get(handlers::api_key_mgmt::list_api_keys).post(handlers::api_key_mgmt::create_api_key),
        )
        .route(
            "/api/v1/api-keys/{id}",
            delete(handlers::api_key_mgmt::revoke_api_key),
        )
        .route(
            "/api/v1/auth/mfa",
            get(handlers::mfa::status).delete(handlers::mfa::disable),
        )
        .route("/api/v1/auth/mfa/enroll", post(handlers::mfa::enroll))
        .route("/api/v1/auth/mfa/verify", post(handlers::mfa::verify_setup))
        .route(
            "/api/v1/auth/sso/providers",
            get(handlers::sso::list_providers).post(handlers::sso::create_provider),
        )
        .route(
            "/api/v1/auth/sso/providers/{id}",
            put(handlers::sso::update_provider).delete(handlers::sso::delete_provider),
        )
        .route(
            "/api/v1/auth/sessions",
            get(handlers::sessions::list_scoped_sessions),
        )
        .route(
            "/api/v1/auth/sessions/scoped",
            post(handlers::sessions::create_scoped_session),
        )
        .route(
            "/api/v1/auth/sessions/guest",
            post(handlers::sessions::create_guest_session),
        )
        .route(
            "/api/v1/auth/sessions/{id}",
            delete(handlers::sessions::revoke_scoped_session),
        )
        .route(
            "/api/v1/auth/cipher/hash",
            post(handlers::security_ops::hash_content),
        )
        .route(
            "/api/v1/auth/cipher/sign",
            post(handlers::security_ops::sign_content),
        )
        .route(
            "/api/v1/auth/cipher/verify",
            post(handlers::security_ops::verify_signature),
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
    tracing::info!("starting auth-service on {addr}");

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("failed to bind");

    axum::serve(listener, app).await.expect("server error");
}
