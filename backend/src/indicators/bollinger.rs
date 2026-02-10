// =============================================================================
// Bollinger Bands
// =============================================================================
//
// Bollinger Bands consist of a middle band (SMA), an upper band (SMA + k*σ),
// and a lower band (SMA - k*σ). The Band Width (BBW) is the normalised
// distance: BBW = (upper - lower) / middle * 100.
//
// BBW is the primary metric used by the regime detector.

/// Result of a Bollinger Band calculation.
#[derive(Debug, Clone)]
pub struct BollingerResult {
    pub upper: f64,
    pub middle: f64,
    pub lower: f64,
    pub width: f64,
}

/// Calculate Bollinger Bands for the given closing prices.
///
/// Returns `Some(BollingerResult)` containing:
/// - `upper`  = SMA + `num_std` * σ
/// - `middle` = SMA
/// - `lower`  = SMA - `num_std` * σ
/// - `width`  = (upper - lower) / middle * 100  (Bollinger Band Width)
///
/// Returns `None` when:
/// - Fewer than `period` data points.
/// - Middle band is zero (degenerate input).
pub fn calculate_bollinger(closes: &[f64], period: usize, num_std: f64) -> Option<BollingerResult> {
    if period == 0 || closes.len() < period {
        return None;
    }

    let window = &closes[closes.len() - period..];
    let sum: f64 = window.iter().sum();
    let middle = sum / period as f64;

    if middle == 0.0 {
        return None;
    }

    let variance = window.iter().map(|x| (x - middle).powi(2)).sum::<f64>() / period as f64;
    let std_dev = variance.sqrt();

    let upper = middle + num_std * std_dev;
    let lower = middle - num_std * std_dev;
    let width = (upper - lower) / middle * 100.0;

    if width.is_finite() {
        Some(BollingerResult {
            upper,
            middle,
            lower,
            width,
        })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bollinger_basic() {
        let closes: Vec<f64> = (1..=20).map(|x| x as f64).collect();
        let result = calculate_bollinger(&closes, 20, 2.0);
        assert!(result.is_some());
        let bb = result.unwrap();
        assert!(bb.upper > bb.middle);
        assert!(bb.lower < bb.middle);
        assert!(bb.width > 0.0);
    }

    #[test]
    fn bollinger_insufficient_data() {
        let closes = vec![1.0, 2.0, 3.0];
        assert!(calculate_bollinger(&closes, 20, 2.0).is_none());
    }

    #[test]
    fn bollinger_flat() {
        let closes = vec![100.0; 20];
        let result = calculate_bollinger(&closes, 20, 2.0);
        assert!(result.is_some());
        assert!((result.unwrap().width - 0.0).abs() < 1e-10);
    }
}
