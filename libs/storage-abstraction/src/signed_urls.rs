use crate::backend::StorageResult;

/// Configuration for generating pre-signed URLs (for direct upload/download).
#[derive(Debug, Clone)]
pub struct SignedUrlConfig {
    pub expiry_secs: u64,
}

impl Default for SignedUrlConfig {
    fn default() -> Self {
        Self { expiry_secs: 3600 }
    }
}

/// Generate an upload pre-signed URL.
/// NOTE: Full implementation requires the object_store signer API
/// which is available for S3. For now we provide the config plumbing.
pub fn presigned_upload_url(
    _bucket: &str,
    _path: &str,
    _config: &SignedUrlConfig,
) -> StorageResult<String> {
    // TODO: integrate object_store::signer::Signer when stabilised
    Ok(String::new())
}

pub fn presigned_download_url(
    _bucket: &str,
    _path: &str,
    _config: &SignedUrlConfig,
) -> StorageResult<String> {
    Ok(String::new())
}

