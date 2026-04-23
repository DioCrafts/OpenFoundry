use bytes::Bytes;
use futures::stream::BoxStream;
use futures::TryStreamExt;
use object_store::local::LocalFileSystem;
use object_store::ObjectStore;
use object_store::path::Path;
use std::sync::Arc;

use crate::backend::{ObjectMeta, StorageBackend, StorageError, StorageResult};

#[derive(Clone)]
pub struct LocalStorage {
    store: Arc<LocalFileSystem>,
    root: String,
}

impl LocalStorage {
    pub fn new(root: &str) -> StorageResult<Self> {
        let store = LocalFileSystem::new_with_prefix(root)
            .map_err(StorageError::Io)?;
        Ok(Self { store: Arc::new(store), root: root.to_string() })
    }

    pub fn root(&self) -> &str {
        &self.root
    }
}

fn to_path(path: &str) -> StorageResult<Path> {
    Path::parse(path).map_err(|e| StorageError::InvalidPath(e.to_string()))
}

fn convert_meta(m: &object_store::ObjectMeta) -> ObjectMeta {
    ObjectMeta {
        path: m.location.to_string(),
        size: m.size as u64,
        last_modified: m.last_modified.into(),
        content_type: None,
    }
}

#[async_trait::async_trait]
impl StorageBackend for LocalStorage {
    async fn put(&self, path: &str, data: Bytes) -> StorageResult<()> {
        let p = to_path(path)?;
        self.store.put(&p, data.into()).await.map_err(StorageError::Io)?;
        Ok(())
    }

    async fn get(&self, path: &str) -> StorageResult<Bytes> {
        let p = to_path(path)?;
        let result = self.store.get(&p).await.map_err(StorageError::Io)?;
        result.bytes().await.map_err(StorageError::Io)
    }

    async fn get_stream(&self, path: &str) -> StorageResult<BoxStream<'static, StorageResult<Bytes>>> {
        let p = to_path(path)?;
        let result = self.store.get(&p).await.map_err(StorageError::Io)?;
        let stream = result.into_stream().map_err(StorageError::Io);
        Ok(Box::pin(stream))
    }

    async fn delete(&self, path: &str) -> StorageResult<()> {
        let p = to_path(path)?;
        self.store.delete(&p).await.map_err(StorageError::Io)
    }

    async fn list(&self, prefix: &str) -> StorageResult<Vec<ObjectMeta>> {
        let p = to_path(prefix)?;
        let mut items = vec![];
        let mut stream = self.store.list(Some(&p));
        while let Some(meta) = stream.try_next().await.map_err(StorageError::Io)? {
            items.push(convert_meta(&meta));
        }
        Ok(items)
    }

    async fn exists(&self, path: &str) -> StorageResult<bool> {
        match self.head(path).await {
            Ok(_) => Ok(true),
            Err(StorageError::NotFound(_)) => Ok(false),
            Err(e) => Err(e),
        }
    }

    async fn copy(&self, from: &str, to: &str) -> StorageResult<()> {
        let f = to_path(from)?;
        let t = to_path(to)?;
        self.store.copy(&f, &t).await.map_err(StorageError::Io)
    }

    async fn head(&self, path: &str) -> StorageResult<ObjectMeta> {
        let p = to_path(path)?;
        let meta = self.store.head(&p).await.map_err(|e| match e {
            object_store::Error::NotFound { .. } => StorageError::NotFound(path.to_string()),
            other => StorageError::Io(other),
        })?;
        Ok(convert_meta(&meta))
    }
}

