// =============================================================================
// Trade Stream Processor — Aggregates real-time trade data
// =============================================================================

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::{Context, Result};
use futures_util::StreamExt;
use parking_lot::RwLock;
use tokio_tungstenite::connect_async;
use tracing::{error, info, warn};

/// Processes and aggregates individual trades from the Binance trade stream.
pub struct TradeStreamProcessor {
    symbol: String,
    /// Cumulative volume delta (buy volume - sell volume).
    cvd: RwLock<f64>,
    /// Total buy volume in the current window.
    buy_volume: RwLock<f64>,
    /// Total sell volume in the current window.
    sell_volume: RwLock<f64>,
    /// Total number of trades processed.
    trade_count: AtomicU64,
    /// Last trade price.
    last_price: RwLock<f64>,
    /// Buy volume ratio (buy_vol / total_vol).
    buy_volume_ratio: RwLock<f64>,
}

impl TradeStreamProcessor {
    pub fn new(symbol: impl Into<String>) -> Self {
        Self {
            symbol: symbol.into(),
            cvd: RwLock::new(0.0),
            buy_volume: RwLock::new(0.0),
            sell_volume: RwLock::new(0.0),
            trade_count: AtomicU64::new(0),
            last_price: RwLock::new(0.0),
            buy_volume_ratio: RwLock::new(0.5),
        }
    }

    /// Process an incoming trade.
    pub fn process_trade(&self, price: f64, quantity: f64, is_buyer_maker: bool) {
        let volume = price * quantity;

        if is_buyer_maker {
            // Buyer is maker => taker is selling.
            *self.sell_volume.write() += volume;
            *self.cvd.write() -= volume;
        } else {
            // Seller is maker => taker is buying.
            *self.buy_volume.write() += volume;
            *self.cvd.write() += volume;
        }

        *self.last_price.write() = price;
        self.trade_count.fetch_add(1, Ordering::Relaxed);

        // Update buy volume ratio.
        let buy = *self.buy_volume.read();
        let sell = *self.sell_volume.read();
        let total = buy + sell;
        if total > 0.0 {
            *self.buy_volume_ratio.write() = buy / total;
        }
    }

    pub fn symbol(&self) -> &str {
        &self.symbol
    }

    pub fn cvd(&self) -> f64 {
        *self.cvd.read()
    }

    pub fn buy_volume_ratio(&self) -> f64 {
        *self.buy_volume_ratio.read()
    }

    pub fn last_price(&self) -> f64 {
        *self.last_price.read()
    }

    pub fn trade_count(&self) -> u64 {
        self.trade_count.load(Ordering::Relaxed)
    }

    /// Reset windowed accumulators (call periodically).
    pub fn reset_window(&self) {
        *self.buy_volume.write() = 0.0;
        *self.sell_volume.write() = 0.0;
        // CVD is cumulative — do not reset.
    }
}

// ---------------------------------------------------------------------------
// Trade WebSocket stream
// ---------------------------------------------------------------------------

/// Connect to the Binance aggTrade WebSocket stream for a single symbol and
/// feed trades into `processor`.
///
/// Runs until the stream disconnects or an error occurs, then returns so that
/// the caller (main.rs) can handle reconnection.
pub async fn run_trade_stream(
    symbol: &str,
    processor: &Arc<TradeStreamProcessor>,
) -> Result<()> {
    let lower = symbol.to_lowercase();
    let url = format!("wss://stream.binance.com:9443/ws/{lower}@aggTrade");
    info!(url = %url, symbol = %symbol, "connecting to trade WebSocket");

    let (ws_stream, _response) = connect_async(&url)
        .await
        .context("failed to connect to trade WebSocket")?;

    info!(symbol = %symbol, "trade WebSocket connected");
    let (_write, mut read) = ws_stream.split();

    loop {
        match read.next().await {
            Some(Ok(msg)) => {
                if let tokio_tungstenite::tungstenite::Message::Text(text) = msg {
                    match parse_agg_trade(&text) {
                        Ok((price, quantity, is_buyer_maker)) => {
                            processor.process_trade(price, quantity, is_buyer_maker);
                        }
                        Err(e) => {
                            warn!(error = %e, "failed to parse aggTrade message");
                        }
                    }
                }
            }
            Some(Err(e)) => {
                error!(symbol = %symbol, error = %e, "trade WebSocket read error");
                return Err(e.into());
            }
            None => {
                warn!(symbol = %symbol, "trade WebSocket stream ended");
                return Ok(());
            }
        }
    }
}

/// Parse a Binance aggTrade message.
///
/// Expected shape:
/// ```json
/// { "e": "aggTrade", "s": "BTCUSDT", "p": "37000.00", "q": "0.123", "m": true }
/// ```
fn parse_agg_trade(text: &str) -> Result<(f64, f64, bool)> {
    let root: serde_json::Value =
        serde_json::from_str(text).context("failed to parse aggTrade JSON")?;

    let price: f64 = root["p"]
        .as_str()
        .context("missing field p")?
        .parse()
        .context("failed to parse price")?;

    let quantity: f64 = root["q"]
        .as_str()
        .context("missing field q")?
        .parse()
        .context("failed to parse quantity")?;

    let is_buyer_maker = root["m"]
        .as_bool()
        .context("missing field m")?;

    Ok((price, quantity, is_buyer_maker))
}
