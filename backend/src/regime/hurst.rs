// =============================================================================
// Hurst Exponent — Rescaled Range (R/S) Analysis
// =============================================================================
//
// The Hurst exponent H characterises the long-term memory of a time series:
//
//   H > 0.55  =>  trending / persistent (momentum regime)
//   H ~ 0.50  =>  random walk (geometric Brownian motion)
//   H < 0.45  =>  mean-reverting / anti-persistent
//
// Algorithm:
//   1. For each window size n in {8, 16, 32, 64}:
//      a. Split the closing prices into non-overlapping chunks of length n.
//      b. For each chunk compute the R/S statistic:
//         - Compute the mean of the chunk.
//         - Compute the cumulative deviation from the mean.
//         - R = max(cumulative) - min(cumulative).
//         - S = standard deviation of the chunk.
//         - R/S = R / S  (skip if S == 0).
//      c. Average the R/S values across all chunks for this window size.
//   2. Perform ordinary least-squares regression of log(avg R/S) on log(n).
//      The slope of the fitted line is the Hurst exponent.
//   3. Clamp the result to [0.0, 1.0].

use tracing::trace;

/// Minimum number of closing prices required for the analysis.
const MIN_CLOSES: usize = 64;

/// The set of window sizes used for the multi-scale R/S computation.
const WINDOW_SIZES: [usize; 4] = [8, 16, 32, 64];

/// Calculate the Hurst exponent of a price series via Rescaled Range analysis.
///
/// Returns `None` when:
/// - Fewer than [`MIN_CLOSES`] data points are supplied.
/// - Every chunk in every window size has zero standard deviation (degenerate
///   series such as a flat line).
/// - The log-log regression is degenerate (all x-values identical, which cannot
///   happen with the fixed window sizes, but is guarded against anyway).
pub fn calculate_hurst_exponent(closes: &[f64]) -> Option<f64> {
    if closes.len() < MIN_CLOSES {
        trace!(
            len = closes.len(),
            min = MIN_CLOSES,
            "Hurst: insufficient data"
        );
        return None;
    }

    let mut log_n: Vec<f64> = Vec::with_capacity(WINDOW_SIZES.len());
    let mut log_rs: Vec<f64> = Vec::with_capacity(WINDOW_SIZES.len());

    for &window in &WINDOW_SIZES {
        if window > closes.len() {
            continue;
        }

        let num_chunks = closes.len() / window;
        if num_chunks == 0 {
            continue;
        }

        let mut rs_sum: f64 = 0.0;
        let mut valid_chunks: usize = 0;

        for chunk_idx in 0..num_chunks {
            let start = chunk_idx * window;
            let end = start + window;
            let chunk = &closes[start..end];

            let mean = chunk.iter().sum::<f64>() / window as f64;

            // Standard deviation (population σ).
            let variance =
                chunk.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / window as f64;
            let std_dev = variance.sqrt();

            if std_dev < f64::EPSILON {
                // Flat chunk — no information; skip.
                continue;
            }

            // Cumulative deviation from the mean.
            let mut cumulative = Vec::with_capacity(window);
            let mut running = 0.0_f64;
            for &val in chunk {
                running += val - mean;
                cumulative.push(running);
            }

            let range = cumulative
                .iter()
                .cloned()
                .fold(f64::NEG_INFINITY, f64::max)
                - cumulative
                    .iter()
                    .cloned()
                    .fold(f64::INFINITY, f64::min);

            let rs = range / std_dev;
            rs_sum += rs;
            valid_chunks += 1;
        }

        if valid_chunks == 0 {
            continue;
        }

        let avg_rs = rs_sum / valid_chunks as f64;
        log_n.push((window as f64).ln());
        log_rs.push(avg_rs.ln());
    }

    // Need at least 2 points for a meaningful regression.
    if log_n.len() < 2 {
        trace!("Hurst: insufficient valid window sizes for regression");
        return None;
    }

    // Ordinary least-squares: slope = Σ((x-x̄)(y-ȳ)) / Σ((x-x̄)²)
    let n = log_n.len() as f64;
    let x_mean = log_n.iter().sum::<f64>() / n;
    let y_mean = log_rs.iter().sum::<f64>() / n;

    let mut numerator = 0.0_f64;
    let mut denominator = 0.0_f64;

    for i in 0..log_n.len() {
        let dx = log_n[i] - x_mean;
        let dy = log_rs[i] - y_mean;
        numerator += dx * dy;
        denominator += dx * dx;
    }

    if denominator.abs() < f64::EPSILON {
        trace!("Hurst: degenerate regression (zero variance in log_n)");
        return None;
    }

    let hurst = (numerator / denominator).clamp(0.0, 1.0);

    trace!(
        hurst = format!("{:.4}", hurst),
        points = log_n.len(),
        "Hurst exponent computed"
    );

    Some(hurst)
}

