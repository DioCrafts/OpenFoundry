use std::collections::BTreeSet;

use chrono::Utc;
use serde_json::Value;

use crate::{
    AppState,
    domain::{discovery, schema_inference},
    models::{
        connection::Connection,
        hyperauto::{
            HyperAutoErpEntityPlan, HyperAutoErpFieldPlan, HyperAutoErpLinkPlan,
            HyperAutoErpPreviewResponse, HyperAutoErpRequest,
        },
        registration::{DiscoveredSource, VirtualTableQueryRequest},
    },
};

const DEFAULT_MAX_ENTITIES: usize = 6;
const DEFAULT_SAMPLE_LIMIT: usize = 25;

pub async fn build_erp_preview(
    state: &AppState,
    connection: &Connection,
    request: &HyperAutoErpRequest,
) -> Result<HyperAutoErpPreviewResponse, String> {
    let Some(erp_system) = resolve_enterprise_system(connection) else {
        return Err(format!(
            "HyperAuto ERP generation currently requires SAP, Salesforce, or config.enterprise_system on the connection; got '{}'",
            connection.connector_type
        ));
    };

    let discovered = discovery::discover_sources(state, connection).await?;
    if discovered.is_empty() {
        return Err("no ERP sources were discovered for this connection".to_string());
    }

    let (selected_sources, mut warnings) = select_sources(&discovered, request);
    if selected_sources.is_empty() {
        return Err("no ERP sources matched the requested selectors".to_string());
    }

    let sample_limit = request
        .sample_limit
        .unwrap_or(DEFAULT_SAMPLE_LIMIT)
        .clamp(1, 100);
    let pipeline_status = normalize_pipeline_status(
        request.pipeline_status.as_deref(),
        request.schedule_cron.as_deref(),
    )
    .to_string();

    let mut entities = Vec::new();
    for source in selected_sources {
        let sample = discovery::query_virtual_table(
            state,
            connection,
            &VirtualTableQueryRequest {
                selector: source.selector.clone(),
                limit: Some(sample_limit),
            },
        )
        .await;

        match sample {
            Ok(sample) => entities.push(plan_entity(connection, source, &sample.rows)),
            Err(error) => {
                warnings.push(format!(
                    "sample query for '{}' failed, generating fallback blueprint: {error}",
                    source.selector
                ));
                entities.push(plan_entity(connection, source, &[]));
            }
        }
    }

    let links = infer_links(&entities);

    Ok(HyperAutoErpPreviewResponse {
        connection_id: connection.id,
        connection_name: connection.name.clone(),
        connector_type: connection.connector_type.clone(),
        erp_system,
        generated_at: Utc::now(),
        entity_count: entities.len(),
        pipeline_status,
        schedule_cron: request.schedule_cron.clone(),
        entities,
        links,
        warnings,
    })
}

pub fn normalize_pipeline_status(
    status: Option<&str>,
    schedule_cron: Option<&str>,
) -> &'static str {
    match status.unwrap_or_else(|| {
        if schedule_cron.is_some() {
            "active"
        } else {
            "draft"
        }
    }) {
        "active" => "active",
        "paused" => "paused",
        _ => "draft",
    }
}

fn select_sources<'a>(
    discovered: &'a [DiscoveredSource],
    request: &HyperAutoErpRequest,
) -> (Vec<&'a DiscoveredSource>, Vec<String>) {
    let mut warnings = Vec::new();
    if !request.selectors.is_empty() {
        let mut selected = Vec::new();
        for selector in &request.selectors {
            match discovered
                .iter()
                .find(|source| source.selector == *selector)
            {
                Some(source) => selected.push(source),
                None => warnings.push(format!(
                    "requested selector '{}' was not found in ERP discovery",
                    selector
                )),
            }
        }
        return (selected, warnings);
    }

    let max_entities = request
        .max_entities
        .unwrap_or(DEFAULT_MAX_ENTITIES)
        .clamp(1, 20);
    let mut selected = discovered.iter().take(max_entities).collect::<Vec<_>>();
    if discovered.len() > selected.len() {
        warnings.push(format!(
            "discovery returned {} ERP sources; preview limited to the first {}. Pass explicit selectors to generate a larger blueprint.",
            discovered.len(),
            selected.len()
        ));
    }
    selected.sort_by_key(|source| source.display_name.to_ascii_lowercase());
    (selected, warnings)
}

