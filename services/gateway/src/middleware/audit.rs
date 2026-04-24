use axum::{extract::Request, middleware::Next, response::Response};
use event_bus::{
    Publisher, subscriber,
    topics::{streams, subjects},
};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
struct GatewayAuditPayload {
    source_service: String,
    channel: String,
    actor: String,
    action: String,
    resource_type: String,
    resource_id: String,
    status: &'static str,
    severity: &'static str,
    classification: &'static str,
    subject_id: Option<String>,
    ip_address: Option<String>,
    location: Option<String>,
    metadata: GatewayAuditMetadata,
    labels: Vec<String>,
    retention_days: i32,
}

#[derive(Debug, Clone, Serialize)]
struct GatewayAuditMetadata {
    request_id: String,
    method: String,
    path: String,
    status: u16,
    user_agent: Option<String>,
}

pub async fn audit_layer(req: Request, next: Next) -> Response {
    let request_id = req
        .headers()
        .get("x-request-id")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("unknown")
        .to_string();
    let method = req.method().to_string();
    let path = req
        .uri()
        .path_and_query()
        .map(|value| value.as_str().to_string())
        .unwrap_or_else(|| req.uri().path().to_string());
    let user_agent = req
        .headers()
        .get(axum::http::header::USER_AGENT)
        .and_then(|value| value.to_str().ok())
        .map(ToString::to_string);

    let response = next.run(req).await;
    let status = response.status().as_u16();

    if let Ok(nats_url) = std::env::var("NATS_URL") {
        let (event_status, severity) = if status >= 500 {
            ("failure", "critical")
        } else if status >= 400 {
            ("failure", "high")
        } else {
            ("success", "low")
        };
        let payload = GatewayAuditPayload {
            source_service: "gateway".to_string(),
            channel: "nats".to_string(),
            actor: "system:gateway".to_string(),
            action: "request.forwarded".to_string(),
            resource_type: "http_request".to_string(),
            resource_id: path.clone(),
            status: event_status,
            severity,
            classification: "confidential",
            subject_id: None,
            ip_address: None,
            location: None,
            metadata: GatewayAuditMetadata {
                request_id,
                method,
                path,
                status,
                user_agent,
            },
            labels: vec!["auto-captured".to_string(), "gateway".to_string()],
            retention_days: 365,
        };

        tokio::spawn(async move {
            match event_bus::connect(&nats_url).await {
                Ok(js) => {
                    if let Err(cause) =
                        subscriber::ensure_stream(&js, streams::AUDIT, &[subjects::AUDIT]).await
                    {
                        tracing::warn!(?cause, "failed to ensure audit stream");
                        return;
                    }

                    let publisher = Publisher::new(js, "gateway");
                    let subject = format!("{}.gateway", subjects::AUDIT);
                    if let Err(cause) = publisher
                        .publish(&subject, "audit.gateway.request.forwarded", payload)
                        .await
                    {
                        tracing::warn!(?cause, "failed to publish gateway audit event");
                    }
                }
                Err(cause) => {
                    tracing::warn!(?cause, "failed to connect to NATS for audit publishing")
                }
            }
        });
    }

    response
}
