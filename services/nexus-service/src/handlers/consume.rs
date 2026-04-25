use std::collections::HashMap;

use axum::{Json, extract::State};

use crate::{
    AppState,
    domain::{audit_bridge, federation, governance, replication, schema_compat},
    handlers::{
        ServiceResult, bad_request, db_error, load_access_grants, load_contracts, load_peers,
        load_shares, load_sync_statuses, not_found,
    },
    models::{
        ListResponse,
        access_grant::{FederatedQueryRequest, FederatedQueryResult},
        sync_status::{AuditBridgeSummary, ReplicationPlan, SchemaCompatibilityReport},
    },
};

pub async fn run_federated_query(
    State(state): State<AppState>,
    Json(request): Json<FederatedQueryRequest>,
) -> ServiceResult<FederatedQueryResult> {
    let shares = load_shares(&state.db)
        .await
        .map_err(|cause| db_error(&cause))?;
    let grants = load_access_grants(&state.db)
        .await
        .map_err(|cause| db_error(&cause))?;
    let contracts = load_contracts(&state.db)
        .await
        .map_err(|cause| db_error(&cause))?;
    let peers = load_peers(&state.db)
        .await
        .map_err(|cause| db_error(&cause))?;

    let share = shares
        .iter()
        .find(|share| share.id == request.share_id)
        .cloned()
        .ok_or_else(|| not_found("shared dataset not found"))?;
    let grant = grants
        .iter()
        .find(|grant| grant.share_id == request.share_id)
        .cloned()
        .ok_or_else(|| not_found("access grant not found for shared dataset"))?;
    let contract = contracts
        .iter()
        .find(|contract| contract.id == share.contract_id)
        .cloned()
        .ok_or_else(|| not_found("sharing contract not found for shared dataset"))?;
    let peer_index = peers
        .into_iter()
        .map(|peer| (peer.id, peer))
        .collect::<HashMap<_, _>>();
    let provider_peer = peer_index
        .get(&share.provider_peer_id)
        .ok_or_else(|| not_found("provider peer not found"))?;
    let consumer_peer = peer_index
        .get(&share.consumer_peer_id)
        .ok_or_else(|| not_found("consumer peer not found"))?;
    governance::validate_federated_runtime(
        &share,
        &contract,
        &grant,
        provider_peer,
        consumer_peer,
        chrono::Utc::now(),
    )
    .map_err(bad_request)?;

    let result =
        federation::execute_query(&request, &share, &grant, &peer_index).map_err(bad_request)?;
    Ok(Json(result))
}

pub async fn list_replication_plans(
    State(state): State<AppState>,
) -> ServiceResult<ListResponse<ReplicationPlan>> {
    let shares = load_shares(&state.db)
        .await
        .map_err(|cause| db_error(&cause))?;
    let sync_statuses = load_sync_statuses(&state.db)
        .await
        .map_err(|cause| db_error(&cause))?;
    let compatibility = shares
        .iter()
        .map(schema_compat::evaluate)
        .collect::<Vec<_>>();
    Ok(Json(ListResponse {
        items: replication::build_plans(&shares, &sync_statuses, &compatibility),
    }))
}

pub async fn list_schema_compatibility(
    State(state): State<AppState>,
) -> ServiceResult<ListResponse<SchemaCompatibilityReport>> {
    let shares = load_shares(&state.db)
        .await
        .map_err(|cause| db_error(&cause))?;
    Ok(Json(ListResponse {
        items: shares.iter().map(schema_compat::evaluate).collect(),
    }))
}

pub async fn get_audit_bridge(State(state): State<AppState>) -> ServiceResult<AuditBridgeSummary> {
    let peers = load_peers(&state.db)
        .await
        .map_err(|cause| db_error(&cause))?;
    let contracts = load_contracts(&state.db)
        .await
        .map_err(|cause| db_error(&cause))?;
    let shares = load_shares(&state.db)
        .await
        .map_err(|cause| db_error(&cause))?;
    let sync_statuses = load_sync_statuses(&state.db)
        .await
        .map_err(|cause| db_error(&cause))?;
    Ok(Json(audit_bridge::summarize(
        &peers,
        &contracts,
        &shares,
        &sync_statuses,
    )))
}
