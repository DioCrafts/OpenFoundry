use std::collections::{HashMap, HashSet, VecDeque};

use axum::{Json, extract::Path};
use chrono::{DateTime, Utc};
use sqlx::types::Json as SqlJson;
use uuid::Uuid;

use crate::{
    AppState,
    domain::{connectors, engine::processor},
    handlers::{ServiceResult, bad_request, db_error, not_found},
    models::{
        ListResponse, StreamingOverview,
        sink::{ConnectorCatalogEntry, LiveTailResponse},
        stream::{ConnectorBinding, StreamDefinition, StreamRow},
        topology::{
            BackpressurePolicy, CepDefinition, CreateTopologyRequest, JoinDefinition,
            ReplayTopologyRequest, ReplayTopologyResponse, TopologyDefinition, TopologyEdge,
            TopologyNode, TopologyRow, TopologyRun, TopologyRunRow, TopologyRuntimeSnapshot,
            UpdateTopologyRequest,
        },
        window::{WindowDefinition, WindowRow},
    },
};

async fn load_topology_row(db: &sqlx::PgPool, id: Uuid) -> Result<TopologyRow, sqlx::Error> {
    sqlx::query_as::<_, TopologyRow>(
        "SELECT id, name, description, status, nodes, edges, join_definition, cep_definition,
		        backpressure_policy, source_stream_ids, sink_bindings, state_backend,
		        created_at, updated_at
		 FROM streaming_topologies
		 WHERE id = $1",
    )
    .bind(id)
    .fetch_one(db)
    .await
}

async fn load_latest_run_row(
    db: &sqlx::PgPool,
    topology_id: Uuid,
) -> Result<Option<TopologyRunRow>, sqlx::Error> {
    sqlx::query_as::<_, TopologyRunRow>(
        "SELECT id, topology_id, status, metrics, aggregate_windows, live_tail, cep_matches,
		        state_snapshot, backpressure_snapshot, started_at, completed_at, created_at, updated_at
		 FROM streaming_topology_runs
		 WHERE topology_id = $1
		 ORDER BY created_at DESC
		 LIMIT 1",
    )
    .bind(topology_id)
    .fetch_optional(db)
    .await
}

async fn load_all_streams(
    db: &sqlx::PgPool,
) -> Result<Vec<crate::models::stream::StreamDefinition>, sqlx::Error> {
    let rows = sqlx::query_as::<_, StreamRow>(
		"SELECT id, name, description, status, schema, source_binding, retention_hours, created_at, updated_at
		 FROM streaming_streams
		 ORDER BY created_at ASC",
	)
	.fetch_all(db)
	.await?;

    Ok(rows.into_iter().map(Into::into).collect())
}

async fn load_all_windows(
    db: &sqlx::PgPool,
) -> Result<Vec<crate::models::window::WindowDefinition>, sqlx::Error> {
    let rows = sqlx::query_as::<_, WindowRow>(
        "SELECT id, name, description, status, window_type, duration_seconds, slide_seconds,
		        session_gap_seconds, allowed_lateness_seconds, aggregation_keys, measure_fields,
		        created_at, updated_at
		 FROM streaming_windows
		 ORDER BY created_at ASC",
    )
    .fetch_all(db)
    .await?;

    Ok(rows.into_iter().map(Into::into).collect())
}

