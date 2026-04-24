use async_nats::jetstream;
use serde::Serialize;
use thiserror::Error;

use crate::schemas::Event;

#[derive(Debug, Error)]
pub enum PublishError {
    #[error("serialization error: {0}")]
    Serialize(#[from] serde_json::Error),
    #[error("NATS publish error: {0}")]
    Nats(#[from] async_nats::error::Error<async_nats::jetstream::context::PublishErrorKind>),
}

/// Publisher wraps a JetStream context for type-safe event publishing.
#[derive(Clone)]
pub struct Publisher {
    js: jetstream::Context,
    source: String,
}

impl Publisher {
    pub fn new(js: jetstream::Context, source: impl Into<String>) -> Self {
        Self {
            js,
            source: source.into(),
        }
    }

    /// Publish a typed event to a subject.
    pub async fn publish<T: Serialize>(
        &self,
        subject: &str,
        event_type: &str,
        payload: T,
    ) -> Result<(), PublishError> {
        let event = Event::new(event_type, &self.source, payload);
        let bytes = serde_json::to_vec(&event)?;
        self.js.publish(subject.to_string(), bytes.into()).await?;
        tracing::debug!(subject, event_type, "event published");
        Ok(())
    }
}
