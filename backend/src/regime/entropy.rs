// =============================================================================
// Shannon Entropy Filter — Information-Theoretic Trade Filter
// =============================================================================
//
// Measures the randomness of the price series by classifying each candle as
// UP (close > open) or DOWN (close <= open) and computing the binary Shannon
// entropy of that distribution over a rolling window.
//
//   H = -p_up * log2(p_up) - p_down * log2(p_down)
//
// For a binary variable the maximum entropy is 1.0 (50/50 split = pure noise).
//
// Decision thresholds:
//   H >= 0.95  =>  BLOCK   — market is essentially random noise
//   0.80 <= H < 0.95 => REDUCE — uncertain, halve position sizes
//   H < 0.80  =>  CLEAR   — sufficient directional bias to trade
//
// The adjustment factor multiplies into position sizing downstream.

use crate::market_data::Candle;
use tracing::{trace, warn};

/// Default lookback window for entropy calculation.
const DEFAULT_WINDOW: usize = 50;

/// Entropy threshold at or above which trading is blocked.
const BLOCK_THRESHOLD: f64 = 0.95;

/// Entropy threshold at or above which position size is reduced.
const REDUCE_THRESHOLD: f64 = 0.80;

/// Shannon Entropy Filter for candle classification.
///
/// Stateless — all state lives in the candle buffer passed by the caller.
pub struct ShannonEntropyFilter;

impl ShannonEntropyFilter {
    /// Calculate Shannon entropy over the last `window` candles.
    ///
    /// Returns `None` if there are fewer candles than `window` or if the window
    /// is zero.
    pub fn calculate(candles: &[Candle], window: usize) -> Option<f64> {
        if window == 0 || candles.len() < window {
            trace!(
                available = candles.len(),
                window,
                "Entropy: insufficient candles"
            );
            return None;
        }

        let slice = &candles[candles.len() - window..];

        let up_count = slice
            .iter()
            .filter(|c| c.close > c.open)
            .count();

        let p_up = up_count as f64 / window as f64;
        let p_down = 1.0 - p_up;

        let entropy = binary_entropy(p_up, p_down);

        trace!(
            p_up = format!("{:.4}", p_up),
            entropy = format!("{:.4}", entropy),
            window,
            "Entropy calculated"
        );

        Some(entropy)
    }

    /// Convenience method: compute entropy with the default window and return a
    /// trading decision tuple.
    ///
    /// Returns `(allowed, entropy, adjustment_factor)`:
    ///
    /// - `allowed`: `false` when entropy >= 0.95 (BLOCK).
    /// - `entropy`: the raw Shannon entropy value.
    /// - `adjustment_factor`: 0.0 (BLOCK), 0.5 (REDUCE), or 1.0 (CLEAR).
    ///
    /// If entropy cannot be computed (insufficient data) the filter defaults to
    /// CLEAR (1.0) — we do not want missing data to silently block all trading.
    pub fn check(candles: &[Candle]) -> (bool, f64, f64) {
        match Self::calculate(candles, DEFAULT_WINDOW) {
            Some(entropy) => {
                let (allowed, factor) = if entropy >= BLOCK_THRESHOLD {
                    warn!(
                        entropy = format!("{:.4}", entropy),
                        "Entropy BLOCK: market is near-random noise"
                    );
                    (false, 0.0)
                } else if entropy >= REDUCE_THRESHOLD {
                    trace!(
                        entropy = format!("{:.4}", entropy),
                        "Entropy REDUCE: uncertain regime"
                    );
                    (true, 0.5)
                } else {
                    trace!(
                        entropy = format!("{:.4}", entropy),
                        "Entropy CLEAR: sufficient directional bias"
                    );
                    (true, 1.0)
                };

                (allowed, entropy, factor)
            }
            None => {
                // Insufficient data — default to permissive.
                trace!("Entropy: insufficient data, defaulting to CLEAR");
                (true, 0.0, 1.0)
            }
        }
    }
}

/// Binary Shannon entropy: H = -p * log2(p) - q * log2(q).
///
/// Handles the degenerate cases where p or q is 0 (0 * log2(0) := 0).
#[inline]
fn binary_entropy(p: f64, q: f64) -> f64 {
    let h_p = if p > 0.0 { -p * p.log2() } else { 0.0 };
    let h_q = if q > 0.0 { -q * q.log2() } else { 0.0 };
    h_p + h_q
}

