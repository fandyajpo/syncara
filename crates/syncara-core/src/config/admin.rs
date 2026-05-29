/// Admin / metrics server configuration.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdminConfig {
    /// Metrics endpoint port (default: 9090).
    #[serde(default = "default_admin_port")]
    pub port: u16,

    /// Bind address (default: "127.0.0.1").
    #[serde(default = "default_admin_host")]
    pub host: String,

    /// Optional API key for admin endpoint authentication.
    /// When set, all admin endpoints require `Authorization: Bearer <key>`.
    #[serde(default)]
    pub api_key: Option<String>,

    /// Drain timeout on shutdown (e.g. "10s").
    /// Defaults to 5 seconds.
    #[serde(default)]
    pub drain_timeout: Option<String>,
}

fn default_admin_port() -> u16 {
    9090
}

fn default_admin_host() -> String {
    "127.0.0.1".to_string()
}

impl Default for AdminConfig {
    fn default() -> Self {
        Self {
            port: default_admin_port(),
            host: default_admin_host(),
            api_key: None,
            drain_timeout: None,
        }
    }
}
