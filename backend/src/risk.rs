// =============================================================================
// Risk Engine — four circuit breakers protecting capital
// =============================================================================
//
// Circuit breakers:
//   1. Daily Loss       — trips when cumulative daily PnL loss exceeds a
//                         percentage of starting capital.
//   2. Consecutive Losses — trips after N consecutive losing trades.
//   3. Max Drawdown      — trips when intra-day drawdown from peak equity
//                         exceeds the threshold.
//   4. Trade Limit       — trips when daily trade count reaches the cap.
//
// The engine automatically resets daily statistics when the date rolls over.
// =============================================================================

use chrono::Utc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Snapshot of a single circuit breaker for dashboard display.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerInfo {
    pub name: String,
    pub current: f64,
    pub limit: f64,
    pub tripped: bool,
}

/// Full snapshot of the risk engine's internal state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskState {
    pub risk_mode: String,
    #[serde(default)]
    pub daily_pnl: f64,
    #[serde(default)]
    pub daily_pnl_pct: f64,
    #[serde(default)]
    pub remaining_daily_loss_pct: f64,
    #[serde(default)]
    pub consecutive_losses: u32,
    #[serde(default)]
    pub daily_trades_count: u32,
    #[serde(default)]
    pub daily_wins: u32,
    #[serde(default)]
    pub daily_losses: u32,
    #[serde(default)]
    pub max_drawdown_today: f64,
    #[serde(default)]
    pub peak_equity_today: f64,
    #[serde(default)]
    pub circuit_breakers: Vec<CircuitBreakerInfo>,
    #[serde(default)]
    pub current_date: String,
}

// ---------------------------------------------------------------------------
// Internal mutable state (behind RwLock)
// ---------------------------------------------------------------------------

struct Inner {
    risk_mode: String,
    daily_pnl: f64,
    consecutive_losses: u32,
    daily_trades_count: u32,
    daily_wins: u32,
    daily_losses: u32,
    max_drawdown_today: f64,
    peak_equity_today: f64,
    current_date: String,
    killed: bool,
}

// ---------------------------------------------------------------------------
// Risk Engine
// ---------------------------------------------------------------------------

/// Configuration limits supplied at construction time.
pub struct RiskEngine {
    state: RwLock<Inner>,
    /// Starting capital for the current session (used to compute percentages).
    capital: f64,
    /// Maximum daily loss allowed as a fraction (e.g. 0.03 = 3 %).
    max_daily_loss_pct: f64,
    /// Maximum consecutive losing trades before the breaker trips.
    max_consecutive_losses: u32,
    /// Maximum intra-day drawdown as a fraction.
    max_drawdown_pct: f64,
    /// Maximum number of trades per day.
    max_daily_trades: u32,
}

impl RiskEngine {
    /// Create a new risk engine.
    ///
    /// # Arguments
    /// * `capital`              — Starting capital for percentage calculations.
    /// * `max_daily_loss_pct`   — e.g. 0.03 for 3 %.
    /// * `max_consecutive_losses` — e.g. 5.
    /// * `max_drawdown_pct`     — e.g. 0.05 for 5 %.
    /// * `max_daily_trades`     — e.g. 50.
    pub fn new(
        capital: f64,
        max_daily_loss_pct: f64,
        max_consecutive_losses: u32,
        max_drawdown_pct: f64,
        max_daily_trades: u32,
    ) -> Self {
        let today = Utc::now().format("%Y-%m-%d").to_string();
        info!(
            capital,
            max_daily_loss_pct,
            max_consecutive_losses,
            max_drawdown_pct,
            max_daily_trades,
            "RiskEngine initialised"
        );

        Self {
            state: RwLock::new(Inner {
                risk_mode: "Normal".to_string(),
                daily_pnl: 0.0,
                consecutive_losses: 0,
                daily_trades_count: 0,
                daily_wins: 0,
                daily_losses: 0,
                max_drawdown_today: 0.0,
                peak_equity_today: capital,
                current_date: today,
                killed: false,
            }),
            capital,
            max_daily_loss_pct,
            max_consecutive_losses,
            max_drawdown_pct,
            max_daily_trades,
        }
    }

    // -------------------------------------------------------------------------
    // Trade recording
    // -------------------------------------------------------------------------

