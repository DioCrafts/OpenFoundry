use chrono::{DateTime, Utc};

use crate::models::{
    access_grant::AccessGrant, contract::SharingContract, peer::PeerOrganization,
    share::SharedDataset, space::NexusSpace,
};

const ACTIVE_PEER_STATUS: &str = "authenticated";

pub fn validate_contract(
    peer: &PeerOrganization,
    name: &str,
    query_template: &str,
    allowed_purposes: &[String],
    max_rows_per_query: i64,
    replication_mode: &str,
    retention_days: i32,
    status: &str,
    expires_at: DateTime<Utc>,
    now: DateTime<Utc>,
) -> Result<(), String> {
    if name.trim().is_empty() {
        return Err("contract name is required".to_string());
    }
    if query_template.trim().is_empty() {
        return Err("query template is required".to_string());
    }
    if max_rows_per_query <= 0 {
        return Err("max_rows_per_query must be greater than zero".to_string());
    }
    if retention_days <= 0 {
        return Err("retention_days must be greater than zero".to_string());
    }
    if !matches_contract_status(status) {
        return Err(format!("unsupported contract status '{status}'"));
    }
    ensure_valid_replication_mode(replication_mode)?;
    if status != "expired" && expires_at <= now {
        return Err("contract expiry must be in the future".to_string());
    }
    if status == "active" {
        if allowed_purposes.is_empty() {
            return Err("active contracts require at least one allowed purpose".to_string());
        }
        ensure_peer_authenticated(peer, "contract peer")?;
    }
    Ok(())
}

pub fn validate_share_state(
    contract: &SharingContract,
    provider_peer: &PeerOrganization,
    consumer_peer: &PeerOrganization,
    provider_space: Option<&NexusSpace>,
    consumer_space: Option<&NexusSpace>,
    dataset_name: &str,
    replication_mode: &str,
    status: &str,
    now: DateTime<Utc>,
) -> Result<(), String> {
    if dataset_name.trim().is_empty() {
        return Err("dataset name is required".to_string());
    }
    if provider_peer.id == consumer_peer.id {
        return Err("provider and consumer peers must differ".to_string());
    }
    if contract.peer_id != provider_peer.id {
        return Err("contract peer must match the provider peer".to_string());
    }
    if !matches_share_status(status) {
        return Err(format!("unsupported share status '{status}'"));
    }

    ensure_replication_compatible(replication_mode, &contract.replication_mode)?;
    ensure_space_membership(provider_space, provider_peer.id, "provider")?;
    ensure_space_membership(consumer_space, consumer_peer.id, "consumer")?;

    if status == "active" {
        if contract.status != "active" {
            return Err("shares can only be activated from an active contract".to_string());
        }
        if contract.expires_at <= now {
            return Err("shares cannot be activated from an expired contract".to_string());
        }
        ensure_peer_authenticated(provider_peer, "provider peer")?;
        ensure_peer_authenticated(consumer_peer, "consumer peer")?;
    }

    Ok(())
}

pub fn validate_federated_runtime(
    share: &SharedDataset,
    contract: &SharingContract,
    grant: &AccessGrant,
    provider_peer: &PeerOrganization,
    consumer_peer: &PeerOrganization,
    now: DateTime<Utc>,
) -> Result<(), String> {
    if share.status != "active" {
        return Err("shared dataset is not active".to_string());
    }
    if contract.status != "active" || contract.expires_at <= now {
        return Err("sharing contract is not active".to_string());
    }
    if contract.peer_id != share.provider_peer_id {
        return Err("share provider does not match the contract owner peer".to_string());
    }
    if grant.peer_id != share.consumer_peer_id {
        return Err("access grant is not bound to the consumer peer".to_string());
    }

    ensure_peer_authenticated(provider_peer, "provider peer")?;
    ensure_peer_authenticated(consumer_peer, "consumer peer")?;

    Ok(())
}

