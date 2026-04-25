use axum::{Json, extract::Path};
use chrono::Utc;
use sqlx::types::Json as SqlJson;
use uuid::Uuid;

use crate::{
    AppState,
    handlers::{ServiceResult, bad_request, db_error, not_found},
    models::{
        ListResponse,
        stream::{
            ConnectorBinding, CreateStreamRequest, StreamDefinition, StreamRow, StreamSchema,
            PushStreamEventsRequest, PushStreamEventsResponse, UpdateStreamRequest,
        },
        window::{CreateWindowRequest, UpdateWindowRequest, WindowDefinition, WindowRow},
    },
};

async fn load_stream_row(db: &sqlx::PgPool, id: Uuid) -> Result<StreamRow, sqlx::Error> {
    sqlx::query_as::<_, StreamRow>(
		"SELECT id, name, description, status, schema, source_binding, retention_hours, created_at, updated_at
		 FROM streaming_streams
		 WHERE id = $1",
	)
	.bind(id)
	.fetch_one(db)
	.await
}

async fn load_window_row(db: &sqlx::PgPool, id: Uuid) -> Result<WindowRow, sqlx::Error> {
    sqlx::query_as::<_, WindowRow>(
        "SELECT id, name, description, status, window_type, duration_seconds, slide_seconds,
		        session_gap_seconds, allowed_lateness_seconds, aggregation_keys, measure_fields,
		        created_at, updated_at
		 FROM streaming_windows
		 WHERE id = $1",
    )
    .bind(id)
    .fetch_one(db)
    .await
}

