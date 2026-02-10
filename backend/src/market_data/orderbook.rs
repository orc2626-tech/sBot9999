// =============================================================================
// Order Book Manager — Real-time orderbook aggregation
// =============================================================================

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Context, Result};
use futures_util::StreamExt;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio_tungstenite::connect_async;
use tracing::{error, info, warn};

/// Manages orderbook state for multiple symbols.
pub struct OrderBookManager {
    books: RwLock<HashMap<String, OrderBookState>>,
}

/// Orderbook state for a single symbol.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBookState {
    pub symbol: String,
    pub best_bid: f64,
    pub best_ask: f64,
    pub bid_depth: f64,
    pub ask_depth: f64,
    pub spread_bps: f64,
    pub imbalance: f64,
    pub last_update_id: u64,
}

impl OrderBookManager {
    pub fn new() -> Self {
        Self {
            books: RwLock::new(HashMap::new()),
        }
    }

    /// Update the orderbook state for a symbol.
    pub fn update(
        &self,
        symbol: &str,
        best_bid: f64,
        best_ask: f64,
        bid_depth: f64,
        ask_depth: f64,
        update_id: u64,
    ) {
        let mid = (best_bid + best_ask) / 2.0;
        let spread_bps = if mid > 0.0 {
            ((best_ask - best_bid) / mid) * 10_000.0
        } else {
            0.0
        };

        let total_depth = bid_depth + ask_depth;
        let imbalance = if total_depth > 0.0 {
            (bid_depth - ask_depth) / total_depth
        } else {
            0.0
        };

        let state = OrderBookState {
            symbol: symbol.to_string(),
            best_bid,
            best_ask,
            bid_depth,
            ask_depth,
            spread_bps,
            imbalance,
            last_update_id: update_id,
        };

        self.books.write().insert(symbol.to_string(), state);
    }

    /// Get the current orderbook state for a symbol.
    pub fn get(&self, symbol: &str) -> Option<OrderBookState> {
        self.books.read().get(symbol).cloned()
    }

    /// Get the spread in basis points for a symbol.
    pub fn spread_bps(&self, symbol: &str) -> Option<f64> {
        self.books.read().get(symbol).map(|s| s.spread_bps)
    }

    /// Get the orderbook imbalance for a symbol (-1 to +1).
    pub fn imbalance(&self, symbol: &str) -> Option<f64> {
        self.books.read().get(symbol).map(|s| s.imbalance)
    }

    /// Get all tracked symbols.
    pub fn symbols(&self) -> Vec<String> {
        self.books.read().keys().cloned().collect()
    }
}

impl Default for OrderBookManager {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Depth WebSocket stream
// ---------------------------------------------------------------------------

/// Connect to the Binance partial depth WebSocket stream for a single symbol
/// and feed orderbook updates into `manager`.
///
/// Uses the `@depth20@100ms` stream which provides the top 20 levels of the
/// orderbook at 100ms update intervals.
///
/// Runs until the stream disconnects or an error occurs, then returns so that
/// the caller (main.rs) can handle reconnection.
pub async fn run_depth_stream(
    symbol: &str,
    manager: &Arc<OrderBookManager>,
) -> Result<()> {
    let lower = symbol.to_lowercase();
    let url = format!("wss://stream.binance.com:9443/ws/{lower}@depth20@100ms");
    info!(url = %url, symbol = %symbol, "connecting to depth WebSocket");

    let (ws_stream, _response) = connect_async(&url)
        .await
        .context("failed to connect to depth WebSocket")?;

    info!(symbol = %symbol, "depth WebSocket connected");
    let (_write, mut read) = ws_stream.split();

    loop {
        match read.next().await {
            Some(Ok(msg)) => {
                if let tokio_tungstenite::tungstenite::Message::Text(text) = msg {
                    match parse_depth_message(symbol, &text) {
                        Ok((best_bid, best_ask, bid_depth, ask_depth, update_id)) => {
                            manager.update(symbol, best_bid, best_ask, bid_depth, ask_depth, update_id);
                        }
                        Err(e) => {
                            warn!(error = %e, "failed to parse depth message");
                        }
                    }
                }
            }
            Some(Err(e)) => {
                error!(symbol = %symbol, error = %e, "depth WebSocket read error");
                return Err(e.into());
            }
            None => {
                warn!(symbol = %symbol, "depth WebSocket stream ended");
                return Ok(());
            }
        }
    }
}

/// Parse a Binance partial-depth message.
///
/// Expected shape:
/// ```json
/// {
///   "lastUpdateId": 12345,
///   "bids": [["37000.00", "1.5"], ...],
///   "asks": [["37001.00", "1.2"], ...]
/// }
/// ```
fn parse_depth_message(
    _symbol: &str,
    text: &str,
) -> Result<(f64, f64, f64, f64, u64)> {
    let root: serde_json::Value =
        serde_json::from_str(text).context("failed to parse depth JSON")?;

    let update_id = root["lastUpdateId"]
        .as_u64()
        .context("missing field lastUpdateId")?;

    let bids = root["bids"]
        .as_array()
        .context("missing field bids")?;

    let asks = root["asks"]
        .as_array()
        .context("missing field asks")?;

    // Best bid = first entry in bids array.
    let best_bid: f64 = bids
        .first()
        .and_then(|b| b.get(0))
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.0);

    // Best ask = first entry in asks array.
    let best_ask: f64 = asks
        .first()
        .and_then(|a| a.get(0))
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.0);

    // Total bid depth (sum of quantities across all levels).
    let bid_depth: f64 = bids
        .iter()
        .filter_map(|b| {
            b.get(1)
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<f64>().ok())
        })
        .sum();

    // Total ask depth (sum of quantities across all levels).
    let ask_depth: f64 = asks
        .iter()
        .filter_map(|a| {
            a.get(1)
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<f64>().ok())
        })
        .sum();

    Ok((best_bid, best_ask, bid_depth, ask_depth, update_id))
}
