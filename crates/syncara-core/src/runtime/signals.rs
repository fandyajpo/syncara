/// Signal handling for graceful shutdown and config reload.
///
/// Unix:
///   - SIGTERM / SIGINT → graceful shutdown
///   - SIGHUP → config hot reload
/// Windows:
///   - Ctrl-C → graceful shutdown (no reload support)

#[cfg(unix)]
use tokio::signal::unix::{signal, SignalKind};

/// Represents a signal that was received.
pub enum Signal {
    Shutdown,
    Reload,
}

/// Wait for the next OS signal.
#[cfg(unix)]
pub async fn next_signal() -> Signal {
    let mut term = signal(SignalKind::terminate()).expect("failed to register SIGTERM handler");
    let mut intr = signal(SignalKind::interrupt()).expect("failed to register SIGINT handler");
    let mut hup = signal(SignalKind::hangup()).expect("failed to register SIGHUP handler");

    tokio::select! {
        _ = term.recv() => Signal::Shutdown,
        _ = intr.recv() => Signal::Shutdown,
        _ = hup.recv() => Signal::Reload,
    }
}

/// Windows — only Ctrl-C shutdown, no reload.
#[cfg(windows)]
pub async fn next_signal() -> Signal {
    tokio::signal::ctrl_c().await.expect("failed to register Ctrl-C handler");
    Signal::Shutdown
}
