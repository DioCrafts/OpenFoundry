use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A strongly-typed wrapper around UUID v7 for entity identification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TypedId<T: ?Sized>(Uuid, #[serde(skip)] std::marker::PhantomData<T>);

impl<T: ?Sized> TypedId<T> {
    /// Create a new ID using UUID v7 (time-ordered).
    pub fn new() -> Self {
        Self(Uuid::now_v7(), std::marker::PhantomData)
    }

    /// Get the inner UUID value.
    pub fn into_inner(self) -> Uuid {
        self.0
    }
}

impl<T: ?Sized> Default for TypedId<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: ?Sized> std::fmt::Display for TypedId<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
