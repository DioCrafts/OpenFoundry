use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Standard timestamp fields for all entities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Timestamps {
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Default for Timestamps {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            created_at: now,
            updated_at: now,
        }
    }
}
