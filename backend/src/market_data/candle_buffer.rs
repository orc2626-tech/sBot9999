use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use anyhow::{Context, Result};
use futures_util::StreamExt;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio_tungstenite::connect_async;
use tracing::{debug, error, info, warn};

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

/// A single OHLCV candle from the Binance kline stream.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candle {
    pub open_time: i64,
    pub close_time: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub quote_volume: f64,
    pub trades_count: u64,
    pub taker_buy_volume: f64,
    pub taker_buy_quote_volume: f64,
    pub is_closed: bool,
}

/// Composite key that identifies a unique candle series.
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct CandleKey {
    pub symbol: String,
    pub interval: String,
}

impl std::fmt::Display for CandleKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}@{}", self.symbol, self.interval)
    }
}

// ---------------------------------------------------------------------------
// CandleBuffer -- thread-safe ring buffer per (symbol, interval)
// ---------------------------------------------------------------------------

/// Thread-safe ring-buffer that stores the most recent candles per
/// `(symbol, interval)` pair.  The live (unclosed) candle is continuously
/// updated in-place; when a candle closes it becomes permanent and the ring is
/// trimmed to `max_candles`.
pub struct CandleBuffer {
    buffers: RwLock<HashMap<CandleKey, VecDeque<Candle>>>,
    max_candles: usize,
}

impl CandleBuffer {
    /// Create a new buffer that retains at most `max_candles` closed candles per
    /// key, plus one in-progress candle.
    pub fn new(max_candles: usize) -> Self {
        Self {
            buffers: RwLock::new(HashMap::new()),
            max_candles,
        }
    }

    /// Insert or replace the latest candle for the given key.
    ///
    /// * If the incoming candle is closed (`is_closed == true`) it is appended
    ///   and the ring is trimmed to `max_candles`.
    /// * If the incoming candle is still open it replaces the last element when
    ///   that element is also an open candle with the same `open_time`
    ///   (in-progress update), otherwise it is simply appended.
    pub fn update(&self, key: CandleKey, candle: Candle) {
        let mut map = self.buffers.write();
        let ring = map
            .entry(key)
            .or_insert_with(|| VecDeque::with_capacity(self.max_candles + 1));

        if candle.is_closed {
            // If the last entry was the in-progress version of this same
            // candle, replace it with the finalized version.
            if let Some(last) = ring.back() {
                if !last.is_closed && last.open_time == candle.open_time {
                    ring.pop_back();
                }
            }
            ring.push_back(candle);
            // Trim oldest to stay within budget.
            while ring.len() > self.max_candles {
                ring.pop_front();
            }
        } else {
            // In-progress candle -- replace existing in-progress or append.
            if let Some(last) = ring.back() {
                if !last.is_closed && last.open_time == candle.open_time {
                    ring.pop_back();
                }
            }
            ring.push_back(candle);
        }
    }

    /// Return the most recent `count` **closed** candles (oldest-first order).
    pub fn get_closed(&self, key: &CandleKey, count: usize) -> Vec<Candle> {
        let map = self.buffers.read();
        match map.get(key) {
            Some(ring) => {
                let closed: Vec<&Candle> = ring.iter().filter(|c| c.is_closed).collect();
                let start = closed.len().saturating_sub(count);
                closed[start..].iter().map(|c| (*c).clone()).collect()
            }
            None => Vec::new(),
        }
    }

    /// Return the most recent `count` close prices from closed candles
    /// (oldest-first order).
    pub fn get_closes(&self, key: &CandleKey, count: usize) -> Vec<f64> {
        self.get_closed(key, count)
            .iter()
            .map(|c| c.close)
            .collect()
    }

    /// Alias for [`get_closed`] — used by strategy.rs and main.rs.
    pub fn get_closed_candles(&self, key: &CandleKey, count: usize) -> Vec<Candle> {
        self.get_closed(key, count)
    }

