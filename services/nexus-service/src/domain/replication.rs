use crate::models::{
    share::SharedDataset,
    sync_status::{ReplicationPlan, SchemaCompatibilityReport, SyncStatus},
};

pub fn build_plans(
    shares: &[SharedDataset],
    sync_statuses: &[SyncStatus],
    compatibility: &[SchemaCompatibilityReport],
) -> Vec<ReplicationPlan> {
    shares
        .iter()
        .map(|share| {
            let sync_status = sync_statuses
                .iter()
                .find(|status| status.share_id == share.id);
            let compatibility = compatibility
                .iter()
                .find(|report| report.share_id == share.id);
            let encrypted = sync_status
                .map(|status| status.encrypted_in_transit && status.encrypted_at_rest)
                .unwrap_or(false);
            let status = if compatibility
                .map(|report| report.compatible)
                .unwrap_or(false)
            {
                sync_status
                    .map(|status| status.status.clone())
                    .unwrap_or_else(|| "ready".to_string())
            } else {
                "schema_review".to_string()
            };

            ReplicationPlan {
                share_id: share.id,
                dataset_name: share.dataset_name.clone(),
                mode: share.replication_mode.clone(),
                status,
                rows_replicated: sync_status
                    .map(|status| status.rows_replicated)
                    .unwrap_or_default(),
                backlog_rows: sync_status
                    .map(|status| status.backlog_rows)
                    .unwrap_or_default(),
                next_sync_at: sync_status.and_then(|status| status.next_sync_at),
                selective_filter: share.selector.clone(),
                encrypted,
            }
        })
        .collect()
}
