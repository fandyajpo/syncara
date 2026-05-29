pub mod brain;
pub mod config;
pub mod dns;
pub mod proxy;
pub mod routing;
pub mod balancer;
pub mod health;
pub mod security;
pub mod session;
pub mod observability;
pub mod runtime;
pub mod support;

use std::path::Path;

use crate::config::Config;

/// Entry point called from the CLI binary.
pub fn bootstrap(config_path: &str, log_level: &str, validate_only: bool) -> anyhow::Result<()> {
    observability::logging::init(log_level);

    let config = config::load(Path::new(config_path))?;
    config::validate(&config)?;

    if validate_only {
        tracing::info!("configuration is valid");
        return Ok(());
    }

    tracing::info!(config_path, "starting syncara");

    runtime::set_config_path(config_path);

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_name("syncara-worker")
        .build()?;

    rt.block_on(runtime::run(config))
}

/// Bootstrap with a pre-built config (zero-config mode).
pub fn bootstrap_with_config(config: Config, log_level: &str) -> anyhow::Result<()> {
    observability::logging::init(log_level);
    config::validate(&config)?;

    tracing::info!("starting syncara (inline config)");

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_name("syncara-worker")
        .build()?;

    rt.block_on(runtime::run(config))
}