fn plan_entity(
    connection: &Connection,
    source: &DiscoveredSource,
    sample_rows: &[Value],
) -> HyperAutoErpEntityPlan {
    let erp_system = resolve_enterprise_system(connection).unwrap_or_else(|| "erp".to_string());
    let object_stem = object_stem(&source.selector, &source.display_name);
    let connection_slug = slugify(&connection.name);
    let entity_slug = slugify(&source.selector);
    let sample_map = sample_rows
        .first()
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    let mut fields = schema_inference::infer_from_json_sample(sample_rows)
        .into_iter()
        .map(|column| {
            let source_name = column.name;
            let property_name = slugify(&source_name);
            let sample_value = sample_map.get(&source_name);
            HyperAutoErpFieldPlan {
                source_name: source_name.clone(),
                property_name,
                property_type: infer_property_type(&column.inferred_type, sample_value).to_string(),
                nullable: column.nullable,
                unique_constraint: false,
                semantic_role: if looks_reference_field(&source_name) {
                    "reference_candidate".to_string()
                } else {
                    "attribute".to_string()
                },
            }
        })
        .collect::<Vec<_>>();

    let primary_key_property = infer_primary_key(&fields, &source.selector, &source.display_name);
    if let Some(primary_key_property) = primary_key_property.as_deref() {
        for field in &mut fields {
            if field.property_name == primary_key_property {
                field.unique_constraint = true;
                field.nullable = false;
                field.semantic_role = "primary_key".to_string();
            }
        }
    }

    let module = infer_module(&source.selector, &fields);
    let object_display_name = display_label(&source.display_name);
    let object_type_name = format!("{erp_system}_{object_stem}");
    let raw_dataset_name = format!("erp_{connection_slug}_{entity_slug}_raw");
    let curated_dataset_name = format!("erp_{connection_slug}_{entity_slug}_curated");
    let alias = raw_dataset_name.clone();
    let normalization_sql = build_normalization_sql(&alias, &fields);
    let pipeline_name = format!("ERP {} {} Pipeline", connection.name, object_display_name);

    HyperAutoErpEntityPlan {
        selector: source.selector.clone(),
        display_name: source.display_name.clone(),
        source_kind: source.source_kind.clone(),
        module,
        sample_row_count: sample_rows.len(),
        raw_dataset_name,
        curated_dataset_name,
        pipeline_name,
        object_type_name,
        object_display_name,
        primary_key_property,
        normalization_sql,
        fields,
    }
}

fn infer_property_type(inferred_type: &str, sample_value: Option<&Value>) -> &'static str {
    match inferred_type {
        "boolean" => "boolean",
        "float64" => "float",
        "int64" => "integer",
        "list" => "array",
        "struct" => "json",
        _ => match sample_value {
            Some(Value::String(value)) if looks_like_timestamp(value) => "timestamp",
            Some(Value::String(value)) if looks_like_date(value) => "date",
            _ => "string",
        },
    }
}

