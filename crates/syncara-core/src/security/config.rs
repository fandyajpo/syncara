#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SecurityConfig {
    #[serde(default)]
    pub request_timeout: Option<String>,

    #[serde(default)]
    pub upstream_timeout: Option<String>,

    #[serde(default)]
    pub websocket_timeout: Option<String>,

    #[serde(default)]
    pub http1_header_read_timeout: Option<String>,

    #[serde(default)]
    pub rate_limit: Option<RateLimitConfig>,

    #[serde(default)]
    pub connections: Option<ConnectionLimitsConfig>,

    #[serde(default)]
    pub blocklist: Option<BlocklistConfig>,

    #[serde(default)]
    pub max_body_size: Option<String>,

    #[serde(default)]
    pub tcp_keepalive: Option<String>,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            request_timeout: Some("30s".into()),
            upstream_timeout: Some("30s".into()),
            websocket_timeout: Some("30m".into()),
            http1_header_read_timeout: None,
            rate_limit: None,
            connections: None,
            blocklist: None,
            max_body_size: None,
            tcp_keepalive: None,
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RateLimitConfig {
    #[serde(default)]
    pub enabled: bool,

    #[serde(default = "default_rpm")]
    pub requests_per_minute: u32,
}

fn default_rpm() -> u32 {
    300
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConnectionLimitsConfig {
    #[serde(default = "default_max_active")]
    pub max_active: u32,

    #[serde(default = "default_ws_max")]
    pub websocket_max: u32,

    #[serde(default)]
    pub per_upstream: Option<u32>,
}

fn default_max_active() -> u32 {
    10_000
}

fn default_ws_max() -> u32 {
    5_000
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BlocklistConfig {
    #[serde(default)]
    pub allowed_cidrs: Vec<String>,

    #[serde(default)]
    pub denied_cidrs: Vec<String>,

    #[serde(default)]
    pub auto_block_after: Option<u32>,

    #[serde(default = "default_block_ttl")]
    pub auto_block_ttl: String,
}

fn default_block_ttl() -> String {
    "5m".into()
}
