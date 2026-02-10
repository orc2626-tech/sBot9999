// =============================================================================
// Position Engine — state machine for open / partially-closed / closed trades
// =============================================================================
//
// Life-cycle:
//   Open  ->  PartialTP1  ->  Closed
//   Open  ->  Closed (SL / TP2 / trailing stop / manual)
//
// Exit logic checked by `check_exits`:
//   1. Stop-loss hit            -> full close
//   2. Take-profit-2 hit        -> full close
//   3. Take-profit-1 hit        -> partial close (60 % of quantity)
//   4. Trailing stop triggered   -> full close of remaining quantity
//
// Thread-safety: all mutable state is behind `parking_lot::RwLock`.
// =============================================================================

use chrono::Utc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Position model
// ---------------------------------------------------------------------------

/// Current status of a position.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PositionStatus {
    Open,
    PartialTP1,
    Closed,
}

impl std::fmt::Display for PositionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Open => write!(f, "Open"),
            Self::PartialTP1 => write!(f, "PartialTP1"),
            Self::Closed => write!(f, "Closed"),
        }
    }
}

/// A single tracked position.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    /// Unique identifier (UUID v4).
    pub id: String,
    pub symbol: String,
    /// "BUY" (long) or "SELL" (short).
    pub side: String,
    pub entry_price: f64,
    /// Remaining open quantity (reduced on partial close).
    pub quantity: f64,
    #[serde(default)]
    pub current_price: f64,
    #[serde(default)]
    pub unrealized_pnl: f64,
    #[serde(default)]
    pub unrealized_pnl_pct: f64,
    pub stop_loss: f64,
    pub take_profit_1: f64,
    pub take_profit_2: f64,
    /// Trailing stop price — set dynamically as price moves in our favour.
    #[serde(default)]
    pub trailing_stop: Option<f64>,
    /// Highest (for longs) or lowest (for shorts) price seen since open.
    #[serde(default)]
    pub highest_price: f64,
    pub status: PositionStatus,
    pub opened_at: String,
    #[serde(default)]
    pub closed_at: Option<String>,
    #[serde(default)]
    pub close_reason: Option<String>,
    #[serde(default)]
    pub realized_pnl: f64,
}

// ---------------------------------------------------------------------------
// Position Manager
// ---------------------------------------------------------------------------

/// Thread-safe manager that owns the lists of open and closed positions.
pub struct PositionManager {
    open: RwLock<Vec<Position>>,
    closed: RwLock<Vec<Position>>,
}

/// Default trailing-stop distance as a fraction of highest price (0.5 %).
const TRAILING_STOP_PCT: f64 = 0.005;
/// Fraction of quantity closed when TP1 is hit.
const TP1_CLOSE_FRACTION: f64 = 0.60;

impl PositionManager {
    /// Create an empty manager.
    pub fn new() -> Self {
        Self {
            open: RwLock::new(Vec::new()),
            closed: RwLock::new(Vec::new()),
        }
    }

    // -------------------------------------------------------------------------
    // Open a new position
    // -------------------------------------------------------------------------

    /// Open a new position and return its UUID.
    pub fn open_position(
        &self,
        symbol: &str,
        side: &str,
        entry_price: f64,
        quantity: f64,
        stop_loss: f64,
        take_profit_1: f64,
        take_profit_2: f64,
    ) -> String {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        let pos = Position {
            id: id.clone(),
            symbol: symbol.to_string(),
            side: side.to_uppercase(),
            entry_price,
            quantity,
            current_price: entry_price,
            unrealized_pnl: 0.0,
            unrealized_pnl_pct: 0.0,
            stop_loss,
            take_profit_1,
            take_profit_2,
            trailing_stop: None,
            highest_price: entry_price,
            status: PositionStatus::Open,
            opened_at: now,
            closed_at: None,
            close_reason: None,
            realized_pnl: 0.0,
        };

        info!(
            id = %id,
            symbol,
            side,
            entry_price,
            quantity,
            stop_loss,
            take_profit_1,
            take_profit_2,
            "position opened"
        );

        self.open.write().push(pos);
        id
    }

    // -------------------------------------------------------------------------
    // Price updates
    // -------------------------------------------------------------------------

    /// Update the `current_price` for every open position matching `symbol` and
    /// recompute unrealised PnL, highest-price tracking, and trailing stop.
    pub fn update_price(&self, symbol: &str, current_price: f64) {
        let mut positions = self.open.write();
        for pos in positions.iter_mut().filter(|p| p.symbol == symbol) {
            pos.current_price = current_price;

            // Unrealised PnL
            let direction = if pos.side == "BUY" { 1.0 } else { -1.0 };
            pos.unrealized_pnl = direction * (current_price - pos.entry_price) * pos.quantity;
            pos.unrealized_pnl_pct = if pos.entry_price > 0.0 {
                direction * ((current_price - pos.entry_price) / pos.entry_price) * 100.0
            } else {
                0.0
            };

            // Track highest (long) / lowest (short) price
            let is_long = pos.side == "BUY";
            if is_long {
                if current_price > pos.highest_price {
                    pos.highest_price = current_price;
                    // Update trailing stop
                    let trail = pos.highest_price * (1.0 - TRAILING_STOP_PCT);
                    pos.trailing_stop = Some(trail);
                    debug!(
                        id = %pos.id,
                        highest_price = pos.highest_price,
                        trailing_stop = trail,
                        "trailing stop updated (long)"
                    );
                }
            } else {
                // For shorts, "highest_price" tracks the *lowest* price.
                if pos.highest_price == pos.entry_price || current_price < pos.highest_price {
                    pos.highest_price = current_price;
                    let trail = pos.highest_price * (1.0 + TRAILING_STOP_PCT);
                    pos.trailing_stop = Some(trail);
                    debug!(
                        id = %pos.id,
                        lowest_price = pos.highest_price,
                        trailing_stop = trail,
                        "trailing stop updated (short)"
                    );
                }
            }
        }
    }

