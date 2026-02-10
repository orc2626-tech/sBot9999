// =============================================================================
// Long/Short Ratio Monitor — Crowd positioning contrarian signal
// =============================================================================
//
// The global long/short account ratio shows the proportion of accounts that are
// net long versus net short.  Extreme readings are contrarian:
//
//   long% > 70%  =>  signal = -0.9  (crowded long, fade)
//   long% > 65%  =>  signal = -0.5  (moderately crowded long)
//   short% > 70% =>  signal = +0.9  (crowded short, fade)
//   short% > 65% =>  signal = +0.5  (moderately crowded short)
//   otherwise    =>  signal =  0.0  (balanced, no signal)

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tracing::debug;

/// Snapshot of the current long/short ratio and derived signal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LSState {
    /// Percentage of accounts that are net long (0-100).
    pub long_pct: f64,

    /// Percentage of accounts that are net short (0-100).
    pub short_pct: f64,

    /// Raw long/short ratio (e.g. 1.5 means 60% long / 40% short).
    pub ratio: f64,

    /// Contrarian signal in [-1.0, +1.0].
    pub signal: f64,

    /// Directional bias label.
    pub bias: String,
}

/// Fetches and interprets the Binance global long/short account ratio.
pub struct LongShortMonitor {
    client: reqwest::Client,
}

impl LongShortMonitor {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .expect("failed to build reqwest client for LongShortMonitor"),
        }
    }

    pub fn with_client(client: reqwest::Client) -> Self {
        Self { client }
    }

    /// Fetch the latest long/short ratio for `symbol`.
    pub async fn fetch(&self, symbol: &str) -> Result<LSState> {
        let url = format!(
            "https://fapi.binance.com/futures/data/globalLongShortAccountRatio?symbol={}&period=5m&limit=1",
            symbol
        );

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .with_context(|| format!("GET long/short ratio for {symbol}"))?;

        let status = resp.status();
        let body: serde_json::Value = resp
            .json()
            .await
            .context("failed to parse long/short ratio response")?;

        if !status.is_success() {
            anyhow::bail!("long/short ratio API returned {}: {}", status, body);
        }

        let arr = body
            .as_array()
            .context("long/short ratio response is not an array")?;
        let entry = arr
            .first()
            .context("long/short ratio response array is empty")?;

        // The API returns longAccount / shortAccount as decimal fractions and
        // longShortRatio as the ratio itself.
        let long_account: f64 = entry["longAccount"]
            .as_str()
            .unwrap_or("0.5")
            .parse()
            .unwrap_or(0.5);
        let short_account: f64 = entry["shortAccount"]
            .as_str()
            .unwrap_or("0.5")
            .parse()
            .unwrap_or(0.5);
        let ratio: f64 = entry["longShortRatio"]
            .as_str()
            .unwrap_or("1.0")
            .parse()
            .unwrap_or(1.0);

        let long_pct = long_account * 100.0;
        let short_pct = short_account * 100.0;

        // Contrarian signal logic.
        let (signal, bias) = if long_pct > 70.0 {
            (-0.9, "BEARISH")
        } else if long_pct > 65.0 {
            (-0.5, "BEARISH")
        } else if short_pct > 70.0 {
            (0.9, "BULLISH")
        } else if short_pct > 65.0 {
            (0.5, "BULLISH")
        } else if long_pct > 55.0 {
            (-0.2, "SLIGHTLY_BEARISH")
        } else if short_pct > 55.0 {
            (0.2, "SLIGHTLY_BULLISH")
        } else {
            (0.0, "NEUTRAL")
        };

        let state = LSState {
            long_pct,
            short_pct,
            ratio,
            signal,
            bias: bias.to_string(),
        };

        debug!(
            symbol,
            long_pct = format!("{:.1}", long_pct),
            short_pct = format!("{:.1}", short_pct),
            ratio = format!("{:.3}", ratio),
            signal,
            bias,
            "long/short ratio fetched"
        );

        Ok(state)
    }
}

impl Default for LongShortMonitor {
    fn default() -> Self {
        Self::new()
    }
}
