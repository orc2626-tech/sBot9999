// =============================================================================
// Exit Management Module
// =============================================================================
//
// Implements the Triple Barrier exit strategy and a background exit monitor
// loop that evaluates all open positions every 5 seconds.

pub mod triple_barrier;
pub mod monitor;