    // -------------------------------------------------------------------------
    // Exit checks
    // -------------------------------------------------------------------------

    /// Scan all open positions and return a list of `(position_id, reason)`
    /// pairs for positions that should be exited.
    ///
    /// **Side-effects**: positions hitting TP1 are partially closed in-place
    /// (quantity reduced by 60 %, status changed to `PartialTP1`, and realised
    /// PnL accumulated).
    pub fn check_exits(&self) -> Vec<(String, String)> {
        let mut exits: Vec<(String, String)> = Vec::new();
        let mut positions = self.open.write();

        for pos in positions.iter_mut() {
            let is_long = pos.side == "BUY";
            let price = pos.current_price;

            // --- 1. Stop-loss ------------------------------------------------
            let sl_hit = if is_long {
                price <= pos.stop_loss
            } else {
                price >= pos.stop_loss
            };
            if sl_hit {
                exits.push((pos.id.clone(), "StopLoss".to_string()));
                continue;
            }

            // --- 2. Take-profit 2 (full close) ------------------------------
            let tp2_hit = if is_long {
                price >= pos.take_profit_2
            } else {
                price <= pos.take_profit_2
            };
            if tp2_hit {
                exits.push((pos.id.clone(), "TakeProfit2".to_string()));
                continue;
            }

            // --- 3. Take-profit 1 (partial 60 %) ----------------------------
            if pos.status == PositionStatus::Open {
                let tp1_hit = if is_long {
                    price >= pos.take_profit_1
                } else {
                    price <= pos.take_profit_1
                };
                if tp1_hit {
                    let close_qty = pos.quantity * TP1_CLOSE_FRACTION;
                    let direction = if is_long { 1.0 } else { -1.0 };
                    let partial_pnl = direction * (price - pos.entry_price) * close_qty;

                    pos.quantity -= close_qty;
                    pos.realized_pnl += partial_pnl;
                    pos.status = PositionStatus::PartialTP1;

                    info!(
                        id = %pos.id,
                        close_qty,
                        remaining_qty = pos.quantity,
                        partial_pnl,
                        "TP1 partial close executed"
                    );
                    // Do NOT push into exits — position stays open with reduced qty.
                    continue;
                }
            }

            // --- 4. Trailing stop --------------------------------------------
            if let Some(trail) = pos.trailing_stop {
                let trail_hit = if is_long {
                    price <= trail
                } else {
                    price >= trail
                };
                if trail_hit {
                    exits.push((pos.id.clone(), "TrailingStop".to_string()));
                    continue;
                }
            }
        }

        exits
    }

    // -------------------------------------------------------------------------
    // Close a position
    // -------------------------------------------------------------------------

    /// Close a position by `id` and move it to the closed list.
    ///
    /// Returns the total realised PnL (partial + final) if the position was
    /// found, or `None` if no matching open position exists.
    pub fn close_position(
        &self,
        id: &str,
        reason: &str,
        close_price: f64,
    ) -> Option<f64> {
        let mut open = self.open.write();
        let idx = open.iter().position(|p| p.id == id)?;
        let mut pos = open.remove(idx);

        let direction = if pos.side == "BUY" { 1.0 } else { -1.0 };
        let final_pnl = direction * (close_price - pos.entry_price) * pos.quantity;
        pos.realized_pnl += final_pnl;
        pos.current_price = close_price;
        pos.unrealized_pnl = 0.0;
        pos.unrealized_pnl_pct = 0.0;
        pos.status = PositionStatus::Closed;
        pos.closed_at = Some(Utc::now().to_rfc3339());
        pos.close_reason = Some(reason.to_string());
        pos.quantity = 0.0;

        let total_pnl = pos.realized_pnl;

        info!(
            id,
            reason,
            close_price,
            realized_pnl = total_pnl,
            "position closed"
        );

        self.closed.write().push(pos);
        Some(total_pnl)
    }

    // -------------------------------------------------------------------------
    // Queries
    // -------------------------------------------------------------------------

    /// Return a snapshot of all currently open positions.
    pub fn get_open_positions(&self) -> Vec<Position> {
        self.open.read().clone()
    }

    /// Return the most recent `count` closed positions (newest first).
    pub fn get_closed_positions(&self, count: usize) -> Vec<Position> {
        let closed = self.closed.read();
        closed.iter().rev().take(count).cloned().collect()
    }
}

impl Default for PositionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for PositionManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let open_count = self.open.read().len();
        let closed_count = self.closed.read().len();
        f.debug_struct("PositionManager")
            .field("open_positions", &open_count)
            .field("closed_positions", &closed_count)
            .finish()
    }
}
