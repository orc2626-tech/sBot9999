// =============================================================================
// Average True Range (ATR) — Wilder's Smoothing Method
// =============================================================================
//
// ATR measures market volatility by decomposing the entire range of a bar.
//
// True Range (TR) for each bar:
//   TR = max(H - L, |H - prevClose|, |L - prevClose|)
//
// ATR is then the smoothed average of TR using Wilder's method:
//   ATR_0   = SMA of first `period` TR values
//   ATR_t   = (ATR_{t-1} * (period - 1) + TR_t) / period
//
// Default period: 14
// =============================================================================

use crate::market_data::Candle;

/// Compute the most recent ATR value from a slice of OHLCV candles using
/// Wilder's smoothing method.
///
/// # Arguments
/// - `candles` — slice of OHLCV candles (oldest first).
/// - `period`  — look-back window for the ATR calculation.
///
/// # Returns
/// `None` when:
/// - `period` is zero.
/// - There are fewer than `period + 1` candles (we need `period` TR values,
///   each requiring a previous candle for the True Range calculation).
/// - Any intermediate value is non-finite.
pub fn calculate_atr(candles: &[Candle], period: usize) -> Option<f64> {
    if period == 0 || candles.len() < period + 1 {
        return None;
    }

    // --- Step 1: Compute True Range for each consecutive pair ----------------
    let mut tr_values: Vec<f64> = Vec::with_capacity(candles.len() - 1);
    for i in 1..candles.len() {
        let high = candles[i].high;
        let low = candles[i].low;
        let prev_close = candles[i - 1].close;

        let hl = high - low;
        let hc = (high - prev_close).abs();
        let lc = (low - prev_close).abs();

        tr_values.push(hl.max(hc).max(lc));
    }

    if tr_values.len() < period {
        return None;
    }

    // --- Step 2: Seed ATR with SMA of first `period` TR values ---------------
    let seed: f64 = tr_values[..period].iter().sum::<f64>() / period as f64;
    if !seed.is_finite() {
        return None;
    }

    // --- Step 3: Wilder's smoothing for remaining TR values ------------------
    let period_f = period as f64;
    let mut atr = seed;
    for &tr in &tr_values[period..] {
        atr = (atr * (period_f - 1.0) + tr) / period_f;
        if !atr.is_finite() {
            return None;
        }
    }

    if atr.is_finite() {
        Some(atr)
    } else {
        None
    }
}

/// Calculate ATR as a percentage of the current price.
///
/// Useful for comparing volatility across assets with different price scales.
pub fn calculate_atr_pct(candles: &[Candle], period: usize) -> Option<f64> {
    let atr = calculate_atr(candles, period)?;
    let last_close = candles.last()?.close;
    if last_close == 0.0 {
        return None;
    }
    Some((atr / last_close) * 100.0)
}

/// Convenience function: compute ATR with the standard 14-period default.
///
/// Used by the regime detector and other modules that want a quick ATR read
/// without specifying the period explicitly.
pub fn calculate(candles: &[Candle]) -> Option<f64> {
    calculate_atr(candles, 14)
}

// =============================================================================
// Unit Tests
// =============================================================================
#[cfg(test)]
mod tests {
    use super::*;

    /// Build a test candle with the given OHLC values.
    fn candle(open: f64, high: f64, low: f64, close: f64) -> Candle {
        Candle {
            open_time: 0,
            close_time: 0,
            open,
            high,
            low,
            close,
            volume: 100.0,
            quote_volume: 200.0,
            trades_count: 50,
            taker_buy_volume: 60.0,
            taker_buy_quote_volume: 120.0,
            is_closed: true,
        }
    }

    #[test]
    fn atr_period_zero() {
        let candles = vec![candle(100.0, 105.0, 95.0, 102.0); 20];
        assert!(calculate_atr(&candles, 0).is_none());
    }

    #[test]
    fn atr_insufficient_data() {
        // Need period + 1 = 15 candles for period=14, only have 10.
        let candles = vec![candle(100.0, 105.0, 95.0, 102.0); 10];
        assert!(calculate_atr(&candles, 14).is_none());
    }

