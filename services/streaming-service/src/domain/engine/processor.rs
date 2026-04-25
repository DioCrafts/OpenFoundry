use std::{collections::HashMap, path::Path};

use auth_middleware::jwt::{build_access_claims, encode_token};
use chrono::{DateTime, Duration, Utc};
use reqwest::multipart::{Form, Part};
use serde_json::{Value, json};
use sqlx::types::Json as SqlJson;
use uuid::Uuid;

use crate::{
    AppState,
    domain::backpressure,
    models::{
        sink::{
            BackpressureSnapshot, CepMatch, LiveTailEvent, StateStoreSnapshot, WindowAggregate,
        },
        stream::{ConnectorBinding, StreamDefinition},
        topology::{TopologyDefinition, TopologyRunMetrics, TopologyRuntimePreview},
        window::WindowDefinition,
    },
};

pub struct TopologyExecution {
    pub metrics: TopologyRunMetrics,
    pub live_tail: Vec<LiveTailEvent>,
    pub cep_matches: Vec<CepMatch>,
    pub aggregate_windows: Vec<WindowAggregate>,
    pub state_snapshot: StateStoreSnapshot,
    pub backpressure_snapshot: BackpressureSnapshot,
    pub started_at: chrono::DateTime<Utc>,
    pub completed_at: chrono::DateTime<Utc>,
}

