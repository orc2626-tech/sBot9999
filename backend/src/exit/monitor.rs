// =============================================================================
// Exit Monitor Loop — Periodic barrier evaluation for all open positions
// =============================================================================
//
// Runs as a background Tokio task, waking every 5 seconds to:
//   1. Iterate all open positions.
//   2. Evaluate each position's barrier state against the current price and
//      wall-clock time.
//   3. Close any position that has triggered a barrier.
//   4. Log every exit with the triggering reason.
//
// The monitor is designed to be spawned once at engine startup:
//
//   tokio::spawn(run_exit_monitor(Arc::clone(&state), barrier_states));
//
// =============================================================================

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use parking_lot::RwLock;
use tokio::time::{interval, Duration};
use tracing::{debug, error, info, warn};

use crate::app_state::AppState;
use crate::exit::triple_barrier::BarrierState;

/// Interval at which the exit monitor evaluates open positions.
const MONITOR_INTERVAL_SECS: u64 = 5;

/// Shared barrier states keyed by position ID.
pub type BarrierStates = Arc<RwLock<HashMap<String, BarrierState>>>;

/// Create a new, empty barrier states map.
pub fn new_barrier_states() -> BarrierStates {
    Arc::new(RwLock::new(HashMap::new()))
}

/// Run the exit monitor loop. This function runs forever and should be spawned
/// as a background Tokio task.
///
/// # Arguments
///
/// * `state` — Shared application state (provides position manager, risk
///   engine, and version tracking).
/// * `barriers` — Mutable map of barrier states, one per open position.
pub async fn run_exit_monitor(state: Arc<AppState>, barriers: BarrierStates) {
    info!(
        interval_secs = MONITOR_INTERVAL_SECS,
        "Exit monitor started"
    );

    let mut ticker = interval(Duration::from_secs(MONITOR_INTERVAL_SECS));

    loop {
        ticker.tick().await;

        let now_secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let open_positions = state.position_manager.get_open_positions();

        if open_positions.is_empty() {
            debug!("Exit monitor: no open positions");
            continue;
        }

        debug!(
            count = open_positions.len(),
            "Exit monitor: evaluating positions"
        );

        // Collect positions to close (we cannot hold the barrier lock while
        // calling close_position, which also writes to AppState).
        let mut to_close: Vec<(String, f64, String)> = Vec::new();

        {
            let mut barrier_map = barriers.write();

            for position in &open_positions {
                let barrier = match barrier_map.get_mut(&position.id) {
                    Some(b) => b,
                    None => {
                        debug!(
                            id = %position.id,
                            symbol = %position.symbol,
                            "No barrier state for position — skipping"
                        );
                        continue;
                    }
                };

                let current_price = position.current_price;
                if current_price <= 0.0 {
                    warn!(
                        id = %position.id,
                        symbol = %position.symbol,
                        price = current_price,
                        "Invalid current price — skipping barrier evaluation"
                    );
                    continue;
                }

                match barrier.evaluate(current_price, now_secs) {
                    Some(exit_reason) => {
                        info!(
                            id = %position.id,
                            symbol = %position.symbol,
                            side = %position.side,
                            entry_price = position.entry_price,
                            exit_price = current_price,
                            reason = %exit_reason,
                            sl = format!("{:.2}", barrier.current_sl_price),
                            tp1 = format!("{:.2}", barrier.tp1_price),
                            tp2 = format!("{:.2}", barrier.tp2_price),
                            elapsed_secs = now_secs.saturating_sub(barrier.opened_at_secs),
                            "BARRIER TRIGGERED — closing position"
                        );

                        to_close.push((
                            position.id.clone(),
                            current_price,
                            exit_reason.to_string(),
                        ));
                    }
                    None => {
                        debug!(
                            id = %position.id,
                            symbol = %position.symbol,
                            price = current_price,
                            sl = format!("{:.2}", barrier.current_sl_price),
                            tp1_hit = barrier.tp1_hit,
                            profit_lock = barrier.profit_lock_active,
                            breakeven_lock = barrier.breakeven_lock_active,
                            "Position evaluated — no barrier triggered"
                        );
                    }
                }
            }
        }

        // Close triggered positions and clean up barrier state.
        for (position_id, exit_price, reason) in to_close {
            match state
                .position_manager
                .close_position(&position_id, exit_price, &reason)
            {
                Some(closed) => {
                    info!(
                        id = %closed.id,
                        symbol = %closed.symbol,
                        pnl = closed.realized_pnl.unwrap_or(0.0),
                        pnl_pct = format!("{:.4}", closed.realized_pnl_pct.unwrap_or(0.0)),
                        reason = &reason,
                        "Position closed by exit monitor"
                    );

                    // Record the trade result in the risk engine.
                    if let Some(pnl_pct) = closed.realized_pnl_pct {
                        state.risk_engine.record_trade_result(pnl_pct);
                    }

                    // Remove the barrier state.
                    barriers.write().remove(&position_id);

                    state.increment_version();
                }
                None => {
                    error!(
                        id = %position_id,
                        "Failed to close position — not found in position manager"
                    );
                }
            }
        }
    }
}
