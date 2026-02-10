// =============================================================================
// Signals Module
// =============================================================================
//
// Signal processing pipeline for the trading engine:
// - Weighted ensemble scoring (regime-aware)
// - Signal decay / half-life freshness management
// - VPIN (Volume-Synchronized Probability of Informed Trading)

pub mod signal_decay;
pub mod vpin;
pub mod weighted_score;

pub use signal_decay::SignalDecayManager;
pub use vpin::{VPINCalculator, VPINState};
pub use weighted_score::{ScoringResult, SignalInput, WeightedScorer};
