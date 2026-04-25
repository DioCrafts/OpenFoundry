mod config;
mod domain;
mod handlers;
mod models;

use auth_middleware::jwt::JwtConfig;
use axum::{
    Router, middleware,
    routing::{delete, get, patch, post, put},
};
use core_models::{health::HealthStatus, observability};
use sqlx::postgres::PgPoolOptions;

#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::PgPool,
    pub jwt_config: JwtConfig,
    pub http_client: reqwest::Client,
    pub audit_service_url: String,
    pub ontology_service_url: String,
    pub ai_service_url: String,
    pub node_runtime_command: String,
}

impl axum::extract::FromRef<AppState> for JwtConfig {
    fn from_ref(state: &AppState) -> Self {
        state.jwt_config.clone()
    }
}

#[tokio::main]
async fn main() {
    observability::init_tracing("ontology-service");

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
        .expect("failed to build ontology HTTP client");

    let state = AppState {
        db: pool,
        jwt_config: jwt_config.clone(),
        http_client,
        audit_service_url: cfg.audit_service_url.clone(),
        ontology_service_url: cfg.ontology_service_url.clone(),
        ai_service_url: cfg.ai_service_url.clone(),
        node_runtime_command: cfg.node_runtime_command.clone(),
    };

    let public = Router::new().route(
        "/health",
        get(|| async { axum::Json(HealthStatus::ok("ontology-service")) }),
    );

    let protected = Router::new()
        // Object types
        .route("/api/v1/ontology/types", post(handlers::types::create_object_type))
        .route("/api/v1/ontology/types", get(handlers::types::list_object_types))
        .route("/api/v1/ontology/types/{id}", get(handlers::types::get_object_type))
        .route("/api/v1/ontology/types/{id}", put(handlers::types::update_object_type))
        .route("/api/v1/ontology/types/{id}", delete(handlers::types::delete_object_type))
        .route(
            "/api/v1/ontology/types/{type_id}/properties",
            get(handlers::properties::list_properties).post(handlers::properties::create_property),
        )
        .route(
            "/api/v1/ontology/types/{type_id}/properties/{property_id}",
            axum::routing::patch(handlers::properties::update_property)
                .delete(handlers::properties::delete_property),
        )
        .route(
            "/api/v1/ontology/interfaces",
            get(handlers::interfaces::list_interfaces).post(handlers::interfaces::create_interface),
        )
        .route(
            "/api/v1/ontology/interfaces/{id}",
            get(handlers::interfaces::get_interface)
                .patch(handlers::interfaces::update_interface)
                .delete(handlers::interfaces::delete_interface),
        )
        .route(
            "/api/v1/ontology/interfaces/{id}/properties",
            get(handlers::interfaces::list_interface_properties)
                .post(handlers::interfaces::create_interface_property),
        )
        .route(
            "/api/v1/ontology/interfaces/{id}/properties/{property_id}",
            axum::routing::patch(handlers::interfaces::update_interface_property)
                .delete(handlers::interfaces::delete_interface_property),
        )
        .route(
            "/api/v1/ontology/shared-property-types",
            get(handlers::shared_properties::list_shared_property_types)
                .post(handlers::shared_properties::create_shared_property_type),
        )
        .route(
            "/api/v1/ontology/shared-property-types/{id}",
            get(handlers::shared_properties::get_shared_property_type)
                .patch(handlers::shared_properties::update_shared_property_type)
                .delete(handlers::shared_properties::delete_shared_property_type),
        )
        .route(
            "/api/v1/ontology/functions",
            get(handlers::functions::list_function_packages)
                .post(handlers::functions::create_function_package),
        )
        .route(
            "/api/v1/ontology/functions/{id}",
            get(handlers::functions::get_function_package)
                .patch(handlers::functions::update_function_package)
                .delete(handlers::functions::delete_function_package),
        )
        .route(
            "/api/v1/ontology/functions/{id}/validate",
            post(handlers::functions::validate_function_package),
        )
        .route(
            "/api/v1/ontology/functions/{id}/simulate",
            post(handlers::functions::simulate_function_package),
        )
        .route(
            "/api/v1/ontology/rules",
            get(handlers::rules::list_rules).post(handlers::rules::create_rule),
        )
        .route(
            "/api/v1/ontology/rules/insights",
            get(handlers::rules::get_machinery_insights),
        )
        .route(
            "/api/v1/ontology/rules/machinery/queue",
            get(handlers::rules::get_machinery_queue),
        )
        .route(
            "/api/v1/ontology/rules/machinery/queue/{id}",
            patch(handlers::rules::update_machinery_queue_item),
        )
        .route(
            "/api/v1/ontology/rules/{id}",
            get(handlers::rules::get_rule)
                .patch(handlers::rules::update_rule)
                .delete(handlers::rules::delete_rule),
        )
        .route(
            "/api/v1/ontology/rules/{id}/simulate",
            post(handlers::rules::simulate_rule),
        )
        .route(
            "/api/v1/ontology/rules/{id}/apply",
            post(handlers::rules::apply_rule),
        )
        .route(
            "/api/v1/ontology/types/{type_id}/interfaces",
            get(handlers::interfaces::list_type_interfaces),
        )
        .route(
            "/api/v1/ontology/types/{type_id}/rules",
            get(handlers::rules::list_rules_for_object_type),
        )
        .route(
            "/api/v1/ontology/types/{type_id}/interfaces/{interface_id}",
            post(handlers::interfaces::attach_interface_to_type)
                .delete(handlers::interfaces::detach_interface_from_type),
        )
        .route(
            "/api/v1/ontology/types/{type_id}/shared-property-types",
            get(handlers::shared_properties::list_type_shared_property_types),
        )
        .route(
            "/api/v1/ontology/types/{type_id}/shared-property-types/{shared_property_type_id}",
            post(handlers::shared_properties::attach_shared_property_type_to_type)
                .delete(handlers::shared_properties::detach_shared_property_type_from_type),
        )
        // Action types
        .route("/api/v1/ontology/actions", post(handlers::actions::create_action_type))
        .route("/api/v1/ontology/actions", get(handlers::actions::list_action_types))
        .route("/api/v1/ontology/actions/{id}", get(handlers::actions::get_action_type))
        .route("/api/v1/ontology/actions/{id}", put(handlers::actions::update_action_type))
        .route("/api/v1/ontology/actions/{id}", delete(handlers::actions::delete_action_type))
        .route("/api/v1/ontology/actions/{id}/validate", post(handlers::actions::validate_action))
        .route("/api/v1/ontology/actions/{id}/execute", post(handlers::actions::execute_action))
        .route(
            "/api/v1/ontology/actions/{id}/what-if",
            get(handlers::actions::list_action_what_if_branches)
                .post(handlers::actions::create_action_what_if_branch),
        )
        .route(
            "/api/v1/ontology/actions/{id}/what-if/{branch_id}",
            delete(handlers::actions::delete_action_what_if_branch),
        )
        .route(
            "/api/v1/ontology/actions/{id}/execute-batch",
            post(handlers::actions::execute_action_batch),
        )
        // Object instances
        .route("/api/v1/ontology/types/{type_id}/objects", post(handlers::objects::create_object))
        .route("/api/v1/ontology/types/{type_id}/objects", get(handlers::objects::list_objects))
        .route(
            "/api/v1/ontology/types/{type_id}/objects/query",
            post(handlers::objects::query_objects),
        )
        .route("/api/v1/ontology/types/{type_id}/objects/{obj_id}", get(handlers::objects::get_object))
        .route(
            "/api/v1/ontology/types/{type_id}/objects/{obj_id}",
            patch(handlers::objects::update_object),
        )
        .route("/api/v1/ontology/types/{type_id}/objects/{obj_id}", delete(handlers::objects::delete_object))
        .route(
            "/api/v1/ontology/types/{type_id}/objects/{obj_id}/neighbors",
            get(handlers::objects::list_neighbors),
        )
        .route(
            "/api/v1/ontology/types/{type_id}/objects/{obj_id}/view",
            get(handlers::objects::get_object_view),
        )
        .route(
            "/api/v1/ontology/types/{type_id}/objects/{obj_id}/simulate",
            post(handlers::objects::simulate_object),
        )
        .route(
            "/api/v1/ontology/objects/{obj_id}/rule-runs",
            get(handlers::rules::list_object_rule_runs),
        )
        .route("/api/v1/ontology/search", post(handlers::search::search_ontology))
        .route("/api/v1/ontology/graph", get(handlers::search::get_graph))
        .route(
            "/api/v1/ontology/quiver/vega-spec",
            post(handlers::search::get_quiver_vega_spec),
        )
        .route(
            "/api/v1/ontology/quiver/visual-functions",
            get(handlers::search::list_quiver_visual_functions)
                .post(handlers::search::create_quiver_visual_function),
        )
        .route(
            "/api/v1/ontology/quiver/visual-functions/{id}",
            get(handlers::search::get_quiver_visual_function)
                .patch(handlers::search::update_quiver_visual_function)
                .delete(handlers::search::delete_quiver_visual_function),
        )
        .route(
            "/api/v1/ontology/object-sets",
            get(handlers::object_sets::list_object_sets)
                .post(handlers::object_sets::create_object_set),
        )
        .route(
            "/api/v1/ontology/object-sets/{id}",
            get(handlers::object_sets::get_object_set)
                .patch(handlers::object_sets::update_object_set)
                .delete(handlers::object_sets::delete_object_set),
        )
        .route(
            "/api/v1/ontology/object-sets/{id}/evaluate",
            post(handlers::object_sets::evaluate_object_set),
        )
        .route(
            "/api/v1/ontology/object-sets/{id}/materialize",
            post(handlers::object_sets::materialize_object_set),
        )

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
