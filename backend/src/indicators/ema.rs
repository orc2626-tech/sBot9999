// =============================================================================
// Exponential Moving Average (EMA)
// =============================================================================
//
// EMA gives more weight to recent prices, making it more responsive to new
// information than the Simple Moving Average (SMA).
//
// Formula:
//   multiplier = 2 / (period + 1)
//   EMA_t      = close_t * multiplier + EMA_{t-1} * (1 - multiplier)
//
// The very first EMA value is seeded with the SMA of the first `period` closes.
// =============================================================================

/// Compute the EMA series for the given `closes` slice and look-back `period`.
///
/// Returns an empty `Vec` when the input is too short or the period is zero.
/// Each output element corresponds to a close starting at index `period - 1`.
///
/// # Edge cases
/// - `period == 0` => empty vec (division by zero guard)
/// - `closes.len() < period` => empty vec
/// - Non-finite intermediate values are skipped; the computation resets.
pub fn calculate_ema(closes: &[f64], period: usize) -> Vec<f64> {
    if period == 0 || closes.len() < period {
        return Vec::new();
    }

    let divisor = (period + 1) as f64;
    // Guard against degenerate (should never happen with period >= 1, but be safe).
    if divisor == 0.0 {
        return Vec::new();
    }
    let multiplier = 2.0 / divisor;

    // Seed: SMA of the first `period` values.
    let sma: f64 = closes[..period].iter().sum::<f64>() / period as f64;
    if !sma.is_finite() {
        return Vec::new();
    }

    let mut result = Vec::with_capacity(closes.len() - period + 1);
    result.push(sma);

    let mut prev_ema = sma;
    for &close in &closes[period..] {
        let ema = close * multiplier + prev_ema * (1.0 - multiplier);
        if !ema.is_finite() {
            // If we hit a non-finite value, stop producing further results —
            // downstream consumers should not trust a broken series.
            break;
        }
        result.push(ema);
        prev_ema = ema;
    }

    result
}

/// Check whether the EMA-9 / EMA-21 / EMA-55 stack is trend-aligned.
///
/// Returns `Some((is_bullish, strength))` where:
/// - `is_bullish == true`  when EMA9 > EMA21 > EMA55  (bullish alignment)
/// - `is_bullish == false` when EMA9 < EMA21 < EMA55  (bearish alignment)
/// - `strength = |EMA9 - EMA55| / EMA55` — a normalised measure of spread
///
/// Returns `None` when:
/// - There are not enough data points for all three EMAs.
/// - Any of the required EMA series are empty.
/// - The EMAs are *not* fully aligned in either direction (mixed ordering).
/// - EMA55 is zero (division guard).
/// - The computed strength is non-finite.
pub fn ema_trend_aligned(closes: &[f64]) -> Option<(bool, f64)> {
    // We need at least 55 data points for EMA-55.
    if closes.len() < 55 {
        return None;
    }

    let ema9 = calculate_ema(closes, 9);
    let ema21 = calculate_ema(closes, 21);
    let ema55 = calculate_ema(closes, 55);

    // Take the most recent (last) value from each series.
    let e9 = *ema9.last()?;
    let e21 = *ema21.last()?;
    let e55 = *ema55.last()?;

    // Determine alignment.
    let bullish = e9 > e21 && e21 > e55;
    let bearish = e9 < e21 && e21 < e55;

    if !bullish && !bearish {
        return None; // Not clearly aligned.
    }

    // Division-by-zero guard.
    if e55 == 0.0 {
        return None;
    }

    let strength = (e9 - e55).abs() / e55;
    if !strength.is_finite() {
        return None;
    }

    Some((bullish, strength))
}

// =============================================================================
// Unit Tests
// =============================================================================
#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: build a simple ascending price series.
    fn ascending(n: usize) -> Vec<f64> {
        (1..=n).map(|i| i as f64).collect()
    }

    // ---- calculate_ema ---------------------------------------------------

    #[test]
    fn ema_empty_input() {
        assert!(calculate_ema(&[], 5).is_empty());
    }

    #[test]
    fn ema_period_zero() {
        assert!(calculate_ema(&[1.0, 2.0, 3.0], 0).is_empty());
    }

    #[test]
    fn ema_insufficient_data() {
        assert!(calculate_ema(&[1.0, 2.0], 5).is_empty());
    }

    #[test]
    fn ema_period_equals_length() {
        let closes = vec![2.0, 4.0, 6.0];
        let ema = calculate_ema(&closes, 3);
        assert_eq!(ema.len(), 1);
        // Should be the SMA = (2+4+6)/3 = 4.0
        assert!((ema[0] - 4.0).abs() < 1e-10);
    }

    #[test]
    fn ema_known_values() {
        // 5-period EMA of [1,2,3,4,5,6,7,8,9,10]
        // SMA of first 5 = 3.0, multiplier = 2/6 = 1/3
        let closes: Vec<f64> = (1..=10).map(|x| x as f64).collect();
        let ema = calculate_ema(&closes, 5);
        assert_eq!(ema.len(), 6); // indices 4..9

        let mult = 2.0 / 6.0;
        let mut expected = 3.0; // SMA seed
        let mut expected_vec = vec![expected];
        for &c in &closes[5..] {
            expected = c * mult + expected * (1.0 - mult);
            expected_vec.push(expected);
        }
        for (a, b) in ema.iter().zip(expected_vec.iter()) {
            assert!((a - b).abs() < 1e-10, "got {a}, expected {b}");
        }
    }

    #[test]
    fn ema_handles_nan_in_input() {
        let closes = vec![1.0, 2.0, 3.0, f64::NAN, 5.0];
        let ema = calculate_ema(&closes, 3);
        // SMA of first 3 = 2.0, then next value is NaN => EMA becomes NaN => break
        // So the result should just be the seed.
        assert_eq!(ema.len(), 1);
    }

    // ---- ema_trend_aligned -----------------------------------------------

    #[test]
    fn trend_aligned_insufficient_data() {
        let closes = ascending(50); // need 55
        assert!(ema_trend_aligned(&closes).is_none());
    }

    #[test]
    fn trend_aligned_bullish_ascending() {
        // A steadily rising series produces bullish alignment.
        let closes = ascending(200);
        let result = ema_trend_aligned(&closes);
        assert!(result.is_some());
        let (is_bullish, strength) = result.unwrap();
        assert!(is_bullish);
        assert!(strength > 0.0);
        assert!(strength.is_finite());
    }

    #[test]
    fn trend_aligned_bearish_descending() {
        // A steadily falling series produces bearish alignment.
        let closes: Vec<f64> = (1..=200).rev().map(|x| x as f64).collect();
        let result = ema_trend_aligned(&closes);
        assert!(result.is_some());
        let (is_bullish, _strength) = result.unwrap();
        assert!(!is_bullish);
    }

    #[test]
    fn trend_aligned_flat_returns_none() {
        // Perfectly flat series — all EMAs equal, no strict ordering.
        let closes = vec![100.0; 200];
        let result = ema_trend_aligned(&closes);
        // All EMAs converge to 100.0 => not strictly > or <, so None.
        assert!(result.is_none());
    }
}
