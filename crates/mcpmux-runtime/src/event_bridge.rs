//! Bridge the gateway's internal `DomainEvent` broadcast channel into the
//! runtime's shared [`mcpmux_core::EventBus`].
//!
//! The gateway crate owns its own `tokio::sync::broadcast::Sender<DomainEvent>`
//! (`mcpmux_gateway::GatewayState::domain_event_tx`) that drives MCPNotifier,
//! the OAuth event handler, and (historically) the desktop bridge. The
//! runtime's [`mcpmux_core::EventBus`] is the channel that the
//! `*AppService` types emit into.
//!
//! `spawn_event_bridge` subscribes to the gateway's channel and re-emits
//! every event into the shared bus, so:
//!
//! - Application services see both their own events and gateway events.
//! - Desktop / daemon / future CLI subscribers can observe the full stream
//!   by listening to the shared bus alone (the desktop's
//!   `map_domain_event_to_ui` should be re-pointed at this bus in a
//!   follow-up PR; today it still reads the gateway channel directly).
//!
//! The subscription is established synchronously *before* the bridge task
//! is spawned, so any event emitted after the `await` returns is captured
//! — there is no startup window in which events get dropped.
//!
//! Lag is handled the same way `EventBus::recv` handles it — log a
//! warning and continue.

use std::sync::Arc;

use mcpmux_core::{DomainEvent, SharedEventBus};
use tokio::sync::broadcast::{self, error::RecvError};
use tokio::sync::RwLock;
use tracing::warn;

use mcpmux_gateway::GatewayState;

/// Subscribe to the gateway's broadcast and spawn the forwarding task.
///
/// Returns the `JoinHandle` so the caller can `abort()` it during
/// shutdown. The subscription is established before this function
/// returns, so events emitted immediately after `await`ing are captured.
pub async fn spawn_event_bridge(
    runtime_event_bus: SharedEventBus,
    gateway_state: Arc<RwLock<GatewayState>>,
) -> tokio::task::JoinHandle<()> {
    let gateway_rx = {
        let state = gateway_state.read().await;
        state.subscribe_domain_events()
    };

    tokio::spawn(forward_events(gateway_rx, runtime_event_bus))
}

/// Forward every gateway event onto the runtime bus until the gateway
/// channel closes.
async fn forward_events(
    mut gateway_rx: broadcast::Receiver<DomainEvent>,
    runtime_event_bus: SharedEventBus,
) {
    loop {
        match gateway_rx.recv().await {
            Ok(event) => {
                runtime_event_bus.sender().emit(event);
            }
            // A burst (e.g. startup auto-connect) outran the bridge. The
            // skipped events are gone; keep forwarding the rest instead of
            // ending the bridge for the life of the process.
            Err(RecvError::Lagged(skipped)) => {
                warn!(
                    skipped,
                    "[runtime] gateway event bridge lagged; events dropped"
                );
            }
            Err(RecvError::Closed) => break,
        }
    }
    warn!("[runtime] gateway event bridge: gateway channel closed, exiting");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn keeps_forwarding_after_lagging() {
        let (gateway_tx, gateway_rx) = broadcast::channel(4);
        let bus = mcpmux_core::create_shared_event_bus();
        let mut runtime_rx = bus.subscribe();

        // Overflow the bridge's receiver before it ever runs.
        for _ in 0..10 {
            gateway_tx.send(DomainEvent::GatewayStopped).unwrap();
        }
        let bridge = tokio::spawn(forward_events(gateway_rx, bus.clone()));

        // The newest buffered events survive the lag and are forwarded...
        let first = tokio::time::timeout(Duration::from_secs(2), runtime_rx.recv())
            .await
            .expect("forwarded after lag")
            .expect("bus open");
        assert_eq!(first.type_name(), "gateway_stopped");

        // ...and the bridge is still alive for later events.
        gateway_tx
            .send(DomainEvent::GatewayStarted {
                url: "http://127.0.0.1:1".to_string(),
                port: 1,
            })
            .unwrap();
        let later = tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                let event = runtime_rx.recv().await.expect("bus open");
                if event.type_name() == "gateway_started" {
                    return event;
                }
            }
        })
        .await
        .expect("bridge still forwarding after lag");
        assert_eq!(later.type_name(), "gateway_started");

        drop(gateway_tx);
        tokio::time::timeout(Duration::from_secs(2), bridge)
            .await
            .expect("bridge exits when the gateway channel closes")
            .unwrap();
    }
}
