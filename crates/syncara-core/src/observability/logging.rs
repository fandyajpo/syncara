use tracing_subscriber::{EnvFilter, FmtSubscriber};

/// Initialize the global tracing subscriber.
///
/// Outputs structured JSON to stderr by default.
/// The log level is set via the `level` parameter (e.g., "info", "debug").
pub fn init(level: &str) {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(level));

    let subscriber = FmtSubscriber::builder()
        .json()
        .with_env_filter(filter)
        .with_target(true)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false)
        .finish();

    let _ = tracing::subscriber::set_global_default(subscriber)
        .map_err(|e| eprintln!("warning: failed to set tracing subscriber: {e}"));
}