    #[test]
    fn atr_exact_minimum_data() {
        // period=3, need 4 candles to get 3 TR values.
        let candles = vec![
            candle(100.0, 102.0, 98.0, 101.0),
            candle(101.0, 104.0, 99.0, 103.0),
            candle(103.0, 106.0, 100.0, 105.0),
            candle(105.0, 108.0, 102.0, 107.0),
        ];
        let atr = calculate_atr(&candles, 3);
        assert!(atr.is_some());
        let val = atr.unwrap();
        assert!(val > 0.0);
        assert!(val.is_finite());
    }

    #[test]
    fn atr_constant_range() {
        // All candles have the same range (H-L=10), close at midpoint.
        // TR should be constant and ATR should converge to 10.
        let mut candles = Vec::new();
        for i in 0..30 {
            let base = 100.0 + i as f64 * 0.1; // slight drift
            candles.push(candle(base, base + 5.0, base - 5.0, base));
        }
        let atr = calculate_atr(&candles, 14).unwrap();
        assert!(
            (atr - 10.0).abs() < 1.0,
            "expected ATR near 10.0, got {atr}"
        );
    }

    #[test]
    fn atr_increasing_volatility() {
        let mut candles = Vec::new();
        candles.push(candle(100.0, 101.0, 99.0, 100.0));
        for i in 1..30 {
            let spread = 1.0 + i as f64 * 0.5;
            let base = 100.0;
            candles.push(candle(base, base + spread, base - spread, base));
        }
        let atr = calculate_atr(&candles, 5).unwrap();
        assert!(atr > 5.0, "expected ATR > 5.0 for increasing vol, got {atr}");
    }

    #[test]
    fn atr_result_is_positive() {
        let candles: Vec<Candle> = (0..50)
            .map(|i| {
                let base = 100.0 + (i as f64 * 0.5).sin() * 10.0;
                candle(base - 0.5, base + 2.0, base - 2.0, base + 0.5)
            })
            .collect();
        let atr = calculate_atr(&candles, 14).unwrap();
        assert!(atr > 0.0, "ATR must be positive, got {atr}");
    }

    #[test]
    fn atr_true_range_uses_prev_close() {
        // Gap scenario: |H - prevClose| > H - L
        let candles = vec![
            candle(100.0, 105.0, 95.0, 95.0),  // close at low
            candle(110.0, 115.0, 108.0, 112.0), // gap up: |115-95|=20 > 115-108=7
            candle(112.0, 118.0, 110.0, 115.0),
            candle(115.0, 120.0, 113.0, 118.0),
        ];
        let atr = calculate_atr(&candles, 3).unwrap();
        // First TR = 20 (|115-95|), so ATR should reflect this gap.
        assert!(atr > 7.0, "ATR should reflect the gap, got {atr}");
    }

    #[test]
    fn atr_pct() {
        let candles: Vec<Candle> = (0..30)
            .map(|i| {
                let base = 100.0 + i as f64;
                candle(base, base + 3.0, base - 3.0, base + 1.0)
            })
            .collect();
        let atr_pct = calculate_atr_pct(&candles, 14);
        assert!(atr_pct.is_some());
        let val = atr_pct.unwrap();
        assert!(val > 0.0);
        assert!(val.is_finite());
    }

    #[test]
    fn atr_convenience_function() {
        let candles: Vec<Candle> = (0..30)
            .map(|i| {
                let base = 100.0 + i as f64;
                candle(base, base + 3.0, base - 3.0, base + 1.0)
            })
            .collect();
        let atr_14 = calculate_atr(&candles, 14);
        let atr_conv = calculate(&candles);
        assert_eq!(atr_14, atr_conv);
    }

    #[test]
    fn atr_nan_returns_none() {
        let candles = vec![
            candle(100.0, 105.0, 95.0, 100.0),
            candle(100.0, f64::NAN, 95.0, 100.0),
            candle(100.0, 105.0, 95.0, 100.0),
            candle(100.0, 105.0, 95.0, 100.0),
        ];
        assert!(calculate_atr(&candles, 3).is_none());
    }
}
