// =============================================================================
// Futures Intelligence Module
// =============================================================================
//
// Aggregates three independent data sources from the Binance Futures API to
// build a composite directional bias:
//
//   1. Funding Rate   — contrarian signal (extreme funding predicts reversal)
//   2. Open Interest  — participation signal (OI + price divergence = caution)
//   3. Long/Short Ratio — crowd positioning (contrarian fade at extremes)
//
// Each sub-module fetches data independently and produces a normalised signal
// in [-1.0, +1.0].  The composite signal is the equal-weighted average.

pub mod funding_rate;
pub mod long_short_ratio;
pub mod open_interest;

pub use funding_rate::{FundingRateMonitor, FundingState};
pub use long_short_ratio::{LongShortMonitor, LSState};
pub use open_interest::{OIState, OpenInterestTracker};

use chrono::Utc;
use serde::{Deserialize, Serialize};

/// Aggregated futures intelligence for a single symbol.
#[derive(Debug, Clone, Serialize)]
pub struct FuturesIntelState {
    /// The symbol this intelligence pertains to.
    pub symbol: String,

    /// Equal-weighted average of available sub-signals in [-1.0, +1.0].
    pub composite_signal: f64,

    /// Human-readable bias label: BULLISH / BEARISH / NEUTRAL.
    pub composite_bias: String,

    /// ISO 8601 timestamp of the last update.
    pub last_update: String,
}

impl FuturesIntelState {
    /// Create a new blank state for `symbol`.
    pub fn new(symbol: impl Into<String>) -> Self {
        Self {
            symbol: symbol.into(),
            composite_signal: 0.0,
            composite_bias: "NEUTRAL".to_string(),
            last_update: Utc::now().to_rfc3339(),
        }
    }

    /// Recompute the composite signal and bias from individual signal values.
    pub fn update_composite(&mut self, signals: &[f64]) {
        let count = signals.len();
        if count > 0 {
            self.composite_signal = signals.iter().sum::<f64>() / count as f64;
        } else {
            self.composite_signal = 0.0;
        }

        self.composite_bias = if self.composite_signal > 0.2 {
            "BULLISH".to_string()
        } else if self.composite_signal < -0.2 {
            "BEARISH".to_string()
        } else {
            "NEUTRAL".to_string()
        };

        self.last_update = Utc::now().to_rfc3339();
    }
}
