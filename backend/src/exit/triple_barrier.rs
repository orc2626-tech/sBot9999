// =============================================================================
// Triple Barrier Exit Management — TP / SL / Time
// =============================================================================
//
// The triple barrier method defines three exit conditions for every trade:
//
//   1. **Stop Loss (SL)** — price moves against us beyond a threshold.
//   2. **Take Profit (TP1 / TP2)** — price moves in our favour.
//   3. **Time Barrier** — position has been open too long without resolution.
//
// Barriers are derived from ATR and the current market regime, with hard
// minimum floors to prevent nonsensically tight exits on low-volatility pairs.
//
// Progressive tightening rules:
//   - At 50% of time elapsed: SL progressively moves toward the entry price.
//   - At 75% of time elapsed: SL locks at breakeven + 0.05%.
//   - When price reaches 50% of the TP1 distance: SL moves to
//     breakeven + 0.05% (profit lock).
// =============================================================================

use serde::{Deserialize, Serialize};
use tracing::{debug, info};

// =============================================================================
// Minimum Floor Constants
// =============================================================================

/// Minimum stop-loss distance as a percentage.
const MIN_SL_PCT: f64 = 0.4;

/// Minimum first take-profit distance as a percentage.
const MIN_TP1_PCT: f64 = 0.6;

/// Minimum second take-profit distance as a percentage.
const MIN_TP2_PCT: f64 = 1.0;

/// Breakeven buffer added when locking SL at breakeven.
const BREAKEVEN_BUFFER_PCT: f64 = 0.05;

/// Fraction of time elapsed at which progressive tightening begins.
const TIGHTEN_START_FRACTION: f64 = 0.50;

/// Fraction of time elapsed at which breakeven lock activates.
const BREAKEVEN_LOCK_FRACTION: f64 = 0.75;

/// Fraction of TP1 distance at which the profit lock triggers.
const PROFIT_LOCK_TRIGGER: f64 = 0.50;

// =============================================================================
// Regime Multipliers
// =============================================================================

/// ATR multipliers for each regime.
///
/// Returns `(sl_multiplier, tp1_multiplier, tp2_multiplier, time_limit_secs)`.
fn regime_params(regime: &str) -> (f64, f64, f64, u64) {
    match regime.to_uppercase().as_str() {
        "TRENDING" => (1.5, 2.0, 4.0, 3600),    // Wide — let trends run
        "RANGING" => (1.0, 1.5, 2.5, 1800),      // Tight — quick scalps
        "VOLATILE" => (2.0, 2.5, 5.0, 2400),     // Extra wide for swings
        "SQUEEZE" => (0.8, 1.2, 2.0, 1200),      // Tight — breakout or bust
        "DEAD" => (1.0, 1.0, 1.5, 600),          // Minimal — should not trade
        _ => (1.2, 1.8, 3.0, 2400),              // Conservative defaults
    }
}

// =============================================================================
// BarrierConfig
// =============================================================================

/// Configuration for a specific position's triple barriers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BarrierConfig {
    /// Stop-loss distance as a percentage from entry.
    pub sl_pct: f64,

    /// First take-profit target as a percentage from entry.
    pub tp1_pct: f64,

    /// Second take-profit target as a percentage from entry.
    pub tp2_pct: f64,

    /// Maximum number of seconds the position may remain open.
    pub time_limit_secs: u64,

    /// The market regime at the time this config was created.
    pub regime: String,
}

