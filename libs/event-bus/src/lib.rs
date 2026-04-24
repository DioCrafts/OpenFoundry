pub mod publisher;
pub mod schemas;
pub mod subscriber;
pub mod topics;

pub use publisher::Publisher;
pub use schemas::Event;

/// Connect to NATS and return a JetStream context.
pub async fn connect(
    url: &str,
) -> Result<async_nats::jetstream::Context, async_nats::ConnectError> {
    let client = async_nats::connect(url).await?;
    Ok(async_nats::jetstream::new(client))
}
