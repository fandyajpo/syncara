use std::sync::Arc;

use tokio::sync::{OwnedSemaphorePermit, Semaphore, TryAcquireError};

/// Global connection and WebSocket tunnel limiter.
///
/// Uses tokio semaphores to cap the number of concurrent TCP
/// connections and WebSocket tunnels.  Permits are acquired on accept
/// / upgrade and released automatically on drop (RAII).
pub struct ConnectionLimiter {
    conn_semaphore: Arc<Semaphore>,
    ws_semaphore: Arc<Semaphore>,
}

/// RAII guard for a connection permit.  The inner permit is never
/// directly read — holding it alive prevents the semaphore from
/// recycling the capacity.
#[allow(dead_code)]
pub struct ConnPermit(OwnedSemaphorePermit);

/// RAII guard for a WebSocket tunnel permit.
#[allow(dead_code)]
pub struct WsPermit(OwnedSemaphorePermit);

impl ConnectionLimiter {
    pub fn new(max_connections: u32, max_websocket: u32) -> Self {
        Self {
            conn_semaphore: Arc::new(Semaphore::new(max_connections as usize)),
            ws_semaphore: Arc::new(Semaphore::new(max_websocket as usize)),
        }
    }

    /// Try to acquire a connection permit.
    ///
    /// Returns `None` when at capacity — the caller should close the
    /// connection immediately.
    pub fn try_acquire_conn(&self) -> Option<ConnPermit> {
        match self
            .conn_semaphore
            .clone()
            .try_acquire_owned()
        {
            Ok(permit) => Some(ConnPermit(permit)),
            Err(TryAcquireError::NoPermits) => None,
            Err(TryAcquireError::Closed) => None,
        }
    }

    /// Try to acquire a WebSocket tunnel permit.
    ///
    /// Called during WS upgrade, on top of an already-held connection
    /// permit.  Returns `None` when at WS capacity.
    pub fn try_acquire_ws(&self) -> Option<WsPermit> {
        match self.ws_semaphore.clone().try_acquire_owned() {
            Ok(permit) => Some(WsPermit(permit)),
            Err(TryAcquireError::NoPermits) => None,
            Err(TryAcquireError::Closed) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permits_available_up_to_limit() {
        let limiter = ConnectionLimiter::new(3, 2);
        let mut permits = Vec::new();

        for _ in 0..3 {
            permits.push(limiter.try_acquire_conn().expect("expected permit"));
        }
        // All 3 permits held — 4th should fail.
        assert!(limiter.try_acquire_conn().is_none());

        // Drop one permit; now one slot is free.
        permits.pop();
        assert!(limiter.try_acquire_conn().is_some());
    }

    #[test]
    fn permit_drop_releases() {
        let limiter = ConnectionLimiter::new(1, 1);
        let p = limiter.try_acquire_conn();
        assert!(p.is_some());
        drop(p);

        // After dropping, a new permit is available.
        assert!(limiter.try_acquire_conn().is_some());
    }

    #[test]
    fn ws_permits_separate_pool() {
        let limiter = ConnectionLimiter::new(10, 2);
        let mut ws_permits = Vec::new();

        for _ in 0..2 {
            ws_permits.push(limiter.try_acquire_ws().expect("expected WS permit"));
        }
        assert!(limiter.try_acquire_ws().is_none());

        // Connection permits are still available.
        assert!(limiter.try_acquire_conn().is_some());
    }
}
