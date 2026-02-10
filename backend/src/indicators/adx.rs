// =============================================================================
// Average Directional Index (ADX)
// =============================================================================
//
// ADX quantifies trend **strength** regardless of direction.
//
// Calculation pipeline:
//   1. Compute +DM (positive directional movement) and -DM per bar.
//   2. Compute True Range (TR) per bar.
//   3. Apply Wilder's smoothing (period) to +DM, -DM, and TR.
//   4. Derive +DI = smoothed(+DM) / smoothed(TR) * 100
//            -DI = smoothed(-DM) / smoothed(TR) * 100
//   5. DX  = |+DI - -DI| / (+DI + -DI) * 100
//   6. ADX = Wilder's smoothed average of DX over `period` bars.
//
// Interpretation:
//   ADX > 25  => trending market
//   ADX < 20  => ranging / choppy market
// =============================================================================

use crate::market_data::Candle;

/// Compute the most recent ADX value from a slice of OHLCV candles.
///
/// Returns `None` when:
/// - `period` is zero.
/// - There are fewer than `2 * period` candles (we need `period` bars for the
///   initial Wilder's smoothing **and** another `period` DX values to seed the
///   ADX average).
/// - Any intermediate calculation produces a non-finite result.
pub fn calculate_adx(candles: &[Candle], period: usize) -> Option<f64> {
    if period == 0 {
        return None;
    }

    // We need at least 2*period + 1 candles to produce one ADX value.
    // (period candles for initial smoothing of +DM/-DM/TR, then period DX
    // values to seed the ADX, plus the very first candle that has no
    // predecessor.)
    let min_candles = 2 * period + 1;
    if candles.len() < min_candles {
        return None;
    }

    let period_f = period as f64;

    // ------------------------------------------------------------------
    // Step 1 & 2: Raw +DM, -DM, and True Range for each consecutive pair
    // ------------------------------------------------------------------
    let n = candles.len();
    let bar_count = n - 1; // number of bar-to-bar transitions

    let mut plus_dm = Vec::with_capacity(bar_count);
    let mut minus_dm = Vec::with_capacity(bar_count);
    let mut tr_vals = Vec::with_capacity(bar_count);

    for i in 1..n {
        let high = candles[i].high;
        let low = candles[i].low;
        let prev_high = candles[i - 1].high;
        let prev_low = candles[i - 1].low;
        let prev_close = candles[i - 1].close;

        // True Range
        let tr = (high - low)
            .max((high - prev_close).abs())
            .max((low - prev_close).abs());

        // Directional Movement
        let up_move = high - prev_high;
        let down_move = prev_low - low;

        let pdm = if up_move > down_move && up_move > 0.0 {
            up_move
        } else {
            0.0
        };
        let mdm = if down_move > up_move && down_move > 0.0 {
            down_move
        } else {
            0.0
        };

        plus_dm.push(pdm);
        minus_dm.push(mdm);
        tr_vals.push(tr);
    }

    // ------------------------------------------------------------------
    // Step 3: Wilder's smoothing of +DM, -DM, TR (first `period` values)
    // ------------------------------------------------------------------
    let mut smooth_plus_dm: f64 = plus_dm[..period].iter().sum();
    let mut smooth_minus_dm: f64 = minus_dm[..period].iter().sum();
    let mut smooth_tr: f64 = tr_vals[..period].iter().sum();

    // Collect DX values starting at index `period`.
    let mut dx_values: Vec<f64> = Vec::with_capacity(bar_count - period + 1);

    // First DI / DX at index `period - 1` (after initial sum).
    if let Some(dx) = compute_dx(smooth_plus_dm, smooth_minus_dm, smooth_tr) {
        dx_values.push(dx);
    } else {
        return None;
    }

    // Continue Wilder's smoothing for bars `period .. bar_count`.
    for i in period..bar_count {
        smooth_plus_dm = smooth_plus_dm - smooth_plus_dm / period_f + plus_dm[i];
        smooth_minus_dm = smooth_minus_dm - smooth_minus_dm / period_f + minus_dm[i];
        smooth_tr = smooth_tr - smooth_tr / period_f + tr_vals[i];

        if let Some(dx) = compute_dx(smooth_plus_dm, smooth_minus_dm, smooth_tr) {
            dx_values.push(dx);
        } else {
            return None;
        }
    }

    // ------------------------------------------------------------------
    // Step 6: ADX = Wilder's smoothed average of DX
    // ------------------------------------------------------------------
    if dx_values.len() < period {
        return None;
    }

    // Seed ADX with SMA of first `period` DX values.
    let adx_seed: f64 = dx_values[..period].iter().sum::<f64>() / period_f;
    if !adx_seed.is_finite() {
        return None;
    }

    let mut adx = adx_seed;
    for &dx in &dx_values[period..] {
        adx = (adx * (period_f - 1.0) + dx) / period_f;
        if !adx.is_finite() {
            return None;
        }
    }

    if adx.is_finite() {
        Some(adx)
    } else {
        None
    }
}

