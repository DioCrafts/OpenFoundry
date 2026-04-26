use std::collections::{HashMap, HashSet};

use auth_middleware::claims::Claims;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::{
    AppState,
    domain::access::ensure_object_access,
    handlers::objects::ObjectInstance,
    models::{
        action_type::ActionTypeRow,
        interface::{ObjectTypeInterfaceBinding, OntologyInterface},
        link_type::LinkType,
        object_type::ObjectType,
        shared_property::SharedPropertyType,
    },
};

#[derive(Debug, Clone)]
pub struct SearchDocument {
    pub kind: String,
    pub id: Uuid,
    pub object_type_id: Option<Uuid>,
    pub title: String,
    pub subtitle: Option<String>,
    pub snippet: String,
    pub body: String,
    pub route: String,
    pub metadata: Value,
}

fn normalize_kind_filter(kind: Option<&str>) -> Option<&str> {
    kind.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    })
}

fn kind_matches(kind_filter: Option<&str>, candidate_kind: &str) -> bool {
    normalize_kind_filter(kind_filter)
        .map(|kind| kind == candidate_kind)
        .unwrap_or(true)
}

fn compact_json(value: &Value) -> String {
    serde_json::to_string(value).unwrap_or_default()
}

fn summarize_object_properties(value: &Value) -> String {
    let rendered = compact_json(value);
    if rendered.len() > 220 {
        format!("{}...", &rendered[..220])
    } else {
        rendered
    }
}

fn object_title(object_type: &ObjectType, object: &ObjectInstance) -> String {
    let primary_key = object_type
        .primary_key_property
        .as_deref()
        .and_then(|property_name| object.properties.get(property_name))
        .map(|value| match value {
            Value::String(value) => value.clone(),
            _ => compact_json(value),
        });

    match primary_key {
        Some(primary_key) if !primary_key.is_empty() => {
            format!("{} · {}", object_type.display_name, primary_key)
        }
        _ => format!("{} · {}", object_type.display_name, object.id),
    }
}

