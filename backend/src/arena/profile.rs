// =============================================================================
// Strategy Profiles — Definitions for the Arena system
// =============================================================================
//
// Each profile encapsulates a distinct trading style. In the full Arena
// implementation (Phase 5), these profiles will be scored using Thompson
// Sampling and dynamically selected based on regime and performance.
// =============================================================================

use serde::{Deserialize, Serialize};

/// A strategy profile definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyProfile {
    /// Unique identifier (e.g. "momentum", "mean_revert").
    pub id: String,

    /// Human-readable display name.
    pub name: String,

    /// Description of the strategy's approach.
    pub description: String,

    /// Whether this profile is currently enabled for selection.
    pub enabled: bool,
}

impl StrategyProfile {
    /// Create a new profile.
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: description.into(),
            enabled: true,
        }
    }
}

/// Return the default set of strategy profiles.
///
/// These represent the four core trading personalities that the Arena system
/// will compete against each other.
pub fn default_profiles() -> Vec<StrategyProfile> {
    vec![
        StrategyProfile::new(
            "momentum",
            "Momentum",
            "Follows strong directional moves. Best in TRENDING regimes with \
             high ADX and Hurst > 0.55. Uses EMA alignment and RSI momentum \
             confirmation. Wider stops, bigger targets.",
        ),
        StrategyProfile::new(
            "mean_revert",
            "MeanRevert",
            "Fades overextended moves back to the mean. Best in RANGING \
             regimes with low ADX and Hurst < 0.45. Uses Bollinger Band \
             touches and RSI extremes. Tight stops, quick profits.",
        ),
        StrategyProfile::new(
            "breakout",
            "Breakout",
            "Captures explosive moves out of compression zones. Best in \
             SQUEEZE regimes with contracting BBW and declining ADX. Waits \
             for volume confirmation on the breakout candle. Small stops, \
             very wide targets.",
        ),
        StrategyProfile::new(
            "scalp",
            "Scalp",
            "High-frequency short-duration trades exploiting micro-structure \
             signals. Uses orderbook imbalance, VPIN toxicity, and CVD \
             divergence. Very tight stops and targets. Works across multiple \
             regimes but avoids DEAD.",
        ),
    ]
}

// =============================================================================
// Stub for future Thompson Sampling implementation
// =============================================================================

/// Thompson Sampling state for a single profile.
///
/// Uses a Beta(alpha, beta) distribution where:
///   - `alpha` starts at 1 and increments on each win.
///   - `beta` starts at 1 and increments on each loss.
///
/// Sampling from Beta(alpha, beta) gives a random variable in [0, 1] that
/// represents the estimated win probability of this profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThompsonState {
    pub profile_id: String,
    pub alpha: f64,
    pub beta: f64,
    pub total_trades: u64,
    pub wins: u64,
    pub losses: u64,
}

impl ThompsonState {
    /// Create a new Thompson state with uniform prior (alpha=1, beta=1).
    pub fn new(profile_id: impl Into<String>) -> Self {
        Self {
            profile_id: profile_id.into(),
            alpha: 1.0,
            beta: 1.0,
            total_trades: 0,
            wins: 0,
            losses: 0,
        }
    }

    /// Record a win for this profile.
    pub fn record_win(&mut self) {
        self.alpha += 1.0;
        self.wins += 1;
        self.total_trades += 1;
    }

    /// Record a loss for this profile.
    pub fn record_loss(&mut self) {
        self.beta += 1.0;
        self.losses += 1;
        self.total_trades += 1;
    }

    /// Estimated win rate (posterior mean of the Beta distribution).
    pub fn estimated_win_rate(&self) -> f64 {
        self.alpha / (self.alpha + self.beta)
    }

    /// Thompson score — a simple deterministic approximation.
    ///
    /// In the full implementation, this will sample from Beta(alpha, beta)
    /// using a proper random number generator. For now we return the posterior
    /// mean as a placeholder.
    pub fn thompson_score(&self) -> f64 {
        self.estimated_win_rate()
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_profiles_count() {
        let profiles = default_profiles();
        assert_eq!(profiles.len(), 4);
    }

    #[test]
    fn default_profiles_all_enabled() {
        let profiles = default_profiles();
        assert!(profiles.iter().all(|p| p.enabled));
    }

    #[test]
    fn default_profiles_unique_ids() {
        let profiles = default_profiles();
        let ids: Vec<&str> = profiles.iter().map(|p| p.id.as_str()).collect();
        let mut unique = ids.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(ids.len(), unique.len());
    }

    #[test]
    fn thompson_state_initial() {
        let ts = ThompsonState::new("momentum");
        assert_eq!(ts.alpha, 1.0);
        assert_eq!(ts.beta, 1.0);
        assert_eq!(ts.total_trades, 0);
        // Uniform prior => 50% win rate.
        assert!((ts.estimated_win_rate() - 0.5).abs() < 1e-10);
    }

    #[test]
    fn thompson_state_after_wins() {
        let mut ts = ThompsonState::new("momentum");
        for _ in 0..10 {
            ts.record_win();
        }
        // alpha = 11, beta = 1 => win rate ≈ 11/12 ≈ 0.917
        assert!(ts.estimated_win_rate() > 0.9);
    }

    #[test]
    fn thompson_state_after_losses() {
        let mut ts = ThompsonState::new("mean_revert");
        for _ in 0..10 {
            ts.record_loss();
        }
        // alpha = 1, beta = 11 => win rate ≈ 1/12 ≈ 0.083
        assert!(ts.estimated_win_rate() < 0.1);
    }
}
