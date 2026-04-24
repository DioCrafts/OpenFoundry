use axum::{
    Json,
    extract::{Path, State},
};
use chrono::Utc;

use crate::{
    AppState,
    domain::{audit_bridge, schema_compat},
    handlers::{
        ServiceResult, bad_request, db_error, internal_error, load_contracts, load_peer_row,
        load_peers, load_shares, load_sync_statuses, not_found,
    },
    models::{
        ListResponse,
        peer::{CreatePeerRequest, PeerOrganization, UpdatePeerRequest},
        sync_status::NexusOverview,
    },
};

pub async fn get_overview(State(state): State<AppState>) -> ServiceResult<NexusOverview> {
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
    let compatibility = shares
        .iter()
        .map(schema_compat::evaluate)
        .collect::<Vec<_>>();
    let audit_bridge = audit_bridge::summarize(&peers, &contracts, &shares, &sync_statuses);
    let latest_sync_at = sync_statuses
        .iter()
        .filter_map(|status| status.last_sync_at)
        .max();

    Ok(Json(NexusOverview {
        peer_count: peers.len() as i64,
        active_peer_count: peers
            .iter()
            .filter(|peer| peer.status == "authenticated")
            .count() as i64,
        contract_count: contracts.len() as i64,
        active_contract_count: contracts
            .iter()
            .filter(|contract| contract.status == "active")
            .count() as i64,
        share_count: shares.len() as i64,
        federated_access_count: shares
            .iter()
            .filter(|share| share.status == "active")
            .count() as i64,
        encrypted_share_count: sync_statuses
            .iter()
            .filter(|status| status.encrypted_in_transit && status.encrypted_at_rest)
            .count() as i64,
        replication_ready_count: sync_statuses
            .iter()
            .filter(|status| status.status == "ready")
            .count() as i64,
        pending_schema_reviews: compatibility
            .iter()
            .filter(|report| !report.compatible)
            .count() as i64,
        audit_bridge_status: audit_bridge.bridge_status,
        latest_sync_at,
    }))
}

pub async fn list_peers(
    State(state): State<AppState>,
) -> ServiceResult<ListResponse<PeerOrganization>> {
    let items = load_peers(&state.db)
        .await
        .map_err(|cause| db_error(&cause))?;
    Ok(Json(ListResponse { items }))
}

pub async fn create_peer(
    State(state): State<AppState>,
    Json(request): Json<CreatePeerRequest>,
) -> ServiceResult<PeerOrganization> {
    if request.slug.trim().is_empty() || request.display_name.trim().is_empty() {
        return Err(bad_request("peer slug and display name are required"));
    }

    let id = uuid::Uuid::now_v7();
    let now = Utc::now();
    let shared_scopes = serde_json::to_value(&request.shared_scopes)
        .map_err(|cause| internal_error(cause.to_string()))?;

    sqlx::query(
		"INSERT INTO nexus_peers (id, slug, display_name, region, endpoint_url, auth_mode, trust_level, public_key_fingerprint, shared_scopes, status, last_handshake_at, created_at, updated_at)
		 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9::jsonb, $10, $11, $12, $13)",
	)
	.bind(id)
	.bind(&request.slug)
	.bind(&request.display_name)
	.bind(&request.region)
	.bind(&request.endpoint_url)
	.bind(&request.auth_mode)
	.bind(&request.trust_level)
	.bind(&request.public_key_fingerprint)
	.bind(shared_scopes)
	.bind("pending")
	.bind(Option::<chrono::DateTime<chrono::Utc>>::None)
	.bind(now)
	.bind(now)
	.execute(&state.db)
	.await
	.map_err(|cause| db_error(&cause))?;

    let row = load_peer_row(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?
        .ok_or_else(|| internal_error("created peer could not be reloaded"))?;
    let peer =
        PeerOrganization::try_from(row).map_err(|cause| internal_error(cause.to_string()))?;
    Ok(Json(peer))
}

pub async fn update_peer(
    Path(id): Path<uuid::Uuid>,
    State(state): State<AppState>,
    Json(request): Json<UpdatePeerRequest>,
) -> ServiceResult<PeerOrganization> {
    let current = load_peer_row(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?
        .ok_or_else(|| not_found("peer not found"))?;
    let current =
        PeerOrganization::try_from(current).map_err(|cause| internal_error(cause.to_string()))?;
    let now = Utc::now();
    let shared_scopes = serde_json::to_value(
        request
            .shared_scopes
            .clone()
            .unwrap_or(current.shared_scopes.clone()),
    )
    .map_err(|cause| internal_error(cause.to_string()))?;

    sqlx::query(
        "UPDATE nexus_peers
		 SET display_name = $2,
			 region = $3,
			 endpoint_url = $4,
			 trust_level = $5,
			 shared_scopes = $6::jsonb,
			 status = $7,
			 updated_at = $8
		 WHERE id = $1",
    )
    .bind(id)
    .bind(request.display_name.unwrap_or(current.display_name))
    .bind(request.region.unwrap_or(current.region))
    .bind(request.endpoint_url.unwrap_or(current.endpoint_url))
    .bind(request.trust_level.unwrap_or(current.trust_level))
    .bind(shared_scopes)
    .bind(request.status.unwrap_or(current.status))
    .bind(now)
    .execute(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?;

    let row = load_peer_row(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?
        .ok_or_else(|| internal_error("updated peer could not be reloaded"))?;
    let peer =
        PeerOrganization::try_from(row).map_err(|cause| internal_error(cause.to_string()))?;
    Ok(Json(peer))
}

pub async fn authenticate_peer(
    Path(id): Path<uuid::Uuid>,
    State(state): State<AppState>,
) -> ServiceResult<PeerOrganization> {
    let now = Utc::now();
    let result = sqlx::query(
        "UPDATE nexus_peers
		 SET status = $2,
			 last_handshake_at = $3,
			 updated_at = $4
		 WHERE id = $1",
    )
    .bind(id)
    .bind("authenticated")
    .bind(now)
    .bind(now)
    .execute(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?;

    if result.rows_affected() == 0 {
        return Err(not_found("peer not found"));
    }

    let row = load_peer_row(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?
        .ok_or_else(|| internal_error("authenticated peer could not be reloaded"))?;
    let peer =
        PeerOrganization::try_from(row).map_err(|cause| internal_error(cause.to_string()))?;
    Ok(Json(peer))
}
