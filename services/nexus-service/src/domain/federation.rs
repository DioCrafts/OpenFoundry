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
	access_proxy::validate_access(grant, &request.purpose)?;

	let limit = access_proxy::resolve_limit(grant, request.limit);
	let rows = share.sample_rows.iter().take(limit).cloned().collect::<Vec<Value>>();
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
