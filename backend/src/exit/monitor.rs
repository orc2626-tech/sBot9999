// =============================================================================
// Exit Monitor Loop — Periodic barrier + micro-trail evaluation
// =============================================================================
//
// Runs as a background Tokio task, waking every 5 seconds to:
//   1. Iterate all open positions.
//   2. Evaluate each position's triple-barrier state.
//   3. If no barrier triggered AND enable_micro_trail is ON, evaluate the
//      micro-trail with real-time order flow data.
//   4. Close any position that has triggered an exit.
//   5. Log every exit with the triggering reason.
//
// The monitor is designed to be spawned once at engine startup:
//
//   tokio::spawn(run_exit_monitor(
//       Arc::clone(&state),
//       barrier_states,
//       micro_trail_states,
//   ));
//
// =============================================================================

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use parking_lot::RwLock;
use tokio::time::{interval, Duration};
use tracing::{debug, error, info, warn};

use crate::app_state::AppState;
use crate::exit::micro_trail::{MicroTrailState, OrderFlowContext};
use crate::exit::triple_barrier::BarrierState;

/// Interval at which the exit monitor evaluates open positions.
const MONITOR_INTERVAL_SECS: u64 = 5;

/// Shared barrier states keyed by position ID.
pub type BarrierStates = Arc<RwLock<HashMap<String, BarrierState>>>;

/// Shared micro-trail states keyed by position ID.
pub type MicroTrailStates = Arc<RwLock<HashMap<String, MicroTrailState>>>;

/// Create a new, empty barrier states map.
pub fn new_barrier_states() -> BarrierStates {
    Arc::new(RwLock::new(HashMap::new()))
}

/// Create a new, empty micro-trail states map.
pub fn new_micro_trail_states() -> MicroTrailStates {
    Arc::new(RwLock::new(HashMap::new()))
}

/// Run the exit monitor loop. This function runs forever and should be spawned
/// as a background Tokio task.
///
/// # Arguments
///
/// * `state` — Shared application state (provides position manager, risk
///   engine, order flow data, and version tracking).
/// * `barriers` — Mutable map of barrier states, one per open position.
/// * `micro_trails` — Mutable map of micro-trail states, one per open position.
pub async fn run_exit_monitor(
    state: Arc<AppState>,
    barriers: BarrierStates,
    micro_trails: MicroTrailStates,
) {
    info!(
        interval_secs = MONITOR_INTERVAL_SECS,
        "Exit monitor started (with micro-trail support)"
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

        // Read feature flag once per tick.
        let micro_trail_enabled = state.runtime_config.read().enable_micro_trail;

        debug!(
            count = open_positions.len(),
            micro_trail = micro_trail_enabled,
            "Exit monitor: evaluating positions"
        );

        // Collect positions to close (we cannot hold the barrier lock while
        // calling close_position, which also writes to AppState).
        let mut to_close: Vec<(String, f64, String)> = Vec::new();

        {
            let mut barrier_map = barriers.write();
            let mut trail_map = micro_trails.write();

            for position in &open_positions {
                let current_price = position.current_price;
                if current_price <= 0.0 {
                    warn!(
                        id = %position.id,
                        symbol = %position.symbol,
                        price = current_price,
                        "Invalid current price — skipping exit evaluation"
                    );
                    continue;
                }

                // ── 1. Triple Barrier evaluation ─────────────────────
                let barrier_exit = if let Some(barrier) = barrier_map.get_mut(&position.id) {
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
                            Some(exit_reason.to_string())
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
                                "Triple barrier: no trigger"
                            );
                            None
                        }
                    }
                } else {
                    debug!(
                        id = %position.id,
                        symbol = %position.symbol,
                        "No barrier state for position — skipping barrier eval"
                    );
                    None
                };

                if let Some(reason) = barrier_exit {
                    to_close.push((position.id.clone(), current_price, reason));
                    continue;
                }

                // ── 2. Micro-Trail evaluation (if enabled) ───────────
                // Always update trail state for data collection, but only
                // trigger exits when the feature flag is ON.
                if let Some(trail) = trail_map.get_mut(&position.id) {
                    let of_ctx = build_order_flow_context(&state, &position.symbol, trail);

                    let trail_hit = trail.evaluate(current_price, now_secs, &of_ctx);

                    if trail_hit && micro_trail_enabled {
                        let reason = format!(
                            "MicroTrail_{} | {}",
                            trail.phase, trail.adjustment_reason
                        );
                        info!(
                            id = %position.id,
                            symbol = %position.symbol,
                            side = %position.side,
                            entry_price = position.entry_price,
                            exit_price = current_price,
                            trail_price = format!("{:.4}", trail.trail_price),
                            phase = %trail.phase,
                            of_mult = format!("{:.2}", trail.of_tighten_mult),
                            reason = %reason,
                            "MICRO-TRAIL TRIGGERED — closing position"
                        );
                        to_close.push((position.id.clone(), current_price, reason));
                    } else if trail_hit {
                        // Feature flag OFF — log observation only.
                        debug!(
                            id = %position.id,
                            symbol = %position.symbol,
                            trail_price = format!("{:.4}", trail.trail_price),
                            phase = %trail.phase,
                            "MicroTrail WOULD have triggered (flag OFF)"
                        );
                    }
                }
            }
        }

        // Close triggered positions and clean up state maps.
        for (position_id, exit_price, reason) in to_close {
            match state
                .position_manager
                .close_position(&position_id, &reason, exit_price)
            {
                Some(realized_pnl) => {
                    info!(
                        id = %position_id,
                        pnl = realized_pnl,
                        reason = &reason,
                        "Position closed by exit monitor"
                    );

                    // Record the trade result in the risk engine.
                    state.risk_engine.record_trade_result(realized_pnl);

                    // Remove barrier and micro-trail state.
                    barriers.write().remove(&position_id);
                    micro_trails.write().remove(&position_id);

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

/// Build an `OrderFlowContext` for the given symbol from AppState data.
fn build_order_flow_context(
    state: &AppState,
    symbol: &str,
    trail: &MicroTrailState,
) -> OrderFlowContext {
    let trade_procs = state.trade_processors.read();
    let proc = trade_procs.get(symbol);

    let cvd = proc.map(|p| p.cvd()).unwrap_or(0.0);
    let orderbook_imbalance = state
        .orderbook_manager
        .imbalance(symbol)
        .unwrap_or(0.0);

    let vpin = state
        .vpin_states
        .read()
        .get(symbol)
        .map(|v| v.vpin)
        .unwrap_or(0.0);

    OrderFlowContext {
        cvd,
        cvd_at_entry: trail.cvd_at_entry,
        orderbook_imbalance,
        vpin,
    }
}
