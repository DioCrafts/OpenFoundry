pub mod consume;
pub mod contracts;
pub mod peers;
pub mod shares;
pub mod spaces;

use axum::{Json, http::StatusCode};
use serde::Serialize;

use crate::models::{
    access_grant::AccessGrantRow,
    contract::ContractRow,
    peer::PeerRow,
    share::SharedDatasetRow,
    space::SpaceRow,
    sync_status::SyncStatusRow,
};

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

pub type ServiceResult<T> = Result<Json<T>, (StatusCode, Json<ErrorResponse>)>;

pub fn bad_request(message: impl Into<String>) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::BAD_REQUEST,
        Json(ErrorResponse {
            error: message.into(),
        }),
    )
}

pub fn not_found(message: impl Into<String>) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse {
            error: message.into(),
        }),
    )
}

pub fn internal_error(message: impl Into<String>) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: message.into(),
        }),
    )
}

pub fn db_error(cause: &sqlx::Error) -> (StatusCode, Json<ErrorResponse>) {
    tracing::error!("nexus-service database error: {cause}");
    internal_error("database operation failed")
}

pub async fn load_peers(
    db: &sqlx::PgPool,
) -> Result<Vec<crate::models::peer::PeerOrganization>, sqlx::Error> {
    let rows = sqlx::query_as::<_, PeerRow>(
		"SELECT id, slug, display_name, organization_type, region, endpoint_url, auth_mode, trust_level, public_key_fingerprint, shared_scopes, status, lifecycle_stage, admin_contacts, last_handshake_at, created_at, updated_at
		 FROM nexus_peers
		 ORDER BY updated_at DESC",
	)
	.fetch_all(db)
	.await?;

    rows.into_iter()
        .map(crate::models::peer::PeerOrganization::try_from)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|cause| {
            sqlx::Error::Decode(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                cause,
            )))
        })
}

pub async fn load_peer_row(
    db: &sqlx::PgPool,
    id: uuid::Uuid,
) -> Result<Option<PeerRow>, sqlx::Error> {
    sqlx::query_as::<_, PeerRow>(
		"SELECT id, slug, display_name, organization_type, region, endpoint_url, auth_mode, trust_level, public_key_fingerprint, shared_scopes, status, lifecycle_stage, admin_contacts, last_handshake_at, created_at, updated_at
		 FROM nexus_peers WHERE id = $1",
	)
	.bind(id)
	.fetch_optional(db)
	.await
}

pub async fn load_contracts(
    db: &sqlx::PgPool,
) -> Result<Vec<crate::models::contract::SharingContract>, sqlx::Error> {
    let rows = sqlx::query_as::<_, ContractRow>(
		"SELECT id, peer_id, name, description, dataset_locator, allowed_purposes, data_classes, residency_region, query_template, max_rows_per_query, replication_mode, encryption_profile, retention_days, status, signed_at, expires_at, created_at, updated_at
		 FROM nexus_contracts
		 ORDER BY updated_at DESC",
	)
	.fetch_all(db)
	.await?;

    rows.into_iter()
        .map(crate::models::contract::SharingContract::try_from)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|cause| {
            sqlx::Error::Decode(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                cause,
            )))
        })
}

pub async fn load_contract_row(
    db: &sqlx::PgPool,
    id: uuid::Uuid,
) -> Result<Option<ContractRow>, sqlx::Error> {
    sqlx::query_as::<_, ContractRow>(
		"SELECT id, peer_id, name, description, dataset_locator, allowed_purposes, data_classes, residency_region, query_template, max_rows_per_query, replication_mode, encryption_profile, retention_days, status, signed_at, expires_at, created_at, updated_at
		 FROM nexus_contracts WHERE id = $1",
	)
	.bind(id)
	.fetch_optional(db)
	.await
}

