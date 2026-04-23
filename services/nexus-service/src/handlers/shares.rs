use axum::{extract::{Path, State}, Json};
use chrono::Utc;

use crate::{
	domain::{encryption, schema_compat},
	handlers::{bad_request, db_error, internal_error, load_access_grants, load_contract_row, load_contracts, load_peer_row, load_share_row, load_shares, load_sync_statuses, not_found, ServiceResult},
	models::{
		access_grant::AccessGrant,
		share::{CreateShareRequest, ShareDetail, SharedDataset, UpdateShareRequest},
		sync_status::SyncStatus,
		ListResponse,
	},
	AppState,
};

pub async fn list_shares(State(state): State<AppState>) -> ServiceResult<ListResponse<ShareDetail>> {
	let shares = load_shares(&state.db).await.map_err(|cause| db_error(&cause))?;
	let contracts = load_contracts(&state.db).await.map_err(|cause| db_error(&cause))?;
	let grants = load_access_grants(&state.db).await.map_err(|cause| db_error(&cause))?;
	let sync_statuses = load_sync_statuses(&state.db).await.map_err(|cause| db_error(&cause))?;

	let items = shares
		.iter()
		.map(|share| compose_share_detail(share, &contracts, &grants, &sync_statuses))
		.collect();

	Ok(Json(ListResponse { items }))
}

pub async fn get_share(
	Path(id): Path<uuid::Uuid>,
	State(state): State<AppState>,
) -> ServiceResult<ShareDetail> {
	let row = load_share_row(&state.db, id)
		.await
		.map_err(|cause| db_error(&cause))?
		.ok_or_else(|| not_found("shared dataset not found"))?;
	let share = SharedDataset::try_from(row).map_err(|cause| internal_error(cause.to_string()))?;
	let contracts = load_contracts(&state.db).await.map_err(|cause| db_error(&cause))?;
	let grants = load_access_grants(&state.db).await.map_err(|cause| db_error(&cause))?;
	let sync_statuses = load_sync_statuses(&state.db).await.map_err(|cause| db_error(&cause))?;
	Ok(Json(compose_share_detail(&share, &contracts, &grants, &sync_statuses)))
}

pub async fn create_share(
	State(state): State<AppState>,
	Json(request): Json<CreateShareRequest>,
) -> ServiceResult<ShareDetail> {
	if request.dataset_name.trim().is_empty() {
		return Err(bad_request("dataset name is required"));
	}

	let contract_row = load_contract_row(&state.db, request.contract_id)
		.await
		.map_err(|cause| db_error(&cause))?
		.ok_or_else(|| bad_request("contract not found"))?;
	let contract = crate::models::contract::SharingContract::try_from(contract_row)
		.map_err(|cause| internal_error(cause.to_string()))?;
	if load_peer_row(&state.db, request.provider_peer_id)
		.await
		.map_err(|cause| db_error(&cause))?
		.is_none()
	{
		return Err(bad_request("provider peer not found"));
	}
	if load_peer_row(&state.db, request.consumer_peer_id)
		.await
		.map_err(|cause| db_error(&cause))?
		.is_none()
	{
		return Err(bad_request("consumer peer not found"));
	}

	let id = uuid::Uuid::now_v7();
	let grant_id = uuid::Uuid::now_v7();
	let sync_id = uuid::Uuid::now_v7();
	let now = Utc::now();
	let selector = request.selector.clone();
	let provider_schema = request.provider_schema.clone();
	let consumer_schema = request.consumer_schema.clone();
	let sample_rows = serde_json::to_value(&request.sample_rows).map_err(|cause| internal_error(cause.to_string()))?;
	let allowed_purposes = serde_json::to_value(&contract.allowed_purposes).map_err(|cause| internal_error(cause.to_string()))?;

	sqlx::query(
		"INSERT INTO nexus_shares (id, contract_id, provider_peer_id, consumer_peer_id, dataset_name, selector, provider_schema, consumer_schema, sample_rows, replication_mode, status, last_sync_at, created_at, updated_at)
		 VALUES ($1, $2, $3, $4, $5, $6::jsonb, $7::jsonb, $8::jsonb, $9::jsonb, $10, $11, $12, $13, $14)",
	)
	.bind(id)
	.bind(request.contract_id)
	.bind(request.provider_peer_id)
	.bind(request.consumer_peer_id)
	.bind(&request.dataset_name)
	.bind(selector)
	.bind(provider_schema)
	.bind(consumer_schema)
	.bind(sample_rows)
	.bind(&request.replication_mode)
	.bind("active")
	.bind(Option::<chrono::DateTime<chrono::Utc>>::None)
	.bind(now)
	.bind(now)
	.execute(&state.db)
	.await
	.map_err(|cause| db_error(&cause))?;

	sqlx::query(
		"INSERT INTO nexus_access_grants (id, share_id, peer_id, query_template, max_rows_per_query, can_replicate, allowed_purposes, expires_at, issued_at)
		 VALUES ($1, $2, $3, $4, $5, $6, $7::jsonb, $8, $9)",
	)
	.bind(grant_id)
	.bind(id)
	.bind(request.consumer_peer_id)
	.bind(&contract.query_template)
	.bind(contract.max_rows_per_query)
	.bind(request.replication_mode != "query_only")
	.bind(allowed_purposes)
	.bind(contract.expires_at)
	.bind(now)
	.execute(&state.db)
	.await
	.map_err(|cause| db_error(&cause))?;

	sqlx::query(
		"INSERT INTO nexus_sync_statuses (id, share_id, mode, status, rows_replicated, backlog_rows, encrypted_in_transit, encrypted_at_rest, key_version, last_sync_at, next_sync_at, audit_cursor, updated_at)
		 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)",
	)
	.bind(sync_id)
	.bind(id)
	.bind(&request.replication_mode)
	.bind("ready")
	.bind(0_i64)
	.bind(i64::try_from(request.sample_rows.len()).unwrap_or(0))
	.bind(true)
	.bind(true)
	.bind(&contract.encryption_profile)
	.bind(Option::<chrono::DateTime<chrono::Utc>>::None)
	.bind(Some(now + chrono::Duration::hours(4)))
	.bind(format!("cursor/{}", id))
	.bind(now)
	.execute(&state.db)
	.await
	.map_err(|cause| db_error(&cause))?;

	let row = load_share_row(&state.db, id)
		.await
		.map_err(|cause| db_error(&cause))?
		.ok_or_else(|| internal_error("created share could not be reloaded"))?;
	let share = SharedDataset::try_from(row).map_err(|cause| internal_error(cause.to_string()))?;
	let contracts = load_contracts(&state.db).await.map_err(|cause| db_error(&cause))?;
	let grants = load_access_grants(&state.db).await.map_err(|cause| db_error(&cause))?;
	let sync_statuses = load_sync_statuses(&state.db).await.map_err(|cause| db_error(&cause))?;
	Ok(Json(compose_share_detail(&share, &contracts, &grants, &sync_statuses)))
}

