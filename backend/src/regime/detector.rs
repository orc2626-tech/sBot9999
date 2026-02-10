// =============================================================================
// Market Regime Detector
// =============================================================================
//
// Classifies the current market into one of five regimes using a multi-factor
// approach. Each regime carries recommended risk parameters (R:R ratio and
// maximum position size) so that downstream strategy modules can adapt
// automatically.
//
// Detection hierarchy (evaluated top-to-bottom; first match wins):
//
//   1. DEAD      — Entropy >= 0.95 (pure noise, no edge)
//   2. VOLATILE  — BBW > 5.0       (extreme volatility expansion)
//   3. SQUEEZE   — BBW < 1.5 AND ADX < 20 (compression, pre-breakout)
//   4. TRENDING  — ADX > 25 AND Hurst > 0.55 (persistent directional move)
//   5. RANGING   — ADX < 20 AND Hurst < 0.45 (mean-reverting chop)
//
// If no rule fires, the regime defaults to RANGING with low confidence.

use std::sync::Arc;
use std::time::Instant;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tracing::{debug, trace};

use crate::indicators::adx::calculate_adx;
use crate::indicators::atr::calculate_atr;
use crate::indicators::bollinger::calculate_bollinger;
use crate::market_data::Candle;
use crate::regime::entropy::ShannonEntropyFilter;
use crate::regime::hurst::calculate_hurst_exponent;

// =============================================================================
// Types
// =============================================================================

/// High-level market regime classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MarketRegime {
    /// Strong directional move with persistence.
    Trending,
    /// Sideways chop — mean-reverting price action.
    Ranging,
    /// Extreme volatility expansion — wide swings.
    Volatile,
    /// Low-volatility compression — potential breakout imminent.
    Squeeze,
    /// Near-maximum entropy — market behaves as random noise.
    Dead,
}

impl std::fmt::Display for MarketRegime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Trending => write!(f, "TRENDING"),
            Self::Ranging => write!(f, "RANGING"),
            Self::Volatile => write!(f, "VOLATILE"),
            Self::Squeeze => write!(f, "SQUEEZE"),
            Self::Dead => write!(f, "DEAD"),
        }
    }
}

/// Complete snapshot of the detected regime plus all contributing metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegimeState {
    /// The classified regime.
    pub regime: MarketRegime,

    /// Average Directional Index (trend strength).
    pub adx: f64,

    /// Bollinger Band Width (volatility metric).
    pub bbw: f64,

    /// Hurst exponent (persistence / mean-reversion).
    pub hurst: f64,

    /// Shannon entropy of candle direction distribution.
    pub entropy: f64,

    /// Confidence in the regime classification [0.0, 1.0].
    pub confidence: f64,

    /// Number of seconds the current regime has been active.
    pub regime_age_secs: f64,

    /// Recommended reward : risk ratio (reward, risk) for this regime.
    pub recommended_rr: (f64, f64),

    /// Maximum position size as a percentage of available equity.
    pub max_position_pct: f64,
}

// =============================================================================
// Decision matrix: per-regime risk parameters
// =============================================================================

impl MarketRegime {
    /// Returns the risk-management tuple for this regime:
    /// `(recommended_rr, max_position_pct)`.
    fn risk_params(self) -> ((f64, f64), f64) {
        match self {
            // Trending: let winners run — wider R:R, larger size.
            Self::Trending => ((3.0, 1.0), 100.0),
            // Ranging: tight mean-reversion scalps.
            Self::Ranging => ((1.5, 1.0), 60.0),
            // Volatile: big moves, big risk — tighter position sizing.
            Self::Volatile => ((2.5, 1.0), 40.0),
            // Squeeze: small stop, big potential — aggressive R:R, small size.
            Self::Squeeze => ((4.0, 1.0), 30.0),
            // Dead: no edge — do not trade.
            Self::Dead => ((0.0, 0.0), 0.0),
        }
    }
}

// =============================================================================
// RegimeDetector
// =============================================================================

/// Thread-safe regime detector that caches the latest regime state.
///
/// Designed to be wrapped in an `Arc` and shared across the async runtime.
pub struct RegimeDetector {
    /// The most recently detected regime state (if any).
    state: RwLock<Option<RegimeState>>,

    /// Wall-clock instant of the last regime *change* (not merely re-detection
    /// of the same regime).
    last_change_time: RwLock<Instant>,
}

