use bytes::Bytes;
use chrono::Utc;
use lettre::{AsyncTransport, Message, message::Mailbox};
use serde_json::{Value, json};
use tokio::time::timeout;
use uuid::Uuid;

use crate::AppState;
use crate::models::{
    recipient::{
        DistributionChannel, DistributionChannelCatalogEntry, DistributionRecipient,
        DistributionResult,
    },
    report::ReportDefinition,
    snapshot::{ReportArtifact, ReportExecutionMetrics, ReportExecutionPreview},
};

pub fn catalog() -> Vec<DistributionChannelCatalogEntry> {
    vec![
        DistributionChannelCatalogEntry {
            channel: DistributionChannel::Email,
            display_name: "Email digest".to_string(),
            description: "SMTP or cloud email delivery for recurring reports.".to_string(),
            configuration_fields: vec!["address".to_string(), "subject".to_string()],
        },
        DistributionChannelCatalogEntry {
            channel: DistributionChannel::S3,
            display_name: "S3 archive".to_string(),
            description: "Persist generated artifacts into object storage for downstream sharing."
                .to_string(),
            configuration_fields: vec!["bucket".to_string(), "prefix".to_string()],
        },
        DistributionChannelCatalogEntry {
            channel: DistributionChannel::Slack,
            display_name: "Slack push".to_string(),
            description: "Post report summaries with download links into channels.".to_string(),
            configuration_fields: vec!["channel".to_string(), "webhook".to_string()],
        },
        DistributionChannelCatalogEntry {
            channel: DistributionChannel::Teams,
            display_name: "Teams push".to_string(),
            description: "Deliver report summaries into Microsoft Teams channels.".to_string(),
            configuration_fields: vec!["channel".to_string(), "webhook".to_string()],
        },
        DistributionChannelCatalogEntry {
            channel: DistributionChannel::Webhook,
            display_name: "Webhook callback".to_string(),
            description: "Notify external systems after generation completes.".to_string(),
            configuration_fields: vec!["url".to_string(), "secret".to_string()],
        },
    ]
}

pub async fn deliver_report(
    state: &AppState,
    report: &ReportDefinition,
    execution_id: Uuid,
    generated_at: chrono::DateTime<Utc>,
    preview: &ReportExecutionPreview,
    artifact: &ReportArtifact,
    metrics: &ReportExecutionMetrics,
) -> Vec<DistributionResult> {
    let mut results = Vec::with_capacity(report.recipients.len());
    let context = DeliveryContext {
        execution_id,
        generated_at,
        preview,
        artifact,
        metrics,
    };

    for recipient in &report.recipients {
        results.push(deliver_recipient(state, report, recipient, &context).await);
    }

    results
}

struct DeliveryContext<'a> {
    execution_id: Uuid,
    generated_at: chrono::DateTime<Utc>,
    preview: &'a ReportExecutionPreview,
    artifact: &'a ReportArtifact,
    metrics: &'a ReportExecutionMetrics,
}

async fn deliver_recipient(
    state: &AppState,
    report: &ReportDefinition,
    recipient: &DistributionRecipient,
    context: &DeliveryContext<'_>,
) -> DistributionResult {
    let idempotency_key = idempotency_key(report, context.execution_id, recipient);
    let summary = render_summary(report, context);

    match recipient.channel {
        DistributionChannel::Email => {
            deliver_email(
                state,
                report,
                recipient,
                context,
                &summary,
                &idempotency_key,
            )
            .await
        }
        DistributionChannel::S3 => {
            deliver_object_store(state, report, recipient, context, &idempotency_key).await
        }
        DistributionChannel::Slack => {
            let payload = json!({
                "text": summary,
                "channel": string_field(&recipient.config, "channel").or(recipient.label.clone()),
                "attachments": [{
                    "text": context.artifact.storage_url,
                    "fallback": context.preview.headline,
                }],
            });
            deliver_webhook(
                state,
                report,
                recipient,
                webhook_target(recipient, "webhook"),
                payload,
                Some(("x-openfoundry-channel", "slack")),
                &idempotency_key,
            )
            .await
        }
        DistributionChannel::Teams => {
            let payload = json!({
                "@type": "MessageCard",
                "@context": "http://schema.org/extensions",
                "summary": context.preview.headline,
                "title": report.name,
                "text": summary,
                "potentialAction": [{
                    "@type": "OpenUri",
                    "name": "Open artifact",
                    "targets": [{ "os": "default", "uri": context.artifact.storage_url }]
                }]
            });
            deliver_webhook(
                state,
                report,
                recipient,
                webhook_target(recipient, "webhook"),
                payload,
                Some(("x-openfoundry-channel", "teams")),
                &idempotency_key,
            )
            .await
        }
        DistributionChannel::Webhook => {
            let payload = json!({
                "report_id": report.id,
                "report_name": report.name,
                "execution_id": context.execution_id,
                "generated_at": context.generated_at,
                "preview": context.preview,
                "artifact": context.artifact,
                "metrics": context.metrics,
                "recipient": recipient,
            });
            deliver_webhook(
                state,
                report,
                recipient,
                webhook_target(recipient, "url"),
                payload,
                None,
                &idempotency_key,
            )
            .await
        }
    }
}

