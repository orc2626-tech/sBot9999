// =============================================================================
// Funding Rate Monitor — Contrarian signal from perpetual futures funding
// =============================================================================
//
// Funding rates are periodic payments between longs and shorts that anchor the
// perpetual contract price to spot.
//
// Contrarian logic:
//   rate > +0.05%  =>  signal = -0.8  (overleveraged longs, expect dump)
//   rate > +0.03%  =>  signal = -0.4  (moderate long bias)
//   rate < -0.05%  =>  signal = +0.9  (extreme short squeeze setup)
//   rate < -0.03%  =>  signal = +0.5  (shorts paying, mild bullish)
//   otherwise      =>  signal =  0.0  (neutral)

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

/// Snapshot of the current funding rate and derived signal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundingState {
    /// Raw funding rate as a decimal (e.g. 0.0001 = 0.01%).
    pub rate: f64,

    /// Funding rate as a percentage (e.g. 0.01).
    pub rate_pct: f64,

    /// Contrarian signal in [-1.0, +1.0].
    pub signal: f64,

    /// Directional bias label.
    pub bias: String,

    /// Timestamp (ms) of the next funding event.
    pub next_funding_time: i64,

    /// Human-readable explanation.
    pub interpretation: String,
}

/// Fetches and interprets funding rate data from the Binance Futures API.
pub struct FundingRateMonitor {
    client: reqwest::Client,
}

impl FundingRateMonitor {
    /// Create a new monitor with a default HTTP client.
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .expect("failed to build reqwest client for FundingRateMonitor"),
        }
    }

    /// Create a monitor that re-uses an existing HTTP client.
    pub fn with_client(client: reqwest::Client) -> Self {
        Self { client }
    }

    /// Fetch the latest funding rate for `symbol` and interpret it.
    pub async fn fetch(&self, symbol: &str) -> Result<FundingState> {
        let url = format!(
            "https://fapi.binance.com/fapi/v1/fundingRate?symbol={}&limit=1",
            symbol
        );

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .with_context(|| format!("GET funding rate for {symbol}"))?;

        let status = resp.status();
        let body: serde_json::Value = resp
            .json()
            .await
            .context("failed to parse funding rate response body")?;

        if !status.is_success() {
            anyhow::bail!("funding rate API returned {}: {}", status, body);
        }

        let arr = body
            .as_array()
            .context("funding rate response is not an array")?;
        let entry = arr
            .first()
            .context("funding rate response array is empty")?;

        let rate: f64 = entry["fundingRate"]
            .as_str()
            .unwrap_or("0")
            .parse()
            .unwrap_or(0.0);

        let next_funding_time = entry["fundingTime"].as_i64().unwrap_or(0);
        let rate_pct = rate * 100.0;

        // Contrarian signal interpretation.
        let (signal, bias, interpretation) = if rate_pct > 0.05 {
            (
                -0.8,
                "BEARISH",
                "Extreme positive funding - overleveraged longs, contrarian short",
            )
        } else if rate_pct > 0.03 {
            (
                -0.4,
                "BEARISH",
                "Elevated positive funding - moderate contrarian short",
            )
        } else if rate_pct < -0.05 {
            (
                0.9,
                "BULLISH",
                "Extreme negative funding - short squeeze likely, contrarian long",
            )
        } else if rate_pct < -0.03 {
            (
                0.5,
                "BULLISH",
                "Elevated negative funding - shorts paying, contrarian long",
            )
        } else if rate_pct > 0.01 {
            (
                -0.1,
                "NEUTRAL",
                "Slightly positive funding - normal conditions",
            )
        } else if rate_pct < -0.01 {
            (
                0.2,
                "NEUTRAL",
                "Slightly negative funding - mild bullish lean",
            )
        } else {
            (0.0, "NEUTRAL", "Neutral funding rate - no signal")
        };

        let state = FundingState {
            rate,
            rate_pct,
            signal,
            bias: bias.to_string(),
            next_funding_time,
            interpretation: interpretation.to_string(),
        };

        debug!(
            symbol,
            rate_pct = format!("{:.4}", rate_pct),
            signal,
            bias,
            "funding rate fetched"
        );

        Ok(state)
    }
}

impl Default for FundingRateMonitor {
    fn default() -> Self {
        Self::new()
    }
}
