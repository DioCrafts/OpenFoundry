use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct HealthStatus {
    pub status: &'static str,
    pub service: String,
    pub version: &'static str,
    pub timestamp: DateTime<Utc>,
}

impl HealthStatus {
    pub fn ok(service: impl Into<String>) -> Self {
        Self {
            status: "ok",
            service: service.into(),
            version: env!("CARGO_PKG_VERSION"),
            timestamp: Utc::now(),
        }
    }
}