async fn deliver_email(
    state: &AppState,
    report: &ReportDefinition,
    recipient: &DistributionRecipient,
    context: &DeliveryContext<'_>,
    summary: &str,
    idempotency_key: &str,
) -> DistributionResult {
    let Some(sender) = state.email_sender.as_ref() else {
        return skipped_result(recipient, "SMTP adapter not configured for report delivery");
    };
    let Some(from) = state.email_from.as_ref() else {
        return skipped_result(recipient, "SMTP from address not configured");
    };

    let address =
        string_field(&recipient.config, "address").unwrap_or_else(|| recipient.target.clone());
    let to: Mailbox = match address.parse() {
        Ok(mailbox) => mailbox,
        Err(error) => return failed_result(recipient, format!("invalid email recipient: {error}")),
    };
    let subject = string_field(&recipient.config, "subject")
        .unwrap_or_else(|| format!("OpenFoundry report: {}", report.name));
    let body = format!(
        "{summary}\n\nArtifact: {}\nExecution: {}\nDelivery key: {idempotency_key}\n",
        context.artifact.storage_url, context.execution_id
    );

    let mut last_error = String::new();
    for attempt in 0..=state.report_delivery_max_retries {
        let message = match Message::builder()
            .from(from.clone())
            .to(to.clone())
            .subject(subject.clone())
            .body(body.clone())
        {
            Ok(message) => message,
            Err(error) => return failed_result(recipient, error.to_string()),
        };

        match timeout(state.report_delivery_timeout, sender.send(message)).await {
            Ok(Ok(_)) => {
                return delivered_result(
                    recipient,
                    format!(
                        "email delivered to {} after {} attempt(s) with key {}",
                        address,
                        attempt + 1,
                        idempotency_key
                    ),
                );
            }
            Ok(Err(error)) => last_error = error.to_string(),
            Err(_) => last_error = "email delivery timed out".to_string(),
        }
    }

    failed_result(recipient, last_error)
}

async fn deliver_object_store(
    state: &AppState,
    report: &ReportDefinition,
    recipient: &DistributionRecipient,
    context: &DeliveryContext<'_>,
    idempotency_key: &str,
) -> DistributionResult {
    let Some(store) = state.object_store.as_ref() else {
        return skipped_result(
            recipient,
            "object-store adapter not configured for report delivery",
        );
    };

    let object_key = object_key(report, recipient, context.execution_id);
    let payload = json!({
        "report_id": report.id,
        "report_name": report.name,
        "execution_id": context.execution_id,
        "generated_at": context.generated_at,
        "preview": context.preview,
        "artifact": context.artifact,
        "metrics": context.metrics,
        "delivery_key": idempotency_key,
        "channel": recipient.channel.as_str(),
    });
    let bytes = match serde_json::to_vec_pretty(&payload) {
        Ok(bytes) => Bytes::from(bytes),
        Err(error) => return failed_result(recipient, error.to_string()),
    };

    let mut last_error = String::new();
    for _attempt in 0..=state.report_delivery_max_retries {
        match timeout(
            state.report_delivery_timeout,
            store.put(&object_key, bytes.clone()),
        )
        .await
        {
            Ok(Ok(())) => {
                return delivered_result(
                    recipient,
                    format!(
                        "stored report delivery manifest in {} object storage at {}",
                        state.object_store_kind, object_key
                    ),
                );
            }
            Ok(Err(error)) => last_error = error.to_string(),
            Err(_) => last_error = "object-store delivery timed out".to_string(),
        }
    }

    failed_result(recipient, last_error)
}

