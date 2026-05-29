pub mod http;
pub mod websocket;

use std::net::SocketAddr;
use std::sync::Arc;

use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;
use tracing::info;

use crate::balancer::UpstreamPool;
use crate::config::Config;
use crate::routing::Router;

/// The central proxy engine.
pub struct ProxyEngine {
    config: Arc<tokio::sync::RwLock<Config>>,
    router: Arc<tokio::sync::RwLock<Router>>,
    pools: Arc<tokio::sync::RwLock<Vec<UpstreamPool>>>,
    shutdown_rx: tokio::sync::watch::Receiver<bool>,
}

impl ProxyEngine {
    pub fn new(
        config: Arc<tokio::sync::RwLock<Config>>,
        router: Arc<tokio::sync::RwLock<Router>>,
        pools: Arc<tokio::sync::RwLock<Vec<UpstreamPool>>>,
        shutdown_rx: tokio::sync::watch::Receiver<bool>,
    ) -> Self {
        Self {
            config,
            router,
            pools,
            shutdown_rx,
        }
    }

    /// Hot-reload the config, pools, and router.
    ///
    /// Called on SIGHUP. Validates the new config, builds fresh pools
    /// and router, then atomically swaps them. In-flight requests
    /// continue with the old data until they complete.
    pub async fn rebuild(
        &self,
        new_config: Config,
        new_router: Router,
        new_pools: Vec<UpstreamPool>,
    ) {
        // Swap config (already validated by caller).
        {
            let mut cfg = self.config.write().await;
            *cfg = new_config;
        }

        // Rebuild router.
        {
            let mut r = self.router.write().await;
            *r = new_router;
        }

        // Swap pools.
        {
            let mut p = self.pools.write().await;
            *p = new_pools;
        }

        info!("config hot-reload completed");
    }

    /// Bind all listeners from config and start accepting connections.
    pub async fn run(&self) -> anyhow::Result<()> {
        let keepalive = crate::security::get().keepalive;
        let client = http::make_client(keepalive);

        let cfg = self.config.read().await;
        let listeners_cfg = cfg.listeners.clone();
        drop(cfg);

        if listeners_cfg.is_empty() {
            anyhow::bail!("no listeners configured");
        }

        let mut handles = Vec::new();

        for listener_cfg in &listeners_cfg {
            let addr: SocketAddr = format!("{}:{}", listener_cfg.host, listener_cfg.port)
                .parse()
                .expect("valid listener address from config");

            let tcp_listener = TcpListener::bind(addr).await?;
            info!(%addr, "listener started");

            let client = client.clone();
            let router = self.router.clone();
            let pools = self.pools.clone();
            let mut shutdown_rx = self.shutdown_rx.clone();

            // Build TLS acceptor if configured.
            let tls_acceptor: Option<TlsAcceptor> = listener_cfg
                .tls
                .as_ref()
                .map(|tls_cfg| build_tls_acceptor(tls_cfg));

            let handle = tokio::spawn(async move {
                loop {
                    tokio::select! {
                        result = tcp_listener.accept() => {
                            match result {
                                Ok((stream, peer)) => {
                                    // Clone the TLS acceptor into each spawn
                                    // so the borrow is 'static.
                                    if let Some(ref acceptor) = tls_acceptor {
                                        let acceptor = acceptor.clone();
                                        let client = client.clone();
                                        let router = router.clone();
                                        let pools = pools.clone();
                                        tokio::spawn(async move {
                                            match acceptor.accept(stream).await {
                                                Ok(tls_stream) => {
                                                    http::handle_connection(
                                                        hyper_util::rt::TokioIo::new(tls_stream),
                                                        peer, client, router, pools,
                                                    ).await;
                                                }
                                                Err(e) => {
                                                    tracing::warn!(%peer, error = %e, "tls handshake failed");
                                                }
                                            }
                                        });
                                    } else {
                                        let client = client.clone();
                                        let router = router.clone();
                                        let pools = pools.clone();
                                        tokio::spawn(async move {
                                            http::handle_connection(
                                                hyper_util::rt::TokioIo::new(stream),
                                                peer, client, router, pools,
                                            ).await;
                                        });
                                    }
                                }
                                Err(e) => {
                                    tracing::warn!(%addr, error = %e, "accept error");
                                }
                            }
                        }
                        _ = shutdown_rx.changed() => {
                            info!(%addr, "listener shutting down");
                            break;
                        }
                    }
                }
            });

            handles.push(handle);
        }

        for handle in handles {
            let _ = handle.await;
        }

        info!("all listeners stopped");
        Ok(())
    }
}

/// Build a TLS acceptor from listener TLS config.
fn build_tls_acceptor(tls_cfg: &crate::config::listener::TlsConfig) -> TlsAcceptor {
    use std::fs;

    let certs = {
        let cert_bytes = fs::read(&tls_cfg.cert)
            .unwrap_or_else(|e| panic!("failed to read TLS cert '{}': {}", tls_cfg.cert, e));
        let certs = rustls_pemfile::certs(&mut &*cert_bytes)
            .collect::<Result<Vec<_>, _>>()
            .unwrap_or_else(|e| panic!("failed to parse TLS cert '{}': {}", tls_cfg.cert, e));
        if certs.is_empty() {
            panic!("TLS cert '{}' contains no certificates", tls_cfg.cert);
        }
        certs
    };

    let key = {
        let key_bytes = fs::read(&tls_cfg.key)
            .unwrap_or_else(|e| panic!("failed to read TLS key '{}': {}", tls_cfg.key, e));
        let keys = rustls_pemfile::pkcs8_private_keys(&mut &*key_bytes)
            .collect::<Result<Vec<_>, _>>()
            .unwrap_or_else(|e| panic!("failed to parse TLS key '{}': {}", tls_cfg.key, e));
        if keys.is_empty() {
            panic!("TLS key '{}' contains no private keys", tls_cfg.key);
        }
        keys.into_iter().next()
            .expect("TLS key file is empty")
    };

    let config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, rustls::pki_types::PrivateKeyDer::Pkcs8(key))
        .expect("invalid TLS configuration");

    TlsAcceptor::from(Arc::new(config))
}