    /// Return the close price of the most recent closed candle, if any.
    pub fn last_close(&self, key: &CandleKey) -> Option<f64> {
        let map = self.buffers.read();
        map.get(key)
            .and_then(|ring| ring.iter().rev().find(|c| c.is_closed).map(|c| c.close))
    }

    /// Total number of candles (including any in-progress candle) stored for a
    /// key.
    pub fn count(&self, key: &CandleKey) -> usize {
        let map = self.buffers.read();
        map.get(key).map_or(0, VecDeque::len)
    }
}

// ---------------------------------------------------------------------------
// Kline WebSocket stream
// ---------------------------------------------------------------------------

/// Supported intervals that the bot subscribes to.
const SUPPORTED_INTERVALS: &[&str] = &["1m", "5m", "15m", "1h"];

/// Build the Binance combined-stream URL for all (symbol, interval) pairs.
#[cfg(test)]
fn build_kline_url(symbols: &[String], intervals: &[String]) -> String {
    let mut streams: Vec<String> = Vec::new();
    for sym in symbols {
        let lower = sym.to_lowercase();
        for iv in intervals {
            streams.push(format!("{lower}@kline_{iv}"));
        }
    }
    format!(
        "wss://stream.binance.com:9443/stream?streams={}",
        streams.join("/")
    )
}

/// Parse a single kline message from the Binance combined-stream JSON envelope.
///
/// Expected shape:
/// ```json
/// { "stream": "btcusdt@kline_1m", "data": { "s": "BTCUSDT", "k": { ... } } }
/// ```
#[cfg(test)]
fn parse_kline_message(text: &str) -> Result<(CandleKey, Candle)> {
    let root: serde_json::Value =
        serde_json::from_str(text).context("failed to parse kline JSON")?;

    let data = &root["data"];

    let symbol = data["s"]
        .as_str()
        .context("missing field data.s")?
        .to_uppercase();

    let k = &data["k"];

    let interval = k["i"]
        .as_str()
        .context("missing field k.i")?
        .to_string();

    let open_time = k["t"].as_i64().context("missing field k.t")?;
    let close_time = k["T"].as_i64().context("missing field k.T")?;

    let open = parse_string_f64(&k["o"], "k.o")?;
    let high = parse_string_f64(&k["h"], "k.h")?;
    let low = parse_string_f64(&k["l"], "k.l")?;
    let close = parse_string_f64(&k["c"], "k.c")?;
    let volume = parse_string_f64(&k["v"], "k.v")?;
    let quote_volume = parse_string_f64(&k["q"], "k.q")?;
    let taker_buy_volume = parse_string_f64(&k["V"], "k.V")?;
    let taker_buy_quote_volume = parse_string_f64(&k["Q"], "k.Q")?;

    let trades_count = k["n"].as_u64().context("missing field k.n")?;
    let is_closed = k["x"].as_bool().context("missing field k.x")?;

    let key = CandleKey { symbol, interval };
    let candle = Candle {
        open_time,
        close_time,
        open,
        high,
        low,
        close,
        volume,
        quote_volume,
        trades_count,
        taker_buy_volume,
        taker_buy_quote_volume,
        is_closed,
    };

    Ok((key, candle))
}

/// Helper: Binance sends numeric values as JSON strings inside kline objects.
fn parse_string_f64(val: &serde_json::Value, name: &str) -> Result<f64> {
    match val {
        serde_json::Value::String(s) => s
            .parse::<f64>()
            .with_context(|| format!("failed to parse {name} as f64: {s}")),
        serde_json::Value::Number(n) => n
            .as_f64()
            .with_context(|| format!("field {name} is not a valid f64")),
        _ => anyhow::bail!("field {name} has unexpected JSON type"),
    }
}

