use async_nats::jetstream::{self, consumer::PullConsumer};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SubscribeError {
    #[error("NATS stream error: {0}")]
    Stream(String),
    #[error("NATS consumer error: {0}")]
    Consumer(String),
}

/// Create or get an existing JetStream stream.
pub async fn ensure_stream(
    js: &jetstream::Context,
    name: &str,
    subjects: &[&str],
) -> Result<jetstream::stream::Stream, SubscribeError> {
    let config = jetstream::stream::Config {
        name: name.to_string(),
        subjects: subjects.iter().map(|s| format!("{s}.>")).collect(),
        retention: jetstream::stream::RetentionPolicy::Limits,
        max_messages: 1_000_000,
        max_age: std::time::Duration::from_secs(7 * 24 * 3600), // 7 days
        ..Default::default()
    };

    js.get_or_create_stream(config)
        .await
        .map_err(|e| SubscribeError::Stream(e.to_string()))
}

/// Create a durable pull consumer on a stream.
pub async fn create_consumer(
    stream: &jetstream::stream::Stream,
    consumer_name: &str,
    filter_subject: Option<&str>,
) -> Result<PullConsumer, SubscribeError> {
    let mut config = jetstream::consumer::pull::Config {
        durable_name: Some(consumer_name.to_string()),
        ..Default::default()
    };
    if let Some(filter) = filter_subject {
        config.filter_subject = filter.to_string();
    }

    stream
        .create_consumer(config)
        .await
        .map_err(|e| SubscribeError::Consumer(e.to_string()))
}
