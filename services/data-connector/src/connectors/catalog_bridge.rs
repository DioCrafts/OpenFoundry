use std::time::Instant;

use reqwest::{Url, header::HeaderMap};
use serde_json::{Value, json};

use super::{
    ConnectionTestResult, SyncPayload, add_source_signature, basic_discovered_source, http_runtime,
    virtual_table_response,
};
use crate::{
    AppState,
    models::registration::{DiscoveredSource, VirtualTableQueryRequest, VirtualTableQueryResponse},
};

#[derive(Debug, Clone)]
struct CatalogEntry {
    selector: String,
    display_name: String,
    source_kind: String,
    path: Option<String>,
    sample_rows: Vec<Value>,
    supports_sync: bool,
    supports_zero_copy: bool,
    metadata: Value,
}

pub fn validate_tabular_connector_config(
    config: &Value,
    connector_name: &str,
    identity_fields: &[&str],
) -> Result<(), String> {
    let entries = inline_catalog_entries(config, connector_name, connector_name)?;
    if !entries.is_empty() {
        return Ok(());
    }

    let has_base_url = string_field(config, "base_url").is_some();
    let has_catalog_path = string_field(config, "catalog_path").is_some();
    let has_resource_template = string_field(config, "resource_path_template")
        .or_else(|| string_field(config, "query_path_template"))
        .is_some();

    if has_base_url && has_catalog_path {
        return Ok(());
    }

    if has_base_url && has_resource_template {
        let missing = identity_fields
            .iter()
            .filter(|field| config.get(**field).is_none())
            .copied()
            .collect::<Vec<_>>();
        if missing.is_empty() {
            return Ok(());
        }
        return Err(format!(
            "{connector_name} connector requires {} when using resource templates",
            quoted_join(&missing)
        ));
    }

    Err(format!(
        "{connector_name} connector requires inline 'tables' or 'base_url' plus 'catalog_path'/'resource_path_template'"
    ))
}

pub async fn test_tabular_connector_connection(
    state: &AppState,
    config: &Value,
    agent_url: Option<&str>,
    connector_name: &str,
    default_source_kind: &str,
) -> Result<ConnectionTestResult, String> {
    let entries = inline_catalog_entries(config, connector_name, default_source_kind)?;
    if let Some(url) = health_url(config, entries.first())? {
        let started = Instant::now();
        let response = http_runtime::get(
            state,
            config,
            url.clone(),
            build_headers(config)?,
            bearer_token(config),
            agent_url,
        )
        .await?;
        if !(200..300).contains(&response.status) {
            return Err(format!(
                "{connector_name} bridge returned HTTP {}",
                response.status
            ));
        }

        return Ok(ConnectionTestResult {
            success: true,
            message: format!(
                "{connector_name} bridge responded with HTTP {}",
                response.status
            ),
            latency_ms: started.elapsed().as_millis(),
            details: Some(json!({
                "url": url.as_str(),
                "catalog_sources": entries.len(),
                "agent_url": agent_url,
            })),
        });
    }

    Ok(ConnectionTestResult {
        success: true,
        message: format!(
            "validated {connector_name} catalog with {} source(s)",
            entries.len()
        ),
        latency_ms: 0,
        details: Some(json!({
            "catalog_sources": entries.len(),
            "mode": "inline_catalog",
        })),
    })
}

pub async fn discover_tabular_sources(
    state: &AppState,
    config: &Value,
    agent_url: Option<&str>,
    connector_name: &str,
    default_source_kind: &str,
) -> Result<Vec<DiscoveredSource>, String> {
    let entries = inline_catalog_entries(config, connector_name, default_source_kind)?;
    if !entries.is_empty() {
        return Ok(entries_to_sources(entries));
    }

    let url = catalog_url(config)?;
    let response = http_runtime::get(
        state,
        config,
        url,
        build_headers(config)?,
        bearer_token(config),
        agent_url,
    )
    .await?;
    if !(200..300).contains(&response.status) {
        return Err(format!(
            "{connector_name} catalog returned HTTP {}",
            response.status
        ));
    }

    let payload = http_runtime::json_body(&response)?;
    let entries = catalog_entries_from_value(payload, connector_name, default_source_kind)?;
    if entries.is_empty() {
        return Err(format!(
            "{connector_name} catalog did not expose any sources"
        ));
    }
    Ok(entries_to_sources(entries))
}