// =============================================================================
// Unit Tests
// =============================================================================
#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to build a candle with just open and close.
    fn candle(open: f64, close: f64) -> Candle {
        Candle {
            open_time: 0,
            close_time: 0,
            open,
            high: close.max(open),
            low: close.min(open),
            close,
            volume: 1.0,
            quote_volume: 0.0,
            trades_count: 0,
            taker_buy_volume: 0.0,
            taker_buy_quote_volume: 0.0,
            is_closed: true,
        }
    }

    #[test]
    fn test_all_up_candles_zero_entropy() {
        // All UP => p_up = 1.0 => H = 0.0
        let candles: Vec<Candle> = (0..50).map(|_| candle(100.0, 110.0)).collect();
        let h = ShannonEntropyFilter::calculate(&candles, 50).unwrap();
        assert!(
            h.abs() < 1e-10,
            "All-UP candles should have entropy ~0, got {:.6}",
            h
        );
    }

    #[test]
    fn test_all_down_candles_zero_entropy() {
        let candles: Vec<Candle> = (0..50).map(|_| candle(110.0, 100.0)).collect();
        let h = ShannonEntropyFilter::calculate(&candles, 50).unwrap();
        assert!(
            h.abs() < 1e-10,
            "All-DOWN candles should have entropy ~0, got {:.6}",
            h
        );
    }

    #[test]
    fn test_balanced_candles_max_entropy() {
        // 25 UP, 25 DOWN => p_up = 0.5 => H = 1.0
        let mut candles = Vec::new();
        for i in 0..50 {
            if i < 25 {
                candles.push(candle(100.0, 110.0));
            } else {
                candles.push(candle(110.0, 100.0));
            }
        }
        let h = ShannonEntropyFilter::calculate(&candles, 50).unwrap();
        assert!(
            (h - 1.0).abs() < 1e-10,
            "Balanced candles should have entropy ~1.0, got {:.6}",
            h
        );
    }

    #[test]
    fn test_check_block() {
        // 25 UP, 25 DOWN => H = 1.0 >= 0.95 => BLOCK
        let mut candles = Vec::new();
        for i in 0..50 {
            if i < 25 {
                candles.push(candle(100.0, 110.0));
            } else {
                candles.push(candle(110.0, 100.0));
            }
        }
        let (allowed, entropy, factor) = ShannonEntropyFilter::check(&candles);
        assert!(!allowed, "Should be blocked when entropy=1.0");
        assert!((entropy - 1.0).abs() < 1e-10);
        assert!((factor - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_check_clear() {
        // All UP => H = 0 => CLEAR
        let candles: Vec<Candle> = (0..50).map(|_| candle(100.0, 110.0)).collect();
        let (allowed, entropy, factor) = ShannonEntropyFilter::check(&candles);
        assert!(allowed, "Should be allowed when entropy=0");
        assert!(entropy.abs() < 1e-10);
        assert!((factor - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_insufficient_data_returns_none() {
        let candles: Vec<Candle> = (0..10).map(|_| candle(100.0, 110.0)).collect();
        assert!(ShannonEntropyFilter::calculate(&candles, 50).is_none());
    }

    #[test]
    fn test_check_insufficient_data_defaults_clear() {
        let candles: Vec<Candle> = (0..10).map(|_| candle(100.0, 110.0)).collect();
        let (allowed, _, factor) = ShannonEntropyFilter::check(&candles);
        assert!(allowed);
        assert!((factor - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_window_zero_returns_none() {
        let candles: Vec<Candle> = (0..50).map(|_| candle(100.0, 110.0)).collect();
        assert!(ShannonEntropyFilter::calculate(&candles, 0).is_none());
    }

    #[test]
    fn test_reduce_zone() {
        // Construct a series with ~85% UP candles.
        // p_up=0.86, H = -0.86*log2(0.86) - 0.14*log2(0.14) ≈ 0.615
        // Actually, need to hit 0.80-0.95 range.
        // p_up = 0.70 => H ≈ 0.881
        let mut candles = Vec::new();
        for i in 0..50 {
            if i < 35 {
                candles.push(candle(100.0, 110.0)); // UP
            } else {
                candles.push(candle(110.0, 100.0)); // DOWN
            }
        }
        let h = ShannonEntropyFilter::calculate(&candles, 50).unwrap();
        // With 35 UP / 15 DOWN, p=0.70 => H ≈ 0.881
        assert!(
            h >= REDUCE_THRESHOLD && h < BLOCK_THRESHOLD,
            "Expected entropy in REDUCE zone, got {:.4}",
            h
        );
        let (allowed, _, factor) = ShannonEntropyFilter::check(&candles);
        assert!(allowed);
        assert!((factor - 0.5).abs() < 1e-10);
    }
}
