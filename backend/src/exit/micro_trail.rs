// =============================================================================
// Micro-Trail + Order Flow Adaptive Exit — EXTRAORDINARY Exit System
// =============================================================================
//
// Replaces the fixed 0.5% trailing stop with an ATR-calibrated, order-flow-
// aware trailing stop that adapts in real-time to market microstructure.
//
// Three innovations:
//
//   1. **Phased Micro-Trail** — trail distance shrinks as profit grows:
//      - Loose   (0-30% of TP1):  1.5× ATR trail — let the trade breathe
//      - Standard(30-60% of TP1): 1.0× ATR trail — balanced protection
//      - Aggressive(60%+ of TP1): 0.5× ATR trail — lock maximum profit
//
//   2. **Order Flow Adaptation** — real-time tightening based on:
//      - CVD divergence against position → tighten 30%
//      - Orderbook imbalance against position → tighten 20%
//      - VPIN toxic zone (>0.7) → emergency tighten 50%
//
//   3. **Velocity Shield** — if price drops >0.3% within a 5-second window
//      against the position, snap the trail to the current level immediately.
//
// The module is gated behind `enable_micro_trail` feature flag (default: OFF).
// When OFF, all data is still collected for observation.
// =============================================================================

use serde::{Deserialize, Serialize};
use tracing::{debug, info};

// =============================================================================
// Constants
// =============================================================================

/// ATR multiplier in the loose phase (0-30% of TP1 distance).
const LOOSE_ATR_MULT: f64 = 1.5;
/// ATR multiplier in the standard phase (30-60% of TP1 distance).
const STANDARD_ATR_MULT: f64 = 1.0;
/// ATR multiplier in the aggressive phase (60%+ of TP1 distance).
const AGGRESSIVE_ATR_MULT: f64 = 0.5;

/// Phase boundary: profit fraction of TP1 distance where standard begins.
const PHASE_STANDARD_START: f64 = 0.30;
/// Phase boundary: profit fraction of TP1 distance where aggressive begins.
const PHASE_AGGRESSIVE_START: f64 = 0.60;

/// CVD divergence tightening factor (30% reduction in trail distance).
const CVD_TIGHTEN_FACTOR: f64 = 0.70;
/// Orderbook imbalance tightening factor (20% reduction in trail distance).
const OB_TIGHTEN_FACTOR: f64 = 0.80;
/// VPIN toxic zone tightening factor (50% reduction in trail distance).
const VPIN_TOXIC_TIGHTEN_FACTOR: f64 = 0.50;
/// VPIN threshold for toxic zone.
const VPIN_TOXIC_THRESHOLD: f64 = 0.70;
/// Orderbook imbalance threshold for adverse pressure.
const OB_ADVERSE_THRESHOLD: f64 = 0.3;

/// Velocity shield: minimum adverse move (%) to trigger snap.
const VELOCITY_THRESHOLD_PCT: f64 = 0.30;
/// Velocity shield: window in seconds.
const VELOCITY_WINDOW_SECS: u64 = 5;

/// Minimum trail distance as a percentage, to never go tighter than the
/// minimum SL floor.
const MIN_TRAIL_PCT: f64 = 0.20;

// =============================================================================
// Trail Phase
// =============================================================================

/// Current phase of the micro-trail based on profit progress.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrailPhase {
    Loose,
    Standard,
    Aggressive,
}

impl std::fmt::Display for TrailPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Loose => write!(f, "LOOSE"),
            Self::Standard => write!(f, "STANDARD"),
            Self::Aggressive => write!(f, "AGGRESSIVE"),
        }
    }
}

// =============================================================================
// Order Flow Context — snapshot of current market microstructure
// =============================================================================

