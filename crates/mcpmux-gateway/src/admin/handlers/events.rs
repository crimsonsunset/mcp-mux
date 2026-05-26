//! Admin SSE event stream (`GET /api/v1/events`).

use std::convert::Infallible;
use std::time::Duration;

use async_stream::stream;
use axum::{
    extract::State,
    http::StatusCode,
    response::sse::{Event, KeepAlive, Sse},
    Json,
};
use futures::Stream;
use serde::Deserialize;
use tracing::debug;

use super::super::router::AdminState;

/// SSE stream bridging merged admin UI events to web clients.
pub async fn sse_events(
    State(state): State<AdminState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let mut rx = state.event_hub.subscribe();

    let stream = stream! {
        loop {
            match rx.recv().await {
                Ok(ui_event) => {
                    debug!(
                        channel = %ui_event.channel,
                        "[Admin] SSE forwarding UI event"
                    );
                    yield Ok(Event::default()
                        .event(ui_event.channel)
                        .json_data(ui_event.payload)
                        .expect("UI event payload serializes"));
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                    debug!("[Admin] SSE client lagged, skipped {skipped} events");
                    continue;
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    };

    Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keepalive"),
    )
}

/// Request body for test-only SSE publish endpoint.
#[derive(Debug, Deserialize)]
pub struct TestPublishEventRequest {
    pub channel: String,
    pub payload: serde_json::Value,
}

/// Test-only endpoint to publish UI events for Playwright SSE smoke tests.
pub async fn publish_test_event(
    State(state): State<AdminState>,
    Json(body): Json<TestPublishEventRequest>,
) -> StatusCode {
    if std::env::var("MCPMUX_ADMIN_TEST").is_err() {
        return StatusCode::NOT_FOUND;
    }
    state
        .event_hub
        .publish_test_event(&body.channel, body.payload);
    StatusCode::NO_CONTENT
}
