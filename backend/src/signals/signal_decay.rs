// =============================================================================
// Signal Decay Manager — Half-life freshness management
// =============================================================================

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Instant;

/// Tracks signal freshness using exponential decay (half-life model).
pub struct SignalDecayManager {
    signals: RwLock<HashMap<String, SignalEntry>>,
    half_life_secs: f64,
}

struct SignalEntry {
    strength: f64,
    recorded_at: Instant,
}

impl SignalDecayManager {
    /// Create a new decay manager with the given half-life in seconds.
    pub fn new(half_life_secs: f64) -> Self {
        Self {
            signals: RwLock::new(HashMap::new()),
            half_life_secs,
        }
    }

    /// Record a new signal strength for the given key.
    pub fn record(&self, key: impl Into<String>, strength: f64) {
        let mut signals = self.signals.write();
        signals.insert(
            key.into(),
            SignalEntry {
                strength,
                recorded_at: Instant::now(),
            },
        );
    }

    /// Get the decayed signal strength for a key.
    pub fn get_decayed(&self, key: &str) -> Option<f64> {
        let signals = self.signals.read();
        let entry = signals.get(key)?;
        let elapsed = entry.recorded_at.elapsed().as_secs_f64();
        let decay_factor = (-elapsed * (2.0_f64.ln()) / self.half_life_secs).exp();
        Some(entry.strength * decay_factor)
    }

    /// Get all active signals with their decayed strengths.
    pub fn all_decayed(&self) -> HashMap<String, f64> {
        let signals = self.signals.read();
        signals
            .iter()
            .map(|(k, entry)| {
                let elapsed = entry.recorded_at.elapsed().as_secs_f64();
                let decay = (-elapsed * (2.0_f64.ln()) / self.half_life_secs).exp();
                (k.clone(), entry.strength * decay)
            })
            .collect()
    }

    /// Remove signals that have decayed below a threshold.
    pub fn prune(&self, threshold: f64) {
        let mut signals = self.signals.write();
        signals.retain(|_, entry| {
            let elapsed = entry.recorded_at.elapsed().as_secs_f64();
            let decay = (-elapsed * (2.0_f64.ln()) / self.half_life_secs).exp();
            entry.strength * decay > threshold
        });
    }
}

impl Default for SignalDecayManager {
    fn default() -> Self {
        Self::new(120.0) // 2-minute half-life
    }
}