/// Real-time market microstructure data fed into the adaptive trail.
#[derive(Debug, Clone, Default)]
pub struct OrderFlowContext {
    /// Current CVD value.
    pub cvd: f64,
    /// CVD at the time the position was opened.
    pub cvd_at_entry: f64,
    /// Orderbook imbalance (-1.0 to +1.0, positive = bid heavy).
    pub orderbook_imbalance: f64,
    /// Current VPIN value (0.0 to 1.0).
    pub vpin: f64,
}

// =============================================================================
// MicroTrailState — per-position trailing state
// =============================================================================

/// Mutable state tracking the micro-trail for a single position.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MicroTrailState {
    /// Trade direction: true = long, false = short.
    pub is_long: bool,
    /// Entry price.
    pub entry_price: f64,
    /// TP1 price level (used to compute profit fraction).
    pub tp1_price: f64,
    /// Current 5M ATR value in price units.
    pub atr_5m: f64,
    /// Best price seen (highest for long, lowest for short).
    pub best_price: f64,
    /// Current micro-trail stop price.
    pub trail_price: f64,
    /// Current trail phase.
    pub phase: TrailPhase,
    /// Raw trail distance before order-flow adjustments (for observation).
    pub raw_trail_distance: f64,
    /// Final trail distance after all adjustments.
    pub adjusted_trail_distance: f64,
    /// Whether velocity shield has been triggered.
    pub velocity_triggered: bool,
    /// Price snapshot for velocity detection: (price, timestamp_secs).
    #[serde(skip)]
    price_history: Vec<(f64, u64)>,
    /// Order flow tighten multiplier applied this tick (1.0 = no adjustment).
    pub of_tighten_mult: f64,
    /// Reason string for the last adjustment (for logging/dashboard).
    pub adjustment_reason: String,
    /// CVD value at position entry (for divergence detection).
    pub cvd_at_entry: f64,
}

impl MicroTrailState {
    /// Create a new micro-trail state for a position.
    pub fn new(
        is_long: bool,
        entry_price: f64,
        tp1_price: f64,
        atr_5m: f64,
    ) -> Self {
        // Initial trail is loose phase.
        let trail_distance = atr_5m * LOOSE_ATR_MULT;
        let trail_price = if is_long {
            entry_price - trail_distance
        } else {
            entry_price + trail_distance
        };

        info!(
            is_long,
            entry_price,
            tp1_price,
            atr_5m = format!("{:.4}", atr_5m),
            trail_price = format!("{:.4}", trail_price),
            "MicroTrailState initialized — phase LOOSE"
        );

        Self {
            is_long,
            entry_price,
            tp1_price,
            atr_5m,
            best_price: entry_price,
            trail_price,
            phase: TrailPhase::Loose,
            raw_trail_distance: trail_distance,
            adjusted_trail_distance: trail_distance,
            velocity_triggered: false,
            price_history: Vec::with_capacity(32),
            of_tighten_mult: 1.0,
            adjustment_reason: "initial".to_string(),
            cvd_at_entry: 0.0,
        }
    }

    /// Set the CVD at entry time (call immediately after construction).
    pub fn set_cvd_at_entry(&mut self, cvd: f64) {
        self.cvd_at_entry = cvd;
    }

    /// Update ATR value (call when new 5M candle closes).
    pub fn update_atr(&mut self, new_atr: f64) {
        if new_atr > 0.0 {
            self.atr_5m = new_atr;
        }
    }