pub async fn update_share(
	Path(id): Path<uuid::Uuid>,
	State(state): State<AppState>,
	Json(request): Json<UpdateShareRequest>,
) -> ServiceResult<ShareDetail> {
	let current = load_share_row(&state.db, id)
		.await
		.map_err(|cause| db_error(&cause))?
		.ok_or_else(|| not_found("shared dataset not found"))?;
	let current = SharedDataset::try_from(current).map_err(|cause| internal_error(cause.to_string()))?;
	let now = Utc::now();
	let sample_rows = serde_json::to_value(request.sample_rows.clone().unwrap_or(current.sample_rows.clone()))
		.map_err(|cause| internal_error(cause.to_string()))?;

	sqlx::query(
		"UPDATE nexus_shares
		 SET dataset_name = $2,
			 selector = $3::jsonb,
			 consumer_schema = $4::jsonb,
			 sample_rows = $5::jsonb,
			 replication_mode = $6,
			 status = $7,
			 updated_at = $8
		 WHERE id = $1",
	)
	.bind(id)
	.bind(request.dataset_name.unwrap_or(current.dataset_name))
	.bind(request.selector.unwrap_or(current.selector))
	.bind(request.consumer_schema.unwrap_or(current.consumer_schema))
	.bind(sample_rows)
	.bind(request.replication_mode.unwrap_or(current.replication_mode))
	.bind(request.status.unwrap_or(current.status))
	.bind(now)
	.execute(&state.db)
	.await
	.map_err(|cause| db_error(&cause))?;

	let row = load_share_row(&state.db, id)
		.await
		.map_err(|cause| db_error(&cause))?
		.ok_or_else(|| internal_error("updated share could not be reloaded"))?;
	let share = SharedDataset::try_from(row).map_err(|cause| internal_error(cause.to_string()))?;
	let contracts = load_contracts(&state.db).await.map_err(|cause| db_error(&cause))?;
	let grants = load_access_grants(&state.db).await.map_err(|cause| db_error(&cause))?;
	let sync_statuses = load_sync_statuses(&state.db).await.map_err(|cause| db_error(&cause))?;
	Ok(Json(compose_share_detail(&share, &contracts, &grants, &sync_statuses)))
}

fn compose_share_detail(
	share: &SharedDataset,
	contracts: &[crate::models::contract::SharingContract],
	grants: &[AccessGrant],
	sync_statuses: &[SyncStatus],
) -> ShareDetail {
	let contract = contracts.iter().find(|contract| contract.id == share.contract_id);
	let access_grant = grants.iter().find(|grant| grant.share_id == share.id).cloned();
	let sync_status = sync_statuses.iter().find(|status| status.share_id == share.id).cloned();
	let compatibility = schema_compat::evaluate(share);
	let encryption = encryption::posture(share, contract, sync_status.as_ref());

	ShareDetail {
		share: share.clone(),
		access_grant,
		sync_status,
		encryption,
		compatibility,
	}
}
