use chrono::{Duration, Utc};

use crate::models::{
    recipient::{
        DistributionChannel, DistributionChannelCatalogEntry, DistributionRecipient,
        DistributionResult,
    },
    report::ReportDefinition,
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
            channel: DistributionChannel::Webhook,
            display_name: "Webhook callback".to_string(),
            description: "Notify external systems after generation completes.".to_string(),
            configuration_fields: vec!["url".to_string(), "secret".to_string()],
        },
    ]
}

pub fn simulate_distribution(
    report: &ReportDefinition,
    generated_at: chrono::DateTime<Utc>,
) -> Vec<DistributionResult> {
    report
        .recipients
        .iter()
        .enumerate()
        .map(|(index, recipient)| {
            delivery_result(recipient, generated_at, report.name.as_str(), index as i64)
        })
        .collect()
}

fn delivery_result(
    recipient: &DistributionRecipient,
    generated_at: chrono::DateTime<Utc>,
    report_name: &str,
    index: i64,
) -> DistributionResult {
    let delivered_at = generated_at + Duration::seconds(20 + index * 11);
    let detail = match recipient.channel {
        DistributionChannel::Email => format!("Queued {report_name} for {}", recipient.target),
        DistributionChannel::S3 => format!("Stored artifact in {}", recipient.target),
        DistributionChannel::Slack => format!("Posted digest card to {}", recipient.target),
        DistributionChannel::Webhook => format!("Triggered callback {}", recipient.target),
    };

    DistributionResult {
        channel: recipient.channel,
        target: recipient.target.clone(),
        status: "delivered".to_string(),
        delivered_at,
        detail,
    }
}