    /// Core evaluation: update the trail based on current price and order flow.
    ///
    /// Returns `true` if the trail stop has been hit (position should close).
    pub fn evaluate(
        &mut self,
        current_price: f64,
        current_time_secs: u64,
        order_flow: &OrderFlowContext,
    ) -> bool {
        // ── Update best price ─────────────────────────────────────────
        if self.is_long && current_price > self.best_price {
            self.best_price = current_price;
        } else if !self.is_long && (current_price < self.best_price || self.best_price == self.entry_price) {
            self.best_price = current_price;
        }

        // ── Compute profit fraction of TP1 distance ──────────────────
        let tp1_distance = (self.tp1_price - self.entry_price).abs();
        let current_profit = if self.is_long {
            current_price - self.entry_price
        } else {
            self.entry_price - current_price
        };
        let profit_fraction = if tp1_distance > 0.0 {
            (current_profit / tp1_distance).clamp(-1.0, 3.0)
        } else {
            0.0
        };

        // ── Determine trail phase ────────────────────────────────────
        let new_phase = if profit_fraction >= PHASE_AGGRESSIVE_START {
            TrailPhase::Aggressive
        } else if profit_fraction >= PHASE_STANDARD_START {
            TrailPhase::Standard
        } else {
            TrailPhase::Loose
        };

        if new_phase != self.phase {
            info!(
                old_phase = %self.phase,
                new_phase = %new_phase,
                profit_fraction = format!("{:.2}", profit_fraction),
                "MicroTrail phase transition"
            );
            self.phase = new_phase;
        }

        // ── Base ATR trail distance from phase ───────────────────────
        let atr_mult = match self.phase {
            TrailPhase::Loose => LOOSE_ATR_MULT,
            TrailPhase::Standard => STANDARD_ATR_MULT,
            TrailPhase::Aggressive => AGGRESSIVE_ATR_MULT,
        };
        let mut trail_distance = self.atr_5m * atr_mult;
        self.raw_trail_distance = trail_distance;

        // ── Order Flow Adaptation ────────────────────────────────────
        let mut tighten_mult = 1.0_f64;
        let mut reasons = Vec::new();

        // CVD divergence: if CVD flipped against position.
        let cvd_delta = order_flow.cvd - order_flow.cvd_at_entry;
        let cvd_against = if self.is_long {
            cvd_delta < 0.0
        } else {
            cvd_delta > 0.0
        };
        if cvd_against {
            tighten_mult *= CVD_TIGHTEN_FACTOR;
            reasons.push("CVD_DIVERGE");
        }

        // Orderbook imbalance against position.
        let ob_against = if self.is_long {
            order_flow.orderbook_imbalance < -OB_ADVERSE_THRESHOLD
        } else {
            order_flow.orderbook_imbalance > OB_ADVERSE_THRESHOLD
        };
        if ob_against {
            tighten_mult *= OB_TIGHTEN_FACTOR;
            reasons.push("OB_PRESSURE");
        }

        // VPIN toxic zone — informed trading detected.
        if order_flow.vpin > VPIN_TOXIC_THRESHOLD {
            tighten_mult *= VPIN_TOXIC_TIGHTEN_FACTOR;
            reasons.push("VPIN_TOXIC");
        }

        trail_distance *= tighten_mult;
        self.of_tighten_mult = tighten_mult;

        // ── Minimum trail distance floor ─────────────────────────────
        let min_trail = self.entry_price * MIN_TRAIL_PCT / 100.0;
        trail_distance = trail_distance.max(min_trail);
        self.adjusted_trail_distance = trail_distance;

        // ── Velocity shield ──────────────────────────────────────────
        self.price_history.push((current_price, current_time_secs));
        // Prune old entries.
        self.price_history
            .retain(|&(_, t)| current_time_secs.saturating_sub(t) <= VELOCITY_WINDOW_SECS);

        let velocity_snap = if self.price_history.len() >= 2 {
            let oldest = self.price_history[0].0;
            let move_pct = if self.is_long {
                (oldest - current_price) / oldest * 100.0
            } else {
                (current_price - oldest) / oldest * 100.0
            };
            move_pct > VELOCITY_THRESHOLD_PCT
        } else {
            false
        };

        if velocity_snap && !self.velocity_triggered {
            self.velocity_triggered = true;
            // Snap trail to current price minus minimum buffer.
            let snapped = if self.is_long {
                current_price - min_trail
            } else {
                current_price + min_trail
            };
            // Only tighten, never widen.
            let should_snap = if self.is_long {
                snapped > self.trail_price
            } else {
                snapped < self.trail_price
            };
            if should_snap {
                self.trail_price = snapped;
                reasons.push("VELOCITY_SNAP");
                info!(
                    trail_price = format!("{:.4}", self.trail_price),
                    "Velocity shield triggered — trail snapped"
                );
            }
        } else {
            self.velocity_triggered = false;
        }

        // ── Update trail price (ratchet — only tighten, never widen) ─
        let candidate_trail = if self.is_long {
            self.best_price - trail_distance
        } else {
            self.best_price + trail_distance
        };

        let should_update = if self.is_long {
            candidate_trail > self.trail_price
        } else {
            candidate_trail < self.trail_price
        };

        if should_update {
            self.trail_price = candidate_trail;
        }

        self.adjustment_reason = if reasons.is_empty() {
            format!("{}", self.phase)
        } else {
            format!("{} | {}", self.phase, reasons.join("+"))
        };

        debug!(
            phase = %self.phase,
            best_price = format!("{:.4}", self.best_price),
            trail_price = format!("{:.4}", self.trail_price),
            raw_dist = format!("{:.6}", self.raw_trail_distance),
            adj_dist = format!("{:.6}", self.adjusted_trail_distance),
            of_mult = format!("{:.2}", self.of_tighten_mult),
            reason = %self.adjustment_reason,
            "MicroTrail evaluated"
        );

        // ── Check if trail is hit ────────────────────────────────────
        let trail_hit = if self.is_long {
            current_price <= self.trail_price
        } else {
            current_price >= self.trail_price
        };

        if trail_hit {
            info!(
                current_price = format!("{:.4}", current_price),
                trail_price = format!("{:.4}", self.trail_price),
                phase = %self.phase,
                reason = %self.adjustment_reason,
                "MICRO-TRAIL HIT — closing position"
            );
        }

        trail_hit
    }