fn infer_primary_key(
    fields: &[HyperAutoErpFieldPlan],
    selector: &str,
    display_name: &str,
) -> Option<String> {
    if fields.is_empty() {
        return None;
    }

    let preferred_prefixes = [
        slugify(selector),
        slugify(display_name),
        slugify(&singularize(selector)),
        slugify(&singularize(display_name)),
    ];

    for preferred in &preferred_prefixes {
        if preferred.is_empty() {
            continue;
        }
        for suffix in ["id", "code", "number", "key"] {
            let candidate = format!("{preferred}_{suffix}");
            if let Some(field) = fields.iter().find(|field| field.property_name == candidate) {
                return Some(field.property_name.clone());
            }
        }
    }

    for candidate in ["id", "object_id", "entity_id", "record_id"] {
        if let Some(field) = fields.iter().find(|field| field.property_name == candidate) {
            return Some(field.property_name.clone());
        }
    }

    fields
        .iter()
        .find(|field| {
            field.property_name.ends_with("_id")
                || field.property_name.ends_with("_code")
                || field.property_name.ends_with("_number")
        })
        .map(|field| field.property_name.clone())
        .or_else(|| fields.first().map(|field| field.property_name.clone()))
}

fn infer_module(selector: &str, fields: &[HyperAutoErpFieldPlan]) -> String {
    let haystack = format!(
        "{} {}",
        selector.to_ascii_lowercase(),
        fields
            .iter()
            .map(|field| field.property_name.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    );

    if contains_any(
        &haystack,
        &[
            "sales", "customer", "order", "invoice", "billing", "kna1", "knvv", "vbak", "vbap",
        ],
    ) {
        "sales".to_string()
    } else if contains_any(
        &haystack,
        &[
            "vendor",
            "supplier",
            "purchase",
            "procurement",
            "ekko",
            "ekpo",
            "lfa1",
        ],
    ) {
        "procurement".to_string()
    } else if contains_any(
        &haystack,
        &[
            "material",
            "inventory",
            "stock",
            "warehouse",
            "mara",
            "marc",
            "mseg",
            "mkpf",
        ],
    ) {
        "inventory".to_string()
    } else if contains_any(
        &haystack,
        &[
            "finance",
            "ledger",
            "account",
            "gl",
            "bkpf",
            "bseg",
            "faglflex",
            "cost_center",
        ],
    ) {
        "finance".to_string()
    } else if contains_any(
        &haystack,
        &["employee", "personnel", "payroll", "hr", "pernr", "human"],
    ) {
        "hr".to_string()
    } else if contains_any(
        &haystack,
        &["delivery", "shipment", "route", "logistics", "likp", "lips"],
    ) {
        "logistics".to_string()
    } else {
        "enterprise".to_string()
    }
}

fn build_normalization_sql(alias: &str, fields: &[HyperAutoErpFieldPlan]) -> String {
    if fields.is_empty() {
        return format!("SELECT * FROM {}", quote_identifier(alias));
    }

    let projection = fields
        .iter()
        .map(|field| {
            format!(
                "  {} AS {}",
                quote_identifier(&field.source_name),
                quote_identifier(&field.property_name)
            )
        })
        .collect::<Vec<_>>()
        .join(",\n");

    format!("SELECT\n{projection}\nFROM {}", quote_identifier(alias))
}

pub fn infer_links(entities: &[HyperAutoErpEntityPlan]) -> Vec<HyperAutoErpLinkPlan> {
    let mut links = Vec::new();
    let mut seen = BTreeSet::new();

    for source in entities {
        for field in &source.fields {
            if field.semantic_role != "reference_candidate" {
                continue;
            }
            let reference_base = reference_base(&field.property_name);
            if reference_base.is_empty() {
                continue;
            }

            let mut best_target = None;
            let mut best_score = 0usize;
            for target in entities {
                if target.object_type_name == source.object_type_name {
                    continue;
                }
                for alias in target_aliases(target) {
                    if alias.is_empty() {
                        continue;
                    }
                    if reference_base == alias
                        || reference_base.ends_with(&alias)
                        || alias.ends_with(&reference_base)
                    {
                        let score = alias.len();
                        if score > best_score {
                            best_score = score;
                            best_target = Some(target);
                        }
                    }
                }
            }

            let Some(target) = best_target else {
                continue;
            };

            let key = (
                source.object_type_name.clone(),
                target.object_type_name.clone(),
                field.property_name.clone(),
            );
            if !seen.insert(key) {
                continue;
            }

            links.push(HyperAutoErpLinkPlan {
                name: format!("{}_to_{}", source.object_type_name, target.object_type_name),
                display_name: format!(
                    "{} to {}",
                    source.object_display_name, target.object_display_name
                ),
                source_object_type_name: source.object_type_name.clone(),
                target_object_type_name: target.object_type_name.clone(),
                source_property_name: field.property_name.clone(),
                target_primary_key_property: target.primary_key_property.clone(),
                cardinality: "many_to_one".to_string(),
                rationale: format!(
                    "Field '{}' in '{}' resembles a foreign key to '{}'.",
                    field.source_name, source.display_name, target.display_name
                ),
            });
        }
    }

    links
}

fn target_aliases(entity: &HyperAutoErpEntityPlan) -> Vec<String> {
    let mut aliases = vec![
        slugify(&entity.display_name),
        slugify(&entity.object_display_name),
        slugify(&entity.selector),
        slugify(&singularize(&entity.display_name)),
        slugify(&singularize(&entity.selector)),
        object_type_entity_suffix(&entity.object_type_name),
    ];

    if let Some(primary_key_property) = entity.primary_key_property.as_deref() {
        let key_base = reference_base(primary_key_property);
        if !key_base.is_empty() {
            aliases.push(key_base);
        }
    }

    aliases.sort();
    aliases.dedup();
    aliases
}

fn resolve_enterprise_system(connection: &Connection) -> Option<String> {
    if let Some(explicit) = connection
        .config
        .get("enterprise_system")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return Some(slugify(explicit));
    }

    match connection.connector_type.as_str() {
        "sap" => Some("sap".to_string()),
        "salesforce" => Some("salesforce".to_string()),
        _ => None,
    }
}