fn ensure_peer_authenticated(peer: &PeerOrganization, label: &str) -> Result<(), String> {
    if peer.status == ACTIVE_PEER_STATUS {
        Ok(())
    } else {
        Err(format!("{label} must be authenticated"))
    }
}

fn ensure_space_membership(
    space: Option<&NexusSpace>,
    peer_id: uuid::Uuid,
    label: &str,
) -> Result<(), String> {
    let Some(space) = space else {
        return Ok(());
    };
    if !matches!(space.space_kind.as_str(), "private" | "shared") {
        return Err(format!("{label} space has unsupported space_kind"));
    }
    if !matches!(space.status.as_str(), "draft" | "active" | "paused") {
        return Err(format!("{label} space has unsupported status"));
    }
    if !space.member_peer_ids.contains(&peer_id) {
        return Err(format!(
            "{label} peer is not a member of the selected space"
        ));
    }
    Ok(())
}

fn ensure_replication_compatible(share_mode: &str, contract_mode: &str) -> Result<(), String> {
    let share_rank = replication_rank(share_mode)
        .ok_or_else(|| format!("unsupported replication mode '{share_mode}'"))?;
    let contract_rank = replication_rank(contract_mode)
        .ok_or_else(|| format!("unsupported replication mode '{contract_mode}'"))?;

    if share_rank <= contract_rank {
        Ok(())
    } else {
        Err(format!(
            "share replication mode '{share_mode}' exceeds contract allowance '{contract_mode}'"
        ))
    }
}

fn ensure_valid_replication_mode(mode: &str) -> Result<(), String> {
    replication_rank(mode)
        .map(|_| ())
        .ok_or_else(|| format!("unsupported replication mode '{mode}'"))
}

fn replication_rank(mode: &str) -> Option<u8> {
    match mode {
        "query_only" => Some(0),
        "snapshot" => Some(1),
        "incremental_replication" => Some(2),
        "continuous" => Some(3),
        _ => None,
    }
}

fn matches_contract_status(status: &str) -> bool {
    matches!(status, "draft" | "active" | "suspended" | "expired")
}

