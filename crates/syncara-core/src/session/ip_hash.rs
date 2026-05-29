use std::time::Duration;

use dashmap::DashMap;

use super::{SessionEntry, SessionKey, SessionStore};
use crate::balancer::UpstreamAddr;

/// IP-hash session store.
///
/// Uses a DashMap keyed by client IP string. Each client IP is pinned
/// to the same upstream once assigned. Entries expire after the
/// configured TTL.
pub struct IpHashSession {
    store: DashMap<String, SessionEntry>,
}

impl IpHashSession {
    pub fn new() -> Self {
        Self {
            store: DashMap::new(),
        }
    }
}

impl Default for IpHashSession {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionStore for IpHashSession {
    fn lookup(&self, key: &SessionKey) -> Option<UpstreamAddr> {
        let entry = self.store.get(&key.identifier)?;
        if entry.expires_at < std::time::Instant::now() {
            let id = key.identifier.clone();
            drop(entry);
            self.store.remove(&id);
            return None;
        }
        Some(entry.upstream.clone())
    }

    fn set(&self, key: SessionKey, upstream: UpstreamAddr, ttl: Duration) {
        self.store.insert(
            key.identifier,
            SessionEntry {
                upstream,
                expires_at: std::time::Instant::now() + ttl,
            },
        );
    }

    fn remove(&self, key: &SessionKey) {
        self.store.remove(&key.identifier);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lookup_set_roundtrip() {
        let store = IpHashSession::new();
        let key = SessionKey {
            identifier: "192.168.1.1".into(),
        };
        store.set(key.clone(), "10.0.0.1:8080".into(), Duration::from_secs(60));
        assert_eq!(store.lookup(&key), Some("10.0.0.1:8080".into()));
    }

    #[test]
    fn test_lookup_expired() {
        let store = IpHashSession::new();
        let key = SessionKey {
            identifier: "10.0.0.1".into(),
        };
        store.set(key.clone(), "10.0.0.1:8080".into(), Duration::from_millis(1));
        std::thread::sleep(Duration::from_millis(5));
        assert_eq!(store.lookup(&key), None);
    }

    #[test]
    fn test_different_ips_different_bindings() {
        let store = IpHashSession::new();
        let key1 = SessionKey {
            identifier: "1.1.1.1".into(),
        };
        let key2 = SessionKey {
            identifier: "2.2.2.2".into(),
        };
        store.set(key1.clone(), "10.0.0.1:8080".into(), Duration::from_secs(60));
        store.set(key2.clone(), "10.0.0.2:8080".into(), Duration::from_secs(60));
        assert_eq!(store.lookup(&key1), Some("10.0.0.1:8080".into()));
        assert_eq!(store.lookup(&key2), Some("10.0.0.2:8080".into()));
    }
}