pub async fn list_streams(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> ServiceResult<ListResponse<StreamDefinition>> {
    let rows = sqlx::query_as::<_, StreamRow>(
		"SELECT id, name, description, status, schema, source_binding, retention_hours, created_at, updated_at
		 FROM streaming_streams
		 ORDER BY created_at ASC",
	)
	.fetch_all(&state.db)
	.await
	.map_err(|cause| db_error(&cause))?;

    Ok(Json(ListResponse {
        data: rows.into_iter().map(Into::into).collect(),
    }))
}

pub async fn create_stream(
    axum::extract::State(state): axum::extract::State<AppState>,
    Json(payload): Json<CreateStreamRequest>,
) -> ServiceResult<StreamDefinition> {
    if payload.name.trim().is_empty() {
        return Err(bad_request("stream name is required"));
    }

    let stream_id = Uuid::now_v7();
    let schema = payload.schema.unwrap_or_else(StreamSchema::default);
    let binding = payload
        .source_binding
        .unwrap_or_else(ConnectorBinding::default);

    sqlx::query(
		"INSERT INTO streaming_streams (id, name, description, status, schema, source_binding, retention_hours)
		 VALUES ($1, $2, $3, $4, $5, $6, $7)",
	)
	.bind(stream_id)
	.bind(payload.name.trim())
	.bind(payload.description.unwrap_or_default())
	.bind(payload.status.unwrap_or_else(|| "active".to_string()))
	.bind(SqlJson(schema))
	.bind(SqlJson(binding))
	.bind(payload.retention_hours.unwrap_or(72))
	.execute(&state.db)
	.await
	.map_err(|cause| db_error(&cause))?;

    let row = load_stream_row(&state.db, stream_id)
        .await
        .map_err(|cause| db_error(&cause))?;

    Ok(Json(row.into()))
}

pub async fn update_stream(
    axum::extract::State(state): axum::extract::State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateStreamRequest>,
) -> ServiceResult<StreamDefinition> {
    let existing = match load_stream_row(&state.db, id).await {
        Ok(row) => row,
        Err(sqlx::Error::RowNotFound) => return Err(not_found("stream not found")),
        Err(cause) => return Err(db_error(&cause)),
    };

    let schema = payload.schema.unwrap_or(existing.schema.0);
    let binding = payload.source_binding.unwrap_or(existing.source_binding.0);

    sqlx::query(
        "UPDATE streaming_streams
		 SET name = $2,
		     description = $3,
		     status = $4,
		     schema = $5,
		     source_binding = $6,
		     retention_hours = $7,
		     updated_at = now()
		 WHERE id = $1",
    )
    .bind(id)
    .bind(payload.name.unwrap_or(existing.name))
    .bind(payload.description.unwrap_or(existing.description))
    .bind(payload.status.unwrap_or(existing.status))
    .bind(SqlJson(schema))
    .bind(SqlJson(binding))
    .bind(payload.retention_hours.unwrap_or(existing.retention_hours))
    .execute(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?;

    let row = load_stream_row(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?;

    Ok(Json(row.into()))
}

pub async fn list_windows(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> ServiceResult<ListResponse<WindowDefinition>> {
    let rows = sqlx::query_as::<_, WindowRow>(
        "SELECT id, name, description, status, window_type, duration_seconds, slide_seconds,
		        session_gap_seconds, allowed_lateness_seconds, aggregation_keys, measure_fields,
		        created_at, updated_at
		 FROM streaming_windows
		 ORDER BY created_at ASC",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?;

    Ok(Json(ListResponse {
        data: rows.into_iter().map(Into::into).collect(),
    }))
}

pub async fn create_window(
    axum::extract::State(state): axum::extract::State<AppState>,
    Json(payload): Json<CreateWindowRequest>,
) -> ServiceResult<WindowDefinition> {
    if payload.name.trim().is_empty() {
        return Err(bad_request("window name is required"));
    }

    let window_id = Uuid::now_v7();

    sqlx::query(
        "INSERT INTO streaming_windows (
		    id, name, description, status, window_type, duration_seconds, slide_seconds,
		    session_gap_seconds, allowed_lateness_seconds, aggregation_keys, measure_fields
		 ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
    )
    .bind(window_id)
    .bind(payload.name.trim())
    .bind(payload.description.unwrap_or_default())
    .bind(payload.status.unwrap_or_else(|| "active".to_string()))
    .bind(
        payload
            .window_type
            .unwrap_or_else(|| "tumbling".to_string()),
    )
    .bind(payload.duration_seconds.unwrap_or(300))
    .bind(payload.slide_seconds.unwrap_or(300))
    .bind(payload.session_gap_seconds.unwrap_or(180))
    .bind(payload.allowed_lateness_seconds.unwrap_or(30))
    .bind(SqlJson(payload.aggregation_keys))
    .bind(SqlJson(payload.measure_fields))
    .execute(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?;

    let row = load_window_row(&state.db, window_id)
        .await
        .map_err(|cause| db_error(&cause))?;

    Ok(Json(row.into()))
}

pub async fn update_window(
    axum::extract::State(state): axum::extract::State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateWindowRequest>,
) -> ServiceResult<WindowDefinition> {
    let existing = match load_window_row(&state.db, id).await {
        Ok(row) => row,
        Err(sqlx::Error::RowNotFound) => return Err(not_found("window not found")),
        Err(cause) => return Err(db_error(&cause)),
    };

    sqlx::query(
        "UPDATE streaming_windows
		 SET name = $2,
		     description = $3,
		     status = $4,
		     window_type = $5,
		     duration_seconds = $6,
		     slide_seconds = $7,
		     session_gap_seconds = $8,
		     allowed_lateness_seconds = $9,
		     aggregation_keys = $10,
		     measure_fields = $11,
		     updated_at = now()
		 WHERE id = $1",
    )
    .bind(id)
    .bind(payload.name.unwrap_or(existing.name))
    .bind(payload.description.unwrap_or(existing.description))
    .bind(payload.status.unwrap_or(existing.status))
    .bind(payload.window_type.unwrap_or(existing.window_type))
    .bind(
        payload
            .duration_seconds
            .unwrap_or(existing.duration_seconds),
    )
    .bind(payload.slide_seconds.unwrap_or(existing.slide_seconds))
    .bind(
        payload
            .session_gap_seconds
            .unwrap_or(existing.session_gap_seconds),
    )
    .bind(
        payload
            .allowed_lateness_seconds
            .unwrap_or(existing.allowed_lateness_seconds),
    )
    .bind(SqlJson(
        payload
            .aggregation_keys
            .unwrap_or(existing.aggregation_keys.0),
    ))
    .bind(SqlJson(
        payload.measure_fields.unwrap_or(existing.measure_fields.0),
    ))
    .execute(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?;

    let row = load_window_row(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?;

    Ok(Json(row.into()))
}

pub async fn push_events(
    axum::extract::State(state): axum::extract::State<AppState>,
    Path(stream_id): Path<Uuid>,
    Json(payload): Json<PushStreamEventsRequest>,
) -> ServiceResult<PushStreamEventsResponse> {
    let PushStreamEventsRequest { events } = payload;

    if events.is_empty() {
        return Err(bad_request("at least one event is required"));
    }

    match load_stream_row(&state.db, stream_id).await {
        Ok(_) => {}
        Err(sqlx::Error::RowNotFound) => return Err(not_found("stream not found")),
        Err(cause) => return Err(db_error(&cause)),
    }

    let mut first_sequence_no = None;
    let mut last_sequence_no = None;
    let accepted_events = events.len();
    for event in events {
        let event_time = event
            .event_time
            .or_else(|| {
                event
                    .payload
                    .get("event_time")
                    .and_then(|value| value.as_str())
                    .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
                    .map(|value| value.with_timezone(&Utc))
            })
            .unwrap_or_else(Utc::now);

        let sequence_no = sqlx::query_scalar::<_, i64>(
            r#"INSERT INTO streaming_events (id, stream_id, payload, event_time)
               VALUES ($1, $2, $3, $4)
               RETURNING sequence_no"#,
        )
        .bind(Uuid::now_v7())
        .bind(stream_id)
        .bind(SqlJson(event.payload))
        .bind(event_time)
        .fetch_one(&state.db)
        .await
        .map_err(|cause| db_error(&cause))?;

        if first_sequence_no.is_none() {
            first_sequence_no = Some(sequence_no);
        }
        last_sequence_no = Some(sequence_no);
    }

    Ok(Json(PushStreamEventsResponse {
        stream_id,
        accepted_events,
        first_sequence_no,
        last_sequence_no,
    }))
}