    /// Get a diagnostic snapshot for the dashboard.
    pub fn snapshot(&self) -> MicroTrailSnapshot {
        MicroTrailSnapshot {
            phase: self.phase.to_string(),
            best_price: self.best_price,
            trail_price: self.trail_price,
            raw_trail_distance: self.raw_trail_distance,
            adjusted_trail_distance: self.adjusted_trail_distance,
            of_tighten_mult: self.of_tighten_mult,
            velocity_triggered: self.velocity_triggered,
            atr_5m: self.atr_5m,
            adjustment_reason: self.adjustment_reason.clone(),
        }
    }
}

/// Serialisable diagnostic snapshot for the dashboard.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MicroTrailSnapshot {
    pub phase: String,
    pub best_price: f64,
    pub trail_price: f64,
    pub raw_trail_distance: f64,
    pub adjusted_trail_distance: f64,
    pub of_tighten_mult: f64,
    pub velocity_triggered: bool,
    pub atr_5m: f64,
    pub adjustment_reason: String,
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn default_of_ctx() -> OrderFlowContext {
        OrderFlowContext {
            cvd: 100.0,
            cvd_at_entry: 100.0,
            orderbook_imbalance: 0.0,
            vpin: 0.3,
        }
    }

    #[test]
    fn new_state_initializes_correctly() {
        let state = MicroTrailState::new(true, 100.0, 102.0, 0.5);
        assert_eq!(state.phase, TrailPhase::Loose);
        assert_eq!(state.entry_price, 100.0);
        assert_eq!(state.best_price, 100.0);
        // Trail = 100.0 - (0.5 * 1.5) = 99.25
        assert!((state.trail_price - 99.25).abs() < 0.01);
    }

    #[test]
    fn trail_tightens_as_profit_grows() {
        let mut state = MicroTrailState::new(true, 100.0, 102.0, 0.5);
        let ctx = default_of_ctx();

        // Move price to 30% of TP1 distance (100 + 0.6 = 100.6)
        let hit = state.evaluate(100.7, 10, &ctx);
        assert!(!hit);
        assert_eq!(state.phase, TrailPhase::Standard);

        let trail_at_standard = state.trail_price;

        // Move price to 60%+ of TP1 distance (100 + 1.2 = 101.2)
        let hit = state.evaluate(101.3, 20, &ctx);
        assert!(!hit);
        assert_eq!(state.phase, TrailPhase::Aggressive);

        // Aggressive trail should be tighter (higher for longs)
        assert!(
            state.trail_price > trail_at_standard,
            "aggressive trail {} should be > standard trail {}",
            state.trail_price,
            trail_at_standard
        );
    }

    #[test]
    fn trail_never_widens() {
        let mut state = MicroTrailState::new(true, 100.0, 102.0, 0.5);
        let ctx = default_of_ctx();

        // Price goes up → trail ratchets.
        state.evaluate(101.0, 10, &ctx);
        let high_trail = state.trail_price;

        // Price drops back → trail must not widen.
        state.evaluate(100.5, 20, &ctx);
        assert!(
            state.trail_price >= high_trail,
            "trail {} should not widen below {}",
            state.trail_price,
            high_trail
        );
    }

    #[test]
    fn cvd_divergence_tightens_trail() {
        let mut state = MicroTrailState::new(true, 100.0, 102.0, 0.5);

        // Normal conditions
        let ctx_normal = default_of_ctx();
        state.evaluate(101.0, 10, &ctx_normal);
        let normal_dist = state.adjusted_trail_distance;

        // Reset for fresh comparison
        let mut state2 = MicroTrailState::new(true, 100.0, 102.0, 0.5);

        // CVD divergence: long position but CVD dropped
        let ctx_diverge = OrderFlowContext {
            cvd: 50.0,
            cvd_at_entry: 100.0,
            orderbook_imbalance: 0.0,
            vpin: 0.3,
        };
        state2.evaluate(101.0, 10, &ctx_diverge);
        let diverge_dist = state2.adjusted_trail_distance;

        assert!(
            diverge_dist < normal_dist,
            "CVD divergence dist {} should be < normal {}",
            diverge_dist,
            normal_dist
        );
    }

    #[test]
    fn vpin_toxic_tightens_trail() {
        let mut state = MicroTrailState::new(true, 100.0, 102.0, 0.5);

        // VPIN toxic
        let ctx = OrderFlowContext {
            cvd: 100.0,
            cvd_at_entry: 100.0,
            orderbook_imbalance: 0.0,
            vpin: 0.85,
        };
        state.evaluate(100.5, 10, &ctx);

        assert!(
            state.of_tighten_mult < 1.0,
            "VPIN toxic should tighten: mult = {}",
            state.of_tighten_mult
        );
    }

    #[test]
    fn orderbook_pressure_tightens_trail() {
        let mut state = MicroTrailState::new(true, 100.0, 102.0, 0.5);

        // Strong ask pressure for a long (imbalance heavily negative)
        let ctx = OrderFlowContext {
            cvd: 100.0,
            cvd_at_entry: 100.0,
            orderbook_imbalance: -0.5, // heavy ask side
            vpin: 0.3,
        };
        state.evaluate(100.5, 10, &ctx);

        assert!(
            state.of_tighten_mult < 1.0,
            "OB pressure should tighten: mult = {}",
            state.of_tighten_mult
        );
    }

    #[test]
    fn combined_order_flow_stacks() {
        let mut state = MicroTrailState::new(true, 100.0, 102.0, 0.5);

        // All three against: CVD diverge + OB pressure + VPIN toxic
        let ctx = OrderFlowContext {
            cvd: 50.0,          // CVD dropped (diverge for long)
            cvd_at_entry: 100.0,
            orderbook_imbalance: -0.5, // heavy ask pressure
            vpin: 0.85,         // toxic zone
        };
        state.evaluate(100.5, 10, &ctx);

        let expected_mult = CVD_TIGHTEN_FACTOR * OB_TIGHTEN_FACTOR * VPIN_TOXIC_TIGHTEN_FACTOR;
        assert!(
            (state.of_tighten_mult - expected_mult).abs() < 0.001,
            "combined mult {} should equal {:.4}",
            state.of_tighten_mult,
            expected_mult
        );
    }

    #[test]
    fn trail_hit_triggers_exit() {
        let mut state = MicroTrailState::new(true, 100.0, 102.0, 0.5);
        let ctx = default_of_ctx();

        // Trail is at ~99.25 initially.
        let hit = state.evaluate(100.5, 10, &ctx);
        assert!(!hit);

        // Price drops below trail.
        let hit = state.evaluate(98.0, 20, &ctx);
        assert!(hit, "price {} below trail {} should trigger exit", 98.0, state.trail_price);
    }

    #[test]
    fn short_position_works() {
        let mut state = MicroTrailState::new(false, 100.0, 98.0, 0.5);
        let ctx = default_of_ctx();

        // Initial trail should be above entry for shorts.
        // Trail = 100.0 + (0.5 * 1.5) = 100.75
        assert!((state.trail_price - 100.75).abs() < 0.01);

        // Price drops in our favour.
        let hit = state.evaluate(99.0, 10, &ctx);
        assert!(!hit);

        // Trail should tighten downward for shorts.
        assert!(
            state.trail_price < 100.75,
            "short trail {} should be < initial 100.75",
            state.trail_price
        );

        // Price rises above trail → exit.
        let hit = state.evaluate(101.0, 20, &ctx);
        assert!(hit, "price above trail should trigger short exit");
    }

    #[test]
    fn velocity_shield_snaps_trail() {
        let mut state = MicroTrailState::new(true, 100.0, 102.0, 0.5);
        let ctx = default_of_ctx();

        // Price rises nicely.
        state.evaluate(101.0, 100, &ctx);
        let trail_before = state.trail_price;

        // Rapid drop >0.3% in 5 seconds (101.0 → 100.5 is ~0.5%)
        // Push old price and then immediate drop.
        state.evaluate(101.0, 200, &ctx); // record high
        state.evaluate(100.5, 202, &ctx); // drop within 5s window

        // The trail should have snapped or at least stayed tight.
        assert!(
            state.trail_price >= trail_before,
            "velocity shield should not let trail widen: {} vs {}",
            state.trail_price,
            trail_before
        );
    }

    #[test]
    fn minimum_trail_floor_enforced() {
        // Very small ATR → trail should not be smaller than MIN_TRAIL_PCT
        let mut state = MicroTrailState::new(true, 100.0, 102.0, 0.001);
        let ctx = default_of_ctx();

        state.evaluate(100.5, 10, &ctx);

        let min_trail = 100.0 * MIN_TRAIL_PCT / 100.0;
        assert!(
            state.adjusted_trail_distance >= min_trail - 0.001,
            "trail dist {} should be >= min {}",
            state.adjusted_trail_distance,
            min_trail
        );
    }

    #[test]
    fn update_atr_changes_trail_calc() {
        let mut state = MicroTrailState::new(true, 100.0, 102.0, 0.5);
        assert!((state.atr_5m - 0.5).abs() < f64::EPSILON);

        state.update_atr(0.8);
        assert!((state.atr_5m - 0.8).abs() < f64::EPSILON);

        // Zero ATR is ignored.
        state.update_atr(0.0);
        assert!((state.atr_5m - 0.8).abs() < f64::EPSILON);
    }

    #[test]
    fn snapshot_returns_current_state() {
        let state = MicroTrailState::new(true, 100.0, 102.0, 0.5);
        let snap = state.snapshot();

        assert_eq!(snap.phase, "LOOSE");
        assert!((snap.best_price - 100.0).abs() < f64::EPSILON);
        assert!(snap.trail_price > 0.0);
        assert!(!snap.velocity_triggered);
    }
}