impl RegimeDetector {
    /// Create a new detector with no initial state.
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            state: RwLock::new(None),
            last_change_time: RwLock::new(Instant::now()),
        })
    }

    /// Run full regime detection on the provided candles and closing prices.
    ///
    /// `candles` — the most recent OHLCV candles (latest last).
    /// `closes`  — closing prices extracted from `candles` (same order/length).
    ///
    /// Returns the freshly computed [`RegimeState`], or `None` when input data
    /// is insufficient for any of the underlying indicators.
    pub fn detect(&self, candles: &[Candle], closes: &[f64]) -> Option<RegimeState> {
        // --- Compute indicators ------------------------------------------------
        let adx_value = calculate_adx(candles, 14).unwrap_or(0.0);
        let bb_result = calculate_bollinger(closes, 20, 2.0)?;
        let bbw_value = bb_result.width;
        let _atr_value = calculate_atr(candles, 14).unwrap_or(0.0);
        let hurst_value = calculate_hurst_exponent(closes).unwrap_or(0.50);
        let entropy_value = ShannonEntropyFilter::calculate(candles, 50).unwrap_or(0.0);

        // --- Classification (ordered by priority) ------------------------------
        let (regime, confidence) = classify(adx_value, bbw_value, hurst_value, entropy_value);

        // --- Risk parameters ---------------------------------------------------
        let (recommended_rr, max_position_pct) = regime.risk_params();

        // --- Regime age tracking -----------------------------------------------
        let now = Instant::now();

        let prev_regime = self.state.read().as_ref().map(|s| s.regime);
        if prev_regime != Some(regime) {
            *self.last_change_time.write() = now;
        }

        let regime_age_secs = now
            .duration_since(*self.last_change_time.read())
            .as_secs_f64();

        let new_state = RegimeState {
            regime,
            adx: adx_value,
            bbw: bbw_value,
            hurst: hurst_value,
            entropy: entropy_value,
            confidence,
            regime_age_secs,
            recommended_rr,
            max_position_pct,
        };

        debug!(
            regime = %regime,
            adx = format!("{:.2}", adx_value),
            bbw = format!("{:.2}", bbw_value),
            hurst = format!("{:.4}", hurst_value),
            entropy = format!("{:.4}", entropy_value),
            confidence = format!("{:.2}", confidence),
            age_secs = format!("{:.1}", regime_age_secs),
            "Regime detected"
        );

        *self.state.write() = Some(new_state.clone());
        Some(new_state)
    }

    /// Convenience wrapper around [`detect`] that extracts closing prices from
    /// the candle slice automatically. This is the entry point used by the
    /// regime detection loop in `main.rs`.
    pub fn update(&self, candles: &[Candle]) -> Option<RegimeState> {
        let closes: Vec<f64> = candles.iter().map(|c| c.close).collect();
        self.detect(candles, &closes)
    }

    /// Return the most recently detected regime state without recomputing.
    pub fn current_regime(&self) -> Option<RegimeState> {
        self.state.read().clone()
    }
}

impl Default for RegimeDetector {
    fn default() -> Self {
        Self {
            state: RwLock::new(None),
            last_change_time: RwLock::new(Instant::now()),
        }
    }
}

// =============================================================================
// Classification logic
// =============================================================================

/// Determine the regime and a confidence score from the raw indicator values.
fn classify(adx: f64, bbw: f64, hurst: f64, entropy: f64) -> (MarketRegime, f64) {
    // 1. DEAD — entropy dominates; market is noise.
    if entropy >= 0.95 {
        let confidence = remap(entropy, 0.95, 1.0, 0.70, 1.0);
        return (MarketRegime::Dead, confidence);
    }

    // 2. VOLATILE — extreme band expansion.
    if bbw > 5.0 {
        let confidence = remap(bbw, 5.0, 10.0, 0.65, 1.0);
        return (MarketRegime::Volatile, confidence);
    }

    // 3. SQUEEZE — compression zone.
    if bbw < 1.5 && adx < 20.0 {
        // Confidence increases as BBW contracts and ADX falls.
        let bbw_conf = remap(bbw, 1.5, 0.5, 0.50, 1.0);
        let adx_conf = remap(adx, 20.0, 5.0, 0.50, 1.0);
        let confidence = (bbw_conf + adx_conf) / 2.0;
        return (MarketRegime::Squeeze, confidence);
    }

    // 4. TRENDING — strong directional persistence.
    if adx > 25.0 && hurst > 0.55 {
        let adx_conf = remap(adx, 25.0, 50.0, 0.60, 1.0);
        let hurst_conf = remap(hurst, 0.55, 0.80, 0.60, 1.0);
        let confidence = (adx_conf + hurst_conf) / 2.0;
        return (MarketRegime::Trending, confidence);
    }

    // 5. RANGING — sideways / mean-reversion.
    if adx < 20.0 && hurst < 0.45 {
        let adx_conf = remap(adx, 20.0, 5.0, 0.50, 1.0);
        let hurst_conf = remap(hurst, 0.45, 0.20, 0.50, 1.0);
        let confidence = (adx_conf + hurst_conf) / 2.0;
        return (MarketRegime::Ranging, confidence);
    }

    // Default: ambiguous — fall back to RANGING with low confidence.
    trace!(
        adx = format!("{:.2}", adx),
        bbw = format!("{:.2}", bbw),
        hurst = format!("{:.4}", hurst),
        entropy = format!("{:.4}", entropy),
        "Regime: no rule matched, defaulting to RANGING"
    );
    (MarketRegime::Ranging, 0.30)
}

