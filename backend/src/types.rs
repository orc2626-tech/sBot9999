// =============================================================================
// Shared types used across the Aurora trading engine
// =============================================================================

use serde::{Deserialize, Serialize};

/// Balance snapshot for a single asset from the exchange.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceInfo {
    pub asset: String,
    #[serde(default)]
    pub free: f64,
    #[serde(default)]
    pub locked: f64,
}

/// Whether the engine is actively trading, paused, or killed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TradingMode {
    Live,
    Paused,
    Killed,
}

impl Default for TradingMode {
    fn default() -> Self {
        Self::Paused
    }
}

impl std::fmt::Display for TradingMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Live => write!(f, "Live"),
            Self::Paused => write!(f, "Paused"),
            Self::Killed => write!(f, "Killed"),
        }
    }
}

/// Whether we are running against real funds or simulated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccountMode {
    Demo,
    Live,
}

impl Default for AccountMode {
    fn default() -> Self {
        Self::Demo
    }
}

impl std::fmt::Display for AccountMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Demo => write!(f, "Demo"),
            Self::Live => write!(f, "Live"),
        }
    }
}
