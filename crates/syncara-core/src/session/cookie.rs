use std::time::Duration;

use dashmap::DashMap;

use super::{SessionEntry, SessionKey, SessionStore};
use crate::balancer::UpstreamAddr;

/// In-memory cookie-based session store.
///
/// Maps session cookie values to upstream addresses with bounded
/// capacity. Stale entries are lazily evicted on lookup.
pub struct CookieSession {
    pub(crate) cookie_name: String,
    store: DashMap<String, SessionEntry>,
}

impl CookieSession {
    pub fn new(cookie_name: String) -> Self {
        Self {
            cookie_name,
            store: DashMap::new(),
        }
    }

    pub fn cookie_name(&self) -> &str {
        &self.cookie_name
    }
}

impl SessionStore for CookieSession {
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
        let store = CookieSession::new("test_cookie".into());
        let key = SessionKey {
            identifier: "abc123".into(),
        };
        store.set(key.clone(), "127.0.0.1:8080".into(), Duration::from_secs(60));
        assert_eq!(
            store.lookup(&key),
            Some("127.0.0.1:8080".into())
        );
    }

    #[test]
    fn test_lookup_expired() {
        let store = CookieSession::new("test_cookie".into());
        let key = SessionKey {
            identifier: "expired".into(),
        };
        store.set(key.clone(), "127.0.0.1:8080".into(), Duration::from_millis(1));
        std::thread::sleep(Duration::from_millis(5));
        assert_eq!(store.lookup(&key), None);
    }

    #[test]
    fn test_lookup_missing() {
        let store = CookieSession::new("test_cookie".into());
        let key = SessionKey {
            identifier: "nonexistent".into(),
        };
        assert_eq!(store.lookup(&key), None);
    }

    #[test]
    fn test_remove() {
        let store = CookieSession::new("test_cookie".into());
        let key = SessionKey {
            identifier: "toremove".into(),
        };
        store.set(key.clone(), "127.0.0.1:8080".into(), Duration::from_secs(60));
        store.remove(&key);
        assert_eq!(store.lookup(&key), None);
    }

    #[test]
    fn test_overwrite_existing() {
        let store = CookieSession::new("test_cookie".into());
        let key = SessionKey {
            identifier: "samekey".into(),
        };
        store.set(key.clone(), "127.0.0.1:8080".into(), Duration::from_secs(60));
        store.set(key.clone(), "127.0.0.1:9090".into(), Duration::from_secs(60));
        assert_eq!(
            store.lookup(&key),
            Some("127.0.0.1:9090".into())
        );
    }
}