async fn deliver_webhook(
    state: &AppState,
    report: &ReportDefinition,
    recipient: &DistributionRecipient,
    target_url: Option<String>,
    payload: Value,
    extra_header: Option<(&str, &str)>,
    idempotency_key: &str,
) -> DistributionResult {
    let Some(url) = target_url else {
        return failed_result(recipient, "missing distribution target URL".to_string());
    };

    let secret = string_field(&recipient.config, "secret");
    let mut last_error = String::new();

    for attempt in 0..=state.report_delivery_max_retries {
        let mut request = state
            .http_client
            .post(url.clone())
            .header("x-openfoundry-idempotency-key", idempotency_key)
            .header("x-openfoundry-report-id", report.id.to_string())
            .header("x-openfoundry-delivery-attempt", (attempt + 1).to_string())
            .json(&payload);
        if let Some(secret) = secret.as_deref() {
            request = request.header("x-openfoundry-secret", secret);
        }
        if let Some((name, value)) = extra_header {
            request = request.header(name, value);
        }

        match timeout(state.report_delivery_timeout, request.send()).await {
            Ok(Ok(response)) if response.status().is_success() => {
                return delivered_result(
                    recipient,
                    format!(
                        "delivered {} payload to {} with status {} after {} attempt(s)",
                        recipient.channel.as_str(),
                        url,
                        response.status(),
                        attempt + 1
                    ),
                );
            }
            Ok(Ok(response)) => {
                last_error = format!("delivery returned status {}", response.status());
            }
            Ok(Err(error)) => last_error = error.to_string(),
            Err(_) => last_error = "delivery timed out".to_string(),
        }
    }

    failed_result(recipient, last_error)
}

fn idempotency_key(
    report: &ReportDefinition,
    execution_id: Uuid,
    recipient: &DistributionRecipient,
) -> String {
    format!(
        "report:{}:execution:{}:recipient:{}",
        report.id, execution_id, recipient.id
    )
}

fn render_summary(report: &ReportDefinition, context: &DeliveryContext<'_>) -> String {
    format!(
        "{}\n{}\nRows: {} • Sections: {} • Recipients: {}\nArtifact: {}",
        context.preview.headline,
        report.description,
        context.metrics.row_count,
        context.metrics.section_count,
        context.metrics.recipient_count,
        context.artifact.storage_url
    )
}

fn webhook_target(recipient: &DistributionRecipient, config_field: &str) -> Option<String> {
    string_field(&recipient.config, config_field).or_else(|| {
        recipient
            .target
            .starts_with("http://")
            .then(|| recipient.target.clone())
            .or_else(|| {
                recipient
                    .target
                    .starts_with("https://")
                    .then(|| recipient.target.clone())
            })
    })
}

fn string_field(value: &Value, field: &str) -> Option<String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn object_key(
    report: &ReportDefinition,
    recipient: &DistributionRecipient,
    execution_id: Uuid,
) -> String {
    let prefix = string_field(&recipient.config, "prefix")
        .unwrap_or_else(|| "reports/distributions".to_string())
        .trim_matches('/')
        .to_string();
    let report_slug = report
        .name
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>();

    format!(
        "{}/{}/{}/delivery-manifest.json",
        prefix,
        report_slug.trim_matches('-'),
        execution_id
    )
}

fn delivered_result(recipient: &DistributionRecipient, detail: String) -> DistributionResult {
    DistributionResult {
        channel: recipient.channel,
        target: recipient.target.clone(),
        status: "delivered".to_string(),
        delivered_at: Utc::now(),
        detail,
    }
}

fn failed_result(
    recipient: &DistributionRecipient,
    detail: impl Into<String>,
) -> DistributionResult {
    DistributionResult {
        channel: recipient.channel,
        target: recipient.target.clone(),
        status: "failed".to_string(),
        delivered_at: Utc::now(),
        detail: detail.into(),
    }
}