/// Connect to the Binance kline WebSocket stream for a single (symbol, interval)
/// pair and feed candles into `buffer`.
///
/// Runs until the stream disconnects or an error occurs, then returns so that
/// the caller (main.rs) can handle reconnection.
///
/// ```ignore
/// let buf = Arc::new(CandleBuffer::new(500));
/// loop {
///     if let Err(e) = run_kline_stream("BTCUSDT", "1m", &buf).await {
///         error!("stream error: {e}");
///     }
///     tokio::time::sleep(Duration::from_secs(5)).await;
/// }
/// ```
pub async fn run_kline_stream(
    symbol: &str,
    interval: &str,
    buffer: &Arc<CandleBuffer>,
) -> Result<()> {
    if !SUPPORTED_INTERVALS.contains(&interval) {
        warn!(
            interval = %interval,
            "unsupported kline interval requested -- it will still be subscribed"
        );
    }

    let lower = symbol.to_lowercase();
    let url = format!(
        "wss://stream.binance.com:9443/ws/{lower}@kline_{interval}"
    );
    info!(url = %url, symbol = %symbol, interval = %interval, "connecting to kline WebSocket");

    let (ws_stream, _response) = connect_async(&url)
        .await
        .context("failed to connect to kline WebSocket")?;

    info!(symbol = %symbol, interval = %interval, "kline WebSocket connected");
    let (_write, mut read) = ws_stream.split();

    loop {
        match read.next().await {
            Some(Ok(msg)) => {
                if let tokio_tungstenite::tungstenite::Message::Text(text) = msg {
                    match parse_kline_message_single(&text) {
                        Ok((key, candle)) => {
                            debug!(
                                key = %key,
                                close = candle.close,
                                closed = candle.is_closed,
                                "candle update"
                            );
                            buffer.update(key, candle);
                        }
                        Err(e) => {
                            warn!(error = %e, "failed to parse kline message");
                        }
                    }
                }
                // Silently ignore Ping / Pong / Binary / Close frames --
                // tungstenite handles pong replies automatically.
            }
            Some(Err(e)) => {
                error!(error = %e, "kline WebSocket read error");
                return Err(e.into());
            }
            None => {
                warn!(symbol = %symbol, interval = %interval, "kline WebSocket stream ended");
                return Ok(());
            }
        }
    }
}

