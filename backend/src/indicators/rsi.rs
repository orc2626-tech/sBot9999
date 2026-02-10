// =============================================================================
// Relative Strength Index (RSI) — Wilder's Smoothing
// =============================================================================
//
// RSI measures the speed and magnitude of recent price changes to evaluate
// whether an asset is overbought or oversold.
//
// Step 1 — Compute price changes (deltas) from consecutive closes.
// Step 2 — Seed average gain / average loss with the SMA of the first `period`
//          gains / losses.
// Step 3 — Apply Wilder's exponential smoothing:
//            avg_gain = (prev_avg_gain * (period - 1) + current_gain) / period
//            avg_loss = (prev_avg_loss * (period - 1) + current_loss) / period
// Step 4 — RS  = avg_gain / avg_loss
//          RSI = 100 - 100 / (1 + RS)
//
// Thresholds:  RSI > 70 => OVERBOUGHT,  RSI < 30 => OVERSOLD.
// =============================================================================

/// Compute the full RSI series for the given `closes` and `period`.
///
/// The returned vector has one RSI value for each close starting at index
/// `period` (the first `period` closes are consumed to seed the averages).
///
/// # Edge cases
/// - `period == 0` => empty vec
/// - `closes.len() < period + 1` => empty vec (need at least `period` deltas)
/// - If average loss is zero (no down moves), RSI is clamped to 100.0.
/// - Non-finite results are dropped and the series is truncated.
pub fn calculate_rsi(closes: &[f64], period: usize) -> Vec<f64> {
    if period == 0 || closes.len() < period + 1 {
        return Vec::new();
    }

    // --- Compute price deltas ------------------------------------------------
    let deltas: Vec<f64> = closes.windows(2).map(|w| w[1] - w[0]).collect();

    // --- Seed averages with SMA of first `period` deltas ---------------------
    let (sum_gain, sum_loss) = deltas[..period].iter().fold((0.0_f64, 0.0_f64), |(g, l), &d| {
        if d > 0.0 {
            (g + d, l)
        } else {
            (g, l + d.abs())
        }
    });

    let period_f = period as f64;
    let mut avg_gain = sum_gain / period_f;
    let mut avg_loss = sum_loss / period_f;

    // First RSI value.
    let first_rsi = rsi_from_averages(avg_gain, avg_loss);
    if first_rsi.is_none() {
        return Vec::new();
    }

    let mut result = Vec::with_capacity(deltas.len() - period + 1);
    result.push(first_rsi.unwrap());

    // --- Wilder's smoothing for subsequent values ----------------------------
    for &delta in &deltas[period..] {
        let gain = if delta > 0.0 { delta } else { 0.0 };
        let loss = if delta < 0.0 { delta.abs() } else { 0.0 };

        avg_gain = (avg_gain * (period_f - 1.0) + gain) / period_f;
        avg_loss = (avg_loss * (period_f - 1.0) + loss) / period_f;

        match rsi_from_averages(avg_gain, avg_loss) {
            Some(rsi) => result.push(rsi),
            None => break, // Non-finite — stop producing values.
        }
    }

    result
}

/// Convenience function: return the most recent RSI value together with a
/// human-readable label.
///
/// Returns `None` when there is insufficient data or the calculation produces
/// a non-finite result.
pub fn current_rsi(closes: &[f64], period: usize) -> Option<(f64, &'static str)> {
    let series = calculate_rsi(closes, period);
    let value = *series.last()?;

    let label = if value >= 70.0 {
        "OVERBOUGHT"
    } else if value <= 30.0 {
        "OVERSOLD"
    } else {
        "NEUTRAL"
    };

    Some((value, label))
}

// =============================================================================
// Internal helpers
// =============================================================================

