pub mod cookie;
pub mod ip_hash;

use std::time::{Duration, Instant};

use crate::balancer::UpstreamAddr;

/// Unique key for a session.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct SessionKey {
    pub identifier: String,
}

/// A binding entry with expiry.
pub(crate) struct SessionEntry {
    pub upstream: UpstreamAddr,
    pub expires_at: Instant,
}

/// Interface for sticky session storage.
pub trait SessionStore: Send + Sync {
    fn lookup(&self, key: &SessionKey) -> Option<UpstreamAddr>;
    fn set(&self, key: SessionKey, upstream: UpstreamAddr, ttl: Duration);
    fn remove(&self, key: &SessionKey);
}