    /// Record the PnL of a completed trade and update all internal counters.
    pub fn record_trade_result(&self, pnl: f64) {
        self.maybe_reset_daily();
        let mut s = self.state.write();

        s.daily_pnl += pnl;
        s.daily_trades_count += 1;

        if pnl >= 0.0 {
            s.daily_wins += 1;
            s.consecutive_losses = 0;
        } else {
            s.daily_losses += 1;
            s.consecutive_losses += 1;
        }

        // Track peak equity and drawdown.
        let current_equity = self.capital + s.daily_pnl;
        if current_equity > s.peak_equity_today {
            s.peak_equity_today = current_equity;
        }
        let drawdown = if s.peak_equity_today > 0.0 {
            (s.peak_equity_today - current_equity) / s.peak_equity_today
        } else {
            0.0
        };
        if drawdown > s.max_drawdown_today {
            s.max_drawdown_today = drawdown;
        }

        // Update risk mode label.
        s.risk_mode = self.compute_risk_mode(&s);

        debug!(
            pnl,
            daily_pnl = s.daily_pnl,
            consecutive_losses = s.consecutive_losses,
            daily_trades = s.daily_trades_count,
            drawdown = s.max_drawdown_today,
            risk_mode = %s.risk_mode,
            "trade result recorded"
        );
    }

    // -------------------------------------------------------------------------
    // Pre-trade gate
    // -------------------------------------------------------------------------

    /// Check whether trading is currently allowed.
    ///
    /// Returns `(true, None)` if all breakers are clear, or `(false,
    /// Some(reason))` if a breaker has tripped.
    pub fn can_trade(&self) -> (bool, Option<String>) {
        self.maybe_reset_daily();
        let s = self.state.read();

        if s.killed {
            return (false, Some("Kill switch activated".to_string()));
        }

        // 1. Daily loss
        let daily_loss_pct = if self.capital > 0.0 {
            (-s.daily_pnl) / self.capital
        } else {
            0.0
        };
        if daily_loss_pct >= self.max_daily_loss_pct {
            let msg = format!(
                "Daily Loss breaker tripped: {:.2}% lost (limit {:.2}%)",
                daily_loss_pct * 100.0,
                self.max_daily_loss_pct * 100.0
            );
            warn!("{}", msg);
            return (false, Some(msg));
        }

        // 2. Consecutive losses
        if s.consecutive_losses >= self.max_consecutive_losses {
            let msg = format!(
                "Consecutive Losses breaker tripped: {} consecutive losses (limit {})",
                s.consecutive_losses, self.max_consecutive_losses
            );
            warn!("{}", msg);
            return (false, Some(msg));
        }

        // 3. Max drawdown
        if s.max_drawdown_today >= self.max_drawdown_pct {
            let msg = format!(
                "Max Drawdown breaker tripped: {:.2}% drawdown (limit {:.2}%)",
                s.max_drawdown_today * 100.0,
                self.max_drawdown_pct * 100.0
            );
            warn!("{}", msg);
            return (false, Some(msg));
        }

        // 4. Trade limit
        if s.daily_trades_count >= self.max_daily_trades {
            let msg = format!(
                "Trade Limit breaker tripped: {} trades today (limit {})",
                s.daily_trades_count, self.max_daily_trades
            );
            warn!("{}", msg);
            return (false, Some(msg));
        }

        (true, None)
    }

    // -------------------------------------------------------------------------
    // State snapshot
    // -------------------------------------------------------------------------

    /// Build a serialisable snapshot of the current risk state.
    pub fn get_state(&self) -> RiskState {
        self.maybe_reset_daily();
        let s = self.state.read();

        let daily_pnl_pct = if self.capital > 0.0 {
            (s.daily_pnl / self.capital) * 100.0
        } else {
            0.0
        };
        let remaining_daily_loss_pct = (self.max_daily_loss_pct * 100.0) - ((-s.daily_pnl / self.capital.max(1.0)) * 100.0);

        let breakers = self.build_circuit_breaker_info(&s);

        RiskState {
            risk_mode: s.risk_mode.clone(),
            daily_pnl: s.daily_pnl,
            daily_pnl_pct,
            remaining_daily_loss_pct: remaining_daily_loss_pct.max(0.0),
            consecutive_losses: s.consecutive_losses,
            daily_trades_count: s.daily_trades_count,
            daily_wins: s.daily_wins,
            daily_losses: s.daily_losses,
            max_drawdown_today: s.max_drawdown_today,
            peak_equity_today: s.peak_equity_today,
            circuit_breakers: breakers,
            current_date: s.current_date.clone(),
        }
    }

    // -------------------------------------------------------------------------
    // Daily reset
    // -------------------------------------------------------------------------