/// Convert average gain / average loss into an RSI value in [0, 100].
///
/// - If both averages are zero, RSI is 50.0 (no movement).
/// - If average loss is zero (only gains), RSI is 100.0.
/// - Returns `None` when the result is non-finite.
fn rsi_from_averages(avg_gain: f64, avg_loss: f64) -> Option<f64> {
    let rsi = if avg_loss == 0.0 && avg_gain == 0.0 {
        50.0 // No movement at all — neutral.
    } else if avg_loss == 0.0 {
        100.0 // All gains, no losses.
    } else {
        let rs = avg_gain / avg_loss;
        100.0 - 100.0 / (1.0 + rs)
    };

    if rsi.is_finite() {
        Some(rsi)
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

    // ---- calculate_rsi ---------------------------------------------------

    #[test]
    fn rsi_empty_input() {
        assert!(calculate_rsi(&[], 14).is_empty());
    }

    #[test]
    fn rsi_period_zero() {
        assert!(calculate_rsi(&[1.0, 2.0, 3.0], 0).is_empty());
    }

    #[test]
    fn rsi_insufficient_data() {
        // Need period+1 closes (period deltas). 14 closes => 13 deltas < 14.
        assert!(calculate_rsi(&(1..=14).map(|x| x as f64).collect::<Vec<_>>(), 14).is_empty());
    }

    #[test]
    fn rsi_all_gains() {
        // Strictly ascending prices => RSI should be 100.
        let closes: Vec<f64> = (1..=30).map(|x| x as f64).collect();
        let series = calculate_rsi(&closes, 14);
        assert!(!series.is_empty());
        for &v in &series {
            assert!((v - 100.0).abs() < 1e-10, "expected 100.0, got {v}");
        }
    }

    #[test]
    fn rsi_all_losses() {
        // Strictly descending prices => RSI should be 0.
        let closes: Vec<f64> = (1..=30).rev().map(|x| x as f64).collect();
        let series = calculate_rsi(&closes, 14);
        assert!(!series.is_empty());
        for &v in &series {
            assert!(v.abs() < 1e-10, "expected 0.0, got {v}");
        }
    }

    #[test]
    fn rsi_flat_market() {
        // No price change at all => RSI = 50 (neutral).
        let closes = vec![100.0; 30];
        let series = calculate_rsi(&closes, 14);
        assert!(!series.is_empty());
        for &v in &series {
            assert!((v - 50.0).abs() < 1e-10, "expected 50.0, got {v}");
        }
    }

    #[test]
    fn rsi_range_check() {
        // Arbitrary data — RSI must always be in [0, 100].
        let closes = vec![
            44.34, 44.09, 44.15, 43.61, 44.33, 44.83, 45.10, 45.42, 45.84, 46.08,
            45.89, 46.03, 44.18, 44.22, 44.57, 43.42, 42.66, 43.13,
        ];
        let series = calculate_rsi(&closes, 14);
        for &v in &series {
            assert!((0.0..=100.0).contains(&v), "RSI {v} out of range");
        }
    }

    // ---- current_rsi -----------------------------------------------------

    #[test]
    fn current_rsi_overbought() {
        // All gains => RSI = 100 => "OVERBOUGHT"
        let closes: Vec<f64> = (1..=30).map(|x| x as f64).collect();
        let (val, label) = current_rsi(&closes, 14).unwrap();
        assert!((val - 100.0).abs() < 1e-10);
        assert_eq!(label, "OVERBOUGHT");
    }

    #[test]
    fn current_rsi_oversold() {
        // All losses => RSI = 0 => "OVERSOLD"
        let closes: Vec<f64> = (1..=30).rev().map(|x| x as f64).collect();
        let (val, label) = current_rsi(&closes, 14).unwrap();
        assert!(val.abs() < 1e-10);
        assert_eq!(label, "OVERSOLD");
    }

    #[test]
    fn current_rsi_neutral() {
        let closes = vec![100.0; 30];
        let (val, label) = current_rsi(&closes, 14).unwrap();
        assert!((val - 50.0).abs() < 1e-10);
        assert_eq!(label, "NEUTRAL");
    }

    #[test]
    fn current_rsi_none_on_bad_input() {
        assert!(current_rsi(&[], 14).is_none());
    }
}