impl BarrierConfig {
    /// Create a `BarrierConfig` from ATR and market regime.
    ///
    /// The ATR value is expressed as a percentage of the current price
    /// (e.g. if ATR is 150 and price is 30000, pass `atr_pct = 0.5`).
    ///
    /// Minimum floors are enforced on all barriers.
    pub fn from_atr(atr_pct: f64, regime: &str) -> Self {
        let (sl_mult, tp1_mult, tp2_mult, time_secs) = regime_params(regime);

        let raw_sl = atr_pct * sl_mult;
        let raw_tp1 = atr_pct * tp1_mult;
        let raw_tp2 = atr_pct * tp2_mult;

        // Enforce minimum floors.
        let sl_pct = raw_sl.max(MIN_SL_PCT);
        let tp1_pct = raw_tp1.max(MIN_TP1_PCT);
        let tp2_pct = raw_tp2.max(MIN_TP2_PCT);

        debug!(
            regime,
            atr_pct = format!("{:.4}", atr_pct),
            sl_pct = format!("{:.4}", sl_pct),
            tp1_pct = format!("{:.4}", tp1_pct),
            tp2_pct = format!("{:.4}", tp2_pct),
            time_limit_secs = time_secs,
            "BarrierConfig created"
        );

        Self {
            sl_pct,
            tp1_pct,
            tp2_pct,
            time_limit_secs: time_secs,
            regime: regime.to_string(),
        }
    }

    /// Create a config with explicit percentages (useful for testing).
    pub fn explicit(sl_pct: f64, tp1_pct: f64, tp2_pct: f64, time_limit_secs: u64) -> Self {
        Self {
            sl_pct: sl_pct.max(MIN_SL_PCT),
            tp1_pct: tp1_pct.max(MIN_TP1_PCT),
            tp2_pct: tp2_pct.max(MIN_TP2_PCT),
            time_limit_secs,
            regime: "MANUAL".to_string(),
        }
    }
}

// =============================================================================
// BarrierState — Live tracking of a position's barriers
// =============================================================================

/// Mutable state for a position's barrier tracking during its lifetime.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BarrierState {
    /// Original barrier configuration (immutable after creation).
    pub config: BarrierConfig,

    /// Entry price.
    pub entry_price: f64,

    /// Trade direction: "BUY" (long) or "SELL" (short).
    pub side: String,

    /// Current effective stop-loss price (may tighten over time).
    pub current_sl_price: f64,

    /// TP1 price level.
    pub tp1_price: f64,

    /// TP2 price level.
    pub tp2_price: f64,

    /// Whether TP1 has been hit (for partial-close logic).
    pub tp1_hit: bool,

    /// Whether the profit lock has been activated.
    pub profit_lock_active: bool,

    /// Whether the breakeven lock has been activated.
    pub breakeven_lock_active: bool,

    /// Epoch timestamp (seconds) when the position was opened.
    pub opened_at_secs: u64,
}

/// The reason a barrier was triggered.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExitReason {
    StopLoss,
    TakeProfit1,
    TakeProfit2,
    TimeBarrier,
}

impl std::fmt::Display for ExitReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StopLoss => write!(f, "SL"),
            Self::TakeProfit1 => write!(f, "TP1"),
            Self::TakeProfit2 => write!(f, "TP2"),
            Self::TimeBarrier => write!(f, "TIME"),
        }
    }
}

impl BarrierState {
    /// Create a new barrier state for a position.
    pub fn new(
        config: BarrierConfig,
        entry_price: f64,
        side: &str,
        opened_at_secs: u64,
    ) -> Self {
        let (sl_price, tp1_price, tp2_price) = if side == "BUY" {
            let sl = entry_price * (1.0 - config.sl_pct / 100.0);
            let tp1 = entry_price * (1.0 + config.tp1_pct / 100.0);
            let tp2 = entry_price * (1.0 + config.tp2_pct / 100.0);
            (sl, tp1, tp2)
        } else {
            let sl = entry_price * (1.0 + config.sl_pct / 100.0);
            let tp1 = entry_price * (1.0 - config.tp1_pct / 100.0);
            let tp2 = entry_price * (1.0 - config.tp2_pct / 100.0);
            (sl, tp1, tp2)
        };

        info!(
            side,
            entry_price,
            sl_price = format!("{:.2}", sl_price),
            tp1_price = format!("{:.2}", tp1_price),
            tp2_price = format!("{:.2}", tp2_price),
            time_limit_secs = config.time_limit_secs,
            "Barrier state initialized"
        );

        Self {
            config,
            entry_price,
            side: side.to_string(),
            current_sl_price: sl_price,
            tp1_price,
            tp2_price,
            tp1_hit: false,
            profit_lock_active: false,
            breakeven_lock_active: false,
            opened_at_secs,
        }
    }

