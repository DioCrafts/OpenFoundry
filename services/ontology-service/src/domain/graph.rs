use std::collections::{HashMap, HashSet, VecDeque};

use auth_middleware::claims::Claims;
use serde_json::json;
use uuid::Uuid;

use crate::{
    AppState,
    domain::access::ensure_object_access,
    handlers::{
        links::LinkInstance,
        objects::{ObjectInstance, load_object_instance},
    },
    models::{
        graph::{GraphEdge, GraphNode, GraphQuery, GraphResponse},
        interface::{ObjectTypeInterfaceBinding, OntologyInterface},
        link_type::LinkType,
        object_type::ObjectType,
    },
};

fn type_node_id(type_id: Uuid) -> String {
    format!("type:{type_id}")
}

fn interface_node_id(interface_id: Uuid) -> String {
    format!("interface:{interface_id}")
}

fn object_node_id(object_id: Uuid) -> String {
    format!("object:{object_id}")
}

fn object_route(object_type_id: Uuid, object_id: Uuid) -> String {
    format!("/ontology/{}#object-{}", object_type_id, object_id)
}

fn object_label(object_type: &ObjectType, object: &ObjectInstance) -> String {
    let primary_key = object_type
        .primary_key_property
        .as_deref()
        .and_then(|property_name| object.properties.get(property_name))
        .map(|value| match value {
            serde_json::Value::String(value) => value.clone(),
            _ => serde_json::to_string(value).unwrap_or_else(|_| object.id.to_string()),
        });

    match primary_key {
        Some(primary_key) if !primary_key.is_empty() => primary_key,
        _ => object.id.to_string(),
    }
}

pub async fn build_graph(
    state: &AppState,
    claims: &Claims,
    query: &GraphQuery,
) -> Result<GraphResponse, String> {
    if let Some(root_object_id) = query.root_object_id {
        build_object_graph(state, claims, root_object_id, query.depth, query.limit).await
    } else {
        build_schema_graph(state, query.root_type_id).await
    }
}

async fn build_schema_graph(
    state: &AppState,
    root_type_id: Option<Uuid>,
) -> Result<GraphResponse, String> {
    let object_types = sqlx::query_as::<_, ObjectType>("SELECT * FROM object_types ORDER BY created_at DESC")
        .fetch_all(&state.db)
        .await
        .map_err(|error| format!("failed to load object types: {error}"))?;
    let interfaces = sqlx::query_as::<_, OntologyInterface>(
        "SELECT * FROM ontology_interfaces ORDER BY created_at DESC",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|error| format!("failed to load interfaces: {error}"))?;
    let bindings = sqlx::query_as::<_, ObjectTypeInterfaceBinding>(
        "SELECT object_type_id, interface_id, created_at FROM object_type_interfaces",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|error| format!("failed to load interface bindings: {error}"))?;
    let link_types = sqlx::query_as::<_, LinkType>("SELECT * FROM link_types ORDER BY created_at DESC")
        .fetch_all(&state.db)
        .await
        .map_err(|error| format!("failed to load link types: {error}"))?;

    let mut allowed_types = object_types
        .iter()
        .map(|object_type| object_type.id)
        .collect::<HashSet<_>>();
    if let Some(root_type_id) = root_type_id {
        let mut focused = HashSet::from([root_type_id]);
        for link_type in &link_types {
            if link_type.source_type_id == root_type_id || link_type.target_type_id == root_type_id {
                focused.insert(link_type.source_type_id);
                focused.insert(link_type.target_type_id);
            }
        }
        allowed_types = focused;
    }

    let allowed_interfaces = bindings
        .iter()
        .filter(|binding| allowed_types.contains(&binding.object_type_id))
        .map(|binding| binding.interface_id)
        .collect::<HashSet<_>>();

    let nodes = object_types
        .iter()
        .filter(|object_type| allowed_types.contains(&object_type.id))
        .map(|object_type| GraphNode {
            id: type_node_id(object_type.id),
            kind: "object_type".to_string(),
            label: object_type.display_name.clone(),
            secondary_label: Some(object_type.name.clone()),
            color: object_type.color.clone(),
            route: Some(format!("/ontology/{}", object_type.id)),
            metadata: json!({
                "icon": object_type.icon,
                "description": object_type.description,
                "primary_key_property": object_type.primary_key_property,
            }),
        })
        .chain(
            interfaces
                .iter()
                .filter(|interface_row| allowed_interfaces.contains(&interface_row.id))
                .map(|interface_row| GraphNode {
                    id: interface_node_id(interface_row.id),
                    kind: "interface".to_string(),
                    label: interface_row.display_name.clone(),
                    secondary_label: Some(interface_row.name.clone()),
                    color: Some("#0f766e".to_string()),
                    route: Some("/ontology/graph".to_string()),
                    metadata: json!({
                        "description": interface_row.description,
                    }),
                }),
        )
        .collect::<Vec<_>>();

    let mut edges = Vec::new();
    for link_type in &link_types {
        if !allowed_types.contains(&link_type.source_type_id)
            || !allowed_types.contains(&link_type.target_type_id)
        {
            continue;
        }
        edges.push(GraphEdge {
            id: format!("link_type:{}", link_type.id),
            kind: "link_type".to_string(),
            source: type_node_id(link_type.source_type_id),
            target: type_node_id(link_type.target_type_id),
            label: link_type.display_name.clone(),
            metadata: json!({
                "name": link_type.name,
                "cardinality": link_type.cardinality,
                "description": link_type.description,
            }),
        });
    }

    for binding in &bindings {
        if !allowed_types.contains(&binding.object_type_id)
            || !allowed_interfaces.contains(&binding.interface_id)
        {
            continue;
        }
        edges.push(GraphEdge {
            id: format!("interface_binding:{}:{}", binding.object_type_id, binding.interface_id),
            kind: "interface_binding".to_string(),
            source: type_node_id(binding.object_type_id),
            target: interface_node_id(binding.interface_id),
            label: "implements".to_string(),
            metadata: json!({}),
        });
    }

    Ok(GraphResponse {
        mode: "schema".to_string(),
        root_object_id: None,
        root_type_id,
        depth: 1,
        total_nodes: nodes.len(),
        total_edges: edges.len(),
        nodes,
        edges,
    })
}

