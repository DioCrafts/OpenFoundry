use axum::{Json, extract::Path};
use chrono::Utc;
use serde_json::Value;
use sqlx::types::Json as SqlJson;
use uuid::Uuid;

use crate::{
    AppState,
    handlers::{ServiceResult, bad_request, db_error, not_found},
    models::{
        ListResponse,
        dead_letter::{
            ReplayDeadLetterRequest, ReplayDeadLetterResponse, StreamingDeadLetter,
            StreamingDeadLetterRow,
        },
        stream::{
            ConnectorBinding, CreateStreamRequest, PushStreamEventsRequest,
            PushStreamEventsResponse, StreamDefinition, StreamRow, StreamSchema,
            UpdateStreamRequest,
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

async fn load_dead_letter_row(
    db: &sqlx::PgPool,
    id: Uuid,
) -> Result<StreamingDeadLetterRow, sqlx::Error> {
    sqlx::query_as::<_, StreamingDeadLetterRow>(
        "SELECT id, stream_id, payload, event_time, reason, validation_errors, status,
                replay_count, last_replayed_at, created_at, updated_at
         FROM streaming_dead_letters
         WHERE id = $1",
    )
    .bind(id)
    .fetch_one(db)
    .await
}

fn field_present(payload: &serde_json::Map<String, Value>, name: &str) -> bool {
    payload.get(name).is_some_and(|value| !value.is_null())
}

fn field_type_matches(value: &Value, data_type: &str) -> bool {
    match data_type {
        "string" => value.is_string(),
        "integer" | "int64" => value.as_i64().is_some(),
        "float" | "float64" => value.as_f64().is_some(),
        "boolean" => value.is_boolean(),
        "timestamp" => value
            .as_str()
            .and_then(|text| chrono::DateTime::parse_from_rfc3339(text).ok())
            .is_some(),
        "array" | "list" => value.is_array(),
        "json" | "struct" | "object" => value.is_object(),
        _ => true,
    }
}

fn validate_event_against_schema(
    schema: &StreamSchema,
    payload: &Value,
    event_time: chrono::DateTime<Utc>,
) -> Result<(), Vec<String>> {
    let Some(object) = payload.as_object() else {
        return Err(vec![
            "stream event payload must be a JSON object".to_string(),
        ]);
    };

    let mut errors = Vec::new();
    for field in &schema.fields {
        let is_watermark = schema.watermark_field.as_deref() == Some(field.name.as_str())
            || field.semantic_role == "event_time";
        if !field.nullable && !field_present(object, &field.name) && !is_watermark {
            errors.push(format!("missing required field '{}'", field.name));
            continue;
        }
        if let Some(value) = object.get(&field.name) {
            if value.is_null() && !field.nullable && !is_watermark {
                errors.push(format!("field '{}' cannot be null", field.name));
                continue;
            }
            if !value.is_null() && !field_type_matches(value, &field.data_type) {
                errors.push(format!(
                    "field '{}' does not match declared type '{}'",
                    field.name, field.data_type
                ));
            }
        } else if is_watermark && field.data_type == "timestamp" {
            let _ = event_time;
        }
    }

    if let Some(watermark_field) = schema.watermark_field.as_deref() {
        if let Some(value) = object.get(watermark_field) {
            if value
                .as_str()
                .and_then(|text| chrono::DateTime::parse_from_rfc3339(text).ok())
                .is_none()
            {
                errors.push(format!(
                    "watermark field '{}' must be an RFC3339 timestamp",
                    watermark_field
                ));
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

async fn insert_dead_letter(
    db: &sqlx::PgPool,
    stream_id: Uuid,
    payload: Value,
    event_time: chrono::DateTime<Utc>,
    reason: &str,
    validation_errors: Vec<String>,
) -> Result<Uuid, sqlx::Error> {
    let id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO streaming_dead_letters (
             id, stream_id, payload, event_time, reason, validation_errors, status
         ) VALUES ($1, $2, $3, $4, $5, $6, 'queued')",
    )
    .bind(id)
    .bind(stream_id)
    .bind(payload)
    .bind(event_time)
    .bind(reason)
    .bind(SqlJson(validation_errors))
    .execute(db)
    .await?;
    Ok(id)
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

    let stream = match load_stream_row(&state.db, stream_id).await {
        Ok(stream) => stream,
        Err(sqlx::Error::RowNotFound) => return Err(not_found("stream not found")),
        Err(cause) => return Err(db_error(&cause)),
    };

    let mut first_sequence_no = None;
    let mut last_sequence_no = None;
    let mut accepted_events = 0usize;
    let mut dead_lettered_events = 0usize;
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

        if let Err(validation_errors) =
            validate_event_against_schema(&stream.schema.0, &event.payload, event_time)
        {
            insert_dead_letter(
                &state.db,
                stream_id,
                event.payload,
                event_time,
                "schema_validation_failed",
                validation_errors,
            )
            .await
            .map_err(|cause| db_error(&cause))?;
            dead_lettered_events += 1;
            continue;
        }

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

        accepted_events += 1;
        if first_sequence_no.is_none() {
            first_sequence_no = Some(sequence_no);
        }
        last_sequence_no = Some(sequence_no);
    }

    Ok(Json(PushStreamEventsResponse {
        stream_id,
        accepted_events,
        dead_lettered_events,
        first_sequence_no,
        last_sequence_no,
    }))
}

pub async fn list_dead_letters(
    axum::extract::State(state): axum::extract::State<AppState>,
    Path(stream_id): Path<Uuid>,
) -> ServiceResult<ListResponse<StreamingDeadLetter>> {
    match load_stream_row(&state.db, stream_id).await {
        Ok(_) => {}
        Err(sqlx::Error::RowNotFound) => return Err(not_found("stream not found")),
        Err(cause) => return Err(db_error(&cause)),
    }

    let rows = sqlx::query_as::<_, StreamingDeadLetterRow>(
        "SELECT id, stream_id, payload, event_time, reason, validation_errors, status,
                replay_count, last_replayed_at, created_at, updated_at
         FROM streaming_dead_letters
         WHERE stream_id = $1
         ORDER BY created_at DESC",
    )
    .bind(stream_id)
    .fetch_all(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?;

    Ok(Json(ListResponse {
        data: rows.into_iter().map(Into::into).collect(),
    }))
}

pub async fn replay_dead_letter(
    axum::extract::State(state): axum::extract::State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<ReplayDeadLetterRequest>,
) -> ServiceResult<ReplayDeadLetterResponse> {
    let dead_letter = match load_dead_letter_row(&state.db, id).await {
        Ok(row) => row,
        Err(sqlx::Error::RowNotFound) => return Err(not_found("dead letter not found")),
        Err(cause) => return Err(db_error(&cause)),
    };
    let stream = match load_stream_row(&state.db, dead_letter.stream_id).await {
        Ok(stream) => stream,
        Err(sqlx::Error::RowNotFound) => return Err(not_found("stream not found")),
        Err(cause) => return Err(db_error(&cause)),
    };

    let next_payload = payload.payload.unwrap_or(dead_letter.payload.clone());
    let next_event_time = payload.event_time.unwrap_or(dead_letter.event_time);
    if let Err(errors) =
        validate_event_against_schema(&stream.schema.0, &next_payload, next_event_time)
    {
        return Err(bad_request(format!(
            "dead letter replay still violates schema: {}",
            errors.join("; ")
        )));
    }

    let sequence_no = sqlx::query_scalar::<_, i64>(
        r#"INSERT INTO streaming_events (id, stream_id, payload, event_time)
           VALUES ($1, $2, $3, $4)
           RETURNING sequence_no"#,
    )
    .bind(Uuid::now_v7())
    .bind(dead_letter.stream_id)
    .bind(SqlJson(next_payload.clone()))
    .bind(next_event_time)
    .fetch_one(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?;

    sqlx::query(
        "UPDATE streaming_dead_letters
         SET payload = $2,
             event_time = $3,
             status = 'replayed',
             replay_count = replay_count + 1,
             last_replayed_at = now(),
             validation_errors = '[]'::jsonb,
             updated_at = now()
         WHERE id = $1",
    )
    .bind(id)
    .bind(next_payload)
    .bind(next_event_time)
    .execute(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?;

    let dead_letter = load_dead_letter_row(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?;

    Ok(Json(ReplayDeadLetterResponse {
        dead_letter: dead_letter.into(),
        replay_sequence_no: sequence_no,
    }))
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use serde_json::json;

    use crate::models::stream::{StreamField, StreamSchema};

    use super::validate_event_against_schema;

    fn sample_schema() -> StreamSchema {
        StreamSchema {
            fields: vec![
                StreamField {
                    name: "event_time".to_string(),
                    data_type: "timestamp".to_string(),
                    nullable: false,
                    semantic_role: "event_time".to_string(),
                },
                StreamField {
                    name: "customer_id".to_string(),
                    data_type: "string".to_string(),
                    nullable: false,
                    semantic_role: "join_key".to_string(),
                },
                StreamField {
                    name: "amount".to_string(),
                    data_type: "float".to_string(),
                    nullable: false,
                    semantic_role: "measure".to_string(),
                },
            ],
            primary_key: Some("customer_id".to_string()),
            watermark_field: Some("event_time".to_string()),
        }
    }

    #[test]
    fn rejects_invalid_stream_payloads_into_dlq_path() {
        let result = validate_event_against_schema(
            &sample_schema(),
            &json!({
                "customer_id": 42,
                "amount": "high"
            }),
            Utc::now(),
        );

        let errors = result.expect_err("payload should fail schema validation");
        assert!(errors.iter().any(|error| error.contains("customer_id")));
        assert!(errors.iter().any(|error| error.contains("amount")));
    }

    #[test]
    fn accepts_valid_payloads_with_rfc3339_watermark() {
        validate_event_against_schema(
            &sample_schema(),
            &json!({
                "event_time": "2026-04-25T10:15:00Z",
                "customer_id": "C-100",
                "amount": 87.5
            }),
            Utc::now(),
        )
        .expect("payload should pass schema validation");
    }
}
