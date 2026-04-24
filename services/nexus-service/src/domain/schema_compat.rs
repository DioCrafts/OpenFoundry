use chrono::Utc;

use crate::models::{share::SharedDataset, sync_status::SchemaCompatibilityReport};

pub fn evaluate(share: &SharedDataset) -> SchemaCompatibilityReport {
    let provider = share
        .provider_schema
        .as_object()
        .cloned()
        .unwrap_or_default();
    let consumer = share
        .consumer_schema
        .as_object()
        .cloned()
        .unwrap_or_default();

    let missing_fields = consumer
        .keys()
        .filter(|field| !provider.contains_key(*field))
        .cloned()
        .collect::<Vec<_>>();

    let type_mismatches = consumer
        .iter()
        .filter_map(|(field, consumer_type)| {
            let provider_type = provider.get(field)?;
            if provider_type == consumer_type {
                None
            } else {
                Some(format!(
                    "{field}: provider={} consumer={}",
                    provider_type, consumer_type
                ))
            }
        })
        .collect::<Vec<_>>();

    let compatible = missing_fields.is_empty() && type_mismatches.is_empty();
    let summary = if compatible {
        "schemas compatible for federated access"
    } else {
        "schema review required before replication"
    };

    SchemaCompatibilityReport {
        share_id: share.id,
        compatible,
        missing_fields,
        type_mismatches,
        reviewed_at: Utc::now(),
        summary: summary.to_string(),
    }
}
