/// Signal handling for graceful shutdown and config reload.
///
/// Watches for:
///   - SIGTERM / SIGINT → graceful shutdown
///   - SIGHUP → config hot reload
use tokio::signal::unix::{signal, SignalKind};

/// Represents a signal that was received.
pub enum Signal {
    Shutdown,
    Reload,
}

/// Wait for the next OS signal.
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
