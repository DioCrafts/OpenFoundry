use axum::{
    Json,
    extract::{Path, Query, State},
};
use chrono::{Duration, Utc};
use serde::Deserialize;

use crate::{
    AppState,
    domain::{alerting, collector, immutable_log},
    handlers::{
        ServiceResult, bad_request, db_error, internal_error, load_event_row, load_events,
        load_policies,
    },
    models::{
        audit_event::{AppendAuditEventRequest, AuditEvent, AuditOverview, EventListResponse},
        data_classification::AnomalyAlert,
        policy::CollectorStatus,
    },
};

#[derive(Debug, Deserialize)]
pub struct EventQuery {
    pub source_service: Option<String>,
    pub subject_id: Option<String>,
    pub classification: Option<String>,
}

pub async fn get_overview(State(state): State<AppState>) -> ServiceResult<AuditOverview> {
    let events = load_events(&state.db)
        .await
        .map_err(|cause| db_error(&cause))?;
    let policies = load_policies(&state.db)
        .await
        .map_err(|cause| db_error(&cause))?;
    let anomalies = alerting::detect_anomalies(&events);
    let collectors = collector::collector_catalog(&events);

    Ok(Json(AuditOverview {
        event_count: events.len() as i64,
        critical_event_count: events
            .iter()
            .filter(|event| event.severity.is_critical())
            .count() as i64,
        collector_count: collectors.len() as i64,
        active_policy_count: policies.iter().filter(|policy| policy.active).count() as i64,
        anomaly_count: anomalies.len() as i64,
        gdpr_subject_count: events
            .iter()
            .filter(|event| event.subject_id.is_some())
            .count() as i64,
        latest_event: events.first().cloned(),
    }))
}

pub async fn list_events(
    Query(query): Query<EventQuery>,
    State(state): State<AppState>,
) -> ServiceResult<EventListResponse> {
    let events = load_events(&state.db)
        .await
        .map_err(|cause| db_error(&cause))?;
    let items = events
        .into_iter()
        .filter(|event| {
            query
                .source_service
                .as_ref()
                .map(|value| value == &event.source_service)
                .unwrap_or(true)
        })
        .filter(|event| {
            query
                .subject_id
                .as_ref()
                .map(|value| event.subject_id.as_deref() == Some(value.as_str()))
                .unwrap_or(true)
        })
        .filter(|event| {
            query
                .classification
                .as_ref()
                .map(|value| event.classification.as_str() == value)
                .unwrap_or(true)
        })
        .collect::<Vec<_>>();
    Ok(Json(EventListResponse {
        items,
        anomalies: alerting::detect_anomalies(
            &load_events(&state.db)
                .await
                .map_err(|cause| db_error(&cause))?,
        ),
    }))
}

pub async fn get_event(
    Path(id): Path<uuid::Uuid>,
    State(state): State<AppState>,
) -> ServiceResult<AuditEvent> {
    let row = load_event_row(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?
        .ok_or_else(|| crate::handlers::not_found("audit event not found"))?;
    let event = AuditEvent::try_from(row).map_err(|cause| internal_error(cause.to_string()))?;
    Ok(Json(event))
}

pub async fn append_event(
    State(state): State<AppState>,
    Json(mut request): Json<AppendAuditEventRequest>,
) -> ServiceResult<AuditEvent> {
    if request.action.trim().is_empty() {
        return Err(bad_request("action is required"));
    }

    let event = persist_event(&state.db, &mut request)
        .await
        .map_err(|cause| internal_error(cause))?;
    Ok(Json(event))
}

pub async fn persist_event(
    db: &sqlx::PgPool,
    request: &mut AppendAuditEventRequest,
) -> Result<AuditEvent, String> {
    if request.action.trim().is_empty() {
        return Err("action is required".to_string());
    }

    let latest = sqlx::query_as::<_, (Option<i64>, Option<String>)>(
		"SELECT MAX(sequence) AS sequence, (ARRAY_AGG(entry_hash ORDER BY sequence DESC))[1] AS entry_hash FROM audit_events",
	)
	.fetch_one(db)
	.await
	.map_err(|cause| format!("failed to load latest audit sequence: {cause}"))?;

    let sequence = immutable_log::next_sequence(latest.0);
    let previous_hash = immutable_log::previous_hash_value(latest.1.as_deref());
    let entry_hash = immutable_log::chain_hash(
        sequence,
        &previous_hash,
        &request.source_service,
        &request.action,
    );
    let now = Utc::now();
    let id = uuid::Uuid::now_v7();
    request.labels.sort();
    request.labels.dedup();
    let labels = serde_json::to_value(&request.labels).map_err(|cause| cause.to_string())?;

    sqlx::query(
		"INSERT INTO audit_events (id, sequence, previous_hash, entry_hash, source_service, channel, actor, action, resource_type, resource_id, status, severity, classification, subject_id, ip_address, location, metadata, labels, retention_until, occurred_at, ingested_at)
		 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17::jsonb, $18::jsonb, $19, $20, $21)",
	)
	.bind(id)
	.bind(sequence)
	.bind(&previous_hash)
	.bind(&entry_hash)
	.bind(&request.source_service)
	.bind(&request.channel)
	.bind(&request.actor)
	.bind(&request.action)
	.bind(&request.resource_type)
	.bind(&request.resource_id)
	.bind(request.status.as_str())
	.bind(request.severity.as_str())
	.bind(request.classification.as_str())
	.bind(&request.subject_id)
	.bind(&request.ip_address)
	.bind(&request.location)
	.bind(request.metadata.clone())
	.bind(labels)
	.bind(now + Duration::days(request.retention_days as i64))
	.bind(now)
	.bind(now)
	.execute(db)
	.await
	.map_err(|cause| format!("failed to insert audit event: {cause}"))?;

    let row = load_event_row(db, id)
        .await
        .map_err(|cause| format!("failed to reload audit event: {cause}"))?
        .ok_or_else(|| "created audit event could not be reloaded".to_string())?;
    let mut event = AuditEvent::try_from(row).map_err(|cause| cause.to_string())?;
    event.labels = immutable_log::label_event(&event);
    Ok(event)
}

pub async fn list_collectors(State(state): State<AppState>) -> ServiceResult<Vec<CollectorStatus>> {
    let events = load_events(&state.db)
        .await
        .map_err(|cause| db_error(&cause))?;
    Ok(Json(collector::collector_catalog(&events)))
}

pub async fn list_anomalies(State(state): State<AppState>) -> ServiceResult<Vec<AnomalyAlert>> {
    let events = load_events(&state.db)
        .await
        .map_err(|cause| db_error(&cause))?;
    Ok(Json(alerting::detect_anomalies(&events)))
}