fn matches_share_status(status: &str) -> bool {
    matches!(status, "draft" | "active" | "paused" | "revoked")
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, Utc};
    use serde_json::json;

    use crate::models::{
        access_grant::AccessGrant, contract::SharingContract, peer::PeerOrganization,
        share::SharedDataset, space::NexusSpace,
    };

    use super::{validate_contract, validate_federated_runtime, validate_share_state};

    fn peer(status: &str) -> PeerOrganization {
        let now = Utc::now();
        PeerOrganization {
            id: uuid::Uuid::now_v7(),
            slug: "peer".to_string(),
            display_name: "Peer".to_string(),
            organization_type: "partner".to_string(),
            region: "eu-west-1".to_string(),
            endpoint_url: "https://peer.example.com".to_string(),
            auth_mode: "mtls".to_string(),
            trust_level: "partner".to_string(),
            public_key_fingerprint: "fp-1".to_string(),
            shared_scopes: vec!["datasets".to_string()],
            status: status.to_string(),
            lifecycle_stage: "active".to_string(),
            admin_contacts: vec!["ops@example.com".to_string()],
            last_handshake_at: Some(now),
            created_at: now,
            updated_at: now,
        }
    }

    fn contract(peer_id: uuid::Uuid) -> SharingContract {
        let now = Utc::now();
        SharingContract {
            id: uuid::Uuid::now_v7(),
            peer_id,
            name: "Contract".to_string(),
            description: "Contract".to_string(),
            dataset_locator: "partner://dataset".to_string(),
            allowed_purposes: vec!["analytics".to_string()],
            data_classes: vec!["internal".to_string()],
            residency_region: "eu-west-1".to_string(),
            query_template: "select * from shared_dataset".to_string(),
            max_rows_per_query: 500,
            replication_mode: "incremental_replication".to_string(),
            encryption_profile: "aes256".to_string(),
            retention_days: 30,
            status: "active".to_string(),
            signed_at: Some(now),
            expires_at: now + Duration::days(30),
            created_at: now,
            updated_at: now,
        }
    }

    fn share(
        contract_id: uuid::Uuid,
        provider_peer_id: uuid::Uuid,
        consumer_peer_id: uuid::Uuid,
    ) -> SharedDataset {
        let now = Utc::now();
        SharedDataset {
            id: uuid::Uuid::now_v7(),
            contract_id,
            provider_peer_id,
            consumer_peer_id,
            provider_space_id: None,
            consumer_space_id: None,
            dataset_name: "sales".to_string(),
            selector: json!({"region": "eu"}),
            provider_schema: json!({"region": "string", "amount": "number"}),
            consumer_schema: json!({"region": "string", "amount": "number"}),
            sample_rows: vec![json!({"region": "eu", "amount": 12})],
            replication_mode: "query_only".to_string(),
            status: "active".to_string(),
            last_sync_at: None,
            created_at: now,
            updated_at: now,
        }
    }

    fn space(member_peer_id: uuid::Uuid) -> NexusSpace {
        let now = Utc::now();
        NexusSpace {
            id: uuid::Uuid::now_v7(),
            slug: "shared-space".to_string(),
            display_name: "Shared Space".to_string(),
            description: "Cross-org".to_string(),
            space_kind: "shared".to_string(),
            owner_peer_id: None,
            region: "eu-west-1".to_string(),
            member_peer_ids: vec![member_peer_id],
            governance_tags: vec!["partners".to_string()],
            status: "active".to_string(),
            created_at: now,
            updated_at: now,
        }
    }

    fn grant(share_id: uuid::Uuid, peer_id: uuid::Uuid) -> AccessGrant {
        let now = Utc::now();
        AccessGrant {
            id: uuid::Uuid::now_v7(),
            share_id,
            peer_id,
            query_template: "select * from shared_dataset".to_string(),
            max_rows_per_query: 500,
            can_replicate: false,
            allowed_purposes: vec!["analytics".to_string()],
            expires_at: now + Duration::days(7),
            issued_at: now,
        }
    }

    #[test]
    fn rejects_active_contract_with_unauthenticated_peer() {
        let now = Utc::now();
        let result = validate_contract(
            &peer("pending"),
            "Partner contract",
            "select * from shared_dataset",
            &["analytics".to_string()],
            100,
            "query_only",
            30,
            "active",
            now + Duration::days(30),
            now,
        );

        assert!(result.is_err());
    }

    #[test]
    fn rejects_share_that_exceeds_contract_replication_mode() {
        let provider = peer("authenticated");
        let consumer = peer("authenticated");
        let mut contract = contract(provider.id);
        contract.replication_mode = "query_only".to_string();

        let result = validate_share_state(
            &contract,
            &provider,
            &consumer,
            None,
            None,
            "sales",
            "incremental_replication",
            "active",
            Utc::now(),
        );

        assert!(result.is_err());
    }

    #[test]
    fn rejects_federated_runtime_with_mismatched_grant_peer() {
        let provider = peer("authenticated");
        let consumer = peer("authenticated");
        let contract = contract(provider.id);
        let share = share(contract.id, provider.id, consumer.id);
        let grant = grant(share.id, provider.id);

        let result =
            validate_federated_runtime(&share, &contract, &grant, &provider, &consumer, Utc::now());

        assert!(result.is_err());
    }

    #[test]
    fn rejects_share_when_peer_is_not_member_of_space() {
        let provider = peer("authenticated");
        let consumer = peer("authenticated");
        let contract = contract(provider.id);
        let outsider_space = space(uuid::Uuid::now_v7());

        let result = validate_share_state(
            &contract,
            &provider,
            &consumer,
            Some(&outsider_space),
            None,
            "sales",
            "query_only",
            "active",
            Utc::now(),
        );

        assert!(result.is_err());
    }
}
