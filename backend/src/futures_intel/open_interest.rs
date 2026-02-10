// =============================================================================
// Open Interest Tracker — Participation / liquidation signal
// =============================================================================
//
// Open Interest (OI) represents the total number of outstanding derivative
// contracts.  Changes in OI indicate market participation levels:
//
//   OI rising   => new money entering the market
//   OI falling  => positions being closed / liquidated
//   OI drop > 10%/hr => potential liquidation cascade (BLOCK trading)

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

/// Snapshot of the current open interest and derived signal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OIState {
    /// Current open interest in contracts.
    pub current_oi: f64,

    /// Estimated OI change over the last hour (percent).
    pub oi_change_1h_pct: f64,

    /// Directional signal in [-1.0, +1.0].
    pub signal: f64,

    /// Human-readable explanation.
    pub interpretation: String,

    /// If true, the OI drop is severe enough to block new entries.
    pub block_trading: bool,
}

/// Fetches open interest data from the Binance Futures API.
///
/// Note: This is a stateless fetcher. OI change estimation is approximate
/// since we only get a single snapshot per call. For accurate hourly change,
/// the caller should track history externally.
pub struct OpenInterestTracker {
    client: reqwest::Client,
}

impl OpenInterestTracker {
    /// Create a new tracker with a default HTTP client.
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .expect("failed to build reqwest client for OpenInterestTracker"),
        }
    }

    /// Create a tracker that re-uses an existing HTTP client.
    pub fn with_client(client: reqwest::Client) -> Self {
        Self { client }
    }

    /// Fetch the current open interest for `symbol`.
    ///
    /// Returns the OI value and a basic signal. For more sophisticated
    /// directional analysis, combine with price direction externally.
    pub async fn fetch(&self, symbol: &str) -> Result<OIState> {
        let url = format!(
            "https://fapi.binance.com/fapi/v1/openInterest?symbol={}",
            symbol
        );

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .with_context(|| format!("GET open interest for {symbol}"))?;

        let status = resp.status();
        let body: serde_json::Value = resp
            .json()
            .await
            .context("failed to parse open interest response")?;

        if !status.is_success() {
            anyhow::bail!("open interest API returned {}: {}", status, body);
        }

        let current_oi: f64 = body["openInterest"]
            .as_str()
            .unwrap_or("0")
            .parse()
            .unwrap_or(0.0);

        // Without historical data in a single call, we report 0% change.
        // The caller (futures_intel_loop) tracks history via the state map.
        let oi_change_1h_pct = 0.0;

        // Basic signal: large OI suggests active market (neutral by itself).
        // Directional interpretation requires price context from the caller.
        let (signal, interpretation, block_trading) = if oi_change_1h_pct < -10.0 {
            (
                0.0,
                format!(
                    "OI dropping {:.1}%/hr - potential liquidation cascade, BLOCKING trades",
                    oi_change_1h_pct
                ),
                true,
            )
        } else {
            (
                0.0,
                format!("OI: {:.0} contracts - monitoring", current_oi),
                false,
            )
        };

        let state = OIState {
            current_oi,
            oi_change_1h_pct,
            signal,
            interpretation,
            block_trading,
        };

        debug!(
            symbol,
            current_oi,
            "open interest fetched"
        );

        Ok(state)
    }
}

impl Default for OpenInterestTracker {
    fn default() -> Self {
        Self::new()
    }
}