pub async fn query_tabular_virtual_table(
    state: &AppState,
    config: &Value,
    request: &VirtualTableQueryRequest,
    agent_url: Option<&str>,
    connector_name: &str,
    default_source_kind: &str,
) -> Result<VirtualTableQueryResponse, String> {
    let (rows, metadata) = resolve_rows(
        state,
        config,
        &request.selector,
        request.limit,
        agent_url,
        connector_name,
        default_source_kind,
    )
    .await?;

    Ok(virtual_table_response(&request.selector, rows, metadata))
}

pub async fn fetch_tabular_dataset(
    state: &AppState,
    config: &Value,
    selector: &str,
    agent_url: Option<&str>,
    connector_name: &str,
    default_source_kind: &str,
) -> Result<SyncPayload, String> {
    let (rows, metadata) = resolve_rows(
        state,
        config,
        selector,
        None,
        agent_url,
        connector_name,
        default_source_kind,
    )
    .await?;

    let rows_synced = rows.len() as i64;
    let mut payload = SyncPayload {
        bytes: serde_json::to_vec(&rows).map_err(|error| error.to_string())?,
        format: "json".to_string(),
        rows_synced,
        file_name: format!("{}.json", sanitize_file_stem(selector, connector_name)),
        metadata,
    };
    add_source_signature(&mut payload);
    Ok(payload)
}

async fn resolve_rows(
    state: &AppState,
    config: &Value,
    selector: &str,
    limit: Option<usize>,
    agent_url: Option<&str>,
    connector_name: &str,
    default_source_kind: &str,
) -> Result<(Vec<Value>, Value), String> {
    let limit = limit.unwrap_or(50).clamp(1, 500);
    let entries = inline_catalog_entries(config, connector_name, default_source_kind)?;
    let selected_entry = entries.into_iter().find(|entry| entry.selector == selector);

    if let Some(entry) = selected_entry.as_ref()
        && !entry.sample_rows.is_empty()
    {
        let rows = entry
            .sample_rows
            .iter()
            .take(limit)
            .cloned()
            .collect::<Vec<_>>();
        return Ok((
            rows,
            json!({
                "connector": connector_name,
                "selector": selector,
                "mode": "inline_catalog",
                "entry": entry.metadata,
            }),
        ));
    }

    let url = source_url(config, selector, selected_entry.as_ref())?;
    let response = http_runtime::get(
        state,
        config,
        url.clone(),
        build_headers(config)?,
        bearer_token(config),
        agent_url,
    )
    .await?;
    if !(200..300).contains(&response.status) {
        return Err(format!(
            "{connector_name} source '{selector}' returned HTTP {}",
            response.status
        ));
    }

    let payload = http_runtime::json_body(&response)?;
    let rows = normalize_records(payload)
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .take(limit)
        .collect::<Vec<_>>();

    Ok((
        rows,
        json!({
            "connector": connector_name,
            "selector": selector,
            "mode": "bridge_fetch",
            "url": url.as_str(),
            "entry": selected_entry.map(|entry| entry.metadata),
            "agent_url": agent_url,
        }),
    ))
}

fn inline_catalog_entries(
    config: &Value,
    connector_name: &str,
    default_source_kind: &str,
) -> Result<Vec<CatalogEntry>, String> {
    let Some(entries) = config.get("tables") else {
        return Ok(Vec::new());
    };
    let entries = entries
        .as_array()
        .ok_or_else(|| format!("{connector_name} connector expects 'tables' to be an array"))?;

    let mut parsed = Vec::with_capacity(entries.len());
    for (index, entry) in entries.iter().enumerate() {
        parsed.push(
            parse_catalog_entry(entry, default_source_kind).ok_or_else(|| {
                format!(
                    "{connector_name} connector tables[{index}] requires 'selector', 'name' or 'table'"
                )
            })?,
        );
    }
    Ok(parsed)
}

fn catalog_entries_from_value(
    payload: Value,
    connector_name: &str,
    default_source_kind: &str,
) -> Result<Vec<CatalogEntry>, String> {
    let rows = normalize_records(payload)
        .as_array()
        .cloned()
        .unwrap_or_default();
    let mut entries = Vec::with_capacity(rows.len());
    for (index, entry) in rows.iter().enumerate() {
        entries.push(
            parse_catalog_entry(entry, default_source_kind).ok_or_else(|| {
                format!(
                    "{connector_name} catalog row {index} requires 'selector', 'name' or 'table'"
                )
            })?,
        );
    }
    Ok(entries)
}