/// Linearly remap `value` from `[in_lo, in_hi]` to `[out_lo, out_hi]`, clamped
/// to the output range. Works regardless of whether `in_lo < in_hi` or vice
/// versa.
fn remap(value: f64, in_lo: f64, in_hi: f64, out_lo: f64, out_hi: f64) -> f64 {
    let t = if (in_hi - in_lo).abs() < f64::EPSILON {
        0.5
    } else {
        (value - in_lo) / (in_hi - in_lo)
    };
    let clamped = t.clamp(0.0, 1.0);
    out_lo + clamped * (out_hi - out_lo)
}

// =============================================================================
// Unit Tests
// =============================================================================
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_dead() {
        let (regime, conf) = classify(30.0, 3.0, 0.50, 0.98);
        assert_eq!(regime, MarketRegime::Dead);
        assert!(conf > 0.0);
    }

    #[test]
    fn test_classify_volatile() {
        let (regime, _) = classify(30.0, 7.0, 0.50, 0.50);
        assert_eq!(regime, MarketRegime::Volatile);
    }

    #[test]
    fn test_classify_squeeze() {
        let (regime, _) = classify(15.0, 1.0, 0.50, 0.50);
        assert_eq!(regime, MarketRegime::Squeeze);
    }

    #[test]
    fn test_classify_trending() {
        let (regime, _) = classify(35.0, 3.0, 0.65, 0.50);
        assert_eq!(regime, MarketRegime::Trending);
    }

    #[test]
    fn test_classify_ranging() {
        let (regime, _) = classify(15.0, 3.0, 0.40, 0.50);
        assert_eq!(regime, MarketRegime::Ranging);
    }

    #[test]
    fn test_classify_default_ranging() {
        // Values that do not match any rule.
        let (regime, conf) = classify(22.0, 3.0, 0.50, 0.50);
        assert_eq!(regime, MarketRegime::Ranging);
        assert!((conf - 0.30).abs() < 1e-10);
    }

    #[test]
    fn test_dead_priority_over_trending() {
        // Even with strong ADX/Hurst, entropy >= 0.95 should classify as Dead.
        let (regime, _) = classify(40.0, 3.0, 0.70, 0.97);
        assert_eq!(regime, MarketRegime::Dead);
    }

    #[test]
    fn test_risk_params_dead() {
        let ((rr_reward, rr_risk), max_pos) = MarketRegime::Dead.risk_params();
        assert!((rr_reward - 0.0).abs() < f64::EPSILON);
        assert!((rr_risk - 0.0).abs() < f64::EPSILON);
        assert!((max_pos - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_risk_params_trending() {
        let ((rr_reward, rr_risk), max_pos) = MarketRegime::Trending.risk_params();
        assert!((rr_reward - 3.0).abs() < f64::EPSILON);
        assert!((rr_risk - 1.0).abs() < f64::EPSILON);
        assert!((max_pos - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_regime_display() {
        assert_eq!(format!("{}", MarketRegime::Trending), "TRENDING");
        assert_eq!(format!("{}", MarketRegime::Dead), "DEAD");
    }

    #[test]
    fn test_remap() {
        assert!((remap(0.5, 0.0, 1.0, 0.0, 10.0) - 5.0).abs() < 1e-10);
        // Clamping above.
        assert!((remap(2.0, 0.0, 1.0, 0.0, 10.0) - 10.0).abs() < 1e-10);
        // Clamping below.
        assert!((remap(-1.0, 0.0, 1.0, 0.0, 10.0) - 0.0).abs() < 1e-10);
    }
}
