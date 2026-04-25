use std::collections::HashMap;

use serde_json::Value;

use crate::{
    domain::access_proxy,
    models::{
        access_grant::{AccessGrant, FederatedQueryRequest, FederatedQueryResult},
        peer::PeerOrganization,
        share::SharedDataset,
    },
};

pub fn execute_query(
    request: &FederatedQueryRequest,
    share: &SharedDataset,
    grant: &AccessGrant,
    peers: &HashMap<uuid::Uuid, PeerOrganization>,
) -> Result<FederatedQueryResult, String> {
    ensure_read_only_sql(&request.sql)?;
    access_proxy::validate_access(grant, &request.purpose)?;

    let limit = access_proxy::resolve_limit(grant, request.limit);
    let rows = share
        .sample_rows
        .iter()
        .take(limit)
        .cloned()
        .collect::<Vec<Value>>();
    let columns = rows
        .first()
        .and_then(|value| value.as_object())
        .map(|object| object.keys().cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    let source_peer = peers
        .get(&share.provider_peer_id)
        .map(|peer| peer.display_name.clone())
        .unwrap_or_else(|| "unknown peer".to_string());

    Ok(FederatedQueryResult {
        share_id: share.id,
        dataset_name: share.dataset_name.clone(),
        source_peer,
        executed_sql: request.sql.clone(),
        query_mode: share.replication_mode.clone(),
        limit,
        columns,
        rows,
    })
}

fn ensure_read_only_sql(sql: &str) -> Result<(), String> {
    let normalized = sql.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return Err("federated query SQL is required".to_string());
    }
    if !(normalized.starts_with("select") || normalized.starts_with("with")) {
        return Err("federated query must be read-only".to_string());
    }

    let forbidden = [
        " insert ",
        " update ",
        " delete ",
        " drop ",
        " alter ",
        " truncate ",
        " create ",
        " revoke ",
        " grant ",
    ];
    let padded = format!(" {normalized} ");
    if forbidden.iter().any(|keyword| padded.contains(keyword)) {
        return Err("federated query contains a write-oriented SQL keyword".to_string());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::ensure_read_only_sql;

    #[test]
    fn rejects_non_read_only_sql() {
        let result = ensure_read_only_sql("DELETE FROM shared_dataset");
        assert!(result.is_err());
    }

    #[test]
    fn accepts_select_sql() {
        let result = ensure_read_only_sql("SELECT * FROM shared_dataset LIMIT 10");
        assert!(result.is_ok());
    }
}
