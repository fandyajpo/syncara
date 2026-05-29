use thiserror::Error;

/// Unified error type for Syncara core.
#[derive(Error, Debug)]
pub enum SyncaraError {
    #[error("configuration error: {0}")]
    Config(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("TLS error: {0}")]
    Tls(String),

    #[error("proxy error: {0}")]
    Proxy(String),

    #[error("upstream unavailable: {0}")]
    UpstreamUnavailable(String),

    #[error("shutdown timeout exceeded")]
    ShutdownTimeout,
}