async fn restore_topology_events(
    db: &sqlx::PgPool,
    stream_ids: &[Uuid],
    from_sequence_no: Option<i64>,
) -> Result<i64, sqlx::Error> {
    if stream_ids.is_empty() {
        return Ok(0);
    }

    let result = match from_sequence_no {
        Some(sequence_no) => {
            sqlx::query(
                "UPDATE streaming_events
                 SET processed_at = NULL,
                     archived_at = NULL,
                     archive_path = NULL
                 WHERE stream_id = ANY($1)
                   AND sequence_no >= $2",
            )
            .bind(stream_ids)
            .bind(sequence_no)
            .execute(db)
            .await?
        }
        None => {
            sqlx::query(
                "UPDATE streaming_events
                 SET processed_at = NULL,
                     archived_at = NULL,
                     archive_path = NULL
                 WHERE stream_id = ANY($1)",
            )
            .bind(stream_ids)
            .execute(db)
            .await?
        }
    };

    Ok(result.rows_affected() as i64)
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct StreamActivityRow {
    stream_id: Uuid,
    backlog: i64,
    recent_events: i64,
    oldest_event_time: Option<DateTime<Utc>>,
    newest_event_time: Option<DateTime<Utc>>,
}

fn validate_topology_configuration(
    name: &str,
    source_stream_ids: &[Uuid],
    nodes: &[TopologyNode],
    edges: &[TopologyEdge],
    join_definition: Option<&JoinDefinition>,
    cep_definition: Option<&CepDefinition>,
    backpressure_policy: &BackpressurePolicy,
    sink_bindings: &[ConnectorBinding],
    streams: &[StreamDefinition],
    windows: &[WindowDefinition],
) -> Result<(), String> {
    if name.trim().is_empty() {
        return Err("topology name is required".to_string());
    }
    if source_stream_ids.is_empty() {
        return Err("at least one source stream is required".to_string());
    }
    if nodes.is_empty() {
        return Err("topology must contain at least one node".to_string());
    }
    if backpressure_policy.max_in_flight <= 0 || backpressure_policy.queue_capacity <= 0 {
        return Err(
            "backpressure policy requires positive max_in_flight and queue_capacity".to_string(),
        );
    }
    if backpressure_policy.queue_capacity < backpressure_policy.max_in_flight {
        return Err("queue_capacity must be greater than or equal to max_in_flight".to_string());
    }

    let known_streams = streams
        .iter()
        .map(|stream| stream.id)
        .collect::<HashSet<_>>();
    let known_windows = windows
        .iter()
        .map(|window| window.id)
        .collect::<HashSet<_>>();
    let mut source_stream_set = HashSet::new();
    for stream_id in source_stream_ids {
        if !known_streams.contains(stream_id) {
            return Err(format!("source stream '{stream_id}' does not exist"));
        }
        if !source_stream_set.insert(*stream_id) {
            return Err(format!("source stream '{stream_id}' is duplicated"));
        }
    }

    let mut node_ids = HashSet::new();
    let mut source_nodes = 0usize;
    let mut sink_nodes = 0usize;
    let mut represented_source_streams = HashSet::new();
    for node in nodes {
        if node.id.trim().is_empty() {
            return Err("topology nodes require non-empty ids".to_string());
        }
        if !node_ids.insert(node.id.clone()) {
            return Err(format!("duplicate node id '{}'", node.id));
        }

        match node.node_type.as_str() {
            "source" => {
                source_nodes += 1;
                let stream_id = node
                    .stream_id
                    .ok_or_else(|| format!("source node '{}' must reference a stream", node.id))?;
                if !source_stream_set.contains(&stream_id) {
                    return Err(format!(
                        "source node '{}' references stream '{}' outside source_stream_ids",
                        node.id, stream_id
                    ));
                }
                represented_source_streams.insert(stream_id);
            }
            "window" => {
                let window_id = node
                    .window_id
                    .ok_or_else(|| format!("window node '{}' must reference a window", node.id))?;
                if !known_windows.contains(&window_id) {
                    return Err(format!(
                        "window node '{}' references unknown window '{}'",
                        node.id, window_id
                    ));
                }
            }
            "sink" => sink_nodes += 1,
            _ => {}
        }
    }

    if source_nodes == 0 {
        return Err("topology must contain at least one source node".to_string());
    }
    if !sink_bindings.is_empty() && sink_nodes == 0 {
        return Err("topology sink bindings require at least one sink node".to_string());
    }
    for stream_id in &source_stream_set {
        if !represented_source_streams.contains(stream_id) {
            return Err(format!(
                "source stream '{}' is configured but not represented by a source node",
                stream_id
            ));
        }
    }

    let mut indegree = nodes
        .iter()
        .map(|node| (node.id.clone(), 0usize))
        .collect::<HashMap<_, _>>();
    let mut adjacency = nodes
        .iter()
        .map(|node| (node.id.clone(), Vec::<String>::new()))
        .collect::<HashMap<_, _>>();
    for edge in edges {
        if edge.source_node_id == edge.target_node_id {
            return Err(format!(
                "edge '{}' -> '{}' creates an invalid self-cycle",
                edge.source_node_id, edge.target_node_id
            ));
        }
        if !node_ids.contains(&edge.source_node_id) {
            return Err(format!(
                "edge references unknown source node '{}'",
                edge.source_node_id
            ));
        }
        if !node_ids.contains(&edge.target_node_id) {
            return Err(format!(
                "edge references unknown target node '{}'",
                edge.target_node_id
            ));
        }
        adjacency
            .entry(edge.source_node_id.clone())
            .or_default()
            .push(edge.target_node_id.clone());
        *indegree.entry(edge.target_node_id.clone()).or_default() += 1;
    }

    let mut queue = indegree
        .iter()
        .filter(|(_, degree)| **degree == 0)
        .map(|(node_id, _)| node_id.clone())
        .collect::<VecDeque<_>>();
    let mut visited = 0usize;
    while let Some(node_id) = queue.pop_front() {
        visited += 1;
        if let Some(targets) = adjacency.get(&node_id) {
            for target in targets {
                if let Some(degree) = indegree.get_mut(target) {
                    *degree -= 1;
                    if *degree == 0 {
                        queue.push_back(target.clone());
                    }
                }
            }
        }
    }
    if visited != nodes.len() {
        return Err("topology graph contains a cycle".to_string());
    }

    if let Some(join_definition) = join_definition {
        if join_definition.left_stream_id == join_definition.right_stream_id {
            return Err("join_definition requires distinct left and right streams".to_string());
        }
        if !source_stream_set.contains(&join_definition.left_stream_id)
            || !source_stream_set.contains(&join_definition.right_stream_id)
        {
            return Err("join_definition streams must belong to source_stream_ids".to_string());
        }
        if join_definition.key_fields.is_empty() {
            return Err("join_definition requires at least one key field".to_string());
        }
        if join_definition.window_seconds <= 0 {
            return Err("join_definition.window_seconds must be positive".to_string());
        }
    }

    if let Some(cep_definition) = cep_definition {
        if cep_definition.pattern_name.trim().is_empty() {
            return Err("cep_definition.pattern_name is required".to_string());
        }
        if cep_definition.sequence.len() < 2 {
            return Err("cep_definition.sequence must contain at least two steps".to_string());
        }
        if cep_definition.within_seconds <= 0 {
            return Err("cep_definition.within_seconds must be positive".to_string());
        }
        if cep_definition.output_stream.trim().is_empty() {
            return Err("cep_definition.output_stream is required".to_string());
        }
    }

    Ok(())
}

fn connector_status_from_backlog(backlog: i32, capacity: i32) -> String {
    let effective_capacity = capacity.max(1);
    let ratio = backlog.max(0) as f32 / effective_capacity as f32;
    if ratio >= 0.8 {
        "throttling".to_string()
    } else if ratio >= 0.5 {
        "elevated".to_string()
    } else {
        "healthy".to_string()
    }
}

fn throughput_from_window(
    count: i64,
    oldest: Option<DateTime<Utc>>,
    newest: Option<DateTime<Utc>>,
) -> f32 {
    if count <= 0 {
        return 0.0;
    }
    let elapsed_secs = match (oldest, newest) {
        (Some(start), Some(end)) if end > start => (end - start)
            .to_std()
            .map(|duration| duration.as_secs_f32().max(1.0))
            .unwrap_or(1.0),
        _ => 300.0,
    };
    count as f32 / elapsed_secs
}

async fn load_stream_activity_stats(
    db: &sqlx::PgPool,
) -> Result<HashMap<Uuid, StreamActivityRow>, sqlx::Error> {
    let rows = sqlx::query_as::<_, StreamActivityRow>(
        r#"SELECT stream_id,
                  COUNT(*) FILTER (WHERE archived_at IS NULL AND processed_at IS NULL) AS backlog,
                  COUNT(*) FILTER (WHERE archived_at IS NULL AND event_time >= now() - interval '5 minutes') AS recent_events,
                  MIN(event_time) FILTER (WHERE archived_at IS NULL AND event_time >= now() - interval '5 minutes') AS oldest_event_time,
                  MAX(event_time) FILTER (WHERE archived_at IS NULL AND event_time >= now() - interval '5 minutes') AS newest_event_time
           FROM streaming_events
           GROUP BY stream_id"#,
    )
    .fetch_all(db)
    .await?;

    Ok(rows.into_iter().map(|row| (row.stream_id, row)).collect())
}

fn apply_topology_connector_runtime(
    topology: &TopologyDefinition,
    streams: &[StreamDefinition],
    mut entries: Vec<ConnectorCatalogEntry>,
    source_backlog: &HashMap<Uuid, i32>,
    source_throughput: &HashMap<Uuid, f32>,
    sink_backlog: i32,
    sink_throughput: f32,
    sink_status: &str,
) -> Vec<ConnectorCatalogEntry> {
    let source_streams = streams
        .iter()
        .filter(|stream| topology.source_stream_ids.contains(&stream.id))
        .collect::<Vec<_>>();
    for (entry, stream) in entries
        .iter_mut()
        .take(source_streams.len())
        .zip(source_streams.iter())
    {
        let backlog = source_backlog.get(&stream.id).copied().unwrap_or(0);
        entry.backlog = backlog;
        entry.throughput_per_second = source_throughput.get(&stream.id).copied().unwrap_or(0.0);
        entry.status =
            connector_status_from_backlog(backlog, topology.backpressure_policy.queue_capacity);
    }
    for entry in entries.iter_mut().skip(source_streams.len()) {
        entry.backlog = sink_backlog.max(0);
        entry.throughput_per_second = sink_throughput.max(0.0);
        entry.status = sink_status.to_string();
    }

    entries
}

pub async fn get_overview(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> ServiceResult<StreamingOverview> {
    let stream_rows = sqlx::query_as::<_, StreamRow>(
		"SELECT id, name, description, status, schema, source_binding, retention_hours, created_at, updated_at
		 FROM streaming_streams",
	)
	.fetch_all(&state.db)
	.await
	.map_err(|cause| db_error(&cause))?;

    let topology_rows = sqlx::query_as::<_, TopologyRow>(
        "SELECT id, name, description, status, nodes, edges, join_definition, cep_definition,
		        backpressure_policy, source_stream_ids, sink_bindings, state_backend,
		        created_at, updated_at
		 FROM streaming_topologies",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?;

    let window_count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM streaming_windows")
        .fetch_one(&state.db)
        .await
        .map_err(|cause| db_error(&cause))?;

    let connector_count = stream_rows.len()
        + topology_rows
            .iter()
            .map(|row| row.sink_bindings.0.len())
            .sum::<usize>();

    let all_streams: Vec<crate::models::stream::StreamDefinition> =
        stream_rows.into_iter().map(Into::into).collect();
    let all_topologies: Vec<TopologyDefinition> =
        topology_rows.into_iter().map(Into::into).collect();
    let windows = load_all_windows(&state.db)
        .await
        .map_err(|cause| db_error(&cause))?;
    let mut backpressured_topology_count = 0i64;
    for topology in &all_topologies {
        let preview = processor::preview_topology_runtime(&state, topology, &all_streams, &windows)
            .await
            .map_err(bad_request)?;
        if preview.preview.backpressure_snapshot.status != "healthy" {
            backpressured_topology_count += 1;
        }
    }

    let live_event_count = sqlx::query_scalar::<_, i64>(
        r#"SELECT COUNT(*)
           FROM streaming_events
           WHERE archived_at IS NULL
             AND event_time >= now() - interval '15 minutes'"#,
    )
    .fetch_one(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?;

    Ok(Json(StreamingOverview {
        stream_count: all_streams.len() as i64,
        active_topology_count: all_topologies
            .iter()
            .filter(|topology| topology.status != "archived")
            .count() as i64,
        window_count,
        connector_count: connector_count as i64,
        running_topology_count: all_topologies
            .iter()
            .filter(|topology| topology.status == "running")
            .count() as i64,
        backpressured_topology_count,
        live_event_count,
    }))
}

pub async fn list_topologies(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> ServiceResult<ListResponse<TopologyDefinition>> {
    let rows = sqlx::query_as::<_, TopologyRow>(
        "SELECT id, name, description, status, nodes, edges, join_definition, cep_definition,
		        backpressure_policy, source_stream_ids, sink_bindings, state_backend,
		        created_at, updated_at
		 FROM streaming_topologies
		 ORDER BY created_at ASC",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?;

    Ok(Json(ListResponse {
        data: rows.into_iter().map(Into::into).collect(),
    }))
}

pub async fn create_topology(
    axum::extract::State(state): axum::extract::State<AppState>,
    Json(payload): Json<CreateTopologyRequest>,
) -> ServiceResult<TopologyDefinition> {
    let CreateTopologyRequest {
        name,
        description,
        status,
        nodes,
        edges,
        join_definition,
        cep_definition,
        backpressure_policy,
        source_stream_ids,
        sink_bindings,
        state_backend,
    } = payload;

    if name.trim().is_empty() {
        return Err(bad_request("topology name is required"));
    }
    if source_stream_ids.is_empty() {
        return Err(bad_request("at least one source stream is required"));
    }

    let streams = load_all_streams(&state.db)
        .await
        .map_err(|cause| db_error(&cause))?;
    let windows = load_all_windows(&state.db)
        .await
        .map_err(|cause| db_error(&cause))?;
    let backpressure_policy = backpressure_policy.unwrap_or_default();
    validate_topology_configuration(
        &name,
        &source_stream_ids,
        &nodes,
        &edges,
        join_definition.as_ref(),
        cep_definition.as_ref(),
        &backpressure_policy,
        &sink_bindings,
        &streams,
        &windows,
    )
    .map_err(bad_request)?;

    let topology_id = Uuid::now_v7();

    sqlx::query(
        "INSERT INTO streaming_topologies (
		    id, name, description, status, nodes, edges, join_definition, cep_definition,
		    backpressure_policy, source_stream_ids, sink_bindings, state_backend
		 ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
    )
    .bind(topology_id)
    .bind(name.trim())
    .bind(description.unwrap_or_default())
    .bind(status.unwrap_or_else(|| "active".to_string()))
    .bind(SqlJson(nodes))
    .bind(SqlJson(edges))
    .bind(join_definition.map(SqlJson))
    .bind(cep_definition.map(SqlJson))
    .bind(SqlJson(backpressure_policy))
    .bind(SqlJson(source_stream_ids))
    .bind(SqlJson(sink_bindings))
    .bind(state_backend.unwrap_or_else(|| "rocksdb".to_string()))
    .execute(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?;

    let row = load_topology_row(&state.db, topology_id)
        .await
        .map_err(|cause| db_error(&cause))?;

    Ok(Json(row.into()))
}

pub async fn update_topology(
    axum::extract::State(state): axum::extract::State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateTopologyRequest>,
) -> ServiceResult<TopologyDefinition> {
    let existing = match load_topology_row(&state.db, id).await {
        Ok(row) => row,
        Err(sqlx::Error::RowNotFound) => return Err(not_found("topology not found")),
        Err(cause) => return Err(db_error(&cause)),
    };
    let existing_definition = TopologyDefinition::from(existing.clone());
    let name = payload.name.unwrap_or(existing_definition.name.clone());
    let description = payload
        .description
        .unwrap_or(existing_definition.description.clone());
    let status = payload.status.unwrap_or(existing_definition.status.clone());
    let nodes = payload.nodes.unwrap_or(existing_definition.nodes.clone());
    let edges = payload.edges.unwrap_or(existing_definition.edges.clone());
    let join_definition = payload
        .join_definition
        .or(existing_definition.join_definition.clone());
    let cep_definition = payload
        .cep_definition
        .or(existing_definition.cep_definition.clone());
    let backpressure_policy = payload
        .backpressure_policy
        .unwrap_or(existing_definition.backpressure_policy.clone());
    let source_stream_ids = payload
        .source_stream_ids
        .unwrap_or(existing_definition.source_stream_ids.clone());
    let sink_bindings = payload
        .sink_bindings
        .unwrap_or(existing_definition.sink_bindings.clone());
    let state_backend = payload
        .state_backend
        .unwrap_or(existing_definition.state_backend.clone());
    let streams = load_all_streams(&state.db)
        .await
        .map_err(|cause| db_error(&cause))?;
    let windows = load_all_windows(&state.db)
        .await
        .map_err(|cause| db_error(&cause))?;
    validate_topology_configuration(
        &name,
        &source_stream_ids,
        &nodes,
        &edges,
        join_definition.as_ref(),
        cep_definition.as_ref(),
        &backpressure_policy,
        &sink_bindings,
        &streams,
        &windows,
    )
    .map_err(bad_request)?;

    sqlx::query(
        "UPDATE streaming_topologies
		 SET name = $2,
		     description = $3,
		     status = $4,
		     nodes = $5,
		     edges = $6,
		     join_definition = $7,
		     cep_definition = $8,
		     backpressure_policy = $9,
		     source_stream_ids = $10,
		     sink_bindings = $11,
		     state_backend = $12,
		     updated_at = now()
		 WHERE id = $1",
    )
    .bind(id)
    .bind(name)
    .bind(description)
    .bind(status)
    .bind(SqlJson(nodes))
    .bind(SqlJson(edges))
    .bind(join_definition.map(SqlJson))
    .bind(cep_definition.map(SqlJson))
    .bind(SqlJson(backpressure_policy))
    .bind(SqlJson(source_stream_ids))
    .bind(SqlJson(sink_bindings))
    .bind(state_backend)
    .execute(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?;

    let row = load_topology_row(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?;

    Ok(Json(row.into()))
}

pub async fn run_topology(
    axum::extract::State(state): axum::extract::State<AppState>,
    Path(id): Path<Uuid>,
) -> ServiceResult<TopologyRun> {
    let topology = match load_topology_row(&state.db, id).await {
        Ok(row) => TopologyDefinition::from(row),
        Err(sqlx::Error::RowNotFound) => return Err(not_found("topology not found")),
        Err(cause) => return Err(db_error(&cause)),
    };

    let streams = load_all_streams(&state.db)
        .await
        .map_err(|cause| db_error(&cause))?;
    let windows = load_all_windows(&state.db)
        .await
        .map_err(|cause| db_error(&cause))?;
    let execution = processor::run_topology(&state, &topology, &streams, &windows)
        .await
        .map_err(|message| bad_request(message))?;
    let run_id = Uuid::now_v7();

    sqlx::query(
        "INSERT INTO streaming_topology_runs (
		    id, topology_id, status, metrics, aggregate_windows, live_tail, cep_matches,
		    state_snapshot, backpressure_snapshot, started_at, completed_at
		 ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
    )
    .bind(run_id)
    .bind(id)
    .bind("completed")
    .bind(SqlJson(execution.metrics))
    .bind(SqlJson(execution.aggregate_windows))
    .bind(SqlJson(execution.live_tail))
    .bind(SqlJson(execution.cep_matches))
    .bind(SqlJson(execution.state_snapshot))
    .bind(SqlJson(execution.backpressure_snapshot))
    .bind(execution.started_at)
    .bind(execution.completed_at)
    .execute(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?;

    let row = load_latest_run_row(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?
        .ok_or_else(|| not_found("topology run not found after execution"))?;

    Ok(Json(row.into()))
}

pub async fn replay_topology(
    axum::extract::State(state): axum::extract::State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<ReplayTopologyRequest>,
) -> ServiceResult<ReplayTopologyResponse> {
    let topology = match load_topology_row(&state.db, id).await {
        Ok(row) => TopologyDefinition::from(row),
        Err(sqlx::Error::RowNotFound) => return Err(not_found("topology not found")),
        Err(cause) => return Err(db_error(&cause)),
    };

    let stream_ids = payload
        .stream_ids
        .unwrap_or_else(|| topology.source_stream_ids.clone());
    if stream_ids.is_empty() {
        return Err(bad_request(
            "at least one stream_id is required to replay a topology",
        ));
    }
    if !stream_ids
        .iter()
        .all(|stream_id| topology.source_stream_ids.contains(stream_id))
    {
        return Err(bad_request(
            "stream_ids must be a subset of topology.source_stream_ids",
        ));
    }

    if let Some(sequence_no) = payload.from_sequence_no {
        if sequence_no <= 0 {
            return Err(bad_request(
                "from_sequence_no must be positive when provided",
            ));
        }
    }

    let restored_event_count =
        restore_topology_events(&state.db, &stream_ids, payload.from_sequence_no)
            .await
            .map_err(|cause| db_error(&cause))?;

    match payload.from_sequence_no {
        Some(sequence_no) => {
            for stream_id in &stream_ids {
                sqlx::query(
                    "INSERT INTO streaming_checkpoints (
                         topology_id, stream_id, last_event_id, last_sequence_no, updated_at
                     ) VALUES ($1, $2, NULL, $3, now())
                     ON CONFLICT (topology_id, stream_id)
                     DO UPDATE SET last_event_id = NULL,
                                   last_sequence_no = EXCLUDED.last_sequence_no,
                                   updated_at = now()",
                )
                .bind(id)
                .bind(stream_id)
                .bind(sequence_no - 1)
                .execute(&state.db)
                .await
                .map_err(|cause| db_error(&cause))?;
            }
        }
        None => {
            sqlx::query(
                "DELETE FROM streaming_checkpoints
                 WHERE topology_id = $1
                   AND stream_id = ANY($2)",
            )
            .bind(id)
            .bind(&stream_ids)
            .execute(&state.db)
            .await
            .map_err(|cause| db_error(&cause))?;
        }
    }

    Ok(Json(ReplayTopologyResponse {
        topology_id: id,
        stream_ids,
        replay_from_sequence_no: payload.from_sequence_no,
        restored_event_count,
    }))
}

pub async fn get_runtime(
    axum::extract::State(state): axum::extract::State<AppState>,
    Path(id): Path<Uuid>,
) -> ServiceResult<TopologyRuntimeSnapshot> {
    let topology = match load_topology_row(&state.db, id).await {
        Ok(row) => TopologyDefinition::from(row),
        Err(sqlx::Error::RowNotFound) => return Err(not_found("topology not found")),
        Err(cause) => return Err(db_error(&cause)),
    };

    let streams = load_all_streams(&state.db)
        .await
        .map_err(|cause| db_error(&cause))?;
    let windows = load_all_windows(&state.db)
        .await
        .map_err(|cause| db_error(&cause))?;
    let preview = processor::preview_topology_runtime(&state, &topology, &streams, &windows)
        .await
        .map_err(bad_request)?;
    let latest_run = load_latest_run_row(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?;
    let connector_statuses = apply_topology_connector_runtime(
        &topology,
        &streams,
        connectors::catalog_entries(&topology, &streams),
        &preview.source_backlog,
        &preview.source_throughput_per_second,
        preview.preview.backpressure_snapshot.queue_depth,
        preview.preview.metrics.throughput_per_second,
        &preview.preview.backpressure_snapshot.status,
    );
    let latest_events = latest_run
        .as_ref()
        .map(|row| row.live_tail.0.clone())
        .unwrap_or_else(|| preview.latest_events.clone());
    let latest_matches = latest_run
        .as_ref()
        .map(|row| row.cep_matches.0.clone())
        .unwrap_or_else(|| preview.latest_matches.clone());

    Ok(Json(TopologyRuntimeSnapshot {
        topology,
        latest_run: latest_run.map(Into::into),
        preview: Some(preview.preview),
        connector_statuses,
        latest_events,
        latest_matches,
    }))
}

pub async fn list_connectors(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> ServiceResult<ListResponse<ConnectorCatalogEntry>> {
    let streams = load_all_streams(&state.db)
        .await
        .map_err(|cause| db_error(&cause))?;
    let stream_activity = load_stream_activity_stats(&state.db)
        .await
        .map_err(|cause| db_error(&cause))?;
    let topologies = sqlx::query_as::<_, TopologyRow>(
        "SELECT id, name, description, status, nodes, edges, join_definition, cep_definition,
		        backpressure_policy, source_stream_ids, sink_bindings, state_backend,
		        created_at, updated_at
		 FROM streaming_topologies
		 ORDER BY created_at ASC",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?;
    let latest_run_rows = sqlx::query_as::<_, TopologyRunRow>(
        "SELECT id, topology_id, status, metrics, aggregate_windows, live_tail, cep_matches,
		        state_snapshot, backpressure_snapshot, started_at, completed_at, created_at, updated_at
		 FROM streaming_topology_runs
		 ORDER BY created_at DESC",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?;
    let mut latest_run_by_topology = HashMap::<Uuid, TopologyRunRow>::new();
    for row in latest_run_rows {
        latest_run_by_topology.entry(row.topology_id).or_insert(row);
    }

    let mut entries = Vec::new();
    for topology in topologies.into_iter().map(TopologyDefinition::from) {
        let mut source_backlog = HashMap::new();
        let mut source_throughput = HashMap::new();
        for stream in streams
            .iter()
            .filter(|stream| topology.source_stream_ids.contains(&stream.id))
        {
            if let Some(activity) = stream_activity.get(&stream.id) {
                source_backlog.insert(stream.id, activity.backlog as i32);
                source_throughput.insert(
                    stream.id,
                    throughput_from_window(
                        activity.recent_events,
                        activity.oldest_event_time,
                        activity.newest_event_time,
                    ),
                );
            }
        }
        let sink_backlog = latest_run_by_topology
            .get(&topology.id)
            .map(|row| row.backpressure_snapshot.0.queue_depth)
            .unwrap_or(0);
        let sink_throughput = latest_run_by_topology
            .get(&topology.id)
            .map(|row| row.metrics.0.throughput_per_second)
            .unwrap_or(0.0);
        let sink_status = latest_run_by_topology
            .get(&topology.id)
            .map(|row| row.backpressure_snapshot.0.status.as_str())
            .unwrap_or("healthy");
        entries.extend(apply_topology_connector_runtime(
            &topology,
            &streams,
            connectors::catalog_entries(&topology, &streams),
            &source_backlog,
            &source_throughput,
            sink_backlog,
            sink_throughput,
            sink_status,
        ));
    }

    Ok(Json(ListResponse { data: entries }))
}

pub async fn get_live_tail(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> ServiceResult<LiveTailResponse> {
    let streams = load_all_streams(&state.db)
        .await
        .map_err(|cause| db_error(&cause))?;
    let windows = load_all_windows(&state.db)
        .await
        .map_err(|cause| db_error(&cause))?;
    let topologies = sqlx::query_as::<_, TopologyRow>(
        "SELECT id, name, description, status, nodes, edges, join_definition, cep_definition,
		        backpressure_policy, source_stream_ids, sink_bindings, state_backend,
		        created_at, updated_at
		 FROM streaming_topologies
		 ORDER BY created_at ASC",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?;

    let mut events = Vec::new();
    let mut matches = Vec::new();
    for topology in topologies.into_iter().map(TopologyDefinition::from) {
        let preview = processor::preview_topology_runtime(&state, &topology, &streams, &windows)
            .await
            .map_err(bad_request)?;
        events.extend(preview.latest_events);
        matches.extend(preview.latest_matches);
    }

    events.sort_by_key(|event| event.processing_time);
    events.reverse();
    events.truncate(24);
    matches.sort_by_key(|item| item.detected_at);
    matches.reverse();
    matches.truncate(10);

    Ok(Json(LiveTailResponse { events, matches }))
}

#[cfg(test)]
mod tests {
    use super::validate_topology_configuration;
    use crate::models::{
        stream::{ConnectorBinding, StreamDefinition, StreamField, StreamSchema},
        topology::{BackpressurePolicy, TopologyEdge, TopologyNode},
        window::WindowDefinition,
    };
    use chrono::Utc;
    use serde_json::json;
    use uuid::Uuid;

    fn sample_stream(id: Uuid, name: &str) -> StreamDefinition {
        StreamDefinition {
            id,
            name: name.to_string(),
            description: String::new(),
            status: "active".to_string(),
            schema: StreamSchema {
                fields: vec![StreamField {
                    name: "event_time".to_string(),
                    data_type: "timestamp".to_string(),
                    nullable: false,
                    semantic_role: "event_time".to_string(),
                }],
                primary_key: None,
                watermark_field: Some("event_time".to_string()),
            },
            source_binding: ConnectorBinding::default(),
            retention_hours: 72,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn sample_window(id: Uuid) -> WindowDefinition {
        WindowDefinition {
            id,
            name: "Five Minute Window".to_string(),
            description: String::new(),
            status: "active".to_string(),
            window_type: "tumbling".to_string(),
            duration_seconds: 300,
            slide_seconds: 300,
            session_gap_seconds: 120,
            allowed_lateness_seconds: 30,
            aggregation_keys: vec!["customer_id".to_string()],
            measure_fields: vec!["amount".to_string()],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn rejects_cyclic_topologies() {
        let stream_id = Uuid::now_v7();
        let window_id = Uuid::now_v7();
        let result = validate_topology_configuration(
            "Cycle",
            &[stream_id],
            &[
                TopologyNode {
                    id: "src".to_string(),
                    label: "Source".to_string(),
                    node_type: "source".to_string(),
                    stream_id: Some(stream_id),
                    window_id: None,
                    config: json!({}),
                },
                TopologyNode {
                    id: "sink".to_string(),
                    label: "Sink".to_string(),
                    node_type: "sink".to_string(),
                    stream_id: None,
                    window_id: None,
                    config: json!({}),
                },
                TopologyNode {
                    id: "window".to_string(),
                    label: "Window".to_string(),
                    node_type: "window".to_string(),
                    stream_id: None,
                    window_id: Some(window_id),
                    config: json!({}),
                },
            ],
            &[
                TopologyEdge {
                    source_node_id: "src".to_string(),
                    target_node_id: "window".to_string(),
                    label: "into-window".to_string(),
                },
                TopologyEdge {
                    source_node_id: "window".to_string(),
                    target_node_id: "sink".to_string(),
                    label: "to-sink".to_string(),
                },
                TopologyEdge {
                    source_node_id: "sink".to_string(),
                    target_node_id: "src".to_string(),
                    label: "cycle".to_string(),
                },
            ],
            None,
            None,
            &BackpressurePolicy::default(),
            &[ConnectorBinding::default()],
            &[sample_stream(stream_id, "Orders")],
            &[sample_window(window_id)],
        );

        assert!(result.is_err());
        assert!(result.expect_err("cycle should fail").contains("cycle"));
    }

    #[test]
    fn rejects_source_nodes_outside_declared_sources() {
        let stream_id = Uuid::now_v7();
        let foreign_stream_id = Uuid::now_v7();
        let result = validate_topology_configuration(
            "Bad Source",
            &[stream_id],
            &[
                TopologyNode {
                    id: "src".to_string(),
                    label: "Source".to_string(),
                    node_type: "source".to_string(),
                    stream_id: Some(foreign_stream_id),
                    window_id: None,
                    config: json!({}),
                },
                TopologyNode {
                    id: "sink".to_string(),
                    label: "Sink".to_string(),
                    node_type: "sink".to_string(),
                    stream_id: None,
                    window_id: None,
                    config: json!({}),
                },
            ],
            &[TopologyEdge {
                source_node_id: "src".to_string(),
                target_node_id: "sink".to_string(),
                label: "forward".to_string(),
            }],
            None,
            None,
            &BackpressurePolicy::default(),
            &[ConnectorBinding::default()],
            &[
                sample_stream(stream_id, "Orders"),
                sample_stream(foreign_stream_id, "Payments"),
            ],
            &[],
        );

        assert!(result.is_err());
        assert!(
            result
                .expect_err("invalid source should fail")
                .contains("outside source_stream_ids")
        );
    }
}