/// Parse a single-stream kline message (non-combined stream).
///
/// Expected shape (single stream — no outer `stream`/`data` wrapper):
/// ```json
/// { "e": "kline", "s": "BTCUSDT", "k": { ... } }
/// ```
fn parse_kline_message_single(text: &str) -> Result<(CandleKey, Candle)> {
    let root: serde_json::Value =
        serde_json::from_str(text).context("failed to parse kline JSON")?;

    // Support both combined-stream envelope and direct single-stream payload.
    let data = if root.get("data").is_some() {
        &root["data"]
    } else {
        &root
    };

    let symbol = data["s"]
        .as_str()
        .context("missing field s")?
        .to_uppercase();

    let k = &data["k"];

    let interval = k["i"]
        .as_str()
        .context("missing field k.i")?
        .to_string();

    let open_time = k["t"].as_i64().context("missing field k.t")?;
    let close_time = k["T"].as_i64().context("missing field k.T")?;

    let open = parse_string_f64(&k["o"], "k.o")?;
    let high = parse_string_f64(&k["h"], "k.h")?;
    let low = parse_string_f64(&k["l"], "k.l")?;
    let close = parse_string_f64(&k["c"], "k.c")?;
    let volume = parse_string_f64(&k["v"], "k.v")?;
    let quote_volume = parse_string_f64(&k["q"], "k.q")?;
    let taker_buy_volume = parse_string_f64(&k["V"], "k.V")?;
    let taker_buy_quote_volume = parse_string_f64(&k["Q"], "k.Q")?;

    let trades_count = k["n"].as_u64().context("missing field k.n")?;
    let is_closed = k["x"].as_bool().context("missing field k.x")?;

    let key = CandleKey { symbol, interval };
    let candle = Candle {
        open_time,
        close_time,
        open,
        high,
        low,
        close,
        volume,
        quote_volume,
        trades_count,
        taker_buy_volume,
        taker_buy_quote_volume,
        is_closed,
    };

    Ok((key, candle))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_candle(open_time: i64, close: f64, is_closed: bool) -> Candle {
        Candle {
            open_time,
            close_time: open_time + 59_999,
            open: close,
            high: close + 1.0,
            low: close - 1.0,
            close,
            volume: 100.0,
            quote_volume: 200.0,
            trades_count: 50,
            taker_buy_volume: 60.0,
            taker_buy_quote_volume: 120.0,
            is_closed,
        }
    }

    fn make_key(sym: &str, iv: &str) -> CandleKey {
        CandleKey {
            symbol: sym.into(),
            interval: iv.into(),
        }
    }

    #[test]
    fn ring_buffer_trimming() {
        let buf = CandleBuffer::new(3);
        let key = make_key("BTCUSDT", "1m");

        for i in 0..5 {
            buf.update(
                key.clone(),
                sample_candle(i * 60_000, 100.0 + i as f64, true),
            );
        }

        assert_eq!(buf.count(&key), 3);
        let closes = buf.get_closes(&key, 10);
        assert_eq!(closes, vec![102.0, 103.0, 104.0]);
    }

    #[test]
    fn in_progress_replacement() {
        let buf = CandleBuffer::new(10);
        let key = make_key("ETHUSDT", "5m");

        buf.update(key.clone(), sample_candle(0, 50.0, false));
        assert_eq!(buf.count(&key), 1);

        // Same open_time, still open -- should replace.
        buf.update(key.clone(), sample_candle(0, 51.0, false));
        assert_eq!(buf.count(&key), 1);

        // Close it.
        buf.update(key.clone(), sample_candle(0, 52.0, true));
        assert_eq!(buf.count(&key), 1);
        assert_eq!(buf.last_close(&key), Some(52.0));
    }

    #[test]
    fn get_closed_filters_out_in_progress() {
        let buf = CandleBuffer::new(10);
        let key = make_key("BTCUSDT", "1m");

        buf.update(key.clone(), sample_candle(0, 100.0, true));
        buf.update(key.clone(), sample_candle(60_000, 101.0, true));
        buf.update(key.clone(), sample_candle(120_000, 102.0, false)); // in progress

        let closed = buf.get_closed(&key, 10);
        assert_eq!(closed.len(), 2);
    }

    #[test]
    fn last_close_empty_returns_none() {
        let buf = CandleBuffer::new(10);
        let key = make_key("XYZUSDT", "1h");
        assert_eq!(buf.last_close(&key), None);
    }

    #[test]
    fn build_url_contains_streams() {
        let url = build_kline_url(
            &["BTCUSDT".to_string()],
            &["1m".to_string(), "5m".to_string()],
        );
        assert!(url.contains("btcusdt@kline_1m"));
        assert!(url.contains("btcusdt@kline_5m"));
        assert!(url.starts_with("wss://stream.binance.com:9443/stream?streams="));
    }

    #[test]
    fn parse_kline_message_ok() {
        let json = r#"{
            "stream": "btcusdt@kline_1m",
            "data": {
                "e": "kline",
                "s": "BTCUSDT",
                "k": {
                    "t": 1700000000000,
                    "T": 1700000059999,
                    "i": "1m",
                    "o": "37000.00",
                    "h": "37050.00",
                    "l": "36990.00",
                    "c": "37020.00",
                    "v": "123.456",
                    "q": "4567890.12",
                    "n": 1500,
                    "V": "60.123",
                    "Q": "2224455.66",
                    "x": false
                }
            }
        }"#;
        let (key, candle) = parse_kline_message(json).expect("should parse");
        assert_eq!(key.symbol, "BTCUSDT");
        assert_eq!(key.interval, "1m");
        assert!((candle.close - 37020.0).abs() < f64::EPSILON);
        assert!(!candle.is_closed);
    }
}
