// =============================================================================
// Rate of Change (ROC) — Momentum Indicator
// =============================================================================
//
// ROC measures the percentage change in price over a look-back period:
//   ROC = ((close - close_n) / close_n) * 100
//
// Positive ROC indicates upward momentum; negative indicates downward.

/// Calculate the Rate of Change (ROC) for the given closing prices and period.
///
/// Returns a vector of ROC values, one per close starting at index `period`.
pub fn calculate_roc(closes: &[f64], period: usize) -> Vec<f64> {
    if period == 0 || closes.len() <= period {
        return Vec::new();
    }

    let mut result = Vec::with_capacity(closes.len() - period);
    for i in period..closes.len() {
        let prev = closes[i - period];
        if prev == 0.0 {
            result.push(0.0);
        } else {
            result.push(((closes[i] - prev) / prev) * 100.0);
        }
    }
    result
}

/// Return the most recent ROC value.
pub fn current_roc(closes: &[f64], period: usize) -> Option<f64> {
    let series = calculate_roc(closes, period);
    series.last().copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roc_basic() {
        let closes: Vec<f64> = (1..=20).map(|x| x as f64).collect();
        let roc = calculate_roc(&closes, 14);
        assert!(!roc.is_empty());
        // From 1 to 15: ROC = (15-1)/1 * 100 = 1400%
        assert!((roc[0] - 1400.0).abs() < 1e-10);
    }

    #[test]
    fn roc_insufficient_data() {
        let closes = vec![1.0, 2.0, 3.0];
        assert!(calculate_roc(&closes, 14).is_empty());
    }
}