// =============================================================================
// Internal helpers
// =============================================================================

/// Compute DX from smoothed +DM, -DM, and TR values.
///
/// Returns `None` if the divisor is zero or the result is non-finite.
fn compute_dx(smooth_plus_dm: f64, smooth_minus_dm: f64, smooth_tr: f64) -> Option<f64> {
    if smooth_tr == 0.0 {
        return None;
    }

    let plus_di = (smooth_plus_dm / smooth_tr) * 100.0;
    let minus_di = (smooth_minus_dm / smooth_tr) * 100.0;

    let di_sum = plus_di + minus_di;
    if di_sum == 0.0 {
        // Both +DI and -DI are zero — no directional movement.
        return Some(0.0);
    }

    let dx = ((plus_di - minus_di).abs() / di_sum) * 100.0;

    if dx.is_finite() {
        Some(dx)
    } else {
        None
    }
}

// =============================================================================
// Unit Tests
// =============================================================================
#[cfg(test)]
mod tests {
    use super::*;

    /// Build a simple candle for testing.  Volume and timestamps are irrelevant
    /// for the ADX calculation so we use fixed dummy values.
    fn candle(open: f64, high: f64, low: f64, close: f64) -> Candle {
        Candle {
            open_time: 0,
            open,
            high,
            low,
            close,
            volume: 1.0,
            close_time: 0,
        }
    }

    #[test]
    fn adx_period_zero() {
        let candles = vec![candle(1.0, 2.0, 0.5, 1.5); 50];
        assert!(calculate_adx(&candles, 0).is_none());
    }

    #[test]
    fn adx_insufficient_data() {
        let candles = vec![candle(1.0, 2.0, 0.5, 1.5); 10];
        assert!(calculate_adx(&candles, 14).is_none());
    }

    #[test]
    fn adx_strong_uptrend() {
        // Consecutive higher highs and higher lows — a strong trend.
        let candles: Vec<Candle> = (0..60)
            .map(|i| {
                let base = 100.0 + i as f64 * 2.0;
                candle(base, base + 1.5, base - 0.5, base + 1.0)
            })
            .collect();

        let adx = calculate_adx(&candles, 14);
        assert!(adx.is_some());
        let value = adx.unwrap();
        // Strong trend should produce ADX well above 25.
        assert!(value > 25.0, "expected ADX > 25 for strong trend, got {value}");
    }

    #[test]
    fn adx_flat_market() {
        // Identical candles — no directional movement.
        let candles = vec![candle(100.0, 101.0, 99.0, 100.0); 60];
        let adx = calculate_adx(&candles, 14);
        // DX = 0 for every bar => ADX converges to 0.
        assert!(adx.is_some());
        let value = adx.unwrap();
        assert!(
            value < 1.0,
            "expected ADX near 0 for flat market, got {value}"
        );
    }

    #[test]
    fn adx_result_range() {
        // ADX should always be in [0, 100].
        let candles: Vec<Candle> = (0..100)
            .map(|i| {
                let base = 50.0 + (i as f64 * 0.3).sin() * 10.0;
                candle(base - 0.5, base + 1.0, base - 1.0, base + 0.5)
            })
            .collect();
        if let Some(value) = calculate_adx(&candles, 14) {
            assert!(
                (0.0..=100.0).contains(&value),
                "ADX {value} out of [0,100] range"
            );
        }
    }

    #[test]
    fn adx_minimum_candles_exact() {
        // Exactly 2*period + 1 candles should produce a result.
        let period = 5;
        let min = 2 * period + 1; // 11
        let candles: Vec<Candle> = (0..min)
            .map(|i| {
                let base = 100.0 + i as f64;
                candle(base, base + 1.0, base - 0.5, base + 0.5)
            })
            .collect();
        assert!(calculate_adx(&candles, period).is_some());

        // One fewer should fail.
        assert!(calculate_adx(&candles[..min - 1], period).is_none());
    }
}