pub struct TopologyRuntimeAnalysis {
    pub preview: TopologyRuntimePreview,
    pub latest_events: Vec<LiveTailEvent>,
    pub latest_matches: Vec<CepMatch>,
    pub source_backlog: HashMap<Uuid, i32>,
    pub source_throughput_per_second: HashMap<Uuid, f32>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct StreamEventRow {
    id: Uuid,
    stream_id: Uuid,
    sequence_no: i64,
    payload: SqlJson<Value>,
    event_time: DateTime<Utc>,
    processed_at: Option<DateTime<Utc>>,
    archived_at: Option<DateTime<Utc>>,
    archive_path: Option<String>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct StreamingCheckpointRow {
    stream_id: Uuid,
    last_sequence_no: i64,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
struct ProcessedEvent {
    id: Uuid,
    stream_id: Uuid,
    stream_name: String,
    connector_type: String,
    payload: Value,
    event_time: DateTime<Utc>,
    processing_time: DateTime<Utc>,
    sequence_no: i64,
}

pub async fn run_topology(
    state: &AppState,
    topology: &TopologyDefinition,
    streams: &[StreamDefinition],
    windows: &[WindowDefinition],
) -> Result<TopologyExecution, String> {
    let started_at = Utc::now();
    let checkpoint_map = load_checkpoints(&state.db, topology.id).await?;
    let stream_lookup = streams
        .iter()
        .map(|stream| (stream.id, stream))
        .collect::<HashMap<_, _>>();
    let source_events = load_source_events_since_checkpoint(
        &state.db,
        topology,
        &stream_lookup,
        &checkpoint_map,
        started_at,
    )
    .await?;
    let live_tail = source_events
        .iter()
        .rev()
        .take(24)
        .cloned()
        .map(to_live_tail_event(topology.id))
        .collect::<Vec<_>>();

    let joined_events = build_joined_events(topology, &source_events);
    let materialization_events = if joined_events.is_empty() {
        source_events.clone()
    } else {
        joined_events
    };
    let aggregate_windows = build_window_aggregates(topology, windows, &materialization_events);
    let cep_matches = detect_cep_matches(topology, &materialization_events);
    let source_backlog = group_event_count_by_stream(&source_events);
    let backpressure_snapshot = backpressure::derive_backpressure_snapshot(
        &topology.backpressure_policy,
        source_events.len() as i32,
        source_backlog.values().copied().max().unwrap_or(0),
        topology.source_stream_ids.len(),
    );

    materialize_sinks(
        state,
        topology,
        &materialization_events,
        &aggregate_windows,
        &cep_matches,
    )
    .await?;
    persist_checkpoints(&state.db, topology.id, &source_events).await?;
    mark_events_processed(&state.db, &source_events).await?;
    archive_processed_events(state, streams, &source_events).await?;

    let completed_at = Utc::now();
    let mut metrics = build_run_metrics(
        &source_events,
        &materialization_events,
        &aggregate_windows,
        &cep_matches,
        completed_at,
    );
    let state_snapshot = build_state_snapshot(
        topology,
        aggregate_windows.len() as i32 + cep_matches.len() as i32 + metrics.input_events,
        topology.source_stream_ids.len() as i32,
        completed_at,
    );
    metrics.backpressure_ratio = backpressure_ratio(&backpressure_snapshot);
    metrics.state_entries = state_snapshot.key_count;

    Ok(TopologyExecution {
        metrics,
        live_tail,
        cep_matches,
        aggregate_windows,
        state_snapshot,
        backpressure_snapshot,
        started_at,
        completed_at,
    })
}

pub async fn preview_topology_runtime(
    state: &AppState,
    topology: &TopologyDefinition,
    streams: &[StreamDefinition],
    windows: &[WindowDefinition],
) -> Result<TopologyRuntimeAnalysis, String> {
    let generated_at = Utc::now();
    let checkpoint_map = load_checkpoints(&state.db, topology.id).await?;
    let stream_lookup = streams
        .iter()
        .map(|stream| (stream.id, stream))
        .collect::<HashMap<_, _>>();

    let pending_events = load_source_events_since_checkpoint(
        &state.db,
        topology,
        &stream_lookup,
        &checkpoint_map,
        generated_at,
    )
    .await?;
    let recent_events =
        load_recent_source_events(&state.db, topology, &stream_lookup, generated_at, 12).await?;
    let analysis_events = if pending_events.is_empty() {
        recent_events.clone()
    } else {
        pending_events.clone()
    };
    let joined_events = build_joined_events(topology, &analysis_events);
    let materialization_events = if joined_events.is_empty() {
        analysis_events.clone()
    } else {
        joined_events
    };
    let aggregate_windows = build_window_aggregates(topology, windows, &materialization_events);
    let latest_matches = detect_cep_matches(topology, &materialization_events);
    let latest_events = recent_events
        .iter()
        .rev()
        .take(24)
        .cloned()
        .map(to_live_tail_event(topology.id))
        .collect::<Vec<_>>();
    let source_backlog = group_event_count_by_stream(&pending_events);
    let source_throughput_per_second = group_throughput_by_stream(&recent_events);
    let backpressure_snapshot = backpressure::derive_backpressure_snapshot(
        &topology.backpressure_policy,
        pending_events.len() as i32,
        source_backlog.values().copied().max().unwrap_or(0),
        topology.source_stream_ids.len(),
    );
    let mut metrics = build_preview_metrics(
        &pending_events,
        &materialization_events,
        &aggregate_windows,
        &latest_matches,
        &recent_events,
        &backpressure_snapshot,
    );
    let last_checkpoint_at = checkpoint_map
        .values()
        .map(|checkpoint| checkpoint.updated_at)
        .max()
        .unwrap_or(generated_at);
    let state_snapshot = build_state_snapshot(
        topology,
        aggregate_windows.len() as i32 + latest_matches.len() as i32 + pending_events.len() as i32,
        checkpoint_map.len() as i32,
        last_checkpoint_at,
    );
    metrics.state_entries = state_snapshot.key_count;

    Ok(TopologyRuntimeAnalysis {
        preview: TopologyRuntimePreview {
            metrics,
            aggregate_windows,
            backpressure_snapshot,
            state_snapshot,
            backlog_events: pending_events.len() as i32,
            generated_at,
        },
        latest_events,
        latest_matches,
        source_backlog,
        source_throughput_per_second,
    })
}

async fn load_checkpoints(
    db: &sqlx::PgPool,
    topology_id: Uuid,
) -> Result<HashMap<Uuid, StreamingCheckpointRow>, String> {
    let rows = sqlx::query_as::<_, StreamingCheckpointRow>(
        r#"SELECT stream_id, last_sequence_no, updated_at
           FROM streaming_checkpoints
           WHERE topology_id = $1"#,
    )
    .bind(topology_id)
    .fetch_all(db)
    .await
    .map_err(|error| error.to_string())?;

    Ok(rows.into_iter().map(|row| (row.stream_id, row)).collect())
}

async fn load_source_events_since_checkpoint(
    db: &sqlx::PgPool,
    topology: &TopologyDefinition,
    stream_lookup: &HashMap<Uuid, &StreamDefinition>,
    checkpoint_map: &HashMap<Uuid, StreamingCheckpointRow>,
    processing_time: DateTime<Utc>,
) -> Result<Vec<ProcessedEvent>, String> {
    let mut source_events = Vec::new();
    for stream_id in &topology.source_stream_ids {
        let Some(stream) = stream_lookup.get(stream_id).copied() else {
            continue;
        };
        let last_sequence_no = checkpoint_map
            .get(stream_id)
            .map(|checkpoint| checkpoint.last_sequence_no)
            .unwrap_or(0);
        let rows = sqlx::query_as::<_, StreamEventRow>(
            r#"SELECT id, stream_id, sequence_no, payload, event_time, processed_at, archived_at, archive_path
               FROM streaming_events
               WHERE stream_id = $1
                 AND sequence_no > $2
                 AND archived_at IS NULL
               ORDER BY sequence_no ASC"#,
        )
        .bind(stream_id)
        .bind(last_sequence_no)
        .fetch_all(db)
        .await
        .map_err(|error| error.to_string())?;

        for row in rows {
            source_events.push(to_processed_event(stream, row, processing_time));
        }
    }

    source_events.sort_by_key(|event| event.sequence_no);
    Ok(source_events)
}

async fn load_recent_source_events(
    db: &sqlx::PgPool,
    topology: &TopologyDefinition,
    stream_lookup: &HashMap<Uuid, &StreamDefinition>,
    processing_time: DateTime<Utc>,
    limit_per_stream: i64,
) -> Result<Vec<ProcessedEvent>, String> {
    let mut source_events = Vec::new();
    for stream_id in &topology.source_stream_ids {
        let Some(stream) = stream_lookup.get(stream_id).copied() else {
            continue;
        };
        let rows = sqlx::query_as::<_, StreamEventRow>(
            r#"SELECT id, stream_id, sequence_no, payload, event_time, processed_at, archived_at, archive_path
               FROM streaming_events
               WHERE stream_id = $1
                 AND archived_at IS NULL
               ORDER BY event_time DESC, sequence_no DESC
               LIMIT $2"#,
        )
        .bind(stream_id)
        .bind(limit_per_stream)
        .fetch_all(db)
        .await
        .map_err(|error| error.to_string())?;

        for row in rows {
            source_events.push(to_processed_event(stream, row, processing_time));
        }
    }

    source_events.sort_by_key(|event| (event.event_time, event.sequence_no));
    Ok(source_events)
}

fn to_processed_event(
    stream: &StreamDefinition,
    row: StreamEventRow,
    default_processing_time: DateTime<Utc>,
) -> ProcessedEvent {
    let processing_time = row.processed_at.unwrap_or(default_processing_time);
    let _ = row.archived_at;
    let _ = row.archive_path;
    ProcessedEvent {
        id: row.id,
        stream_id: row.stream_id,
        stream_name: stream.name.clone(),
        connector_type: stream.source_binding.connector_type.clone(),
        payload: row.payload.0,
        event_time: row.event_time,
        processing_time,
        sequence_no: row.sequence_no,
    }
}

fn to_live_tail_event(topology_id: Uuid) -> impl Fn(ProcessedEvent) -> LiveTailEvent {
    move |event| LiveTailEvent {
        id: format!(
            "{}-{}",
            event.stream_name.to_lowercase().replace(' ', "-"),
            event.sequence_no
        ),
        topology_id,
        stream_name: event.stream_name,
        connector_type: event.connector_type,
        payload: event.payload,
        event_time: event.event_time,
        processing_time: event.processing_time,
        tags: vec![format!("stream:{}", event.stream_id)],
    }
}

fn build_joined_events(
    topology: &TopologyDefinition,
    source_events: &[ProcessedEvent],
) -> Vec<ProcessedEvent> {
    let Some(join_definition) = topology.join_definition.as_ref() else {
        return Vec::new();
    };

    let left_events = source_events
        .iter()
        .filter(|event| event.stream_id == join_definition.left_stream_id)
        .collect::<Vec<_>>();
    let right_events = source_events
        .iter()
        .filter(|event| event.stream_id == join_definition.right_stream_id)
        .collect::<Vec<_>>();
    let mut joined = Vec::new();

    for left in &left_events {
        for right in &right_events {
            if !join_definition
                .key_fields
                .iter()
                .all(|field| left.payload.get(field) == right.payload.get(field))
            {
                continue;
            }

            let delta = (left.event_time - right.event_time)
                .num_seconds()
                .unsigned_abs() as i32;
            if delta > join_definition.window_seconds {
                continue;
            }

            let mut merged = serde_json::Map::new();
            merged.insert("_join".to_string(), json!(join_definition.join_type));
            merged.insert("left_stream".to_string(), json!(left.stream_name));
            merged.insert("right_stream".to_string(), json!(right.stream_name));
            for (key, value) in left.payload.as_object().into_iter().flatten() {
                merged.insert(key.clone(), value.clone());
            }
            for (key, value) in right.payload.as_object().into_iter().flatten() {
                merged.insert(format!("right_{key}"), value.clone());
            }

            joined.push(ProcessedEvent {
                id: Uuid::now_v7(),
                stream_id: left.stream_id,
                stream_name: format!("{}-joined", topology.name),
                connector_type: "join".to_string(),
                payload: Value::Object(merged),
                event_time: left.event_time.max(right.event_time),
                processing_time: left.processing_time.max(right.processing_time),
                sequence_no: left.sequence_no.max(right.sequence_no),
            });
        }
    }

    joined
}

fn build_window_aggregates(
    topology: &TopologyDefinition,
    windows: &[WindowDefinition],
    events: &[ProcessedEvent],
) -> Vec<WindowAggregate> {
    let mut aggregates = Vec::new();
    for node in topology.nodes.iter().filter_map(|node| node.window_id) {
        let Some(window) = windows.iter().find(|window| window.id == node) else {
            continue;
        };

        let mut grouped: HashMap<(DateTime<Utc>, String, String), f64> = HashMap::new();
        for event in events {
            let bucket_start =
                bucket_start(event.event_time, window.duration_seconds.max(1) as i64);
            let group_key = if window.aggregation_keys.is_empty() {
                "all".to_string()
            } else {
                window
                    .aggregation_keys
                    .iter()
                    .map(|key| {
                        let value = event.payload.get(key).cloned().unwrap_or(Value::Null);
                        format!("{key}:{}", stringify_json_value(&value))
                    })
                    .collect::<Vec<_>>()
                    .join("|")
            };
            let measures = if window.measure_fields.is_empty() {
                vec![("events_per_window".to_string(), 1.0)]
            } else {
                window
                    .measure_fields
                    .iter()
                    .map(|field| {
                        (
                            field.clone(),
                            event
                                .payload
                                .get(field)
                                .and_then(Value::as_f64)
                                .unwrap_or(0.0),
                        )
                    })
                    .collect::<Vec<_>>()
            };

            for (measure_name, value) in measures {
                *grouped
                    .entry((bucket_start, group_key.clone(), measure_name))
                    .or_insert(0.0) += value;
            }
        }

        for ((bucket_start, group_key, measure_name), value) in grouped {
            aggregates.push(WindowAggregate {
                window_name: window.name.clone(),
                window_type: window.window_type.clone(),
                bucket_start,
                bucket_end: bucket_start + Duration::seconds(window.duration_seconds as i64),
                group_key,
                measure_name,
                value,
            });
        }
    }

    aggregates.sort_by_key(|aggregate| aggregate.bucket_start);
    aggregates
}

fn detect_cep_matches(topology: &TopologyDefinition, events: &[ProcessedEvent]) -> Vec<CepMatch> {
    let Some(cep_definition) = topology.cep_definition.as_ref() else {
        return Vec::new();
    };
    let sorted = {
        let mut events = events.to_vec();
        events.sort_by_key(|event| event.event_time);
        events
    };
    let mut matches = Vec::new();

    for start_index in 0..sorted.len() {
        let mut cursor = start_index;
        let start_time = sorted[start_index].event_time;
        let mut matched = Vec::new();
        for expected in &cep_definition.sequence {
            while cursor < sorted.len() {
                let label = event_label(&sorted[cursor]);
                if label.eq_ignore_ascii_case(expected) {
                    matched.push(expected.clone());
                    cursor += 1;
                    break;
                }
                cursor += 1;
            }
        }

        if matched.len() == cep_definition.sequence.len()
            && (sorted[cursor.saturating_sub(1)].event_time - start_time).num_seconds()
                <= cep_definition.within_seconds as i64
        {
            matches.push(CepMatch {
                pattern_name: cep_definition.pattern_name.clone(),
                matched_sequence: matched,
                confidence: 0.95,
                detected_at: sorted[cursor.saturating_sub(1)].event_time,
            });
        }
    }

    matches
}

async fn materialize_sinks(
    state: &AppState,
    topology: &TopologyDefinition,
    events: &[ProcessedEvent],
    aggregates: &[WindowAggregate],
    cep_matches: &[CepMatch],
) -> Result<(), String> {
    let actor_id = Uuid::now_v7();
    for sink in &topology.sink_bindings {
        if sink.connector_type != "dataset" {
            continue;
        }

        let Some(dataset_id) = dataset_id_from_binding(sink) else {
            continue;
        };
        let rows = materialization_rows(events, aggregates, cep_matches);
        upload_dataset_rows(state, actor_id, dataset_id, rows).await?;
        for stream_id in &topology.source_stream_ids {
            sqlx::query(
                r#"INSERT INTO streaming_lineage_edges (id, source_stream_id, target_dataset_id, topology_id)
                   VALUES ($1, $2, $3, $4)
                   ON CONFLICT DO NOTHING"#,
            )
            .bind(Uuid::now_v7())
            .bind(stream_id)
            .bind(dataset_id)
            .bind(topology.id)
            .execute(&state.db)
            .await
            .map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

fn materialization_rows(
    events: &[ProcessedEvent],
    aggregates: &[WindowAggregate],
    cep_matches: &[CepMatch],
) -> Vec<Value> {
    if !aggregates.is_empty() {
        return aggregates
            .iter()
            .map(|aggregate| {
                json!({
                    "window_name": aggregate.window_name,
                    "window_type": aggregate.window_type,
                    "bucket_start": aggregate.bucket_start,
                    "bucket_end": aggregate.bucket_end,
                    "group_key": aggregate.group_key,
                    "measure_name": aggregate.measure_name,
                    "value": aggregate.value,
                })
            })
            .collect();
    }

    if !events.is_empty() {
        return events
            .iter()
            .map(|event| {
                let mut object = event.payload.as_object().cloned().unwrap_or_default();
                object.insert("stream_name".to_string(), json!(event.stream_name));
                object.insert("event_time".to_string(), json!(event.event_time));
                Value::Object(object)
            })
            .collect();
    }

    cep_matches
        .iter()
        .map(|item| {
            json!({
                "pattern_name": item.pattern_name,
                "matched_sequence": item.matched_sequence,
                "confidence": item.confidence,
                "detected_at": item.detected_at,
            })
        })
        .collect()
}

async fn persist_checkpoints(
    db: &sqlx::PgPool,
    topology_id: Uuid,
    events: &[ProcessedEvent],
) -> Result<(), String> {
    let mut max_offsets = HashMap::<Uuid, (i64, Uuid)>::new();
    for event in events {
        let entry = max_offsets
            .entry(event.stream_id)
            .or_insert((event.sequence_no, event.id));
        if event.sequence_no > entry.0 {
            *entry = (event.sequence_no, event.id);
        }
    }

    for (stream_id, (sequence_no, event_id)) in max_offsets {
        sqlx::query(
            r#"INSERT INTO streaming_checkpoints (
                   topology_id, stream_id, last_event_id, last_sequence_no, updated_at
               )
               VALUES ($1, $2, $3, $4, now())
               ON CONFLICT (topology_id, stream_id)
               DO UPDATE SET
                   last_event_id = EXCLUDED.last_event_id,
                   last_sequence_no = EXCLUDED.last_sequence_no,
                   updated_at = now()"#,
        )
        .bind(topology_id)
        .bind(stream_id)
        .bind(event_id)
        .bind(sequence_no)
        .execute(db)
        .await
        .map_err(|error| error.to_string())?;
    }

    Ok(())
}

async fn mark_events_processed(db: &sqlx::PgPool, events: &[ProcessedEvent]) -> Result<(), String> {
    for event in events {
        sqlx::query("UPDATE streaming_events SET processed_at = now() WHERE id = $1")
            .bind(event.id)
            .execute(db)
            .await
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

async fn archive_processed_events(
    state: &AppState,
    streams: &[StreamDefinition],
    events: &[ProcessedEvent],
) -> Result<(), String> {
    let stream_lookup = streams
        .iter()
        .map(|stream| (stream.id, stream))
        .collect::<HashMap<_, _>>();
    let now = Utc::now();
    let mut grouped = HashMap::<Uuid, Vec<&ProcessedEvent>>::new();

    for event in events {
        let Some(stream) = stream_lookup.get(&event.stream_id) else {
            continue;
        };
        let retention_cutoff = now - Duration::hours(stream.retention_hours.max(0) as i64);
        if event.event_time <= retention_cutoff {
            grouped.entry(event.stream_id).or_default().push(event);
        }
    }

    for (stream_id, stream_events) in grouped {
        let archive_dir = Path::new(&state.archive_dir).join(stream_id.to_string());
        tokio::fs::create_dir_all(&archive_dir)
            .await
            .map_err(|error| error.to_string())?;
        let archive_path = archive_dir.join(format!("{}.jsonl", Uuid::now_v7()));
        let archive_contents = stream_events
            .iter()
            .map(|event| {
                json!({
                    "stream_id": event.stream_id,
                    "stream_name": event.stream_name,
                    "event_time": event.event_time,
                    "payload": event.payload,
                    "sequence_no": event.sequence_no,
                })
                .to_string()
            })
            .collect::<Vec<_>>()
            .join("\n");
        tokio::fs::write(&archive_path, format!("{archive_contents}\n"))
            .await
            .map_err(|error| error.to_string())?;
        let archive_path_string = archive_path.to_string_lossy().to_string();

        for event in stream_events {
            sqlx::query(
                "UPDATE streaming_events SET archived_at = now(), archive_path = $2 WHERE id = $1",
            )
            .bind(event.id)
            .bind(&archive_path_string)
            .execute(&state.db)
            .await
            .map_err(|error| error.to_string())?;
        }
    }

    Ok(())
}

async fn upload_dataset_rows(
    state: &AppState,
    actor_id: Uuid,
    dataset_id: Uuid,
    rows: Vec<Value>,
) -> Result<(), String> {
    let token = issue_service_token(state, actor_id)?;
    let url = format!(
        "{}/api/v1/datasets/{dataset_id}/upload",
        state.dataset_service_url.trim_end_matches('/')
    );
    let bytes = serde_json::to_vec(&rows).map_err(|error| error.to_string())?;
    let part = Part::bytes(bytes)
        .file_name("streaming-materialization.json".to_string())
        .mime_str("application/json")
        .map_err(|error| error.to_string())?;
    let form = Form::new().part("file", part);

    let response = state
        .http_client
        .post(url)
        .bearer_auth(token)
        .multipart(form)
        .send()
        .await
        .map_err(|error| error.to_string())?;
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(format!(
            "dataset sink upload failed with HTTP {status}: {body}"
        ));
    }

    Ok(())
}

fn issue_service_token(state: &AppState, actor_id: Uuid) -> Result<String, String> {
    let claims = build_access_claims(
        &state.jwt_config,
        actor_id,
        "streaming@openfoundry.local",
        "OpenFoundry Streaming",
        vec!["admin".to_string()],
        vec!["*:*".to_string()],
        None,
        json!({ "source": "streaming_runtime" }),
        vec!["service_streaming".to_string()],
    );
    encode_token(&state.jwt_config, &claims).map_err(|error| error.to_string())
}

fn dataset_id_from_binding(binding: &ConnectorBinding) -> Option<Uuid> {
    binding
        .config
        .get("dataset_id")
        .and_then(Value::as_str)
        .and_then(|value| Uuid::parse_str(value).ok())
        .or_else(|| {
            binding
                .endpoint
                .strip_prefix("dataset://")
                .and_then(|value| Uuid::parse_str(value).ok())
        })
}

fn effective_output_count(events: &[ProcessedEvent], aggregates: &[WindowAggregate]) -> usize {
    if !aggregates.is_empty() {
        aggregates.len()
    } else {
        events.len()
    }
}

fn build_run_metrics(
    source_events: &[ProcessedEvent],
    materialization_events: &[ProcessedEvent],
    aggregate_windows: &[WindowAggregate],
    cep_matches: &[CepMatch],
    completed_at: DateTime<Utc>,
) -> TopologyRunMetrics {
    let input_events = source_events.len() as i32;
    let output_events = effective_output_count(materialization_events, aggregate_windows) as i32;
    let total_latency_ms = source_events
        .iter()
        .map(|event| (completed_at - event.event_time).num_milliseconds().max(0))
        .sum::<i64>();
    let avg_latency_ms = if input_events == 0 {
        0
    } else {
        (total_latency_ms / input_events as i64) as i32
    };
    let p95_latency_ms = source_events
        .iter()
        .map(|event| (completed_at - event.event_time).num_milliseconds().max(0) as i32)
        .max()
        .unwrap_or(0);
    let throughput_per_second = calculate_throughput_per_second(source_events);

    TopologyRunMetrics {
        input_events,
        output_events,
        avg_latency_ms,
        p95_latency_ms,
        throughput_per_second,
        dropped_events: 0,
        backpressure_ratio: 0.0,
        join_output_rows: materialization_events
            .iter()
            .filter(|event| event.payload.get("_join").is_some())
            .count() as i32,
        cep_match_count: cep_matches.len() as i32,
        state_entries: 0,
    }
}

fn build_preview_metrics(
    pending_events: &[ProcessedEvent],
    materialization_events: &[ProcessedEvent],
    aggregate_windows: &[WindowAggregate],
    cep_matches: &[CepMatch],
    recent_events: &[ProcessedEvent],
    backpressure_snapshot: &BackpressureSnapshot,
) -> TopologyRunMetrics {
    let latest_timestamp = pending_events
        .iter()
        .map(|event| event.processing_time)
        .max()
        .unwrap_or_else(Utc::now);
    let mut metrics = build_run_metrics(
        pending_events,
        materialization_events,
        aggregate_windows,
        cep_matches,
        latest_timestamp,
    );
    metrics.throughput_per_second = calculate_throughput_per_second(recent_events);
    metrics.backpressure_ratio = backpressure_ratio(backpressure_snapshot);
    metrics
}

fn build_state_snapshot(
    topology: &TopologyDefinition,
    key_count: i32,
    checkpoint_count: i32,
    last_checkpoint_at: DateTime<Utc>,
) -> StateStoreSnapshot {
    StateStoreSnapshot {
        backend: topology.state_backend.clone(),
        namespace: topology.name.to_lowercase().replace(' ', "-"),
        key_count,
        disk_usage_mb: (key_count.max(1) + checkpoint_count.max(1) * 2).max(1),
        checkpoint_count,
        last_checkpoint_at,
    }
}

fn group_event_count_by_stream(events: &[ProcessedEvent]) -> HashMap<Uuid, i32> {
    let mut counts = HashMap::new();
    for event in events {
        *counts.entry(event.stream_id).or_insert(0) += 1;
    }
    counts
}

fn group_throughput_by_stream(events: &[ProcessedEvent]) -> HashMap<Uuid, f32> {
    let mut grouped = HashMap::<Uuid, Vec<ProcessedEvent>>::new();
    for event in events {
        grouped
            .entry(event.stream_id)
            .or_default()
            .push(event.clone());
    }

    grouped
        .into_iter()
        .map(|(stream_id, items)| (stream_id, calculate_throughput_per_second(&items)))
        .collect()
}

fn calculate_throughput_per_second(events: &[ProcessedEvent]) -> f32 {
    if events.is_empty() {
        return 0.0;
    }
    if events.len() == 1 {
        return 1.0;
    }

    let mut ordered = events.iter().collect::<Vec<_>>();
    ordered.sort_by_key(|event| event.event_time);
    let first = ordered.first().copied().map(|event| event.event_time);
    let last = ordered.last().copied().map(|event| event.event_time);
    let elapsed_secs = match (first, last) {
        (Some(start), Some(end)) if end > start => (end - start)
            .to_std()
            .map(|duration| duration.as_secs_f32().max(1.0))
            .unwrap_or(1.0),
        _ => 60.0,
    };

    events.len() as f32 / elapsed_secs
}

fn backpressure_ratio(snapshot: &BackpressureSnapshot) -> f32 {
    if snapshot.queue_capacity <= 0 {
        0.0
    } else {
        snapshot.queue_depth as f32 / snapshot.queue_capacity as f32
    }
}

fn event_label(event: &ProcessedEvent) -> String {
    event
        .payload
        .get("status")
        .or_else(|| event.payload.get("event_type"))
        .or_else(|| event.payload.get("state"))
        .and_then(Value::as_str)
        .unwrap_or("event")
        .to_string()
}

fn bucket_start(event_time: DateTime<Utc>, duration_seconds: i64) -> DateTime<Utc> {
    let timestamp = event_time.timestamp();
    let bucket = timestamp - (timestamp.rem_euclid(duration_seconds));
    DateTime::<Utc>::from_timestamp(bucket, 0).unwrap_or(event_time)
}

fn stringify_json_value(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(inner) => inner.to_string(),
        Value::Number(inner) => inner.to_string(),
        Value::String(inner) => inner.clone(),
        Value::Array(_) | Value::Object(_) => serde_json::to_string(value).unwrap_or_default(),
    }
}
