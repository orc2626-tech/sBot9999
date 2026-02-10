// =============================================================================
// Regime Detection Module
// =============================================================================
//
// Market regime classification using multiple quantitative indicators:
// - ADX (trend strength)
// - Bollinger Band Width (volatility expansion/contraction)
// - Hurst exponent (persistence vs mean-reversion)
// - Shannon entropy (randomness / information content)

pub mod detector;
pub mod entropy;
pub mod hurst;

pub use detector::{MarketRegime, RegimeDetector, RegimeState};
pub use entropy::ShannonEntropyFilter;
pub use hurst::calculate_hurst_exponent;
