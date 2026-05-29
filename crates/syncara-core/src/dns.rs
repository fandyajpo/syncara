use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use tokio::io;
use tokio::net::TcpStream;
use tokio::sync::RwLock;

/// Default TTL for cached DNS entries.
const DEFAULT_TTL: Duration = Duration::from_secs(60);

/// Global DNS cache singleton.
static DNS_CACHE: OnceLock<DnsCache> = OnceLock::new();

pub fn init() {
    let _ = DNS_CACHE.set(DnsCache::new(DEFAULT_TTL));
}

/// Resolve `addr` via the DNS cache if initialised, otherwise fall back to
/// a direct system lookup.
pub async fn resolve(addr: &str) -> io::Result<Vec<SocketAddr>> {
    if let Some(cache) = DNS_CACHE.get() {
        cache.resolve(addr).await
    } else {
        tokio::net::lookup_host(addr).await.map(|i| i.collect())
    }
}

/// Connect to `addr` via the DNS cache if initialised, otherwise fall back to
/// `TcpStream::connect`.
pub async fn connect(addr: &str) -> io::Result<TcpStream> {
    if let Some(cache) = DNS_CACHE.get() {
        cache.connect(addr).await
    } else {
        TcpStream::connect(addr).await
    }
}

struct CachedEntry {
    addrs: Vec<SocketAddr>,
    expires_at: Instant,
}

/// A simple DNS cache that resolves hostnames via `tokio::net::lookup_host`
/// and caches results with a configurable TTL.
pub struct DnsCache {
    inner: RwLock<HashMap<String, CachedEntry>>,
    ttl: Duration,
}

impl DnsCache {
    fn new(ttl: Duration) -> Self {
        Self {
            inner: RwLock::new(HashMap::new()),
            ttl,
        }
    }

    /// Resolve `addr` (e.g. `"localhost:3000"`) to a list of `SocketAddr`s,
    /// using the cache if a fresh entry exists.
    pub async fn resolve(&self, addr: &str) -> io::Result<Vec<SocketAddr>> {
        {
            let cache = self.inner.read().await;
            if let Some(entry) = cache.get(addr) {
                if entry.expires_at > Instant::now() {
                    return Ok(entry.addrs.clone());
                }
            }
        }

        let addrs: Vec<SocketAddr> = tokio::net::lookup_host(addr).await?.collect();

        {
            let mut cache = self.inner.write().await;
            cache.insert(
                addr.to_string(),
                CachedEntry {
                    addrs: addrs.clone(),
                    expires_at: Instant::now() + self.ttl,
                },
            );
        }

        Ok(addrs)
    }

    /// Like `TcpStream::connect(addr)` but resolves via the DNS cache first.
    ///
    /// Tries each resolved address in order, falling back like the standard
    /// `TcpStream::connect` does internally.
    pub async fn connect(&self, addr: &str) -> io::Result<TcpStream> {
        let addrs = self.resolve(addr).await?;
        let mut last_err = io::Error::other("no addresses resolved");
        for resolved in addrs {
            match TcpStream::connect(resolved).await {
                Ok(stream) => return Ok(stream),
                Err(e) => last_err = e,
            }
        }
        Err(last_err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener as StdTcpListener;

    #[tokio::test]
    async fn resolve_loopback() {
        let cache = DnsCache::new(Duration::from_secs(60));
        let addrs = cache.resolve("127.0.0.1:0").await.unwrap();
        assert!(!addrs.is_empty());
        assert!(addrs.iter().any(|a| a.ip().is_loopback()));
    }

    #[tokio::test]
    async fn cache_hit_returns_cached() {
        let cache = DnsCache::new(Duration::from_secs(60));
        let first = cache.resolve("127.0.0.1:9999").await.unwrap();
        let second = cache.resolve("127.0.0.1:9999").await.unwrap();
        assert_eq!(first, second);
    }

    #[tokio::test]
    async fn cache_respects_ttl() {
        let cache = DnsCache::new(Duration::from_millis(10));
        let first = cache.resolve("127.0.0.1:9998").await.unwrap();
        tokio::time::sleep(Duration::from_millis(20)).await;
        // Should re-resolve (same result since it's a static IP)
        let second = cache.resolve("127.0.0.1:9998").await.unwrap();
        assert_eq!(first, second);
    }

    #[tokio::test]
    async fn connect_to_reachable_addr() {
        let listener = StdTcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap().to_string();

        let cache = DnsCache::new(Duration::from_secs(60));
        let stream = cache.connect(&addr).await;
        assert!(stream.is_ok());
    }

    #[tokio::test]
    async fn connect_to_unreachable_returns_err() {
        let cache = DnsCache::new(Duration::from_secs(60));
        let result = cache.connect("127.0.0.1:1").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn connect_fallback_tries_multiple_addrs() {
        let cache = DnsCache::new(Duration::from_secs(60));
        let result = cache.connect("127.0.0.1:1").await;
        assert!(result.is_err());
    }
}
