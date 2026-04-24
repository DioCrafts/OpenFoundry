use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Envelope for all events published through the bus.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event<T: Serialize> {
    /// Unique event ID (UUID v7).
    pub id: Uuid,
    /// When the event occurred.
    pub timestamp: DateTime<Utc>,
    /// Dot-separated event type (e.g., "auth.user.created").
    pub event_type: String,
    /// Source service name.
    pub source: String,
    /// The event payload.
    pub payload: T,
}

impl<T: Serialize> Event<T> {
    pub fn new(event_type: impl Into<String>, source: impl Into<String>, payload: T) -> Self {
        Self {
            id: Uuid::now_v7(),
            timestamp: Utc::now(),
            event_type: event_type.into(),
            source: source.into(),
            payload,
        }
    }
}
