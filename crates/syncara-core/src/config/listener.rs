/// Listener configuration — defines a single TCP/TLS accept port.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListenerConfig {
    /// Port to bind (1–65535).
    pub port: u16,

    /// Bind address (default: "0.0.0.0").
    #[serde(default = "default_host")]
    pub host: String,

    /// Optional TLS configuration.
    #[serde(default)]
    pub tls: Option<TlsConfig>,
}

/// TLS certificate and key paths.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TlsConfig {
    pub cert: String,
    pub key: String,
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}
