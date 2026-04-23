use axum::{extract::{Path, State}, Json};
use chrono::Utc;

use crate::{
	handlers::{bad_request, db_error, internal_error, load_contract_row, load_contracts, load_peers, not_found, ServiceResult},
	models::{contract::{CreateContractRequest, SharingContract, UpdateContractRequest}, ListResponse},
	AppState,
};

pub async fn list_contracts(State(state): State<AppState>) -> ServiceResult<ListResponse<SharingContract>> {
	let items = load_contracts(&state.db).await.map_err(|cause| db_error(&cause))?;
	Ok(Json(ListResponse { items }))
}

pub async fn create_contract(
	State(state): State<AppState>,
	Json(request): Json<CreateContractRequest>,
) -> ServiceResult<SharingContract> {
	if request.name.trim().is_empty() {
		return Err(bad_request("contract name is required"));
	}

	let peers = load_peers(&state.db).await.map_err(|cause| db_error(&cause))?;
	if !peers.iter().any(|peer| peer.id == request.peer_id) {
		return Err(bad_request("peer does not exist"));
	}

	let id = uuid::Uuid::now_v7();
	let now = Utc::now();
	let allowed_purposes = serde_json::to_value(&request.allowed_purposes).map_err(|cause| internal_error(cause.to_string()))?;
	let data_classes = serde_json::to_value(&request.data_classes).map_err(|cause| internal_error(cause.to_string()))?;
	let signed_at = if request.status == "active" { Some(now) } else { None };

	sqlx::query(
		"INSERT INTO nexus_contracts (id, peer_id, name, description, dataset_locator, allowed_purposes, data_classes, residency_region, query_template, max_rows_per_query, replication_mode, encryption_profile, retention_days, status, signed_at, expires_at, created_at, updated_at)
		 VALUES ($1, $2, $3, $4, $5, $6::jsonb, $7::jsonb, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)",
	)
	.bind(id)
	.bind(request.peer_id)
	.bind(&request.name)
	.bind(&request.description)
	.bind(&request.dataset_locator)
	.bind(allowed_purposes)
	.bind(data_classes)
	.bind(&request.residency_region)
	.bind(&request.query_template)
	.bind(request.max_rows_per_query)
	.bind(&request.replication_mode)
	.bind(&request.encryption_profile)
	.bind(request.retention_days)
	.bind(&request.status)
	.bind(signed_at)
	.bind(request.expires_at)
	.bind(now)
	.bind(now)
	.execute(&state.db)
	.await
	.map_err(|cause| db_error(&cause))?;

	let row = load_contract_row(&state.db, id)
		.await
		.map_err(|cause| db_error(&cause))?
		.ok_or_else(|| internal_error("created contract could not be reloaded"))?;
	let contract = SharingContract::try_from(row).map_err(|cause| internal_error(cause.to_string()))?;
	Ok(Json(contract))
}

pub async fn update_contract(
	Path(id): Path<uuid::Uuid>,
	State(state): State<AppState>,
	Json(request): Json<UpdateContractRequest>,
) -> ServiceResult<SharingContract> {
	let current = load_contract_row(&state.db, id)
		.await
		.map_err(|cause| db_error(&cause))?
		.ok_or_else(|| not_found("contract not found"))?;
	let current = SharingContract::try_from(current).map_err(|cause| internal_error(cause.to_string()))?;
	let now = Utc::now();
	let allowed_purposes = serde_json::to_value(request.allowed_purposes.clone().unwrap_or(current.allowed_purposes.clone()))
		.map_err(|cause| internal_error(cause.to_string()))?;
	let data_classes = serde_json::to_value(request.data_classes.clone().unwrap_or(current.data_classes.clone()))
		.map_err(|cause| internal_error(cause.to_string()))?;
	let status = request.status.clone().unwrap_or(current.status.clone());
	let signed_at = if status == "active" {
		current.signed_at.or(Some(now))
	} else {
		current.signed_at
	};

	sqlx::query(
		"UPDATE nexus_contracts
		 SET name = $2,
			 description = $3,
			 dataset_locator = $4,
			 allowed_purposes = $5::jsonb,
			 data_classes = $6::jsonb,
			 residency_region = $7,
			 query_template = $8,
			 max_rows_per_query = $9,
			 replication_mode = $10,
			 encryption_profile = $11,
			 retention_days = $12,
			 status = $13,
			 signed_at = $14,
			 expires_at = $15,
			 updated_at = $16
		 WHERE id = $1",
	)
	.bind(id)
	.bind(request.name.unwrap_or(current.name))
	.bind(request.description.unwrap_or(current.description))
	.bind(request.dataset_locator.unwrap_or(current.dataset_locator))
	.bind(allowed_purposes)
	.bind(data_classes)
	.bind(request.residency_region.unwrap_or(current.residency_region))
	.bind(request.query_template.unwrap_or(current.query_template))
	.bind(request.max_rows_per_query.unwrap_or(current.max_rows_per_query))
	.bind(request.replication_mode.unwrap_or(current.replication_mode))
	.bind(request.encryption_profile.unwrap_or(current.encryption_profile))
	.bind(request.retention_days.unwrap_or(current.retention_days))
	.bind(status)
	.bind(signed_at)
	.bind(request.expires_at.unwrap_or(current.expires_at))
	.bind(now)
	.execute(&state.db)
	.await
	.map_err(|cause| db_error(&cause))?;

	let row = load_contract_row(&state.db, id)
		.await
		.map_err(|cause| db_error(&cause))?
		.ok_or_else(|| internal_error("updated contract could not be reloaded"))?;
	let contract = SharingContract::try_from(row).map_err(|cause| internal_error(cause.to_string()))?;
	Ok(Json(contract))
}