fn skipped_result(
    recipient: &DistributionRecipient,
    detail: impl Into<String>,
) -> DistributionResult {
    DistributionResult {
        channel: recipient.channel,
        target: recipient.target.clone(),
        status: "skipped".to_string(),
        delivered_at: Utc::now(),
        detail: detail.into(),
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, sync::Arc, time::Duration};

    use auth_middleware::jwt::JwtConfig;
    use chrono::Utc;
    use serde_json::json;
    use storage_abstraction::local::LocalStorage;
    use uuid::Uuid;

    use crate::{
        AppState,
        models::{
            recipient::{DistributionChannel, DistributionRecipient},
            report::{GeneratorKind, ReportDefinition},
            schedule::ReportSchedule,
            snapshot::{ReportArtifact, ReportExecutionMetrics, ReportExecutionPreview},
            template::ReportTemplate,
        },
    };

    use super::{catalog, deliver_report, object_key};

    #[tokio::test]
    async fn stores_delivery_manifest_in_local_object_store() {
        let root =
            std::env::temp_dir().join(format!("openfoundry-report-delivery-{}", Uuid::now_v7()));
        fs::create_dir_all(&root).expect("root");
        let store = Arc::new(LocalStorage::new(root.to_str().expect("path")).expect("store"));
        let state = AppState {
            db: sqlx::PgPool::connect_lazy("postgres://test:test@localhost/test").expect("pool"),
            jwt_config: JwtConfig::new("test"),
            http_client: reqwest::Client::new(),
            dataset_service_url: "http://localhost:50053".to_string(),
            geospatial_service_url: "http://localhost:50068".to_string(),
            email_sender: None,
            email_from: None,
            object_store: Some(store),
            object_store_kind: "local".to_string(),
            report_delivery_timeout: Duration::from_secs(2),
            report_delivery_max_retries: 0,
        };
        let report = sample_report(vec![DistributionRecipient {
            id: "archive".to_string(),
            channel: DistributionChannel::S3,
            target: "archive".to_string(),
            label: Some("Archive".to_string()),
            config: json!({ "prefix": "exports/daily" }),
        }]);
        let execution_id = Uuid::now_v7();
        let preview = sample_preview();
        let artifact = sample_artifact();
        let metrics = sample_metrics();

        let results = deliver_report(
            &state,
            &report,
            execution_id,
            Utc::now(),
            &preview,
            &artifact,
            &metrics,
        )
        .await;

        assert_eq!(results[0].status, "delivered");
        let expected = root.join(object_key(&report, &report.recipients[0], execution_id));
        assert!(expected.exists(), "expected {:?} to exist", expected);
        fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn catalog_includes_teams_delivery() {
        assert!(
            catalog()
                .iter()
                .any(|entry| entry.channel == DistributionChannel::Teams)
        );
    }

    fn sample_report(recipients: Vec<DistributionRecipient>) -> ReportDefinition {
        ReportDefinition {
            id: Uuid::now_v7(),
            name: "Operations Digest".to_string(),
            description: "Daily operations summary".to_string(),
            owner: "platform-ops".to_string(),
            generator_kind: GeneratorKind::Pdf,
            dataset_name: "provider_metrics".to_string(),
            template: ReportTemplate {
                title: "Ops".to_string(),
                subtitle: "Summary".to_string(),
                theme: "stone".to_string(),
                layout: "summary".to_string(),
                sections: Vec::new(),
            },
            schedule: ReportSchedule::default(),
            recipients,
            tags: Vec::new(),
            parameters: json!({}),
            active: true,
            last_generated_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn sample_preview() -> ReportExecutionPreview {
        ReportExecutionPreview {
            headline: "Operations Digest generated".to_string(),
            generated_for: "provider_metrics".to_string(),
            engine: "pdf".to_string(),
            highlights: Vec::new(),
            sections: Vec::new(),
        }
    }

    fn sample_artifact() -> ReportArtifact {
        ReportArtifact {
            file_name: "ops-digest.pdf".to_string(),
            mime_type: "application/pdf".to_string(),
            size_bytes: 1024,
            storage_url: "/api/v1/reports/executions/123/download".to_string(),
            checksum: "sha256:test".to_string(),
        }
    }

    fn sample_metrics() -> ReportExecutionMetrics {
        ReportExecutionMetrics {
            duration_ms: 420,
            row_count: 128,
            section_count: 3,
            recipient_count: 1,
        }
    }
}
