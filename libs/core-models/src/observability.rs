use tracing_subscriber::EnvFilter;

pub fn init_tracing(service: &str) {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let log_format = std::env::var("LOG_FORMAT").unwrap_or_else(|_| "plain".to_string());

    let init_result = if log_format.eq_ignore_ascii_case("json") {
        tracing_subscriber::fmt()
            .json()
            .with_env_filter(env_filter)
            .try_init()
    } else {
        tracing_subscriber::fmt()
            .with_env_filter(env_filter)
            .try_init()
    };

    if let Err(error) = init_result {
        tracing::debug!(%service, %error, "tracing subscriber already initialized");
    }

    tracing::info!(
        service,
        version = env!("CARGO_PKG_VERSION"),
        log_format = %log_format,
        "observability initialized"
    );
}