fn object_type_entity_suffix(object_type_name: &str) -> String {
    object_type_name
        .split_once('_')
        .map(|(_, suffix)| suffix.to_string())
        .unwrap_or_else(|| object_type_name.to_string())
}

fn object_stem(selector: &str, display_name: &str) -> String {
    slugify(&singularize(if display_name.trim().is_empty() {
        selector
    } else {
        display_name
    }))
}

fn singularize(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.ends_with("ies") && trimmed.len() > 3 {
        format!("{}y", &trimmed[..trimmed.len() - 3])
    } else if trimmed.ends_with("ses") && trimmed.len() > 3 {
        trimmed[..trimmed.len() - 2].to_string()
    } else if trimmed.ends_with('s') && trimmed.len() > 1 {
        trimmed[..trimmed.len() - 1].to_string()
    } else {
        trimmed.to_string()
    }
}

fn display_label(raw: &str) -> String {
    let normalized = raw
        .replace(['_', '-'], " ")
        .split_whitespace()
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => {
                    let rest = chars.as_str().to_ascii_lowercase();
                    format!("{}{}", first.to_ascii_uppercase(), rest)
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ");

    if normalized.is_empty() {
        "ERP Entity".to_string()
    } else {
        normalized
    }
}

fn slugify(raw: &str) -> String {
    let mut slug = String::new();
    let mut previous_was_separator = true;

    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() {
            if ch.is_ascii_uppercase() && !previous_was_separator && !slug.ends_with('_') {
                slug.push('_');
            }
            slug.push(ch.to_ascii_lowercase());
            previous_was_separator = false;
        } else if !previous_was_separator {
            slug.push('_');
            previous_was_separator = true;
        }
    }

    slug.trim_matches('_').replace("__", "_")
}

fn looks_reference_field(raw: &str) -> bool {
    let normalized = slugify(raw);
    normalized.ends_with("_id")
        || normalized.ends_with("_code")
        || normalized.ends_with("_number")
        || normalized.ends_with("_key")
}

fn reference_base(raw: &str) -> String {
    let normalized = slugify(raw);
    for suffix in ["_id", "_code", "_number", "_key"] {
        if let Some(base) = normalized.strip_suffix(suffix) {
            return base.to_string();
        }
    }
    String::new()
}