    /// Forcefully reset daily statistics (e.g. called by an admin endpoint).
    pub fn reset_daily(&self) {
        let mut s = self.state.write();
        let today = Utc::now().format("%Y-%m-%d").to_string();
        Self::do_reset(&mut s, &today, self.capital);
        info!(date = %today, "daily risk counters reset (manual)");
    }

    /// Activate the kill switch — blocks all trading until manually cleared.
    pub fn kill(&self) {
        let mut s = self.state.write();
        s.killed = true;
        s.risk_mode = "KILLED".to_string();
        warn!("kill switch activated — all trading halted");
    }

    // -------------------------------------------------------------------------
    // Internals
    // -------------------------------------------------------------------------

    /// If the calendar date has changed since the last check, reset all daily
    /// counters automatically.
    fn maybe_reset_daily(&self) {
        let today = Utc::now().format("%Y-%m-%d").to_string();
        {
            let s = self.state.read();
            if s.current_date == today {
                return;
            }
        }
        // Date has changed — acquire write lock and reset.
        let mut s = self.state.write();
        // Double-check after acquiring write lock (another thread may have
        // already performed the reset).
        if s.current_date != today {
            info!(
                old_date = %s.current_date,
                new_date = %today,
                "date rolled — resetting daily risk counters"
            );
            Self::do_reset(&mut s, &today, self.capital);
        }
    }

    fn do_reset(s: &mut Inner, date: &str, capital: f64) {
        s.daily_pnl = 0.0;
        s.consecutive_losses = 0;
        s.daily_trades_count = 0;
        s.daily_wins = 0;
        s.daily_losses = 0;
        s.max_drawdown_today = 0.0;
        s.peak_equity_today = capital;
        s.current_date = date.to_string();
        s.risk_mode = if s.killed {
            "KILLED".to_string()
        } else {
            "Normal".to_string()
        };
    }

    fn compute_risk_mode(&self, s: &Inner) -> String {
        if s.killed {
            return "KILLED".to_string();
        }

        let daily_loss_pct = if self.capital > 0.0 {
            (-s.daily_pnl) / self.capital
        } else {
            0.0
        };

        if daily_loss_pct >= self.max_daily_loss_pct
            || s.consecutive_losses >= self.max_consecutive_losses
            || s.max_drawdown_today >= self.max_drawdown_pct
            || s.daily_trades_count >= self.max_daily_trades
        {
            "BREAKER_TRIPPED".to_string()
        } else if daily_loss_pct >= self.max_daily_loss_pct * 0.75
            || s.consecutive_losses as f64 >= self.max_consecutive_losses as f64 * 0.75
        {
            "Cautious".to_string()
        } else {
            "Normal".to_string()
        }
    }

    fn build_circuit_breaker_info(&self, s: &Inner) -> Vec<CircuitBreakerInfo> {
        let daily_loss_pct = if self.capital > 0.0 {
            ((-s.daily_pnl) / self.capital) * 100.0
        } else {
            0.0
        };

        vec![
            CircuitBreakerInfo {
                name: "Daily Loss".to_string(),
                current: daily_loss_pct.max(0.0),
                limit: self.max_daily_loss_pct * 100.0,
                tripped: daily_loss_pct >= self.max_daily_loss_pct * 100.0,
            },
            CircuitBreakerInfo {
                name: "Consecutive Losses".to_string(),
                current: s.consecutive_losses as f64,
                limit: self.max_consecutive_losses as f64,
                tripped: s.consecutive_losses >= self.max_consecutive_losses,
            },
            CircuitBreakerInfo {
                name: "Max Drawdown".to_string(),
                current: s.max_drawdown_today * 100.0,
                limit: self.max_drawdown_pct * 100.0,
                tripped: s.max_drawdown_today >= self.max_drawdown_pct,
            },
            CircuitBreakerInfo {
                name: "Trade Limit".to_string(),
                current: s.daily_trades_count as f64,
                limit: self.max_daily_trades as f64,
                tripped: s.daily_trades_count >= self.max_daily_trades,
            },
        ]
    }
}

impl std::fmt::Debug for RiskEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RiskEngine")
            .field("capital", &self.capital)
            .field("max_daily_loss_pct", &self.max_daily_loss_pct)
            .field("max_consecutive_losses", &self.max_consecutive_losses)
            .field("max_drawdown_pct", &self.max_drawdown_pct)
            .field("max_daily_trades", &self.max_daily_trades)
            .finish()
    }
}