    /// Evaluate all three barriers against the current price and elapsed time.
    ///
    /// Returns `Some(ExitReason)` if any barrier is triggered, or `None` if
    /// the position should remain open.
    ///
    /// **Side effects**: may tighten the SL price via progressive tightening,
    /// breakeven lock, or profit lock rules.
    pub fn evaluate(&mut self, current_price: f64, current_time_secs: u64) -> Option<ExitReason> {
        let elapsed_secs = current_time_secs.saturating_sub(self.opened_at_secs);
        let elapsed_fraction = if self.config.time_limit_secs > 0 {
            elapsed_secs as f64 / self.config.time_limit_secs as f64
        } else {
            0.0
        };

        let is_long = self.side == "BUY";

        // ── Time barrier ────────────────────────────────────────────
        if elapsed_secs >= self.config.time_limit_secs {
            info!(
                elapsed_secs,
                limit = self.config.time_limit_secs,
                "Time barrier hit"
            );
            return Some(ExitReason::TimeBarrier);
        }

        // ── Profit lock trigger (50% of TP1 distance reached) ───────
        if !self.profit_lock_active {
            let tp1_distance = (self.tp1_price - self.entry_price).abs();
            let current_distance = if is_long {
                current_price - self.entry_price
            } else {
                self.entry_price - current_price
            };

            if current_distance >= PROFIT_LOCK_TRIGGER * tp1_distance {
                let breakeven_sl = if is_long {
                    self.entry_price * (1.0 + BREAKEVEN_BUFFER_PCT / 100.0)
                } else {
                    self.entry_price * (1.0 - BREAKEVEN_BUFFER_PCT / 100.0)
                };

                // Only tighten, never widen.
                if is_long && breakeven_sl > self.current_sl_price {
                    self.current_sl_price = breakeven_sl;
                    self.profit_lock_active = true;
                    debug!(
                        sl = format!("{:.2}", self.current_sl_price),
                        "Profit lock activated (50% of TP1)"
                    );
                } else if !is_long && breakeven_sl < self.current_sl_price {
                    self.current_sl_price = breakeven_sl;
                    self.profit_lock_active = true;
                    debug!(
                        sl = format!("{:.2}", self.current_sl_price),
                        "Profit lock activated (50% of TP1)"
                    );
                }
            }
        }

        // ── Breakeven lock at 75% time elapsed ──────────────────────
        if !self.breakeven_lock_active && elapsed_fraction >= BREAKEVEN_LOCK_FRACTION {
            let breakeven_sl = if is_long {
                self.entry_price * (1.0 + BREAKEVEN_BUFFER_PCT / 100.0)
            } else {
                self.entry_price * (1.0 - BREAKEVEN_BUFFER_PCT / 100.0)
            };

            if is_long && breakeven_sl > self.current_sl_price {
                self.current_sl_price = breakeven_sl;
                self.breakeven_lock_active = true;
                debug!(
                    sl = format!("{:.2}", self.current_sl_price),
                    "Breakeven lock activated (75% time)"
                );
            } else if !is_long && breakeven_sl < self.current_sl_price {
                self.current_sl_price = breakeven_sl;
                self.breakeven_lock_active = true;
                debug!(
                    sl = format!("{:.2}", self.current_sl_price),
                    "Breakeven lock activated (75% time)"
                );
            }
        }

        // ── Progressive tightening at 50% time elapsed ──────────────
        if !self.breakeven_lock_active
            && !self.profit_lock_active
            && elapsed_fraction >= TIGHTEN_START_FRACTION
        {
            // Linear interpolation from original SL toward entry price.
            let progress = (elapsed_fraction - TIGHTEN_START_FRACTION)
                / (BREAKEVEN_LOCK_FRACTION - TIGHTEN_START_FRACTION);
            let progress = progress.clamp(0.0, 1.0);

            let original_sl = if is_long {
                self.entry_price * (1.0 - self.config.sl_pct / 100.0)
            } else {
                self.entry_price * (1.0 + self.config.sl_pct / 100.0)
            };

            let tightened_sl = original_sl + progress * (self.entry_price - original_sl);

            // Only tighten, never widen.
            if is_long && tightened_sl > self.current_sl_price {
                self.current_sl_price = tightened_sl;
                debug!(
                    sl = format!("{:.2}", self.current_sl_price),
                    progress = format!("{:.2}", progress),
                    "Progressive tightening applied"
                );
            } else if !is_long && tightened_sl < self.current_sl_price {
                self.current_sl_price = tightened_sl;
                debug!(
                    sl = format!("{:.2}", self.current_sl_price),
                    progress = format!("{:.2}", progress),
                    "Progressive tightening applied"
                );
            }
        }

        // ── TP2 check (before TP1, since TP2 > TP1 for longs) ──────
        if is_long && current_price >= self.tp2_price {
            return Some(ExitReason::TakeProfit2);
        }
        if !is_long && current_price <= self.tp2_price {
            return Some(ExitReason::TakeProfit2);
        }

        // ── TP1 check ──────────────────────────────────────────────
        if !self.tp1_hit {
            if is_long && current_price >= self.tp1_price {
                self.tp1_hit = true;
                return Some(ExitReason::TakeProfit1);
            }
            if !is_long && current_price <= self.tp1_price {
                self.tp1_hit = true;
                return Some(ExitReason::TakeProfit1);
            }
        }

        // ── SL check ───────────────────────────────────────────────
        if is_long && current_price <= self.current_sl_price {
            return Some(ExitReason::StopLoss);
        }
        if !is_long && current_price >= self.current_sl_price {
            return Some(ExitReason::StopLoss);
        }

        None
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn barrier_config_enforces_floors() {
        // Very low ATR should still produce minimum barrier widths.
        let config = BarrierConfig::from_atr(0.1, "TRENDING");
        assert!(
            config.sl_pct >= MIN_SL_PCT,
            "SL {} < floor {}",
            config.sl_pct,
            MIN_SL_PCT
        );
        assert!(
            config.tp1_pct >= MIN_TP1_PCT,
            "TP1 {} < floor {}",
            config.tp1_pct,
            MIN_TP1_PCT
        );
        assert!(
            config.tp2_pct >= MIN_TP2_PCT,
            "TP2 {} < floor {}",
            config.tp2_pct,
            MIN_TP2_PCT
        );
    }

    #[test]
    fn barrier_config_atr_scaling() {
        // With sufficient ATR the barriers should exceed the floors.
        let config = BarrierConfig::from_atr(1.0, "TRENDING");
        assert!(config.sl_pct > MIN_SL_PCT);
        assert!(config.tp1_pct > MIN_TP1_PCT);
        assert!(config.tp2_pct > MIN_TP2_PCT);
    }

    #[test]
    fn long_sl_triggered() {
        let config = BarrierConfig::explicit(1.0, 2.0, 4.0, 3600);
        let mut state = BarrierState::new(config, 100.0, "BUY", 1000);

        // Price drops below SL.
        let result = state.evaluate(98.5, 1001);
        assert_eq!(result, Some(ExitReason::StopLoss));
    }

    #[test]
    fn long_tp1_triggered() {
        let config = BarrierConfig::explicit(1.0, 2.0, 4.0, 3600);
        let mut state = BarrierState::new(config, 100.0, "BUY", 1000);

        // Price rises to TP1.
        let result = state.evaluate(102.1, 1001);
        assert_eq!(result, Some(ExitReason::TakeProfit1));
    }

    #[test]
    fn long_tp2_triggered() {
        let config = BarrierConfig::explicit(1.0, 2.0, 4.0, 3600);
        let mut state = BarrierState::new(config, 100.0, "BUY", 1000);

        // Price rises to TP2.
        let result = state.evaluate(104.1, 1001);
        assert_eq!(result, Some(ExitReason::TakeProfit2));
    }

    #[test]
    fn time_barrier_triggered() {
        let config = BarrierConfig::explicit(1.0, 2.0, 4.0, 3600);
        let mut state = BarrierState::new(config, 100.0, "BUY", 1000);

        // Time has elapsed.
        let result = state.evaluate(100.5, 1000 + 3601);
        assert_eq!(result, Some(ExitReason::TimeBarrier));
    }

    #[test]
    fn no_barrier_triggered() {
        let config = BarrierConfig::explicit(1.0, 2.0, 4.0, 3600);
        let mut state = BarrierState::new(config, 100.0, "BUY", 1000);

        // Price is between SL and TP1, time not elapsed.
        let result = state.evaluate(100.5, 1100);
        assert_eq!(result, None);
    }

    #[test]
    fn short_sl_triggered() {
        let config = BarrierConfig::explicit(1.0, 2.0, 4.0, 3600);
        let mut state = BarrierState::new(config, 100.0, "SELL", 1000);

        // Price rises above SL for a short.
        let result = state.evaluate(101.1, 1001);
        assert_eq!(result, Some(ExitReason::StopLoss));
    }

    #[test]
    fn short_tp1_triggered() {
        let config = BarrierConfig::explicit(1.0, 2.0, 4.0, 3600);
        let mut state = BarrierState::new(config, 100.0, "SELL", 1000);

        // Price drops to TP1 for a short.
        let result = state.evaluate(97.9, 1001);
        assert_eq!(result, Some(ExitReason::TakeProfit1));
    }

    #[test]
    fn progressive_tightening_at_50_pct_time() {
        let config = BarrierConfig::explicit(2.0, 4.0, 8.0, 1000);
        let mut state = BarrierState::new(config, 100.0, "BUY", 0);

        let original_sl = state.current_sl_price;

        // At exactly 50% time, price unchanged.
        state.evaluate(100.5, 500);

        // After 50% the SL should be tighter (higher for longs).
        assert!(
            state.current_sl_price >= original_sl,
            "SL should not widen: {} vs {}",
            state.current_sl_price,
            original_sl
        );
    }

    #[test]
    fn breakeven_lock_at_75_pct_time() {
        let config = BarrierConfig::explicit(2.0, 4.0, 8.0, 1000);
        let mut state = BarrierState::new(config, 100.0, "BUY", 0);

        // Price is slightly above entry at 75% time.
        state.evaluate(100.5, 750);

        // SL should be at least breakeven + buffer.
        let breakeven = 100.0 * (1.0 + BREAKEVEN_BUFFER_PCT / 100.0);
        assert!(
            state.current_sl_price >= breakeven - 0.01,
            "SL {} should be >= breakeven {}",
            state.current_sl_price,
            breakeven
        );
        assert!(state.breakeven_lock_active);
    }

    #[test]
    fn profit_lock_at_50_pct_tp1() {
        let config = BarrierConfig::explicit(1.0, 2.0, 4.0, 3600);
        let mut state = BarrierState::new(config, 100.0, "BUY", 0);

        // TP1 is at 102.0, so 50% of distance is 1.0 => price = 101.0.
        state.evaluate(101.1, 100);

        assert!(
            state.profit_lock_active,
            "Profit lock should be active at 50% of TP1 distance"
        );

        let breakeven = 100.0 * (1.0 + BREAKEVEN_BUFFER_PCT / 100.0);
        assert!(
            state.current_sl_price >= breakeven - 0.01,
            "SL {} should be >= breakeven + buffer {}",
            state.current_sl_price,
            breakeven
        );
    }

    #[test]
    fn regime_params_differ() {
        let trending = regime_params("TRENDING");
        let ranging = regime_params("RANGING");

        // Trending should have wider SL and longer time.
        assert!(trending.0 > ranging.0, "Trending SL mult should be wider");
        assert!(trending.3 > ranging.3, "Trending time should be longer");
    }
}
