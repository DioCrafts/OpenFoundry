use std::collections::HashMap;

use axum::{Json, extract::Path};
use sqlx::types::Json as SqlJson;
use uuid::Uuid;

use crate::{
    AppState,
    domain::{connectors, engine::processor},
    handlers::{ServiceResult, bad_request, db_error, not_found},
    models::{
        ListResponse, StreamingOverview,
        sink::{ConnectorCatalogEntry, LiveTailResponse},
        stream::StreamRow,
        topology::{
            CreateTopologyRequest, TopologyDefinition, TopologyRow, TopologyRun, TopologyRunRow,
            TopologyRuntimeSnapshot, UpdateTopologyRequest,
        },
        window::WindowRow,
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

    let run_rows = sqlx::query_as::<_, TopologyRunRow>(
        "SELECT id, topology_id, status, metrics, aggregate_windows, live_tail, cep_matches,
		        state_snapshot, backpressure_snapshot, started_at, completed_at, created_at, updated_at
		 FROM streaming_topology_runs
		 ORDER BY created_at DESC",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?;

    let connector_count = stream_rows.len()
        + topology_rows
            .iter()
            .map(|row| row.sink_bindings.0.len())
            .sum::<usize>();

    let mut latest_backpressure: HashMap<Uuid, String> = HashMap::new();
    for row in &run_rows {
        latest_backpressure
            .entry(row.topology_id)
            .or_insert_with(|| row.backpressure_snapshot.0.status.clone());
    }

    let all_streams: Vec<crate::models::stream::StreamDefinition> =
        stream_rows.into_iter().map(Into::into).collect();
    let all_topologies: Vec<TopologyDefinition> =
        topology_rows.into_iter().map(Into::into).collect();
    let live_event_count = if let Some(topology) = all_topologies.first() {
        connectors::live_events(topology, &all_streams).len() as i64
    } else {
        0
    };

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
        backpressured_topology_count: latest_backpressure
            .values()
            .filter(|status| status.as_str() != "healthy")
            .count() as i64,
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
    if payload.name.trim().is_empty() {
        return Err(bad_request("topology name is required"));
    }
    if payload.source_stream_ids.is_empty() {
        return Err(bad_request("at least one source stream is required"));
    }

    let topology_id = Uuid::now_v7();

    sqlx::query(
        "INSERT INTO streaming_topologies (
		    id, name, description, status, nodes, edges, join_definition, cep_definition,
		    backpressure_policy, source_stream_ids, sink_bindings, state_backend
		 ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
    )
    .bind(topology_id)
    .bind(payload.name.trim())
    .bind(payload.description.unwrap_or_default())
    .bind(payload.status.unwrap_or_else(|| "active".to_string()))
    .bind(SqlJson(payload.nodes))
    .bind(SqlJson(payload.edges))
    .bind(payload.join_definition.map(SqlJson))
    .bind(payload.cep_definition.map(SqlJson))
    .bind(SqlJson(payload.backpressure_policy.unwrap_or_default()))
    .bind(SqlJson(payload.source_stream_ids))
    .bind(SqlJson(payload.sink_bindings))
    .bind(
        payload
            .state_backend
            .unwrap_or_else(|| "rocksdb".to_string()),
    )
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
    .bind(payload.name.unwrap_or(existing.name))
    .bind(payload.description.unwrap_or(existing.description))
    .bind(payload.status.unwrap_or(existing.status))
    .bind(SqlJson(payload.nodes.unwrap_or(existing.nodes.0)))
    .bind(SqlJson(payload.edges.unwrap_or(existing.edges.0)))
    .bind(
        payload
            .join_definition
            .map(SqlJson)
            .or(existing.join_definition),
    )
    .bind(
        payload
            .cep_definition
            .map(SqlJson)
            .or(existing.cep_definition),
    )
    .bind(SqlJson(
        payload
            .backpressure_policy
            .unwrap_or(existing.backpressure_policy.0),
    ))
    .bind(SqlJson(
        payload
            .source_stream_ids
            .unwrap_or(existing.source_stream_ids.0),
    ))
    .bind(SqlJson(
        payload.sink_bindings.unwrap_or(existing.sink_bindings.0),
    ))
    .bind(payload.state_backend.unwrap_or(existing.state_backend))
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
    let connector_statuses = connectors::catalog_entries(&topology, &streams);
    let latest_run = load_latest_run_row(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?;
    let latest_events = latest_run
        .as_ref()
        .map(|row| row.live_tail.0.clone())
        .unwrap_or_else(|| connectors::live_events(&topology, &streams));
    let latest_matches = latest_run
        .as_ref()
        .map(|row| row.cep_matches.0.clone())
        .unwrap_or_else(|| {
            crate::domain::engine::cep::simulate_cep_matches(topology.cep_definition.as_ref())
        });

    Ok(Json(TopologyRuntimeSnapshot {
        topology,
        latest_run: latest_run.map(Into::into),
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

    let mut entries = Vec::new();
    for topology in topologies.into_iter().map(TopologyDefinition::from) {
        entries.extend(connectors::catalog_entries(&topology, &streams));
    }

    Ok(Json(ListResponse { data: entries }))
}

pub async fn get_live_tail(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> ServiceResult<LiveTailResponse> {
    let streams = load_all_streams(&state.db)
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

    let run_rows = sqlx::query_as::<_, TopologyRunRow>(
        "SELECT id, topology_id, status, metrics, aggregate_windows, live_tail, cep_matches,
		        state_snapshot, backpressure_snapshot, started_at, completed_at, created_at, updated_at
		 FROM streaming_topology_runs
		 ORDER BY created_at DESC
		 LIMIT 8",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?;

    let mut events = Vec::new();
    let mut matches = Vec::new();

    if run_rows.is_empty() {
        for topology in topologies.into_iter().map(TopologyDefinition::from) {
            events.extend(connectors::live_events(&topology, &streams));
            matches.extend(crate::domain::engine::cep::simulate_cep_matches(
                topology.cep_definition.as_ref(),
            ));
        }
    } else {
        for row in run_rows {
            events.extend(row.live_tail.0);
            matches.extend(row.cep_matches.0);
        }
    }

    events.sort_by_key(|event| event.processing_time);
    events.reverse();
    events.truncate(24);
    matches.sort_by_key(|item| item.detected_at);
    matches.reverse();
    matches.truncate(10);

    Ok(Json(LiveTailResponse { events, matches }))
}