async fn build_object_graph(
    state: &AppState,
    claims: &Claims,
    root_object_id: Uuid,
    depth: Option<usize>,
    limit: Option<usize>,
) -> Result<GraphResponse, String> {
    let depth = depth.unwrap_or(2).clamp(1, 4);
    let limit = limit.unwrap_or(40).clamp(1, 120);

    let root_object = load_object_instance(&state.db, root_object_id)
        .await
        .map_err(|error| format!("failed to load root object: {error}"))?
        .ok_or_else(|| "root object was not found".to_string())?;
    ensure_object_access(claims, &root_object)?;

    let object_types = sqlx::query_as::<_, ObjectType>("SELECT * FROM object_types")
        .fetch_all(&state.db)
        .await
        .map_err(|error| format!("failed to load object types: {error}"))?;
    let object_type_map = object_types
        .into_iter()
        .map(|object_type| (object_type.id, object_type))
        .collect::<HashMap<_, _>>();
    let link_type_map = sqlx::query_as::<_, LinkType>("SELECT * FROM link_types")
        .fetch_all(&state.db)
        .await
        .map_err(|error| format!("failed to load link types: {error}"))?
        .into_iter()
        .map(|link_type| (link_type.id, link_type))
        .collect::<HashMap<_, _>>();

    let mut visited_objects = HashSet::from([root_object_id]);
    let mut seen_edges = HashSet::new();
    let mut queue = VecDeque::from([(root_object_id, 0usize)]);
    let mut link_instances = Vec::new();

    while let Some((object_id, level)) = queue.pop_front() {
        if level >= depth {
            continue;
        }

        let rows = sqlx::query_as::<_, LinkInstance>(
            r#"SELECT id, link_type_id, source_object_id, target_object_id, properties, created_by, created_at
               FROM link_instances
               WHERE source_object_id = $1 OR target_object_id = $1
               ORDER BY created_at ASC"#,
        )
        .bind(object_id)
        .fetch_all(&state.db)
        .await
        .map_err(|error| format!("failed to load object graph edges: {error}"))?;

        for link_instance in rows {
            if !seen_edges.insert(link_instance.id) {
                continue;
            }

            let neighbor_id = if link_instance.source_object_id == object_id {
                link_instance.target_object_id
            } else {
                link_instance.source_object_id
            };

            link_instances.push(link_instance);

            if visited_objects.len() >= limit {
                continue;
            }
            if visited_objects.insert(neighbor_id) {
                queue.push_back((neighbor_id, level + 1));
            }
        }
    }

    let mut objects = Vec::new();
    let mut allowed_object_ids = HashSet::new();
    for object_id in visited_objects {
        let Some(object) = load_object_instance(&state.db, object_id)
            .await
            .map_err(|error| format!("failed to hydrate object graph node: {error}"))?
        else {
            continue;
        };
        if ensure_object_access(claims, &object).is_err() {
            continue;
        }
        allowed_object_ids.insert(object.id);
        objects.push(object);
    }

    let nodes = objects
        .iter()
        .filter_map(|object| {
            object_type_map.get(&object.object_type_id).map(|object_type| GraphNode {
                id: object_node_id(object.id),
                kind: "object_instance".to_string(),
                label: object_label(object_type, object),
                secondary_label: Some(object_type.display_name.clone()),
                color: object_type.color.clone(),
                route: Some(object_route(object.object_type_id, object.id)),
                metadata: json!({
                    "object_type_id": object.object_type_id,
                    "marking": object.marking,
                    "properties": object.properties,
                }),
            })
        })
        .collect::<Vec<_>>();

    let edges = link_instances
        .into_iter()
        .filter(|link_instance| {
            allowed_object_ids.contains(&link_instance.source_object_id)
                && allowed_object_ids.contains(&link_instance.target_object_id)
        })
        .filter_map(|link_instance| {
            link_type_map.get(&link_instance.link_type_id).map(|link_type| GraphEdge {
                id: format!("link_instance:{}", link_instance.id),
                kind: "link_instance".to_string(),
                source: object_node_id(link_instance.source_object_id),
                target: object_node_id(link_instance.target_object_id),
                label: link_type.display_name.clone(),
                metadata: json!({
                    "link_type_id": link_type.id,
                    "cardinality": link_type.cardinality,
                    "properties": link_instance.properties,
                }),
            })
        })
        .collect::<Vec<_>>();

    let root_type_id = objects
        .iter()
        .find(|object| object.id == root_object_id)
        .map(|object| object.object_type_id);

    Ok(GraphResponse {
        mode: "object".to_string(),
        root_object_id: Some(root_object_id),
        root_type_id,
        depth,
        total_nodes: nodes.len(),
        total_edges: edges.len(),
        nodes,
        edges,
    })
}
