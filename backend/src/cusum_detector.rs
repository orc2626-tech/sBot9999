// =============================================================================
// CUSUM Structural Break Detector — Detecting regime shifts on 5M candles
// =============================================================================
//
// Cumulative Sum (CUSUM) control chart for detecting mean-shifts in the price
// process.  Two one-sided statistics accumulate deviations from the rolling
// mean:
//
//   S+_t = max(0, S+_{t-1} + x_t - mu - k)   (detects upward shift)
//   S-_t = max(0, S-_{t-1} - x_t + mu - k)   (detects downward shift)
//
// When either statistic exceeds `threshold` (= 4 * sigma), a structural break
// is declared.
//
// CUSUM x HTF soft-block: if CUSUM detects a bullish break but the HTF gate
// is bearish (or vice versa), the confidence factor drops to 0.5 instead of
// outright blocking the trade.

use serde::{Deserialize, Serialize};
use tracing::debug;

// =============================================================================
// Types
// =============================================================================

/// Full state from one CUSUM detection pass.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CusumState {
    /// Upper CUSUM statistic (last value).
    pub s_plus: f64,
    /// Lower CUSUM statistic (last value).
    pub s_minus: f64,
    /// Decision threshold (4 * sigma).
    pub threshold: f64,
    /// Rolling mean of the input series.
    pub rolling_mean: f64,
    /// Rolling standard deviation of the input series.
    pub rolling_std: f64,
    /// True if an upward (bullish) structural break was detected.
    pub bullish_break: bool,
    /// True if a downward (bearish) structural break was detected.
    pub bearish_break: bool,
    /// Confidence in the break [0.0, 1.0].
    pub break_confidence: f64,
    /// Number of candles since the most recent break.
    pub candles_since_break: usize,
    /// Human-readable explanation.
    pub reason: String,
}

// =============================================================================
// CusumDetector
// =============================================================================

/// Stateful CUSUM detector that retains a rolling history of past detections.
pub struct CusumDetector {
    history: Vec<CusumState>,
    max_history: usize,
}

impl CusumDetector {
    /// Create a new detector that retains at most `max_history` past states.
    pub fn new(max_history: usize) -> Self {
        Self {
            history: Vec::new(),
            max_history,
        }
    }

    /// Run CUSUM detection on 5M close prices.
    ///
    /// Returns `None` when the input has fewer than 20 data points or zero
    /// variance.
    pub fn detect(&mut self, candles_5m: &[f64]) -> Option<CusumState> {
        if candles_5m.len() < 20 {
            return None;
        }

        let n = candles_5m.len();

        let rolling_mean = candles_5m.iter().sum::<f64>() / n as f64;
        let variance = candles_5m
            .iter()
            .map(|x| (x - rolling_mean).powi(2))
            .sum::<f64>()
            / n as f64;
        let rolling_std = variance.sqrt();

        if rolling_std < f64::EPSILON {
            return None;
        }

        let threshold = rolling_std * 4.0;
        let k = rolling_std * 0.5;

        let mut s_plus = 0.0_f64;
        let mut s_minus = 0.0_f64;
        let mut bullish_break = false;
        let mut bearish_break = false;
        let mut candles_since_break = n;

        for (i, &val) in candles_5m.iter().enumerate() {
            let deviation = val - rolling_mean;

            s_plus = (s_plus + deviation - k).max(0.0);
            s_minus = (s_minus - deviation - k).max(0.0);

            if s_plus > threshold {
                bullish_break = true;
                candles_since_break = n - 1 - i;
                s_plus = 0.0;
            }

            if s_minus > threshold {
                bearish_break = true;
                candles_since_break = n - 1 - i;
                s_minus = 0.0;
            }
        }

        let break_confidence = if bullish_break || bearish_break {
            let recency = 1.0 - (candles_since_break as f64 / n as f64);
            let strength = (s_plus.max(s_minus) / threshold).min(1.0);
            (recency * 0.6 + strength * 0.4).clamp(0.0, 1.0)
        } else {
            0.0
        };

        let reason = if bullish_break && bearish_break {
            format!(
                "Both bullish and bearish breaks (confidence={:.2}, {} candles ago)",
                break_confidence, candles_since_break
            )
        } else if bullish_break {
            format!(
                "Bullish structural break (confidence={:.2}, {} candles ago)",
                break_confidence, candles_since_break
            )
        } else if bearish_break {
            format!(
                "Bearish structural break (confidence={:.2}, {} candles ago)",
                break_confidence, candles_since_break
            )
        } else {
            "No structural break detected".to_string()
        };

        let state = CusumState {
            s_plus,
            s_minus,
            threshold,
            rolling_mean,
            rolling_std,
            bullish_break,
            bearish_break,
            break_confidence,
            candles_since_break,
            reason,
        };

        self.history.push(state.clone());
        if self.history.len() > self.max_history {
            self.history.remove(0);
        }

        debug!(
            bullish_break,
            bearish_break,
            break_confidence = format!("{:.3}", break_confidence),
            candles_since_break,
            "CUSUM detection complete"
        );

        Some(state)
    }

    /// Return the most recently computed state.
    pub fn last_state(&self) -> Option<&CusumState> {
        self.history.last()
    }

    /// CUSUM x HTF soft-block factor.
    ///
    /// CUSUM bullish + HTF bearish (or vice versa) => 0.5 (soft-block).
    pub fn htf_conflict_factor(cusum: &CusumState, htf_bullish: bool) -> f64 {
        if cusum.bullish_break && !htf_bullish {
            0.5
        } else if cusum.bearish_break && htf_bullish {
            0.5
        } else {
            1.0
        }
    }
}

impl Default for CusumDetector {
    fn default() -> Self {
        Self::new(100)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_insufficient_data() {
        let mut d = CusumDetector::new(10);
        assert!(d.detect(&[1.0; 10]).is_none());
    }

    #[test]
    fn detect_flat_series_returns_none() {
        let mut d = CusumDetector::new(10);
        assert!(d.detect(&[100.0; 50]).is_none());
    }

    #[test]
    fn detect_strong_uptrend_bullish_break() {
        let mut d = CusumDetector::new(10);
        let mut data = vec![100.0; 30];
        for i in 0..20 {
            data.push(100.0 + (i as f64) * 3.0);
        }
        let s = d.detect(&data).unwrap();
        assert!(s.bullish_break);
    }

    #[test]
    fn htf_conflict_halves_factor() {
        let state = CusumState {
            s_plus: 0.0,
            s_minus: 0.0,
            threshold: 1.0,
            rolling_mean: 100.0,
            rolling_std: 1.0,
            bullish_break: true,
            bearish_break: false,
            break_confidence: 0.8,
            candles_since_break: 2,
            reason: String::new(),
        };
        assert!((CusumDetector::htf_conflict_factor(&state, false) - 0.5).abs() < f64::EPSILON);
        assert!((CusumDetector::htf_conflict_factor(&state, true) - 1.0).abs() < f64::EPSILON);
    }
}
