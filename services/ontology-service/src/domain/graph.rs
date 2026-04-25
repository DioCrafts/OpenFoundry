use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};

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
        graph::{GraphEdge, GraphNode, GraphQuery, GraphResponse, GraphSummary},
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

fn increment_count(map: &mut BTreeMap<String, usize>, key: impl Into<String>) {
    *map.entry(key.into()).or_default() += 1;
}

fn classify_scope(
    mode: &str,
    root_neighbor_count: usize,
    sensitive_objects: usize,
    boundary_crossings: usize,
) -> String {
    if mode == "schema" {
        "schema".to_string()
    } else if sensitive_objects > 0 {
        "sensitive_connected".to_string()
    } else if boundary_crossings > 0 {
        "cross_boundary".to_string()
    } else if root_neighbor_count > 0 {
        "connected".to_string()
    } else {
        "local".to_string()
    }
}

fn summarize_graph(mode: &str, nodes: &[GraphNode], edges: &[GraphEdge]) -> GraphSummary {
    let mut node_kinds = BTreeMap::new();
    let mut edge_kinds = BTreeMap::new();
    let mut object_types = BTreeMap::new();
    let mut markings = BTreeMap::new();
    let mut sensitive_markings = BTreeSet::new();
    let mut max_hops_reached = 0usize;
    let mut root_neighbor_count = 0usize;
    let mut sensitive_objects = 0usize;

    let node_metadata = nodes
        .iter()
        .map(|node| (node.id.as_str(), &node.metadata))
        .collect::<HashMap<_, _>>();

    for node in nodes {
        increment_count(&mut node_kinds, node.kind.clone());

        match node.kind.as_str() {
            "object_type" => increment_count(&mut object_types, node.label.clone()),
            "object_instance" => {
                if let Some(type_label) = node.secondary_label.as_deref() {
                    increment_count(&mut object_types, type_label.to_string());
                }
            }
            _ => {}
        }

        if let Some(marking) = node
            .metadata
            .get("marking")
            .and_then(|value| value.as_str())
        {
            increment_count(&mut markings, marking.to_string());
            if marking != "public" {
                sensitive_objects += 1;
                sensitive_markings.insert(marking.to_string());
            }
        }

        if let Some(distance) = node
            .metadata
            .get("distance_from_root")
            .and_then(|value| value.as_u64())
            .map(|value| value as usize)
        {
            max_hops_reached = max_hops_reached.max(distance);
            if distance == 1 {
                root_neighbor_count += 1;
            }
        }
    }

    let mut boundary_crossings = 0usize;
    for edge in edges {
        increment_count(&mut edge_kinds, edge.kind.clone());

        let source_org = node_metadata
            .get(edge.source.as_str())
            .and_then(|metadata| metadata.get("organization_id"))
            .and_then(|value| value.as_str());
        let target_org = node_metadata
            .get(edge.target.as_str())
            .and_then(|metadata| metadata.get("organization_id"))
            .and_then(|value| value.as_str());

        if source_org != target_org && (source_org.is_some() || target_org.is_some()) {
            boundary_crossings += 1;
        }
    }

    GraphSummary {
        scope: classify_scope(
            mode,
            root_neighbor_count,
            sensitive_objects,
            boundary_crossings,
        ),
        node_kinds,
        edge_kinds,
        object_types,
        markings,
        root_neighbor_count,
        max_hops_reached,
        boundary_crossings,
        sensitive_objects,
        sensitive_markings: sensitive_markings.into_iter().collect(),
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
    let object_types =
        sqlx::query_as::<_, ObjectType>("SELECT * FROM object_types ORDER BY created_at DESC")
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
    let link_types =
        sqlx::query_as::<_, LinkType>("SELECT * FROM link_types ORDER BY created_at DESC")
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
            if link_type.source_type_id == root_type_id || link_type.target_type_id == root_type_id
            {
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
            id: format!(
                "interface_binding:{}:{}",
                binding.object_type_id, binding.interface_id
            ),
            kind: "interface_binding".to_string(),
            source: type_node_id(binding.object_type_id),
            target: interface_node_id(binding.interface_id),
            label: "implements".to_string(),
            metadata: json!({}),
        });
    }

    let summary = summarize_graph("schema", &nodes, &edges);

    Ok(GraphResponse {
        mode: "schema".to_string(),
        root_object_id: None,
        root_type_id,
        depth: 1,
        total_nodes: nodes.len(),
        total_edges: edges.len(),
        summary,
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
    let mut distance_from_root = HashMap::from([(root_object_id, 0usize)]);
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
                distance_from_root.insert(neighbor_id, level + 1);
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
    objects.sort_by_key(|object| {
        (
            distance_from_root
                .get(&object.id)
                .copied()
                .unwrap_or(depth.saturating_add(1)),
            object.id.to_string(),
        )
    });

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
                    "distance_from_root": distance_from_root.get(&object.id).copied().unwrap_or(depth),
                    "role": match distance_from_root.get(&object.id).copied().unwrap_or(depth) {
                        0 => "root",
                        1 => "neighbor",
                        _ => "extended",
                    },
                    "organization_id": object.organization_id,
                    "marking": object.marking,
                    "properties": object.properties,
                }),
            })
        })
        .collect::<Vec<_>>();

    let mut edges = link_instances
        .into_iter()
        .filter(|link_instance| {
            allowed_object_ids.contains(&link_instance.source_object_id)
                && allowed_object_ids.contains(&link_instance.target_object_id)
        })
        .filter_map(|link_instance| {
            link_type_map
                .get(&link_instance.link_type_id)
                .map(|link_type| GraphEdge {
                    id: format!("link_instance:{}", link_instance.id),
                    kind: "link_instance".to_string(),
                    source: object_node_id(link_instance.source_object_id),
                    target: object_node_id(link_instance.target_object_id),
                    label: link_type.display_name.clone(),
                    metadata: json!({
                        "link_type_id": link_type.id,
                        "cardinality": link_type.cardinality,
                        "crosses_organization_boundary": false,
                        "properties": link_instance.properties,
                    }),
                })
        })
        .collect::<Vec<_>>();
    edges.sort_by(|left, right| left.id.cmp(&right.id));

    let root_type_id = objects
        .iter()
        .find(|object| object.id == root_object_id)
        .map(|object| object.object_type_id);

    let summary = summarize_graph("object", &nodes, &edges);
    for edge in &mut edges {
        let source_org = nodes
            .iter()
            .find(|node| node.id == edge.source)
            .and_then(|node| node.metadata.get("organization_id"))
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        let target_org = nodes
            .iter()
            .find(|node| node.id == edge.target)
            .and_then(|node| node.metadata.get("organization_id"))
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        let crosses_boundary =
            source_org != target_org && (!source_org.is_null() || !target_org.is_null());
        if let Some(metadata) = edge.metadata.as_object_mut() {
            metadata.insert(
                "crosses_organization_boundary".to_string(),
                json!(crosses_boundary),
            );
        }
    }

    Ok(GraphResponse {
        mode: "object".to_string(),
        root_object_id: Some(root_object_id),
        root_type_id,
        depth,
        total_nodes: nodes.len(),
        total_edges: edges.len(),
        summary,
        nodes,
        edges,
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::summarize_graph;
    use crate::models::graph::{GraphEdge, GraphNode};

    #[test]
    fn object_graph_summary_captures_sensitive_cross_boundary_scope() {
        let nodes = vec![
            GraphNode {
                id: "object:root".to_string(),
                kind: "object_instance".to_string(),
                label: "Root".to_string(),
                secondary_label: Some("Case".to_string()),
                color: None,
                route: None,
                metadata: json!({
                    "distance_from_root": 0,
                    "organization_id": "org-a",
                    "marking": "public",
                }),
            },
            GraphNode {
                id: "object:neighbor".to_string(),
                kind: "object_instance".to_string(),
                label: "Neighbor".to_string(),
                secondary_label: Some("Customer".to_string()),
                color: None,
                route: None,
                metadata: json!({
                    "distance_from_root": 1,
                    "organization_id": "org-b",
                    "marking": "pii",
                }),
            },
        ];
        let edges = vec![GraphEdge {
            id: "link:1".to_string(),
            kind: "link_instance".to_string(),
            source: "object:root".to_string(),
            target: "object:neighbor".to_string(),
            label: "linked".to_string(),
            metadata: json!({}),
        }];

        let summary = summarize_graph("object", &nodes, &edges);

        assert_eq!(summary.scope, "sensitive_connected");
        assert_eq!(summary.root_neighbor_count, 1);
        assert_eq!(summary.max_hops_reached, 1);
        assert_eq!(summary.boundary_crossings, 1);
        assert_eq!(summary.sensitive_objects, 1);
        assert_eq!(summary.object_types.get("Case"), Some(&1));
        assert_eq!(summary.object_types.get("Customer"), Some(&1));
        assert_eq!(summary.markings.get("pii"), Some(&1));
    }

    #[test]
    fn schema_graph_summary_stays_in_schema_scope() {
        let nodes = vec![GraphNode {
            id: "type:1".to_string(),
            kind: "object_type".to_string(),
            label: "Case".to_string(),
            secondary_label: Some("case".to_string()),
            color: None,
            route: None,
            metadata: json!({}),
        }];
        let edges = vec![];

        let summary = summarize_graph("schema", &nodes, &edges);

        assert_eq!(summary.scope, "schema");
        assert_eq!(summary.object_types.get("Case"), Some(&1));
        assert_eq!(summary.root_neighbor_count, 0);
        assert_eq!(summary.sensitive_objects, 0);
    }
}