// =============================================================================
// Unit Tests
// =============================================================================
#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: generate a simple trending series (cumulative sum of positive
    /// increments). A strongly trending series should yield H > 0.5.
    fn trending_series(len: usize) -> Vec<f64> {
        let mut v = Vec::with_capacity(len);
        let mut price = 100.0;
        for i in 0..len {
            price += 0.5 + 0.1 * (i as f64).sin().abs();
            v.push(price);
        }
        v
    }

    /// Helper: generate a mean-reverting series (oscillating around a mean).
    fn mean_reverting_series(len: usize) -> Vec<f64> {
        let mut v = Vec::with_capacity(len);
        for i in 0..len {
            // Alternating pattern: high amplitude oscillation
            let base = 100.0;
            let oscillation = if i % 2 == 0 { 5.0 } else { -5.0 };
            v.push(base + oscillation + 0.01 * (i as f64));
        }
        v
    }

    /// Helper: generate a random-walk-like series using a simple deterministic
    /// PRNG so the test is reproducible.
    fn pseudorandom_walk(len: usize, seed: u64) -> Vec<f64> {
        let mut v = Vec::with_capacity(len);
        let mut price = 100.0;
        let mut state = seed;
        for _ in 0..len {
            // xorshift64
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let r = (state as f64 / u64::MAX as f64) - 0.5; // uniform in [-0.5, 0.5]
            price += r;
            v.push(price);
        }
        v
    }

    #[test]
    fn test_insufficient_data_returns_none() {
        let closes = vec![1.0; 63];
        assert!(calculate_hurst_exponent(&closes).is_none());
    }

    #[test]
    fn test_flat_series_returns_none() {
        // All identical values => every chunk has S=0 => no valid R/S => None
        let closes = vec![42.0; 128];
        assert!(calculate_hurst_exponent(&closes).is_none());
    }

    #[test]
    fn test_trending_series_high_hurst() {
        let closes = trending_series(256);
        let h = calculate_hurst_exponent(&closes).expect("should produce a value");
        assert!(
            h > 0.50,
            "Trending series should have H > 0.50, got {:.4}",
            h
        );
    }

    #[test]
    fn test_mean_reverting_series_low_hurst() {
        let closes = mean_reverting_series(256);
        let h = calculate_hurst_exponent(&closes).expect("should produce a value");
        assert!(
            h < 0.55,
            "Mean-reverting series should have H < 0.55, got {:.4}",
            h
        );
    }

    #[test]
    fn test_hurst_clamped_to_unit_interval() {
        // Even with adversarial input the result must be in [0, 1].
        let closes = trending_series(128);
        let h = calculate_hurst_exponent(&closes).unwrap();
        assert!((0.0..=1.0).contains(&h), "H={:.4} out of [0,1]", h);
    }

    #[test]
    fn test_random_walk_near_half() {
        let closes = pseudorandom_walk(512, 123_456_789);
        let h = calculate_hurst_exponent(&closes).expect("should produce a value");
        // Random walks should be *roughly* near 0.5 — allow a generous band.
        assert!(
            (0.25..=0.80).contains(&h),
            "Random walk Hurst should be broadly near 0.5, got {:.4}",
            h
        );
    }

    #[test]
    fn test_exact_min_data() {
        let closes = trending_series(64);
        // Should not panic; may or may not return Some depending on chunk validity.
        let _ = calculate_hurst_exponent(&closes);
    }

    #[test]
    fn test_determinism() {
        let closes = trending_series(256);
        let a = calculate_hurst_exponent(&closes);
        let b = calculate_hurst_exponent(&closes);
        assert_eq!(a, b, "Hurst exponent should be deterministic");
    }
}
