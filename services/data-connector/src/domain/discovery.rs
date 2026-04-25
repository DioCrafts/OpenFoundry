use uuid::Uuid;

use crate::{
    AppState, connectors,
    domain::agent_registry,
    models::{
        connection::Connection,
        registration::{
            AutoRegisterRequest, ConnectionRegistration, DiscoveredSource,
            VirtualTableQueryRequest, VirtualTableQueryResponse,
        },
    },
};

pub async fn discover_sources(
    state: &AppState,
    connection: &Connection,
) -> Result<Vec<DiscoveredSource>, String> {
    let agent_url = agent_registry::resolve_agent_url(state, &connection.config).await?;
    match connection.connector_type.as_str() {
        "postgresql" => connectors::postgres::discover_sources(&connection.config).await,
        "rest_api" => {
            connectors::rest_api::discover_sources(state, &connection.config, agent_url.as_deref())
                .await
        }
        "salesforce" => {
            connectors::salesforce::discover_sources(
                state,
                &connection.config,
                agent_url.as_deref(),
            )
            .await
        }
        "sap" => {
            connectors::sap::discover_sources(state, &connection.config, agent_url.as_deref()).await
        }
        "iot" => {
            connectors::iot::discover_sources(state, &connection.config, agent_url.as_deref()).await
        }
        "csv" | "json" => Ok(vec![DiscoveredSource {
            selector: connection.name.clone(),
            display_name: connection.name.clone(),
            source_kind: connection.connector_type.clone(),
            supports_sync: true,
            supports_zero_copy: true,
            source_signature: None,
            metadata: serde_json::json!({
                "connection_type": connection.connector_type,
            }),
        }]),
        other => Err(format!(
            "discover is not supported for connector type: {other}"
        )),
    }
}

pub async fn query_virtual_table(
    state: &AppState,
    connection: &Connection,
    request: &VirtualTableQueryRequest,
) -> Result<VirtualTableQueryResponse, String> {
    let agent_url = agent_registry::resolve_agent_url(state, &connection.config).await?;
    match connection.connector_type.as_str() {
        "postgresql" => {
            connectors::postgres::query_virtual_table(&connection.config, request).await
        }
        "rest_api" => {
            connectors::rest_api::query_virtual_table(
                state,
                &connection.config,
                request,
                agent_url.as_deref(),
            )
            .await
        }
        "salesforce" => {
            connectors::salesforce::query_virtual_table(
                state,
                &connection.config,
                request,
                agent_url.as_deref(),
            )
            .await
        }
        "sap" => {
            connectors::sap::query_virtual_table(
                state,
                &connection.config,
                request,
                agent_url.as_deref(),
            )
            .await
        }
        "iot" => {
            connectors::iot::query_virtual_table(
                state,
                &connection.config,
                request,
                agent_url.as_deref(),
            )
            .await
        }
        "csv" => connectors::csv::query_virtual_table(state, &connection.config, request).await,
        "json" => connectors::json::query_virtual_table(state, &connection.config, request).await,
        other => Err(format!(
            "zero-copy is not supported for connector type: {other}"
        )),
    }
}

pub async fn upsert_registration(
    state: &AppState,
    connection_id: Uuid,
    discovered: &DiscoveredSource,
    registration_mode: &str,
    auto_sync: bool,
    update_detection: bool,
    target_dataset_id: Option<Uuid>,
    metadata: serde_json::Value,
) -> Result<ConnectionRegistration, String> {
    let id = Uuid::now_v7();
    sqlx::query_as::<_, ConnectionRegistration>(
        r#"INSERT INTO connection_registrations (
               id, connection_id, selector, display_name, source_kind, registration_mode,
               auto_sync, update_detection, target_dataset_id, metadata
           )
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10::jsonb)
           ON CONFLICT (connection_id, selector)
           DO UPDATE SET
               display_name = EXCLUDED.display_name,
               source_kind = EXCLUDED.source_kind,
               registration_mode = EXCLUDED.registration_mode,
               auto_sync = EXCLUDED.auto_sync,
               update_detection = EXCLUDED.update_detection,
               target_dataset_id = EXCLUDED.target_dataset_id,
               metadata = connection_registrations.metadata || EXCLUDED.metadata,
               updated_at = NOW()
           RETURNING *"#,
    )
    .bind(id)
    .bind(connection_id)
    .bind(&discovered.selector)
    .bind(&discovered.display_name)
    .bind(&discovered.source_kind)
    .bind(registration_mode)
    .bind(auto_sync)
    .bind(update_detection)
    .bind(target_dataset_id)
    .bind(serde_json::json!({
        "discovery": discovered.metadata,
        "registration": metadata,
        "supports_zero_copy": discovered.supports_zero_copy,
        "supports_sync": discovered.supports_sync,
    }))
    .fetch_one(&state.db)
    .await
    .map_err(|error| error.to_string())
}

pub fn normalize_registration_mode(mode: Option<&str>) -> Result<&str, String> {
    match mode.unwrap_or("sync") {
        "sync" => Ok("sync"),
        "zero_copy" => Ok("zero_copy"),
        other => Err(format!(
            "registration_mode must be 'sync' or 'zero_copy', got '{other}'"
        )),
    }
}

pub fn select_sources<'a>(
    discovered: &'a [DiscoveredSource],
    request: &'a AutoRegisterRequest,
) -> Vec<&'a DiscoveredSource> {
    if request.selectors.is_empty() {
        return discovered.iter().collect();
    }
    discovered
        .iter()
        .filter(|source| {
            request
                .selectors
                .iter()
                .any(|selector| selector == &source.selector)
        })
        .collect()
}
