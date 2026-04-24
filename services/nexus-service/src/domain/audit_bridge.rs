use crate::models::{
    contract::SharingContract,
    peer::PeerOrganization,
    share::SharedDataset,
    sync_status::{AuditBridgeEntry, AuditBridgeSummary, SyncStatus},
};

pub fn summarize(
    peers: &[PeerOrganization],
    contracts: &[SharingContract],
    shares: &[SharedDataset],
    sync_statuses: &[SyncStatus],
) -> AuditBridgeSummary {
    let entries = shares
        .iter()
        .filter_map(|share| {
            let contract = contracts
                .iter()
                .find(|contract| contract.id == share.contract_id)?;
            let peer = peers
                .iter()
                .find(|peer| peer.id == share.consumer_peer_id)?;
            let sync_status = sync_statuses
                .iter()
                .find(|status| status.share_id == share.id);

            Some(AuditBridgeEntry {
                share_id: share.id,
                dataset_name: share.dataset_name.clone(),
                peer_name: peer.display_name.clone(),
                contract_name: contract.name.clone(),
                audit_cursor: sync_status
                    .map(|status| status.audit_cursor.clone())
                    .unwrap_or_else(|| "cursor/pending".to_string()),
                last_sync_at: sync_status.and_then(|status| status.last_sync_at),
                status: sync_status
                    .map(|status| status.status.clone())
                    .unwrap_or_else(|| "pending".to_string()),
                evidence: vec![
                    format!("contract:{}", contract.id),
                    format!("peer:{}", peer.slug),
                    format!("selector:{}", share.selector),
                ],
            })
        })
        .collect::<Vec<_>>();

    let bridge_status = if entries.iter().any(|entry| entry.status == "degraded") {
        "degraded"
    } else if entries.is_empty() {
        "pending"
    } else {
        "healthy"
    };

    let latest_cursor = entries
        .first()
        .map(|entry| entry.audit_cursor.clone())
        .unwrap_or_else(|| "cursor/pending".to_string());

    AuditBridgeSummary {
        bridge_status: bridge_status.to_string(),
        entry_count: entries.len() as i64,
        latest_cursor,
        entries,
    }
}
