// =============================================================================
// Execution Engine — routes trade proposals through risk checks and the
// exchange client, with full demo-mode simulation support
// =============================================================================

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::binance::client::BinanceClient;
use crate::position_engine::PositionManager;
use crate::risk::RiskEngine;

// ---------------------------------------------------------------------------
// Result type
// ---------------------------------------------------------------------------

/// Outcome of an execution attempt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionResult {
    /// Order was placed on the exchange (live mode).
    Placed(serde_json::Value),
    /// Order was simulated locally (demo mode).
    Simulated(String),
    /// Order was blocked by the risk engine.
    Blocked(String),
    /// An error occurred during execution.
    Error(String),
}

impl std::fmt::Display for ExecutionResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Placed(v) => write!(f, "Placed({})", v),
            Self::Simulated(msg) => write!(f, "Simulated({msg})"),
            Self::Blocked(reason) => write!(f, "Blocked({reason})"),
            Self::Error(err) => write!(f, "Error({err})"),
        }
    }
}

// ---------------------------------------------------------------------------
// Engine
// ---------------------------------------------------------------------------

/// The execution engine ties together the Binance client, position manager,
/// and risk engine to execute (or simulate) trade proposals.
pub struct ExecutionEngine {
    pub client: Arc<BinanceClient>,
    pub position_manager: Arc<PositionManager>,
    pub risk_engine: Arc<RiskEngine>,
}

impl ExecutionEngine {
    /// Create a new execution engine.
    pub fn new(
        client: Arc<BinanceClient>,
        position_manager: Arc<PositionManager>,
        risk_engine: Arc<RiskEngine>,
    ) -> Self {
        Self {
            client,
            position_manager,
            risk_engine,
        }
    }

    /// Execute a trade proposal.
    ///
    /// In **demo mode** the order is simulated locally: no request reaches
    /// Binance, and a synthetic fill is created.
    ///
    /// In **live mode** the risk engine is consulted first; if all circuit
    /// breakers are clear the order is forwarded to Binance via the REST
    /// client.
    ///
    /// Regardless of mode, a new position is opened in the position manager
    /// upon successful (or simulated) fill.
    pub async fn execute_proposal(
        &self,
        symbol: &str,
        side: &str,
        price: f64,
        quantity: f64,
        stop_loss: f64,
        take_profit_1: f64,
        take_profit_2: f64,
        is_demo: bool,
    ) -> ExecutionResult {
        info!(
            symbol,
            side,
            price,
            quantity,
            stop_loss,
            take_profit_1,
            take_profit_2,
            is_demo,
            "execution proposal received"
        );

        // -----------------------------------------------------------------
        // Risk pre-check (applies to both demo and live)
        // -----------------------------------------------------------------
        let (allowed, reason) = self.risk_engine.can_trade();
        if !allowed {
            let msg = reason.unwrap_or_else(|| "unknown risk violation".to_string());
            warn!(symbol, side, reason = %msg, "execution blocked by risk engine");
            return ExecutionResult::Blocked(msg);
        }

        if is_demo {
            return self.execute_demo(symbol, side, price, quantity, stop_loss, take_profit_1, take_profit_2);
        }

        self.execute_live(symbol, side, price, quantity, stop_loss, take_profit_1, take_profit_2).await
    }

    // -------------------------------------------------------------------------
    // Demo execution
    // -------------------------------------------------------------------------

    fn execute_demo(
        &self,
        symbol: &str,
        side: &str,
        price: f64,
        quantity: f64,
        stop_loss: f64,
        take_profit_1: f64,
        take_profit_2: f64,
    ) -> ExecutionResult {
        let sim_order_id = Uuid::new_v4().to_string();

        // Open position in the manager.
        let position_id = self.position_manager.open_position(
            symbol,
            side,
            price,
            quantity,
            stop_loss,
            take_profit_1,
            take_profit_2,
        );

        let msg = format!(
            "Demo fill: symbol={symbol} side={side} price={price} qty={quantity} \
             position_id={position_id} sim_order_id={sim_order_id}"
        );
        info!("{}", msg);
        ExecutionResult::Simulated(msg)
    }

    // -------------------------------------------------------------------------
    // Live execution
    // -------------------------------------------------------------------------

    async fn execute_live(
        &self,
        symbol: &str,
        side: &str,
        price: f64,
        quantity: f64,
        stop_loss: f64,
        take_profit_1: f64,
        take_profit_2: f64,
    ) -> ExecutionResult {
        debug!(symbol, side, price, quantity, "sending live order to Binance");

        let result = self
            .client
            .place_order(
                symbol,
                side,
                "LIMIT",
                quantity,
                Some(price),
                Some("GTC"),
                None,
            )
            .await;

        match result {
            Ok(order_response) => {
                // Open position in the manager upon successful placement.
                let position_id = self.position_manager.open_position(
                    symbol,
                    side,
                    price,
                    quantity,
                    stop_loss,
                    take_profit_1,
                    take_profit_2,
                );

                info!(
                    symbol,
                    side,
                    position_id = %position_id,
                    order_id = %order_response.get("orderId").and_then(|v| v.as_u64()).unwrap_or(0),
                    "live order placed and position created"
                );

                ExecutionResult::Placed(order_response)
            }
            Err(e) => {
                warn!(
                    symbol,
                    side,
                    error = %e,
                    "live order placement failed"
                );
                ExecutionResult::Error(format!("Order placement failed: {e}"))
            }
        }
    }
}

impl std::fmt::Debug for ExecutionEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExecutionEngine")
            .field("client", &"<BinanceClient>")
            .field("position_manager", &self.position_manager)
            .field("risk_engine", &self.risk_engine)
            .finish()
    }
}
