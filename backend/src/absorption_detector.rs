// =============================================================================
// Absorption Detector — Institutional volume absorption detection
// =============================================================================
//
// An "absorption" pattern occurs when large institutional players absorb
// selling (or buying) pressure.  It manifests as:
//
//   1. Very high volume (> 2x recent average)
//   2. Very small price range (< 0.5 ATR) — price did not move despite
//      massive volume
//   3. CVD (Cumulative Volume Delta) confirmation — net flow matches the
//      absorption direction
//
// When detected, this pattern provides a confidence boost for trade entries.

use crate::market_data::Candle;
use serde::{Deserialize, Serialize};
use tracing::debug;

/// Result of absorption pattern analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbsorptionState {
    pub detected: bool,
    pub direction: String,
    pub strength: f64,
    pub cvd_confirmed: bool,
    pub volume_ratio: f64,
    pub range_ratio: f64,
    pub reason: String,
}

/// Stateless absorption detector.
pub struct AbsorptionDetector;

impl AbsorptionDetector {
    /// Analyse recent 1M candles for absorption.
    ///
    /// `cvd_direction`: positive = net buying, negative = net selling.
    pub fn detect(candles_1m: &[Candle], cvd_direction: f64) -> Option<AbsorptionState> {
        if candles_1m.len() < 20 {
            return None;
        }

        let n = candles_1m.len();
        let lookback = 20.min(n);
        let recent = &candles_1m[n - lookback..];

        let avg_volume: f64 =
            recent.iter().map(|c| c.volume).sum::<f64>() / lookback as f64;

        if avg_volume < f64::EPSILON {
            return Some(no_detection("Average volume is zero"));
        }

        // ATR over lookback
        let mut atr_sum = 0.0_f64;
        let mut atr_count = 0usize;
        for i in 1..recent.len() {
            let tr = (recent[i].high - recent[i].low)
                .max((recent[i].high - recent[i - 1].close).abs())
                .max((recent[i].low - recent[i - 1].close).abs());
            atr_sum += tr;
            atr_count += 1;
        }

        if atr_count == 0 {
            return Some(no_detection("Cannot compute ATR"));
        }
        let atr = atr_sum / atr_count as f64;

        if atr < f64::EPSILON {
            return Some(no_detection("ATR is zero"));
        }

        // Scan last 3 candles
        let check_window = 3.min(recent.len());
        let last_candles = &recent[recent.len() - check_window..];

        let mut best: Option<AbsorptionState> = None;

        for candle in last_candles {
            let range = candle.high - candle.low;
            let range_ratio = range / atr;
            let volume_ratio = candle.volume / avg_volume;

            if volume_ratio > 2.0 && range_ratio < 0.5 {
                let direction = if candle.close >= candle.open {
                    "BULLISH"
                } else {
                    "BEARISH"
                };

                let cvd_confirmed = (direction == "BULLISH" && cvd_direction > 0.0)
                    || (direction == "BEARISH" && cvd_direction < 0.0);

                let strength = ((volume_ratio / 3.0).min(1.0) * (1.0 - range_ratio))
                    .clamp(0.0, 1.0);

                let body_ratio = if range > f64::EPSILON {
                    (candle.close - candle.open).abs() / range
                } else {
                    0.0
                };

                let reason = format!(
                    "Absorption {}: vol_ratio={:.2}x, range_ratio={:.3}, body_ratio={:.2}, cvd_confirmed={}",
                    direction, volume_ratio, range_ratio, body_ratio, cvd_confirmed
                );

                let state = AbsorptionState {
                    detected: true,
                    direction: direction.to_string(),
                    strength,
                    cvd_confirmed,
                    volume_ratio,
                    range_ratio,
                    reason,
                };

                match &best {
                    Some(prev) if prev.strength >= state.strength => {}
                    _ => best = Some(state),
                }
            }
        }

        let result = best.unwrap_or_else(|| no_detection("No absorption pattern detected"));

        debug!(
            detected = result.detected,
            direction = %result.direction,
            strength = format!("{:.3}", result.strength),
            "absorption detection complete"
        );

        Some(result)
    }
}

fn no_detection(reason: &str) -> AbsorptionState {
    AbsorptionState {
        detected: false,
        direction: "NONE".to_string(),
        strength: 0.0,
        cvd_confirmed: false,
        volume_ratio: 0.0,
        range_ratio: 0.0,
        reason: reason.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_candle(open: f64, high: f64, low: f64, close: f64, volume: f64) -> Candle {
        Candle {
            open_time: 0,
            close_time: 0,
            open,
            high,
            low,
            close,
            volume,
            quote_volume: 0.0,
            trades_count: 0,
            taker_buy_volume: 0.0,
            taker_buy_quote_volume: 0.0,
            is_closed: true,
        }
    }

    #[test]
    fn insufficient_data() {
        let candles: Vec<Candle> = (0..10)
            .map(|_| make_candle(100.0, 101.0, 99.0, 100.5, 100.0))
            .collect();
        assert!(AbsorptionDetector::detect(&candles, 1.0).is_none());
    }

    #[test]
    fn no_absorption_normal_market() {
        let candles: Vec<Candle> = (0..30)
            .map(|i| {
                let b = 100.0 + (i as f64) * 0.1;
                make_candle(b, b + 1.0, b - 1.0, b + 0.5, 100.0)
            })
            .collect();
        let s = AbsorptionDetector::detect(&candles, 0.5).unwrap();
        assert!(!s.detected);
    }

    #[test]
    fn absorption_high_volume_small_range() {
        let mut candles: Vec<Candle> = (0..25)
            .map(|i| {
                let b = 100.0 + (i as f64) * 0.1;
                make_candle(b, b + 1.0, b - 1.0, b + 0.5, 100.0)
            })
            .collect();
        candles.push(make_candle(102.5, 102.55, 102.49, 102.53, 500.0));
        let s = AbsorptionDetector::detect(&candles, 1.0).unwrap();
        assert!(s.detected);
        assert_eq!(s.direction, "BULLISH");
    }
}
