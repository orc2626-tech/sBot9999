// =============================================================================
// Technical Indicators Module
// =============================================================================
//
// Pure, side-effect-free implementations of the core technical indicators used
// by the trading engine.  Every public function returns `Option<T>` so callers
// are forced to handle insufficient-data and numerical-edge-case scenarios.

pub mod ema;
pub mod rsi;
pub mod adx;
pub mod bollinger;
pub mod atr;
pub mod roc;
