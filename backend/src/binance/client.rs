// =============================================================================
// Binance REST API Client — HMAC-SHA256 signed requests
// =============================================================================
//
// SECURITY: The secret key is never logged or serialized. All signed requests
// include X-MBX-APIKEY as a header and a recvWindow of 5 000 ms to tolerate
// minor clock drift between the bot and Binance servers.
// =============================================================================

use anyhow::{Context, Result};
use hmac::{Hmac, Mac};
use reqwest::header::{HeaderMap, HeaderValue};
use sha2::Sha256;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, instrument, warn};

use crate::market_data::Candle;

type HmacSha256 = Hmac<Sha256>;

/// Default recv-window sent with every signed request (milliseconds).
const RECV_WINDOW: u64 = 5000;

/// Binance REST API client with HMAC-SHA256 request signing.
#[derive(Clone)]
pub struct BinanceClient {
    api_key: String,
    secret: String,
    base_url: String,
    client: reqwest::Client,
}

impl BinanceClient {
    // -------------------------------------------------------------------------
    // Construction
    // -------------------------------------------------------------------------

    /// Create a new `BinanceClient`.
    ///
    /// # Arguments
    /// * `api_key` — Binance API key (sent as a header, never in query params).
    /// * `secret`  — Binance secret key used exclusively for HMAC signing.
    pub fn new(api_key: impl Into<String>, secret: impl Into<String>) -> Self {
        let api_key = api_key.into();
        let secret = secret.into();

        let mut default_headers = HeaderMap::new();
        // The API key header is required for all signed endpoints.
        if let Ok(val) = HeaderValue::from_str(&api_key) {
            default_headers.insert("X-MBX-APIKEY", val);
        }

        let client = reqwest::Client::builder()
            .default_headers(default_headers)
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("failed to build reqwest client");

        debug!("BinanceClient initialised (base_url=https://api.binance.com)");

        Self {
            api_key,
            secret,
            base_url: "https://api.binance.com".to_string(),
            client,
        }
    }

    // -------------------------------------------------------------------------
    // Signing helpers
    // -------------------------------------------------------------------------

    /// Produce an HMAC-SHA256 hex signature of `query`.
    pub fn sign(&self, query: &str) -> String {
        let mut mac =
            HmacSha256::new_from_slice(self.secret.as_bytes()).expect("HMAC accepts any key size");
        mac.update(query.as_bytes());
        hex::encode(mac.finalize().into_bytes())
    }

