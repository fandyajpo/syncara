use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use bytes::Bytes;
use http_body::Frame;
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;

use crate::brain::RoutingDecision;

const CHANNEL_CAPACITY: usize = 512;

/// A health state transition for an upstream.
#[derive(Debug, Clone)]
pub struct HealthTransition {
    pub addr: String,
    pub healthy: bool,
    pub pool_name: String,
    pub timestamp: String,
}

/// Events that can be streamed to dashboard clients.
#[derive(Debug, Clone)]
pub enum RealtimeEvent {
    RoutingDecision(Arc<RoutingDecision>),
    HealthTransition(HealthTransition),
}

static BROADCASTER: std::sync::OnceLock<broadcast::Sender<Arc<RealtimeEvent>>> =
    std::sync::OnceLock::new();

/// Initialise the realtime event bus.
pub fn init() {
    let (tx, _) = broadcast::channel(CHANNEL_CAPACITY);
    BROADCASTER
        .set(tx)
        .ok()
        .expect("realtime::init called more than once");
}

/// Broadcast an event to all connected SSE clients.
pub fn broadcast(event: Arc<RealtimeEvent>) {
    if let Some(tx) = BROADCASTER.get() {
        let _ = tx.send(event);
    }
}

/// Return a new receiver for the realtime event stream.
pub fn subscribe() -> Option<broadcast::Receiver<Arc<RealtimeEvent>>> {
    BROADCASTER.get().map(|tx| tx.subscribe())
}

fn serialize_decision(decision: &RoutingDecision) -> String {
    serde_json::to_string(&serde_json::json!({
        "type": "routing_decision",
        "selected": decision.selected,
        "explanation": decision.explanation,
        "pool_name": decision.pool_name,
        "scores": decision.scores.iter().map(|s| {
            serde_json::json!({
                "addr": s.addr,
                "score": s.score,
                "deductions": s.deductions.iter().map(|d| {
                    serde_json::json!({
                        "reason": d.reason,
                        "points": d.points,
                    })
                }).collect::<Vec<_>>(),
            })
        }).collect::<Vec<_>>(),
        "timestamp": chrono::Utc::now().to_rfc3339(),
    }))
    .unwrap_or_else(|_| "{}".to_string())
}

/// An SSE stream of realtime events.
pub struct SseEventStream {
    inner: BroadcastStream<Arc<RealtimeEvent>>,
}

impl SseEventStream {
    pub fn new(rx: broadcast::Receiver<Arc<RealtimeEvent>>) -> Self {
        Self {
            inner: BroadcastStream::new(rx),
        }
    }
}

impl futures_core::Stream for SseEventStream {
    type Item = Result<Frame<Bytes>, std::convert::Infallible>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            match Pin::new(&mut self.inner).poll_next(cx) {
                Poll::Ready(Some(Ok(event))) => {
    let json = match event.as_ref() {
        RealtimeEvent::RoutingDecision(d) => serialize_decision(d),
        RealtimeEvent::HealthTransition(h) => {
            serde_json::to_string(&serde_json::json!({
                "type": "health_transition",
                "addr": h.addr,
                "healthy": h.healthy,
                "pool_name": h.pool_name,
                "timestamp": h.timestamp,
            }))
            .unwrap_or_else(|_| "{}".to_string())
        }
    };
                    let frame = format!("event: realtime\ndata: {json}\n\n");
                    return Poll::Ready(Some(Ok(Frame::data(Bytes::from(frame)))));
                }
                Poll::Ready(Some(Err(tokio_stream::wrappers::errors::BroadcastStreamRecvError::Lagged(n)))) => {
                    tracing::warn!(n, "realtime SSE client lagged, skipping");
                    continue;
                }
                Poll::Ready(None) => {
                    return Poll::Ready(None);
                }
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}
