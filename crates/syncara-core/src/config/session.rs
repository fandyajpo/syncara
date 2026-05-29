/// Sticky session type.
#[derive(Debug, Clone, serde::Deserialize)]
pub enum StickyType {
    #[serde(rename = "cookie")]
    Cookie,
    #[serde(rename = "ip-hash")]
    IpHash,
}

impl Default for StickyType {
    fn default() -> Self {
        Self::Cookie
    }
}

/// Sticky session configuration.
///
/// Controls how Syncara pins a client to a specific upstream.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionConfig {
    /// Enable sticky sessions for this pool.
    #[serde(default)]
    pub enabled: bool,

    /// Sticky strategy type (default: cookie).
    #[serde(default, rename = "type")]
    pub sticky_type: StickyType,

    /// Cookie name for cookie-based affinity (default: "_syncara_session").
    #[serde(default = "default_cookie_name")]
    pub cookie_name: String,

    /// Session TTL (default: "24h").
    #[serde(default = "default_session_ttl")]
    pub ttl: String,
}

fn default_cookie_name() -> String {
    "_syncara_session".to_string()
}

fn default_session_ttl() -> String {
    "24h".to_string()
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            sticky_type: StickyType::Cookie,
            cookie_name: default_cookie_name(),
            ttl: default_session_ttl(),
        }
    }
}