    /// Current UNIX timestamp in milliseconds.
    pub fn timestamp_ms() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock before UNIX epoch")
            .as_millis() as u64
    }

    /// Build the full query string for a signed request (appends timestamp,
    /// recvWindow, and signature).
    fn signed_query(&self, params: &str) -> String {
        let ts = Self::timestamp_ms();
        let base = if params.is_empty() {
            format!("timestamp={ts}&recvWindow={RECV_WINDOW}")
        } else {
            format!("{params}&timestamp={ts}&recvWindow={RECV_WINDOW}")
        };
        let sig = self.sign(&base);
        format!("{base}&signature={sig}")
    }

    // -------------------------------------------------------------------------
    // Account / balance
    // -------------------------------------------------------------------------

    /// GET /api/v3/account (signed).
    #[instrument(skip(self), name = "binance::get_account")]
    pub async fn get_account(&self) -> Result<serde_json::Value> {
        let qs = self.signed_query("");
        let url = format!("{}/api/v3/account?{}", self.base_url, qs);

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .context("GET /api/v3/account request failed")?;

        let status = resp.status();
        let body: serde_json::Value = resp
            .json()
            .await
            .context("failed to parse account response")?;

        if !status.is_success() {
            anyhow::bail!(
                "Binance GET /api/v3/account returned {}: {}",
                status,
                body
            );
        }

        debug!("account info retrieved successfully");
        Ok(body)
    }

    /// Convenience: extract the free balance for a single `asset`.
    #[instrument(skip(self), name = "binance::get_balance")]
    pub async fn get_balance(&self, asset: &str) -> Result<f64> {
        let account = self.get_account().await?;

        let balances = account["balances"]
            .as_array()
            .context("account response missing 'balances' array")?;

        for b in balances {
            if b["asset"].as_str() == Some(asset) {
                let free: f64 = b["free"]
                    .as_str()
                    .unwrap_or("0")
                    .parse()
                    .unwrap_or(0.0);
                debug!(asset, free, "balance retrieved");
                return Ok(free);
            }
        }

        warn!(asset, "asset not found in balances — returning 0.0");
        Ok(0.0)
    }

    // -------------------------------------------------------------------------
    // Orders
    // -------------------------------------------------------------------------

    /// POST /api/v3/order (signed) — submit a new order.
    ///
    /// # Arguments
    /// * `symbol`          — e.g. "BTCUSDT"
    /// * `side`            — "BUY" or "SELL"
    /// * `order_type`      — "LIMIT", "MARKET", etc.
    /// * `quantity`        — order quantity
    /// * `price`           — required for LIMIT orders
    /// * `time_in_force`   — e.g. "GTC"; required for LIMIT orders
    /// * `client_order_id` — optional custom order id
    #[instrument(skip(self, price, time_in_force, client_order_id), name = "binance::place_order")]
    pub async fn place_order(
        &self,
        symbol: &str,
        side: &str,
        order_type: &str,
        quantity: f64,
        price: Option<f64>,
        time_in_force: Option<&str>,
        client_order_id: Option<&str>,
    ) -> Result<serde_json::Value> {
        let mut params = format!(
            "symbol={symbol}&side={side}&type={order_type}&quantity={quantity}"
        );

        if let Some(p) = price {
            params.push_str(&format!("&price={p}"));
        }
        if let Some(tif) = time_in_force {
            params.push_str(&format!("&timeInForce={tif}"));
        }
        if let Some(coid) = client_order_id {
            params.push_str(&format!("&newClientOrderId={coid}"));
        }

        let qs = self.signed_query(&params);
        let url = format!("{}/api/v3/order?{}", self.base_url, qs);

        debug!(symbol, side, order_type, quantity, "placing order");

        let resp = self
            .client
            .post(&url)
            .send()
            .await
            .context("POST /api/v3/order request failed")?;

        let status = resp.status();
        let body: serde_json::Value = resp
            .json()
            .await
            .context("failed to parse order response")?;

        if !status.is_success() {
            anyhow::bail!(
                "Binance POST /api/v3/order returned {}: {}",
                status,
                body
            );
        }

        debug!(symbol, side, "order placed successfully");
        Ok(body)
    }

    /// DELETE /api/v3/order (signed) — cancel an existing order.
    #[instrument(skip(self), name = "binance::cancel_order")]
    pub async fn cancel_order(
        &self,
        symbol: &str,
        order_id: u64,
    ) -> Result<serde_json::Value> {
        let params = format!("symbol={symbol}&orderId={order_id}");
        let qs = self.signed_query(&params);
        let url = format!("{}/api/v3/order?{}", self.base_url, qs);

        debug!(symbol, order_id, "cancelling order");

        let resp = self
            .client
            .delete(&url)
            .send()
            .await
            .context("DELETE /api/v3/order request failed")?;

        let status = resp.status();
        let body: serde_json::Value = resp
            .json()
            .await
            .context("failed to parse cancel response")?;

        if !status.is_success() {
            anyhow::bail!(
                "Binance DELETE /api/v3/order returned {}: {}",
                status,
                body
            );
        }

        debug!(symbol, order_id, "order cancelled");
        Ok(body)
    }

    /// GET /api/v3/openOrders (signed).
    #[instrument(skip(self), name = "binance::get_open_orders")]
    pub async fn get_open_orders(
        &self,
        symbol: Option<&str>,
    ) -> Result<Vec<serde_json::Value>> {
        let params = match symbol {
            Some(s) => format!("symbol={s}"),
            None => String::new(),
        };
        let qs = self.signed_query(&params);
        let url = format!("{}/api/v3/openOrders?{}", self.base_url, qs);

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .context("GET /api/v3/openOrders request failed")?;

        let status = resp.status();
        let body: serde_json::Value = resp
            .json()
            .await
            .context("failed to parse openOrders response")?;

        if !status.is_success() {
            anyhow::bail!(
                "Binance GET /api/v3/openOrders returned {}: {}",
                status,
                body
            );
        }

        let orders: Vec<serde_json::Value> = body
            .as_array()
            .cloned()
            .unwrap_or_default();

        debug!(count = orders.len(), "open orders retrieved");
        Ok(orders)
    }

    // -------------------------------------------------------------------------
    // Public market data
    // -------------------------------------------------------------------------

    /// GET /api/v3/klines (public — no signature required).
    ///
    /// Returns a vector of [`Candle`] structs parsed from Binance's array-of-
    /// arrays response format.
    ///
    /// Array indices:
    ///   [0] openTime, [1] open, [2] high, [3] low, [4] close, [5] volume,
    ///   [6] closeTime, [7] quoteAssetVolume, [8] numberOfTrades,
    ///   [9] takerBuyBaseVolume, [10] takerBuyQuoteVolume
    #[instrument(skip(self), name = "binance::get_klines")]
    pub async fn get_klines(
        &self,
        symbol: &str,
        interval: &str,
        limit: u32,
    ) -> Result<Vec<Candle>> {
        let url = format!(
            "{}/api/v3/klines?symbol={}&interval={}&limit={}",
            self.base_url, symbol, interval, limit
        );

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .context("GET /api/v3/klines request failed")?;

        let status = resp.status();
        let body: serde_json::Value = resp
            .json()
            .await
            .context("failed to parse klines response")?;

        if !status.is_success() {
            anyhow::bail!(
                "Binance GET /api/v3/klines returned {}: {}",
                status,
                body
            );
        }

        let raw = body
            .as_array()
            .context("klines response is not an array")?;

        let mut candles = Vec::with_capacity(raw.len());

        for entry in raw {
            let arr = entry
                .as_array()
                .context("kline entry is not an array")?;

            if arr.len() < 11 {
                warn!("skipping malformed kline entry with {} elements", arr.len());
                continue;
            }

            let open_time = arr[0].as_i64().unwrap_or(0);
            let open = Self::parse_str_f64(&arr[1])?;
            let high = Self::parse_str_f64(&arr[2])?;
            let low = Self::parse_str_f64(&arr[3])?;
            let close = Self::parse_str_f64(&arr[4])?;
            let volume = Self::parse_str_f64(&arr[5])?;
            let close_time = arr[6].as_i64().unwrap_or(0);

            candles.push(Candle::new(open_time, open, high, low, close, volume, close_time));
        }

        debug!(symbol, interval, count = candles.len(), "klines fetched");
        Ok(candles)
    }

    /// GET /api/v3/exchangeInfo filtered by symbol.
    #[instrument(skip(self), name = "binance::get_symbol_info")]
    pub async fn get_symbol_info(&self, symbol: &str) -> Result<serde_json::Value> {
        let url = format!(
            "{}/api/v3/exchangeInfo?symbol={}",
            self.base_url, symbol
        );

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .context("GET /api/v3/exchangeInfo request failed")?;

        let status = resp.status();
        let body: serde_json::Value = resp
            .json()
            .await
            .context("failed to parse exchangeInfo response")?;

        if !status.is_success() {
            anyhow::bail!(
                "Binance GET /api/v3/exchangeInfo returned {}: {}",
                status,
                body
            );
        }

        // Extract the first (and usually only) symbol entry.
        let info = body["symbols"]
            .as_array()
            .and_then(|arr| arr.first().cloned())
            .context("symbol not found in exchangeInfo response")?;

        debug!(symbol, "symbol info retrieved");
        Ok(info)
    }

    // -------------------------------------------------------------------------
    // Internal helpers
    // -------------------------------------------------------------------------

    /// Parse a JSON value that may be either a string or a number into `f64`.
    fn parse_str_f64(val: &serde_json::Value) -> Result<f64> {
        if let Some(s) = val.as_str() {
            s.parse::<f64>()
                .with_context(|| format!("failed to parse '{s}' as f64"))
        } else if let Some(n) = val.as_f64() {
            Ok(n)
        } else {
            anyhow::bail!("expected string or number, got: {val}")
        }
    }
}

impl std::fmt::Debug for BinanceClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BinanceClient")
            .field("api_key", &"<redacted>")
            .field("secret", &"<redacted>")
            .field("base_url", &self.base_url)
            .finish()
    }
}