fn quote_identifier(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

fn contains_any(haystack: &str, patterns: &[&str]) -> bool {
    patterns.iter().any(|pattern| haystack.contains(pattern))
}

fn looks_like_date(value: &str) -> bool {
    value.len() >= 10
        && value.chars().nth(4) == Some('-')
        && value.chars().nth(7) == Some('-')
        && value
            .chars()
            .take(10)
            .all(|ch| ch.is_ascii_digit() || ch == '-')
}

fn looks_like_timestamp(value: &str) -> bool {
    value.contains('T')
        || value.contains("UTC")
        || value.contains("GMT")
        || value.contains('+')
        || value.ends_with('Z')
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::models::{connection::Connection, registration::DiscoveredSource};

    use super::{infer_links, plan_entity, resolve_enterprise_system};

    fn sample_connection() -> Connection {
        Connection {
            id: uuid::Uuid::nil(),
            name: "ERP Core".to_string(),
            connector_type: "sap".to_string(),
            config: json!({}),
            status: "connected".to_string(),
            owner_id: uuid::Uuid::nil(),
            last_sync_at: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn plans_sales_order_entity_with_primary_key() {
        let entity = plan_entity(
            &sample_connection(),
            &DiscoveredSource {
                selector: "SalesOrders".to_string(),
                display_name: "Sales Orders".to_string(),
                source_kind: "sap_entity_set".to_string(),
                supports_sync: true,
                supports_zero_copy: true,
                source_signature: None,
                metadata: json!({}),
            },
            &[json!({
                "SalesOrderId": "SO-100",
                "CustomerId": "C-200",
                "OrderDate": "2026-04-25",
                "GrossAmount": 1450.42
            })],
        );

        assert_eq!(entity.module, "sales");
        assert_eq!(
            entity.primary_key_property.as_deref(),
            Some("sales_order_id")
        );
        assert!(
            entity
                .fields
                .iter()
                .any(|field| field.property_name == "customer_id"
                    && field.semantic_role == "reference_candidate")
        );
        assert!(
            entity
                .normalization_sql
                .contains("\"SalesOrderId\" AS \"sales_order_id\"")
        );
    }

    #[test]
    fn infers_links_between_generated_entities() {
        let connection = sample_connection();
        let sales_order = plan_entity(
            &connection,
            &DiscoveredSource {
                selector: "SalesOrders".to_string(),
                display_name: "Sales Orders".to_string(),
                source_kind: "sap_entity_set".to_string(),
                supports_sync: true,
                supports_zero_copy: true,
                source_signature: None,
                metadata: json!({}),
            },
            &[json!({
                "SalesOrderId": "SO-100",
                "CustomerId": "C-200"
            })],
        );
        let customer = plan_entity(
            &connection,
            &DiscoveredSource {
                selector: "Customers".to_string(),
                display_name: "Customers".to_string(),
                source_kind: "sap_entity_set".to_string(),
                supports_sync: true,
                supports_zero_copy: true,
                source_signature: None,
                metadata: json!({}),
            },
            &[json!({
                "CustomerId": "C-200",
                "CustomerName": "Acme"
            })],
        );

        let links = infer_links(&[sales_order, customer]);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].source_property_name, "customer_id");
        assert_eq!(links[0].target_object_type_name, "sap_customer");
    }

    #[test]
    fn resolves_salesforce_and_explicit_enterprise_system() {
        let mut salesforce = sample_connection();
        salesforce.connector_type = "salesforce".to_string();
        assert_eq!(
            resolve_enterprise_system(&salesforce).as_deref(),
            Some("salesforce")
        );

        let mut generic = sample_connection();
        generic.connector_type = "postgres".to_string();
        generic.config = json!({ "enterprise_system": "workday" });
        assert_eq!(
            resolve_enterprise_system(&generic).as_deref(),
            Some("workday")
        );
    }
}