fn parse_catalog_entry(value: &Value, default_source_kind: &str) -> Option<CatalogEntry> {
    let selector = value
        .get("selector")
        .or_else(|| value.get("name"))
        .or_else(|| value.get("table"))
        .and_then(Value::as_str)?
        .trim()
        .to_string();
    if selector.is_empty() {
        return None;
    }

    let display_name = value
        .get("display_name")
        .or_else(|| value.get("title"))
        .or_else(|| value.get("name"))
        .and_then(Value::as_str)
        .unwrap_or(&selector)
        .to_string();
    let source_kind = value
        .get("source_kind")
        .or_else(|| value.get("kind"))
        .and_then(Value::as_str)
        .unwrap_or(default_source_kind)
        .to_string();
    let path = value
        .get("path")
        .or_else(|| value.get("resource_path"))
        .or_else(|| value.get("query_path"))
        .and_then(Value::as_str)
        .map(|value| value.to_string());
    let sample_rows = value
        .get("sample_rows")
        .or_else(|| value.get("preview_rows"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let supports_sync = value
        .get("supports_sync")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let supports_zero_copy = value
        .get("supports_zero_copy")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let mut metadata = value.clone();
    if let Some(object) = metadata.as_object_mut() {
        object.remove("sample_rows");
        object.remove("preview_rows");
    }

    Some(CatalogEntry {
        selector,
        display_name,
        source_kind,
        path,
        sample_rows,
        supports_sync,
        supports_zero_copy,
        metadata,
    })
}

fn entries_to_sources(entries: Vec<CatalogEntry>) -> Vec<DiscoveredSource> {
    entries
        .into_iter()
        .map(|entry| {
            let mut source = basic_discovered_source(
                entry.selector,
                entry.display_name,
                entry.source_kind,
                entry.metadata,
            );
            source.supports_sync = entry.supports_sync;
            source.supports_zero_copy = entry.supports_zero_copy;
            source
        })
        .collect()
}

fn health_url(config: &Value, first_entry: Option<&CatalogEntry>) -> Result<Option<Url>, String> {
    let Some(base_url) = string_field(config, "base_url") else {
        return Ok(None);
    };
    let template = string_field(config, "health_path")
        .or_else(|| string_field(config, "catalog_path"))
        .or_else(|| first_entry.and_then(|entry| entry.path.as_deref()));
    let Some(template) = template else {
        return Ok(None);
    };

    build_url(
        base_url,
        &interpolate_template(
            template,
            config,
            first_entry
                .map(|entry| entry.selector.as_str())
                .unwrap_or(""),
        ),
    )
    .map(Some)
}

fn catalog_url(config: &Value) -> Result<Url, String> {
    let base_url = string_field(config, "base_url")
        .ok_or_else(|| "connector bridge requires 'base_url'".to_string())?;
    let catalog_path = string_field(config, "catalog_path").ok_or_else(|| {
        "connector bridge requires 'catalog_path' when tables are not inlined".to_string()
    })?;
    build_url(base_url, &interpolate_template(catalog_path, config, ""))
}

fn source_url(config: &Value, selector: &str, entry: Option<&CatalogEntry>) -> Result<Url, String> {
    let base_url = string_field(config, "base_url").ok_or_else(|| {
        "connector bridge requires 'base_url' for remote table access".to_string()
    })?;
    let template = entry
        .and_then(|entry| entry.path.as_deref())
        .or_else(|| string_field(config, "resource_path_template"))
        .or_else(|| string_field(config, "query_path_template"))
        .unwrap_or(selector);
    build_url(base_url, &interpolate_template(template, config, selector))
}

fn interpolate_template(template: &str, config: &Value, selector: &str) -> String {
    let mut rendered = template.replace("{selector}", selector);
    for field in [
        "project_id",
        "dataset_id",
        "account",
        "database",
        "schema",
        "warehouse",
        "region",
    ] {
        if let Some(value) = string_field(config, field) {
            rendered = rendered.replace(&format!("{{{field}}}"), value);
        }
    }
    rendered
}

fn build_url(base_url: &str, path: &str) -> Result<Url, String> {
    let base = Url::parse(base_url).map_err(|error| error.to_string())?;
    base.join(path).map_err(|error| error.to_string())
}

fn bearer_token(config: &Value) -> Option<String> {
    string_field(config, "bearer_token").map(|value| value.to_string())
}

fn build_headers(config: &Value) -> Result<HeaderMap, String> {
    http_runtime::header_map(config)
}

fn string_field<'a>(config: &'a Value, field: &str) -> Option<&'a str> {
    config
        .get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
}

