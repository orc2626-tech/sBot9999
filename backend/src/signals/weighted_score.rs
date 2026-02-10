// =============================================================================
// Weighted Ensemble Scorer — Regime-aware signal aggregation
// =============================================================================

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A single signal input to the scoring engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalInput {
    pub name: String,
    pub weight: f64,
    pub confidence: f64,
    /// +1.0 for bullish, -1.0 for bearish, 0.0 for neutral.
    pub direction: f64,
}

/// The contribution of a single signal to the final score.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalContribution {
    pub name: String,
    pub weight: f64,
    pub confidence: f64,
    pub direction: f64,
    pub contribution: f64,
}

/// Result of the weighted scoring pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoringResult {
    pub total_score: f64,
    pub decision: String,
    pub signal_contributions: Vec<SignalContribution>,
}

/// Regime-aware weights for the scoring engine.
#[derive(Debug, Clone)]
pub struct RegimeWeights {
    pub weights: HashMap<String, f64>,
}

impl Default for RegimeWeights {
    fn default() -> Self {
        let mut weights = HashMap::new();
        weights.insert("rsi".to_string(), 0.15);
        weights.insert("ema_trend".to_string(), 0.20);
        weights.insert("adx".to_string(), 0.15);
        weights.insert("bbw".to_string(), 0.10);
        weights.insert("roc".to_string(), 0.10);
        weights.insert("vpin".to_string(), 0.10);
        weights.insert("orderbook".to_string(), 0.10);
        weights.insert("cvd".to_string(), 0.10);
        Self { weights }
    }
}

/// The main weighted scoring engine.
pub struct WeightedScorer {
    /// Weight maps per regime.
    regime_weights: HashMap<String, RegimeWeights>,
    /// Default weights when no regime-specific map is available.
    default_weights: RegimeWeights,
    /// Minimum absolute score to generate a trade signal.
    pub entry_threshold: f64,
}

impl WeightedScorer {
    pub fn new(entry_threshold: f64) -> Self {
        Self {
            regime_weights: HashMap::new(),
            default_weights: RegimeWeights::default(),
            entry_threshold,
        }
    }

    /// Register regime-specific weights.
    pub fn set_regime_weights(&mut self, regime: impl Into<String>, weights: RegimeWeights) {
        self.regime_weights.insert(regime.into(), weights);
    }

    /// Score a set of signal inputs under the given market regime.
    pub fn score(&self, signals: &[SignalInput], regime: &str) -> ScoringResult {
        let weights = self
            .regime_weights
            .get(regime)
            .unwrap_or(&self.default_weights);

        let mut contributions = Vec::with_capacity(signals.len());
        let mut total_score = 0.0;

        for signal in signals {
            let base_weight = weights
                .weights
                .get(&signal.name)
                .copied()
                .unwrap_or(signal.weight);

            let contribution = base_weight * signal.confidence * signal.direction;

            contributions.push(SignalContribution {
                name: signal.name.clone(),
                weight: base_weight,
                confidence: signal.confidence,
                direction: signal.direction,
                contribution,
            });

            total_score += contribution;
        }

        let decision = if total_score > self.entry_threshold {
            "BUY".to_string()
        } else if total_score < -self.entry_threshold {
            "SELL".to_string()
        } else {
            "HOLD".to_string()
        };

        ScoringResult {
            total_score,
            decision,
            signal_contributions: contributions,
        }
    }
}

impl Default for WeightedScorer {
    fn default() -> Self {
        Self::new(0.15)
    }
}