pub async fn build_search_documents(
    state: &AppState,
    claims: &Claims,
    object_type_filter: Option<Uuid>,
    kind_filter: Option<&str>,
) -> Result<Vec<SearchDocument>, sqlx::Error> {
    let object_types =
        sqlx::query_as::<_, ObjectType>("SELECT * FROM object_types ORDER BY created_at DESC")
            .fetch_all(&state.db)
            .await?;
    let object_type_map = object_types
        .iter()
        .cloned()
        .map(|object_type| (object_type.id, object_type))
        .collect::<HashMap<_, _>>();

    let mut documents = Vec::new();

    if kind_matches(kind_filter, "object_type") {
        for object_type in &object_types {
            if object_type_filter.is_some_and(|filter| filter != object_type.id) {
                continue;
            }

            documents.push(SearchDocument {
                kind: "object_type".to_string(),
                id: object_type.id,
                object_type_id: Some(object_type.id),
                title: object_type.display_name.clone(),
                subtitle: Some(object_type.name.clone()),
                snippet: object_type.description.clone(),
                body: format!(
                    "{} {} {} {} {}",
                    object_type.name,
                    object_type.display_name,
                    object_type.description,
                    object_type.icon.clone().unwrap_or_default(),
                    object_type.color.clone().unwrap_or_default()
                ),
                route: format!("/ontology/{}", object_type.id),
                metadata: json!({
                    "name": object_type.name,
                    "primary_key_property": object_type.primary_key_property,
                    "icon": object_type.icon,
                    "color": object_type.color,
                }),
            });
        }
    }

    if kind_matches(kind_filter, "interface") {
        let interface_rows = if let Some(object_type_id) = object_type_filter {
            sqlx::query_as::<_, OntologyInterface>(
                r#"SELECT i.*
                   FROM ontology_interfaces i
                   INNER JOIN object_type_interfaces oti ON oti.interface_id = i.id
                   WHERE oti.object_type_id = $1
                   ORDER BY i.created_at DESC"#,
            )
            .bind(object_type_id)
            .fetch_all(&state.db)
            .await?
        } else {
            sqlx::query_as::<_, OntologyInterface>(
                "SELECT * FROM ontology_interfaces ORDER BY created_at DESC",
            )
            .fetch_all(&state.db)
            .await?
        };

        for interface_row in interface_rows {
            documents.push(SearchDocument {
                kind: "interface".to_string(),
                id: interface_row.id,
                object_type_id: object_type_filter,
                title: interface_row.display_name.clone(),
                subtitle: Some(interface_row.name.clone()),
                snippet: interface_row.description.clone(),
                body: format!(
                    "{} {} {}",
                    interface_row.name, interface_row.display_name, interface_row.description
                ),
                route: "/ontology/graph".to_string(),
                metadata: json!({
                    "name": interface_row.name,
                }),
            });
        }
    }

    if kind_matches(kind_filter, "shared_property_type") {
        let shared_property_types = if let Some(object_type_id) = object_type_filter {
            sqlx::query_as::<_, SharedPropertyType>(
                r#"SELECT spt.id, spt.name, spt.display_name, spt.description, spt.property_type,
                          spt.required, spt.unique_constraint, spt.time_dependent, spt.default_value,
                          spt.validation_rules, spt.owner_id, spt.created_at, spt.updated_at
                   FROM shared_property_types spt
                   INNER JOIN object_type_shared_property_types otsp
                        ON otsp.shared_property_type_id = spt.id
                   WHERE otsp.object_type_id = $1
                   ORDER BY otsp.created_at ASC, spt.created_at DESC"#,
            )
            .bind(object_type_id)
            .fetch_all(&state.db)
            .await?
        } else {
            sqlx::query_as::<_, SharedPropertyType>(
                r#"SELECT id, name, display_name, description, property_type, required,
                          unique_constraint, time_dependent, default_value, validation_rules,
                          owner_id, created_at, updated_at
                   FROM shared_property_types
                   ORDER BY created_at DESC"#,
            )
            .fetch_all(&state.db)
            .await?
        };

        for shared_property_type in shared_property_types {
            documents.push(SearchDocument {
                kind: "shared_property_type".to_string(),
                id: shared_property_type.id,
                object_type_id: object_type_filter,
                title: shared_property_type.display_name.clone(),
                subtitle: Some(shared_property_type.name.clone()),
                snippet: if shared_property_type.description.is_empty() {
                    format!("Reusable {} property", shared_property_type.property_type)
                } else {
                    shared_property_type.description.clone()
                },
                body: format!(
                    "{} {} {} {} {} {}",
                    shared_property_type.name,
                    shared_property_type.display_name,
                    shared_property_type.description,
                    shared_property_type.property_type,
                    if shared_property_type.required {
                        "required"
                    } else {
                        "optional"
                    },
                    if shared_property_type.time_dependent {
                        "time-dependent"
                    } else {
                        ""
                    }
                ),
                route: object_type_filter
                    .map(|object_type_id| format!("/ontology/{object_type_id}"))
                    .unwrap_or_else(|| "/ontology/graph".to_string()),
                metadata: json!({
                    "name": shared_property_type.name,
                    "property_type": shared_property_type.property_type,
                    "required": shared_property_type.required,
                    "unique_constraint": shared_property_type.unique_constraint,
                    "time_dependent": shared_property_type.time_dependent,
                }),
            });
        }
    }

    if kind_matches(kind_filter, "link_type") {
        let link_types = if let Some(object_type_id) = object_type_filter {
            sqlx::query_as::<_, LinkType>(
                r#"SELECT * FROM link_types
                   WHERE source_type_id = $1 OR target_type_id = $1
                   ORDER BY created_at DESC"#,
            )
            .bind(object_type_id)
            .fetch_all(&state.db)
            .await?
        } else {
            sqlx::query_as::<_, LinkType>("SELECT * FROM link_types ORDER BY created_at DESC")
                .fetch_all(&state.db)
                .await?
        };

        for link_type in link_types {
            let source = object_type_map.get(&link_type.source_type_id);
            let target = object_type_map.get(&link_type.target_type_id);
            documents.push(SearchDocument {
                kind: "link_type".to_string(),
                id: link_type.id,
                object_type_id: Some(link_type.source_type_id),
                title: link_type.display_name.clone(),
                subtitle: Some(link_type.name.clone()),
                snippet: format!(
                    "{} -> {} ({})",
                    source
                        .map(|value| value.display_name.as_str())
                        .unwrap_or("unknown"),
                    target
                        .map(|value| value.display_name.as_str())
                        .unwrap_or("unknown"),
                    link_type.cardinality
                ),
                body: format!(
                    "{} {} {} {} {} {}",
                    link_type.name,
                    link_type.display_name,
                    link_type.description,
                    source.map(|value| value.name.as_str()).unwrap_or(""),
                    target.map(|value| value.name.as_str()).unwrap_or(""),
                    link_type.cardinality
                ),
                route: "/ontology/graph".to_string(),
                metadata: json!({
                    "source_type_id": link_type.source_type_id,
                    "target_type_id": link_type.target_type_id,
                    "cardinality": link_type.cardinality,
                }),
            });
        }
    }

    if kind_matches(kind_filter, "action_type") {
        let action_rows = if let Some(object_type_id) = object_type_filter {
            sqlx::query_as::<_, ActionTypeRow>(
                r#"SELECT id, name, display_name, description, object_type_id, operation_kind, input_schema,
                          form_schema, config, confirmation_required, permission_key, authorization_policy,
                          owner_id, created_at, updated_at
                   FROM action_types
                   WHERE object_type_id = $1
                   ORDER BY created_at DESC"#,
            )
            .bind(object_type_id)
            .fetch_all(&state.db)
            .await?
        } else {
            sqlx::query_as::<_, ActionTypeRow>(
                r#"SELECT id, name, display_name, description, object_type_id, operation_kind, input_schema,
                          form_schema, config, confirmation_required, permission_key, authorization_policy,
                          owner_id, created_at, updated_at
                   FROM action_types
                   ORDER BY created_at DESC"#,
            )
            .fetch_all(&state.db)
            .await?
        };

        for action in action_rows {
            let permission_key = action.permission_key.clone().unwrap_or_default();
            documents.push(SearchDocument {
                kind: "action_type".to_string(),
                id: action.id,
                object_type_id: Some(action.object_type_id),
                title: action.display_name.clone(),
                subtitle: Some(action.name.clone()),
                snippet: action.description.clone(),
                body: format!(
                    "{} {} {} {} {}",
                    action.name,
                    action.display_name,
                    action.description,
                    action.operation_kind,
                    permission_key
                ),
                route: format!("/ontology/{}", action.object_type_id),
                metadata: json!({
                    "operation_kind": action.operation_kind,
                    "confirmation_required": action.confirmation_required,
                    "permission_key": action.permission_key,
                    "authorization_policy": action.authorization_policy,
                }),
            });
        }
    }

    if kind_matches(kind_filter, "object_instance") {
        let objects = if let Some(object_type_id) = object_type_filter {
            sqlx::query_as::<_, ObjectInstance>(
                r#"SELECT id, object_type_id, properties, created_by, organization_id, marking, created_at, updated_at
                   FROM object_instances
                   WHERE object_type_id = $1
                   ORDER BY created_at DESC"#,
            )
            .bind(object_type_id)
            .fetch_all(&state.db)
            .await?
        } else {
            sqlx::query_as::<_, ObjectInstance>(
                r#"SELECT id, object_type_id, properties, created_by, organization_id, marking, created_at, updated_at
                   FROM object_instances
                   ORDER BY created_at DESC"#,
            )
            .fetch_all(&state.db)
            .await?
        };

        for object in objects {
            if ensure_object_access(claims, &object).is_err() {
                continue;
            }

            let Some(object_type) = object_type_map.get(&object.object_type_id) else {
                continue;
            };

            let mut property_names = HashSet::new();
            let property_tokens = object
                .properties
                .as_object()
                .map(|properties| {
                    properties
                        .iter()
                        .map(|(name, value)| {
                            property_names.insert(name.clone());
                            match value {
                                Value::String(value) => format!("{name}: {value}"),
                                _ => format!("{name}: {}", compact_json(value)),
                            }
                        })
                        .collect::<Vec<_>>()
                        .join(" ")
                })
                .unwrap_or_default();

            documents.push(SearchDocument {
                kind: "object_instance".to_string(),
                id: object.id,
                object_type_id: Some(object.object_type_id),
                title: object_title(object_type, &object),
                subtitle: Some(object_type.name.clone()),
                snippet: summarize_object_properties(&object.properties),
                body: format!(
                    "{} {} {} {} {}",
                    object_type.name,
                    object_type.display_name,
                    property_tokens,
                    object.marking,
                    object.id
                ),
                route: format!("/ontology/{}#object-{}", object.object_type_id, object.id),
                metadata: json!({
                    "marking": object.marking,
                    "organization_id": object.organization_id,
                    "properties": object.properties,
                    "property_names": property_names,
                }),
            });
        }
    }

    if kind_matches(kind_filter, "interface_binding") {
        let bindings = if let Some(object_type_id) = object_type_filter {
            sqlx::query_as::<_, ObjectTypeInterfaceBinding>(
                r#"SELECT object_type_id, interface_id, created_at
                   FROM object_type_interfaces
                   WHERE object_type_id = $1"#,
            )
            .bind(object_type_id)
            .fetch_all(&state.db)
            .await?
        } else {
            sqlx::query_as::<_, ObjectTypeInterfaceBinding>(
                "SELECT object_type_id, interface_id, created_at FROM object_type_interfaces",
            )
            .fetch_all(&state.db)
            .await?
        };

        for binding in bindings {
            let Some(object_type) = object_type_map.get(&binding.object_type_id) else {
                continue;
            };

            documents.push(SearchDocument {
                kind: "interface_binding".to_string(),
                id: binding.interface_id,
                object_type_id: Some(binding.object_type_id),
                title: format!("{} interface binding", object_type.display_name),
                subtitle: Some(binding.interface_id.to_string()),
                snippet: "Interface attached to object type".to_string(),
                body: format!(
                    "{} {} {}",
                    object_type.name, object_type.display_name, binding.interface_id
                ),
                route: format!("/ontology/{}", binding.object_type_id),
                metadata: json!({
                    "interface_id": binding.interface_id,
                }),
            });
        }
    }

    Ok(documents)
}
