// =============================================================================
// VPIN — Volume-Synchronized Probability of Informed Trading
// =============================================================================

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// VPIN state for a single symbol.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VPINState {
    pub vpin: f64,
    pub zone: String,
    pub buy_volume: f64,
    pub sell_volume: f64,
}

impl Default for VPINState {
    fn default() -> Self {
        Self {
            vpin: 0.0,
            zone: "neutral".to_string(),
            buy_volume: 0.0,
            sell_volume: 0.0,
        }
    }
}

/// VPIN calculator for a single symbol.
pub struct VPINCalculator {
    bucket_size: f64,
    num_buckets: usize,
    current_buy_volume: f64,
    current_sell_volume: f64,
    current_bucket_volume: f64,
    buckets: VecDeque<(f64, f64)>, // (buy_vol, sell_vol) per bucket
}

impl VPINCalculator {
    pub fn new(bucket_size: f64, num_buckets: usize) -> Self {
        Self {
            bucket_size,
            num_buckets,
            current_buy_volume: 0.0,
            current_sell_volume: 0.0,
            current_bucket_volume: 0.0,
            buckets: VecDeque::with_capacity(num_buckets),
        }
    }

    /// Feed a new trade into the VPIN calculation.
    pub fn add_trade(&mut self, volume: f64, is_buy: bool) {
        if is_buy {
            self.current_buy_volume += volume;
        } else {
            self.current_sell_volume += volume;
        }
        self.current_bucket_volume += volume;

        // Check if current bucket is full.
        while self.current_bucket_volume >= self.bucket_size {
            let overflow = self.current_bucket_volume - self.bucket_size;
            let ratio = if self.current_bucket_volume > 0.0 {
                (self.current_bucket_volume - overflow) / self.current_bucket_volume
            } else {
                1.0
            };

            let bucket_buy = self.current_buy_volume * ratio;
            let bucket_sell = self.current_sell_volume * ratio;

            self.buckets.push_back((bucket_buy, bucket_sell));
            if self.buckets.len() > self.num_buckets {
                self.buckets.pop_front();
            }

            // Carry over the overflow.
            self.current_buy_volume *= 1.0 - ratio;
            self.current_sell_volume *= 1.0 - ratio;
            self.current_bucket_volume = overflow;
        }
    }

    /// Calculate the current VPIN value.
    pub fn calculate(&self) -> VPINState {
        if self.buckets.is_empty() {
            return VPINState::default();
        }

        let total_imbalance: f64 = self
            .buckets
            .iter()
            .map(|(buy, sell)| (buy - sell).abs())
            .sum();

        let total_volume: f64 = self
            .buckets
            .iter()
            .map(|(buy, sell)| buy + sell)
            .sum();

        let vpin = if total_volume > 0.0 {
            total_imbalance / total_volume
        } else {
            0.0
        };

        let total_buy: f64 = self.buckets.iter().map(|(b, _)| b).sum();
        let total_sell: f64 = self.buckets.iter().map(|(_, s)| s).sum();

        let zone = if vpin > 0.7 {
            "toxic".to_string()
        } else if vpin > 0.4 {
            "elevated".to_string()
        } else {
            "neutral".to_string()
        };

        VPINState {
            vpin,
            zone,
            buy_volume: total_buy,
            sell_volume: total_sell,
        }
    }
}

impl Default for VPINCalculator {
    fn default() -> Self {
        Self::new(1000.0, 50)
    }
}
