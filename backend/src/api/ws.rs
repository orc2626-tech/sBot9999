// =============================================================================
// WebSocket Handler — Push-based state updates
// =============================================================================
//
// Clients connect to `/api/v1/ws?token=<token>` and receive:
//   1. An immediate full StateSnapshot on connect.
//   2. Incremental full snapshots every 500 ms whenever the state_version has
//      changed since the last push.
//
// The handler also:
//   - Responds to Ping frames with Pong frames.
//   - Tracks a per-connection `ws_sequence_number` that increments on every
//     outbound message.
//   - Updates the shared `ws_user_connected` flag and `last_ws_user_event`
//     timestamp on the AppState.
//   - Cleans up on disconnect.
// =============================================================================

use std::sync::Arc;

use axum::{
    extract::{
        ws::{Message, WebSocket},
        Query, State, WebSocketUpgrade,
    },
    response::IntoResponse,
};
use serde::Deserialize;
use tokio::time::{interval, Duration};
use tracing::{debug, info, warn};

use crate::api::auth::validate_token;
use crate::app_state::AppState;

// =============================================================================
// Query parameters
// =============================================================================

#[derive(Deserialize)]
pub struct WsQuery {
    token: Option<String>,
}

// =============================================================================
// WebSocket upgrade handler
// =============================================================================

/// Axum handler for the WebSocket upgrade request.
///
/// Validates the token from the `?token=` query parameter before upgrading.
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    Query(query): Query<WsQuery>,
) -> impl IntoResponse {
    // Validate the token from the query parameter.
    let token = query.token.unwrap_or_default();
    if !validate_token(&token) {
        warn!("WebSocket connection rejected: invalid token");
        return (
            axum::http::StatusCode::FORBIDDEN,
            "Invalid or missing token",
        )
            .into_response();
    }

    info!("WebSocket connection accepted — upgrading");
    ws.on_upgrade(move |socket| handle_ws_connection(socket, state))
        .into_response()
}

// =============================================================================
// Connection handler
// =============================================================================

/// Manages a single WebSocket connection lifecycle.
///
/// Runs two concurrent tasks via `tokio::select!`:
///   1. **Push loop** — every 500 ms, check if state_version changed and send
///      a new snapshot if so.
///   2. **Recv loop** — process incoming client messages (Ping/Pong, Close,
///      heartbeat text messages).
async fn handle_ws_connection(socket: WebSocket, state: Arc<AppState>) {
    // Mark the user as connected.
    {
        *state.ws_user_connected.write() = true;
        *state.last_ws_user_event.write() = std::time::Instant::now();
    }
    state.increment_version();

    let (mut sender, mut receiver) = socket.split();
    use futures_util::{SinkExt, StreamExt};

    // Send the initial full snapshot immediately.
    let mut last_sent_version: u64 = 0;
    let mut sequence: u64 = 0;

    if let Err(e) = send_snapshot(&mut sender, &state, &mut sequence).await {
        warn!(error = %e, "Failed to send initial WebSocket snapshot");
        cleanup(&state);
        return;
    }
    last_sent_version = state.current_state_version();

    // Concurrent push/recv loop.
    let mut push_interval = interval(Duration::from_millis(500));

    loop {
        tokio::select! {
            // ── Push loop: check for version changes every 500 ms ───────
            _ = push_interval.tick() => {
                let current_version = state.current_state_version();
                if current_version != last_sent_version {
                    match send_snapshot(&mut sender, &state, &mut sequence).await {
                        Ok(()) => {
                            last_sent_version = current_version;
                        }
                        Err(e) => {
                            debug!(error = %e, "WebSocket send failed — disconnecting");
                            break;
                        }
                    }
                }
            }

            // ── Recv loop: process incoming messages ────────────────────
            msg = receiver.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        // Treat any text message as a heartbeat.
                        debug!(msg = %text, "WebSocket text message received (heartbeat)");
                        *state.last_ws_user_event.write() = std::time::Instant::now();
                        state.increment_version();
                    }
                    Some(Ok(Message::Ping(data))) => {
                        debug!("WebSocket Ping received — sending Pong");
                        if let Err(e) = sender.send(Message::Pong(data)).await {
                            debug!(error = %e, "Failed to send Pong — disconnecting");
                            break;
                        }
                    }
                    Some(Ok(Message::Pong(_))) => {
                        // Pong received — no action needed.
                        debug!("WebSocket Pong received");
                    }
                    Some(Ok(Message::Close(_))) => {
                        info!("WebSocket Close frame received — disconnecting");
                        break;
                    }
                    Some(Ok(Message::Binary(_))) => {
                        // Ignore binary messages.
                        debug!("WebSocket binary message ignored");
                    }
                    Some(Err(e)) => {
                        warn!(error = %e, "WebSocket receive error — disconnecting");
                        break;
                    }
                    None => {
                        info!("WebSocket stream ended (None)");
                        break;
                    }
                }
            }
        }
    }

    cleanup(&state);
}

// =============================================================================
// Helpers
// =============================================================================

/// Serialize and send the current StateSnapshot over the WebSocket.
///
/// Increments the global `ws_sequence_number` on each send.
async fn send_snapshot<S>(
    sender: &mut S,
    state: &Arc<AppState>,
    sequence: &mut u64,
) -> Result<(), axum::Error>
where
    S: futures_util::Sink<Message, Error = axum::Error> + Unpin,
{
    use futures_util::SinkExt;

    // Increment the global sequence number.
    state
        .ws_sequence_number
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    *sequence += 1;

    let snapshot = state.build_snapshot();

    match serde_json::to_string(&snapshot) {
        Ok(json) => {
            sender.send(Message::Text(json.into())).await?;
            debug!(
                version = snapshot.state_version,
                seq = *sequence,
                "WebSocket snapshot sent"
            );
            Ok(())
        }
        Err(e) => {
            warn!(error = %e, "Failed to serialize snapshot");
            // Serialisation errors are not network errors; don't disconnect.
            Ok(())
        }
    }
}

/// Clean up shared state when a WebSocket connection closes.
fn cleanup(state: &Arc<AppState>) {
    *state.ws_user_connected.write() = false;
    state.increment_version();
    info!("WebSocket connection closed — cleanup complete");
}