pub async fn load_shares(
    db: &sqlx::PgPool,
) -> Result<Vec<crate::models::share::SharedDataset>, sqlx::Error> {
    let rows = sqlx::query_as::<_, SharedDatasetRow>(
		"SELECT id, contract_id, provider_peer_id, consumer_peer_id, provider_space_id, consumer_space_id, dataset_name, selector, provider_schema, consumer_schema, sample_rows, replication_mode, status, last_sync_at, created_at, updated_at
		 FROM nexus_shares
		 ORDER BY updated_at DESC",
	)
	.fetch_all(db)
	.await?;

    rows.into_iter()
        .map(crate::models::share::SharedDataset::try_from)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|cause| {
            sqlx::Error::Decode(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                cause,
            )))
        })
}

pub async fn load_share_row(
    db: &sqlx::PgPool,
    id: uuid::Uuid,
) -> Result<Option<SharedDatasetRow>, sqlx::Error> {
    sqlx::query_as::<_, SharedDatasetRow>(
		"SELECT id, contract_id, provider_peer_id, consumer_peer_id, provider_space_id, consumer_space_id, dataset_name, selector, provider_schema, consumer_schema, sample_rows, replication_mode, status, last_sync_at, created_at, updated_at
		 FROM nexus_shares WHERE id = $1",
	)
	.bind(id)
	.fetch_optional(db)
	.await
}

pub async fn load_spaces(
    db: &sqlx::PgPool,
) -> Result<Vec<crate::models::space::NexusSpace>, sqlx::Error> {
    let rows = sqlx::query_as::<_, SpaceRow>(
		"SELECT id, slug, display_name, description, space_kind, owner_peer_id, region, member_peer_ids, governance_tags, status, created_at, updated_at
		 FROM nexus_spaces
		 ORDER BY updated_at DESC",
	)
	.fetch_all(db)
	.await?;

    rows.into_iter()
        .map(crate::models::space::NexusSpace::try_from)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|cause| {
            sqlx::Error::Decode(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                cause,
            )))
        })
}

pub async fn load_space_row(
    db: &sqlx::PgPool,
    id: uuid::Uuid,
) -> Result<Option<SpaceRow>, sqlx::Error> {
    sqlx::query_as::<_, SpaceRow>(
		"SELECT id, slug, display_name, description, space_kind, owner_peer_id, region, member_peer_ids, governance_tags, status, created_at, updated_at
		 FROM nexus_spaces WHERE id = $1",
	)
	.bind(id)
	.fetch_optional(db)
	.await
}

pub async fn load_access_grants(
    db: &sqlx::PgPool,
) -> Result<Vec<crate::models::access_grant::AccessGrant>, sqlx::Error> {
    let rows = sqlx::query_as::<_, AccessGrantRow>(
		"SELECT id, share_id, peer_id, query_template, max_rows_per_query, can_replicate, allowed_purposes, expires_at, issued_at
		 FROM nexus_access_grants
		 ORDER BY issued_at DESC",
	)
	.fetch_all(db)
	.await?;

    rows.into_iter()
        .map(crate::models::access_grant::AccessGrant::try_from)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|cause| {
            sqlx::Error::Decode(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                cause,
            )))
        })
}

pub async fn load_sync_statuses(
    db: &sqlx::PgPool,
) -> Result<Vec<crate::models::sync_status::SyncStatus>, sqlx::Error> {
    let rows = sqlx::query_as::<_, SyncStatusRow>(
		"SELECT id, share_id, mode, status, rows_replicated, backlog_rows, encrypted_in_transit, encrypted_at_rest, key_version, last_sync_at, next_sync_at, audit_cursor, updated_at
		 FROM nexus_sync_statuses
		 ORDER BY updated_at DESC",
	)
	.fetch_all(db)
	.await?;

    rows.into_iter()
        .map(crate::models::sync_status::SyncStatus::try_from)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|cause| {
            sqlx::Error::Decode(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                cause,
            )))
        })
}
