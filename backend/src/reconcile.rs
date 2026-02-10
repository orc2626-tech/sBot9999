// =============================================================================
// Reconciliation Engine — compare internal state against the exchange
// =============================================================================
//
// SAFETY POLICY: this module logs warnings for any drift it discovers but will
// **never** automatically cancel orders or close positions on the exchange.
// A human operator or explicit admin action must resolve discrepancies.
// =============================================================================

use anyhow::{Context, Result};
use chrono::Utc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::binance::client::BinanceClient;
use crate::position_engine::PositionManager;
use crate::types::BalanceInfo;

// ---------------------------------------------------------------------------
// Result type
// ---------------------------------------------------------------------------

/// Summary of a single reconciliation pass.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconcileResult {
    /// Number of internal positions that matched an exchange order.
    pub positions_matched: u32,
    /// Exchange orders that have no corresponding internal position.
    pub orphan_orders: u32,
    /// Whether the balance snapshot drifted from what we expect.
    pub balance_drift: bool,
    /// ISO-8601 timestamp of this reconciliation run.
    pub timestamp: String,
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Run one reconciliation cycle.
///
/// 1. Fetch open orders from the exchange.
/// 2. Compare them against internally tracked positions.
/// 3. Refresh the balance cache from the exchange.
///
/// # Arguments
/// * `client`           — Binance REST client.
/// * `position_manager` — Internal position store.
/// * `balances`         — Shared balance cache to update.
pub async fn reconcile_once(
    client: &BinanceClient,
    position_manager: &PositionManager,
    balances: &RwLock<Vec<BalanceInfo>>,
) -> Result<ReconcileResult> {
    let now = Utc::now().to_rfc3339();
    info!(timestamp = %now, "reconciliation cycle started");

    // -----------------------------------------------------------------
    // 1. Fetch exchange open orders
    // -----------------------------------------------------------------
    let exchange_orders = client
        .get_open_orders(None)
        .await
        .context("reconcile: failed to fetch open orders")?;

    debug!(exchange_order_count = exchange_orders.len(), "exchange orders fetched");

    // Build a set of symbols from exchange orders for quick lookup.
    let exchange_symbols: std::collections::HashSet<String> = exchange_orders
        .iter()
        .filter_map(|o| o["symbol"].as_str().map(|s| s.to_string()))
        .collect();

    // -----------------------------------------------------------------
    // 2. Compare with internal positions
    // -----------------------------------------------------------------
    let open_positions = position_manager.get_open_positions();
    let mut matched: u32 = 0;

    for pos in &open_positions {
        if exchange_symbols.contains(&pos.symbol) {
            matched += 1;
            debug!(
                position_id = %pos.id,
                symbol = %pos.symbol,
                "position matched with exchange order"
            );
        } else {
            warn!(
                position_id = %pos.id,
                symbol = %pos.symbol,
                "internal position has NO matching exchange order — possible drift"
            );
        }
    }

    // Orphan orders: exchange orders whose symbol has no matching internal
    // position.
    let internal_symbols: std::collections::HashSet<String> = open_positions
        .iter()
        .map(|p| p.symbol.clone())
        .collect();

    let mut orphan_count: u32 = 0;
    for order in &exchange_orders {
        if let Some(sym) = order["symbol"].as_str() {
            if !internal_symbols.contains(sym) {
                orphan_count += 1;
                warn!(
                    symbol = sym,
                    order_id = %order.get("orderId").and_then(|v| v.as_u64()).unwrap_or(0),
                    "orphan exchange order detected — no matching internal position"
                );
            }
        }
    }

    // -----------------------------------------------------------------
    // 3. Update balances
    // -----------------------------------------------------------------
    let balance_drift = refresh_balances(client, balances).await?;

    let result = ReconcileResult {
        positions_matched: matched,
        orphan_orders: orphan_count,
        balance_drift,
        timestamp: now.clone(),
    };

    info!(
        positions_matched = matched,
        orphan_orders = orphan_count,
        balance_drift,
        timestamp = %now,
        "reconciliation cycle completed"
    );

    Ok(result)
}

// ---------------------------------------------------------------------------
// Balance refresh
// ---------------------------------------------------------------------------

/// Fetch account balances from the exchange and update the shared cache.
///
/// Returns `true` if any non-zero balance changed significantly (> 0.01 %
/// relative difference), indicating drift.
async fn refresh_balances(
    client: &BinanceClient,
    balances: &RwLock<Vec<BalanceInfo>>,
) -> Result<bool> {
    let account = client
        .get_account()
        .await
        .context("reconcile: failed to fetch account for balance refresh")?;

    let raw_balances = account["balances"]
        .as_array()
        .context("reconcile: account response missing 'balances'")?;

    let mut new_balances: Vec<BalanceInfo> = Vec::new();
    for b in raw_balances {
        let asset = b["asset"].as_str().unwrap_or("").to_string();
        let free: f64 = b["free"]
            .as_str()
            .unwrap_or("0")
            .parse()
            .unwrap_or(0.0);
        let locked: f64 = b["locked"]
            .as_str()
            .unwrap_or("0")
            .parse()
            .unwrap_or(0.0);

        // Only track assets with non-zero balances to keep the list manageable.
        if free > 0.0 || locked > 0.0 {
            new_balances.push(BalanceInfo { asset, free, locked });
        }
    }

    // Detect drift by comparing with the previous snapshot.
    let drift = {
        let old = balances.read();
        detect_balance_drift(&old, &new_balances)
    };

    if drift {
        warn!("balance drift detected during reconciliation");
    } else {
        debug!("balances refreshed — no significant drift");
    }

    // Replace cached balances atomically.
    *balances.write() = new_balances;

    Ok(drift)
}

/// Compare two balance snapshots and return `true` if any asset changed by
/// more than a small relative threshold.
fn detect_balance_drift(old: &[BalanceInfo], new: &[BalanceInfo]) -> bool {
    use std::collections::HashMap;

    if old.is_empty() {
        // First run — no drift to report.
        return false;
    }

    let old_map: HashMap<&str, (f64, f64)> = old
        .iter()
        .map(|b| (b.asset.as_str(), (b.free, b.locked)))
        .collect();

    for nb in new {
        if let Some(&(old_free, old_locked)) = old_map.get(nb.asset.as_str()) {
            let total_old = old_free + old_locked;
            let total_new = nb.free + nb.locked;
            if total_old > 0.0 {
                let pct_change = ((total_new - total_old) / total_old).abs();
                if pct_change > 0.0001 {
                    debug!(
                        asset = %nb.asset,
                        old_total = total_old,
                        new_total = total_new,
                        pct_change = pct_change * 100.0,
                        "balance drift for asset"
                    );
                    return true;
                }
            } else if total_new > 0.0 {
                // Asset appeared from zero.
                return true;
            }
        } else {
            // Brand-new asset not previously tracked.
            return true;
        }
    }

    // Check for assets that disappeared.
    let new_map: HashMap<&str, ()> = new.iter().map(|b| (b.asset.as_str(), ())).collect();
    for ob in old {
        if !new_map.contains_key(ob.asset.as_str()) && (ob.free + ob.locked) > 0.0 {
            debug!(asset = %ob.asset, "asset disappeared from balances");
            return true;
        }
    }

    false
}
