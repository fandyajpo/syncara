pub mod signals;
pub mod status;

use std::net::SocketAddr;
use std::sync::Arc;

use crate::balancer::UpstreamPool;
use crate::config::Config;
use crate::health::HealthMonitor;
use crate::observability;
use crate::proxy::ProxyEngine;
use crate::routing::Router;

fn build_pools(config: &Config) -> Vec<UpstreamPool> {
    config.pools.iter().map(UpstreamPool::from_config).collect()
}

/// Store the config file path for reloads.
static CONFIG_PATH: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();

/// Set the config file path for reload support.
pub fn set_config_path(path: &str) {
    let _ = CONFIG_PATH.set(std::path::PathBuf::from(path));
}

/// Entry point called from `lib.rs::bootstrap`.
pub async fn run(config: Config) -> anyhow::Result<()> {
    let router = Arc::new(tokio::sync::RwLock::new(Router::new(&config)));
    let pools = Arc::new(tokio::sync::RwLock::new(build_pools(&config)));
    let config_arc = Arc::new(tokio::sync::RwLock::new(config));
    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

    // ---- Observability ----
    observability::metrics::init();
    observability::realtime::init();

    // ---- DNS cache ----
    crate::dns::init();

    // ---- Security ----
    {
        let cfg = config_arc.read().await;
        crate::security::init(&cfg.security);
    }

    // ---- Brain ----
    crate::brain::tracker::init();

    // ---- Status snapshot ----
    {
        let p = pools.read().await;
        crate::runtime::status::init(&p);
    }

    // ---- Admin server ----
    let drain_timeout: std::time::Duration;
    {
        let cfg = config_arc.read().await;
        let admin_addr: SocketAddr = format!("{}:{}", cfg.admin.host, cfg.admin.port)
            .parse()
            .expect("invalid admin address");
        let admin_key = cfg.admin.api_key.clone();
        drain_timeout = cfg.admin.drain_timeout.as_ref()
            .and_then(|s| crate::config::validate::parse_duration(s).ok())
            .map(std::time::Duration::from_secs)
            .unwrap_or(std::time::Duration::from_secs(5));
        tokio::spawn(observability::admin::serve(admin_addr, admin_key, shutdown_rx.clone()));
    }

    // ---- Health monitor (stored in Option for reload) ----
    let mut health_handle: Option<HealthMonitor>;
    {
        let new_h = start_health_checks(&config_arc, &pools, &shutdown_rx).await;
        health_handle = Some(new_h);
    }

    // ---- Proxy engine ----
    let proxy = ProxyEngine::new(
        config_arc.clone(),
        router.clone(),
        pools.clone(),
        shutdown_rx.clone(),
    );

    let proxy_handle = tokio::spawn(async move {
        if let Err(e) = proxy.run().await {
            tracing::error!(error = %e, "proxy engine stopped with error");
        }
    });

    // ---- Signal handling loop ----
    loop {
        match signals::next_signal().await {
            signals::Signal::Shutdown => {
                tracing::info!("shutdown signal received, draining connections");
                let _ = shutdown_tx.send(true);
                tokio::time::sleep(drain_timeout).await;
                break;
            }
            signals::Signal::Reload => {
                tracing::info!("config reload signal received, reloading...");

                let config_path = CONFIG_PATH.get_or_init(|| std::path::PathBuf::from("syncara.yml"));
                match crate::config::load(config_path) {
                    Ok(new_cfg) => {
                        if let Err(e) = crate::config::validate(&new_cfg) {
                            tracing::error!(error = %e, "config reload — validation failed, keeping old config");
                            observability::metrics::get()
                                .config_reloads_total
                                .with_label_values(&["error"])
                                .inc();
                            continue;
                        }
                        tracing::info!("config reload — new configuration is valid");

                        // Build new router & pools.
                        let new_router = Router::new(&new_cfg);
                        let new_pools = build_pools(&new_cfg);

                        // Re-init security layer.
                        crate::security::init(&new_cfg.security);

                        // Re-init status snapshot with new pools.
                        crate::runtime::status::init(&new_pools);

                        // Atomically swap proxy state.
                        {
                            let mut cfg = config_arc.write().await;
                            *cfg = new_cfg;
                        }
                        {
                            let mut r = router.write().await;
                            *r = new_router;
                        }
                        {
                            let mut p = pools.write().await;
                            *p = new_pools;
                        }

                        // Restart health checks: drop old (aborts tasks), start new.
                        drop(health_handle.take());
                        let new_h = start_health_checks(&config_arc, &pools, &shutdown_rx).await;
                        health_handle = Some(new_h);

                        observability::metrics::get()
                            .config_reloads_total
                            .with_label_values(&["success"])
                            .inc();

                        tracing::info!("config reload — successfully applied");
                    }
                    Err(e) => {
                        tracing::error!(error = %e, "config reload — failed to load new config");
                        observability::metrics::get()
                            .config_reloads_total
                            .with_label_values(&["error"])
                            .inc();
                    }
                }
            }
        }
    }

    let _ = proxy_handle.await;
    tracing::info!("syncara stopped");
    Ok(())
}

/// Start health checks for all pools and return the monitor handle.
async fn start_health_checks(
    config_arc: &Arc<tokio::sync::RwLock<Config>>,
    pools: &Arc<tokio::sync::RwLock<Vec<UpstreamPool>>>,
    shutdown_rx: &tokio::sync::watch::Receiver<bool>,
) -> HealthMonitor {
    let cfg = config_arc.read().await;
    let config_pools = cfg.pools.clone();
    drop(cfg);

    let runtime_pools: Vec<UpstreamPool> = {
        let p = pools.read().await;
        p.clone()
    };

    // Independent shutdown channel for health checks so they
    // can be cancelled without triggering the main shutdown.
    let (health_tx, health_rx) = tokio::sync::watch::channel(false);
    let mut main_rx = shutdown_rx.clone();
    tokio::spawn(async move {
        let _ = main_rx.changed().await;
        let _ = health_tx.send(true);
    });

    HealthMonitor::start(&config_pools, Arc::new(runtime_pools), health_rx)
}