fn normalize_records(payload: Value) -> Value {
    match payload {
        Value::Array(rows) => Value::Array(rows),
        Value::Object(mut object) => {
            if let Some(records) = object.remove("data").and_then(array_if_any) {
                Value::Array(records)
            } else if let Some(records) = object.remove("items").and_then(array_if_any) {
                Value::Array(records)
            } else if let Some(records) = object.remove("records").and_then(array_if_any) {
                Value::Array(records)
            } else if let Some(records) = object.remove("value").and_then(array_if_any) {
                Value::Array(records)
            } else {
                Value::Array(vec![Value::Object(object)])
            }
        }
        other => Value::Array(vec![json!({ "value": other })]),
    }
}

fn array_if_any(value: Value) -> Option<Vec<Value>> {
    value.as_array().cloned()
}

fn sanitize_file_stem(selector: &str, fallback: &str) -> String {
    let stem = selector
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect::<String>()
        .trim_matches('_')
        .to_string();
    if stem.is_empty() {
        fallback.to_string()
    } else {
        stem.chars().take(64).collect()
    }
}

fn quoted_join(fields: &[&str]) -> String {
    fields
        .iter()
        .map(|field| format!("'{field}'"))
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{
        inline_catalog_entries, normalize_records, query_tabular_virtual_table,
        validate_tabular_connector_config,
    };
    use crate::{AppState, models::registration::VirtualTableQueryRequest};
    use auth_middleware::jwt::JwtConfig;

    fn state() -> AppState {
        AppState {
            db: sqlx::postgres::PgPoolOptions::new()
                .connect_lazy("postgres://postgres:postgres@localhost/openfoundry")
                .expect("lazy pool"),
            jwt_config: JwtConfig::new("secret"),
            http_client: reqwest::Client::new(),
            dataset_service_url: "http://localhost:50053".to_string(),
            pipeline_service_url: "http://localhost:50056".to_string(),
            ontology_service_url: "http://localhost:50057".to_string(),
            allowed_egress_hosts: Vec::new(),
            allow_private_network_egress: true,
            agent_stale_after: chrono::Duration::seconds(60),
        }
    }

    #[test]
    fn validates_inline_catalogs() {
        let config = json!({
            "tables": [
                {
                    "selector": "analytics.orders",
                    "sample_rows": [{ "id": 1 }, { "id": 2 }]
                }
            ]
        });
        assert!(validate_tabular_connector_config(&config, "bigquery", &["project_id"]).is_ok());
        let entries =
            inline_catalog_entries(&config, "bigquery", "bigquery_table").expect("catalog entries");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].selector, "analytics.orders");
    }

    #[test]
    fn rejects_missing_catalog_and_bridge() {
        let config = json!({ "project_id": "acme-warehouse" });
        let error = validate_tabular_connector_config(&config, "bigquery", &["project_id"])
            .expect_err("validation should fail");
        assert!(error.contains("inline 'tables'"));
    }

    #[test]
    fn normalizes_wrapped_payloads() {
        assert_eq!(
            normalize_records(json!({ "items": [{ "id": 1 }, { "id": 2 }] })),
            json!([{ "id": 1 }, { "id": 2 }])
        );
    }

    #[tokio::test]
    async fn serves_inline_rows_as_virtual_table() {
        let request = VirtualTableQueryRequest {
            selector: "analytics.orders".to_string(),
            limit: Some(1),
        };
        let response = query_tabular_virtual_table(
            &state(),
            &json!({
                "tables": [
                    {
                        "selector": "analytics.orders",
                        "display_name": "Orders",
                        "sample_rows": [{ "id": 1 }, { "id": 2 }]
                    }
                ]
            }),
            &request,
            None,
            "bigquery",
            "bigquery_table",
        )
        .await
        .expect("virtual query should succeed");

        assert_eq!(response.selector, "analytics.orders");
        assert_eq!(response.row_count, 1);
        assert_eq!(response.rows, vec![json!({ "id": 1 })]);
    }
}
