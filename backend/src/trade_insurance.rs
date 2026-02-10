// =============================================================================
// Trade Insurance — 7 mandatory gates before any trade executes
// =============================================================================
//
// Every gate must pass for a trade to proceed. If ANY gate fails, the trade
// is blocked and the blocking reason is recorded in the DecisionEnvelope.
//
// Gates:
//   1. NotKilled       — trading_mode != Killed
//   2. NotPaused       — trading_mode != Paused
//   3. NotDeadRegime   — Dead regime blocks all trades (pure noise)
//   4. MaxPositions    — concurrent open positions < limit
//   5. NoDuplicateSymbol — no existing position for this symbol
//   6. SpreadOk        — bid-ask spread within acceptable range
//   7. RiskOk          — all circuit breakers clear
// =============================================================================

use std::sync::Arc;
use tracing::debug;

use crate::app_state::AppState;
use crate::types::TradingMode;

/// Maximum acceptable spread in basis points.
const MAX_SPREAD_BPS: f64 = 15.0;

pub struct InsuranceGate;

impl InsuranceGate {
    /// Run all insurance gates. Returns `None` if all pass, or `Some(reason)`
    /// if any gate blocks.
    pub fn check_all(
        state: &Arc<AppState>,
        symbol: &str,
        _side: &str,
    ) -> Option<String> {
        let config = state.runtime_config.read();

        // Gate 1: Not Killed
        if config.trading_mode == TradingMode::Killed {
            return Some("Trading mode is KILLED".to_string());
        }

        // Gate 2: Not Paused
        if config.trading_mode == TradingMode::Paused {
            return Some("Trading mode is PAUSED".to_string());
        }

        // Gate 3: Not Dead Regime
        {
            let regime_state = state.regime_detector.read().current_regime();
            if let Some(rs) = regime_state {
                if rs.regime.to_string() == "Dead" {
                    return Some("Market regime is DEAD (pure noise — no edge)".to_string());
                }
            }
        }

        // Gate 4: Max concurrent positions
        let open = state.position_manager.get_open_positions();
        let max_positions = config.max_concurrent_positions as usize;
        if open.len() >= max_positions {
            return Some(format!(
                "Max concurrent positions reached: {} >= {}",
                open.len(),
                max_positions
            ));
        }

        // Gate 5: No duplicate symbol position
        let has_symbol_position = open.iter().any(|p| p.symbol == symbol);
        if has_symbol_position {
            return Some(format!("Already have an open position for {}", symbol));
        }

        // Gate 6: Spread OK
        if let Some(spread) = state.orderbook_manager.spread_bps(symbol) {
            if spread > MAX_SPREAD_BPS {
                return Some(format!(
                    "Spread too wide: {:.1} bps > {:.1} bps limit",
                    spread, MAX_SPREAD_BPS
                ));
            }
        }

        // Gate 7: Risk engine OK (circuit breakers)
        let (allowed, reason) = state.risk_engine.can_trade();
        if !allowed {
            return Some(format!(
                "Risk engine blocked: {}",
                reason.unwrap_or_else(|| "unknown".to_string())
            ));
        }

        // Gate 8: No-go reason check
        {
            let no_go = state.no_go_reason.read();
            if let Some(reason) = no_go.as_ref() {
                return Some(format!("No-go reason active: {}", reason));
            }
        }

        debug!(symbol, "all insurance gates passed");
        None
    }
}
