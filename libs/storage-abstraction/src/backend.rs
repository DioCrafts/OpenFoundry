use bytes::Bytes;
use futures::stream::BoxStream;

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("object not found: {0}")]
    NotFound(String),
    #[error("storage I/O error: {0}")]
    Io(#[from] object_store::Error),
    #[error("invalid path: {0}")]
    InvalidPath(String),
}

pub type StorageResult<T> = Result<T, StorageError>;

#[derive(Debug, Clone)]
pub struct ObjectMeta {
    pub path: String,
    pub size: u64,
    pub last_modified: chrono::DateTime<chrono::Utc>,
    pub content_type: Option<String>,
}

#[async_trait::async_trait]
pub trait StorageBackend: Send + Sync + 'static {
    /// Upload an object from bytes.
    async fn put(&self, path: &str, data: Bytes) -> StorageResult<()>;

    /// Download an object as bytes.
    async fn get(&self, path: &str) -> StorageResult<Bytes>;

    /// Download an object as a byte stream.
    async fn get_stream(
        &self,
        path: &str,
    ) -> StorageResult<BoxStream<'static, StorageResult<Bytes>>>;

    /// Delete an object.
    async fn delete(&self, path: &str) -> StorageResult<()>;

    /// List objects under a prefix.
    async fn list(&self, prefix: &str) -> StorageResult<Vec<ObjectMeta>>;

    /// Check if an object exists.
    async fn exists(&self, path: &str) -> StorageResult<bool>;

    /// Copy an object.
    async fn copy(&self, from: &str, to: &str) -> StorageResult<()>;

    /// Get object metadata.
    async fn head(&self, path: &str) -> StorageResult<ObjectMeta>;
}
