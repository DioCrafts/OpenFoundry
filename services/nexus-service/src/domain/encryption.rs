use crate::models::{
    contract::SharingContract,
    share::SharedDataset,
    sync_status::{EncryptionPosture, SyncStatus},
};

pub fn posture(
    share: &SharedDataset,
    contract: Option<&SharingContract>,
    sync_status: Option<&SyncStatus>,
) -> EncryptionPosture {
    let profile = contract
        .map(|contract| contract.encryption_profile.clone())
        .unwrap_or_else(|| "mutual-tls+envelope".to_string());
    let key_version = sync_status
        .map(|status| status.key_version.clone())
        .unwrap_or_else(|| "key/pending".to_string());
    let encrypted_in_transit = sync_status
        .map(|status| status.encrypted_in_transit)
        .unwrap_or(true);
    let encrypted_at_rest = sync_status
        .map(|status| status.encrypted_at_rest)
        .unwrap_or(true);
    let recommendation = if encrypted_in_transit && encrypted_at_rest {
        "rotation current"
    } else {
        "provision key rotation before replication"
    };

    EncryptionPosture {
        share_id: share.id,
        transport_cipher: if encrypted_in_transit {
            "TLS 1.3 mTLS".to_string()
        } else {
            "disabled".to_string()
        },
        at_rest_cipher: if encrypted_at_rest {
            "AES-256-GCM envelope".to_string()
        } else {
            "disabled".to_string()
        },
        key_version,
        profile,
        encrypted_in_transit,
        encrypted_at_rest,
        recommendation: recommendation.to_string(),
    }
}
