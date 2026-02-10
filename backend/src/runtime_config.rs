// =============================================================================
// Runtime Configuration — Hot-reloadable engine settings with atomic save
// =============================================================================
//
// Central configuration hub for the Aurora trading engine.  Every tunable
// parameter lives here so that the engine can be reconfigured at runtime
// without a restart.
//
// Persistence uses an atomic tmp + rename pattern to prevent corruption on
// crash.  All fields carry `#[serde(default)]` so that adding new fields
// never breaks loading an older config file.
//
// =============================================================================

use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::types::{AccountMode, TradingMode};

// =============================================================================
// Default-value helpers (required by serde `default = "..."` attribute)
// =============================================================================

fn default_true() -> bool {
    true
}

fn default_symbols() -> Vec<String> {
    vec![
        "BTCUSDT".to_string(),
        "ETHUSDT".to_string(),
        "BNBUSDT".to_string(),
        "XRPUSDT".to_string(),
        "SOLUSDT".to_string(),
    ]
}

fn default_max_concurrent_positions() -> u32 {
    3
}

fn default_max_daily_loss_pct() -> f64 {
    3.0
}

fn default_max_consecutive_losses() -> u32 {
    5
}

fn default_max_trades_per_day() -> u32 {
    50
}

fn default_sl_atr_multiplier() -> f64 {
    1.5
}

fn default_tp1_atr_multiplier() -> f64 {
    2.5
}

fn default_tp2_atr_multiplier() -> f64 {
    4.0
}

fn default_min_sl_pct() -> f64 {
    0.4
}

fn default_min_tp1_pct() -> f64 {
    0.6
}

fn default_min_tp2_pct() -> f64 {
    1.0
}

fn default_base_position_pct() -> f64 {
    2.0
}

// =============================================================================
// StrategyParams
// =============================================================================

/// Tunable parameters for the core strategy (SL/TP sizing, position sizing).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyParams {
    /// ATR multiplier for stop-loss distance.
    #[serde(default = "default_sl_atr_multiplier")]
    pub sl_atr_multiplier: f64,

    /// ATR multiplier for take-profit-1 distance.
    #[serde(default = "default_tp1_atr_multiplier")]
    pub tp1_atr_multiplier: f64,

    /// ATR multiplier for take-profit-2 distance.
    #[serde(default = "default_tp2_atr_multiplier")]
    pub tp2_atr_multiplier: f64,

    /// Minimum stop-loss as a percentage of entry price.
    /// CRITICAL FLOOR: must be >= 0.4%.
    #[serde(default = "default_min_sl_pct")]
    pub min_sl_pct: f64,

    /// Minimum take-profit-1 as a percentage of entry price.
    /// CRITICAL FLOOR: must be >= 0.6%.
    #[serde(default = "default_min_tp1_pct")]
    pub min_tp1_pct: f64,

    /// Minimum take-profit-2 as a percentage of entry price.
    /// CRITICAL FLOOR: must be >= 1.0%.
    #[serde(default = "default_min_tp2_pct")]
    pub min_tp2_pct: f64,

    /// Base position size as a percentage of available capital.
    #[serde(default = "default_base_position_pct")]
    pub base_position_pct: f64,
}

impl Default for StrategyParams {
    fn default() -> Self {
        Self {
            sl_atr_multiplier: default_sl_atr_multiplier(),
            tp1_atr_multiplier: default_tp1_atr_multiplier(),
            tp2_atr_multiplier: default_tp2_atr_multiplier(),
            min_sl_pct: default_min_sl_pct(),
            min_tp1_pct: default_min_tp1_pct(),
            min_tp2_pct: default_min_tp2_pct(),
            base_position_pct: default_base_position_pct(),
        }
    }
}

// =============================================================================
// RuntimeConfig
// =============================================================================

/// Top-level runtime configuration for the Aurora engine.
///
/// Every field has a serde default so that older JSON files missing new fields
/// will still deserialise correctly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    // --- Operational modes ---------------------------------------------------

    /// Current trading mode: Live, Paused, or Killed.
    #[serde(default)]
    pub trading_mode: TradingMode,

    /// Whether running against real funds or simulated: Demo or Live.
    #[serde(default)]
    pub account_mode: AccountMode,

    // --- Symbols & limits ---------------------------------------------------

    /// Symbols the engine is watching and trading.
    #[serde(default = "default_symbols")]
    pub symbols: Vec<String>,

    /// Maximum number of concurrent open positions.
    #[serde(default = "default_max_concurrent_positions")]
    pub max_concurrent_positions: u32,

    /// Maximum cumulative daily loss allowed as a percentage of starting
    /// capital (e.g. 3.0 means 3 %).
    #[serde(default = "default_max_daily_loss_pct")]
    pub max_daily_loss_pct: f64,

    /// Maximum consecutive losing trades before the circuit breaker trips.
    #[serde(default = "default_max_consecutive_losses")]
    pub max_consecutive_losses: u32,

    /// Maximum total trades per day.
    #[serde(default = "default_max_trades_per_day")]
    pub max_trades_per_day: u32,

    // --- Feature flags (smart filters) --------------------------------------
    // All default to `true` so that new flags are active by default.

    /// Higher Time Frame (15M + 1H) trend gate.
    #[serde(default = "default_true")]
    pub enable_htf_gate: bool,

    /// Rolling score momentum filter.
    #[serde(default = "default_true")]
    pub enable_score_momentum: bool,

    /// Order Flow Imbalance Persistence filter.
    #[serde(default = "default_true")]
    pub enable_ofip: bool,

    /// Dynamic BUY threshold adjusted by market regime.
    #[serde(default = "default_true")]
    pub enable_adaptive_threshold: bool,

    /// Graduated entropy-based position sizing.
    #[serde(default = "default_true")]
    pub enable_entropy_graduated: bool,

    /// CUSUM structural break detection.
    #[serde(default = "default_true")]
    pub enable_cusum: bool,

    /// Institutional absorption detection.
    #[serde(default = "default_true")]
    pub enable_absorption: bool,

    /// Entropy valley confidence boost.
    #[serde(default = "default_true")]
    pub enable_entropy_valley: bool,

    // --- Strategy parameters ------------------------------------------------

    /// Tunable strategy parameters (SL/TP multipliers, position sizing).
    #[serde(default)]
    pub strategy_params: StrategyParams,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            trading_mode: TradingMode::Paused,
            account_mode: AccountMode::Demo,
            symbols: default_symbols(),
            max_concurrent_positions: default_max_concurrent_positions(),
            max_daily_loss_pct: default_max_daily_loss_pct(),
            max_consecutive_losses: default_max_consecutive_losses(),
            max_trades_per_day: default_max_trades_per_day(),
            enable_htf_gate: true,
            enable_score_momentum: true,
            enable_ofip: true,
            enable_adaptive_threshold: true,
            enable_entropy_graduated: true,
            enable_cusum: true,
            enable_absorption: true,
            enable_entropy_valley: true,
            strategy_params: StrategyParams::default(),
        }
    }
}

impl RuntimeConfig {
    /// Load configuration from a JSON file at `path`.
    ///
    /// If the file does not exist, returns an error so the caller can fall
    /// back to defaults with a warning.
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();

        let content = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read runtime config from {}", path.display()))?;

        let config: Self = serde_json::from_str(&content)
            .with_context(|| format!("failed to parse runtime config from {}", path.display()))?;

        info!(
            path = %path.display(),
            symbols = ?config.symbols,
            trading_mode = %config.trading_mode,
            "runtime config loaded"
        );

        Ok(config)
    }

    /// Persist the current configuration to `path` using an atomic write
    /// (write to `.tmp`, then rename).
    ///
    /// This prevents corruption if the process crashes mid-write.
    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();

        let content = serde_json::to_string_pretty(self)
            .context("failed to serialise runtime config to JSON")?;

        // Atomic write: write to a temporary sibling file, then rename.
        let tmp_path = path.with_extension("json.tmp");

        std::fs::write(&tmp_path, &content)
            .with_context(|| format!("failed to write tmp config to {}", tmp_path.display()))?;

        std::fs::rename(&tmp_path, path)
            .with_context(|| format!("failed to rename tmp config to {}", path.display()))?;

        info!(path = %path.display(), "runtime config saved (atomic)");
        Ok(())
    }
}

// =============================================================================
// Tests
// =============================================================================
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_expected_values() {
        let cfg = RuntimeConfig::default();
        assert_eq!(cfg.trading_mode, TradingMode::Paused);
        assert_eq!(cfg.account_mode, AccountMode::Demo);
        assert_eq!(cfg.symbols.len(), 5);
        assert_eq!(cfg.symbols[0], "BTCUSDT");
        assert_eq!(cfg.symbols[4], "SOLUSDT");
        assert_eq!(cfg.max_concurrent_positions, 3);
        assert!(cfg.enable_htf_gate);
        assert!(cfg.enable_cusum);
        assert!(cfg.enable_absorption);
        assert!(cfg.enable_entropy_valley);
        assert!((cfg.strategy_params.min_sl_pct - 0.4).abs() < f64::EPSILON);
        assert!((cfg.strategy_params.min_tp1_pct - 0.6).abs() < f64::EPSILON);
        assert!((cfg.strategy_params.min_tp2_pct - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn deserialise_empty_json_uses_defaults() {
        let cfg: RuntimeConfig = serde_json::from_str("{}").unwrap();
        assert_eq!(cfg.trading_mode, TradingMode::Paused);
        assert_eq!(cfg.account_mode, AccountMode::Demo);
        assert!(cfg.enable_htf_gate);
        assert!(cfg.enable_score_momentum);
        assert_eq!(cfg.max_consecutive_losses, 5);
    }

    #[test]
    fn deserialise_partial_json_fills_defaults() {
        let json = r#"{ "trading_mode": "Live", "symbols": ["ETHUSDT"] }"#;
        let cfg: RuntimeConfig = serde_json::from_str(json).unwrap();
        assert_eq!(cfg.trading_mode, TradingMode::Live);
        assert_eq!(cfg.symbols, vec!["ETHUSDT"]);
        assert!(cfg.enable_cusum);
        assert_eq!(cfg.max_concurrent_positions, 3);
    }

    #[test]
    fn roundtrip_serialisation() {
        let cfg = RuntimeConfig::default();
        let json = serde_json::to_string(&cfg).unwrap();
        let cfg2: RuntimeConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(cfg.symbols, cfg2.symbols);
        assert_eq!(cfg.max_concurrent_positions, cfg2.max_concurrent_positions);
        assert_eq!(cfg.trading_mode, cfg2.trading_mode);
    }

    #[test]
    fn enum_mode_assignment_compatible() {
        // Verify that trading_mode and account_mode can be assigned from
        // enum variants, matching the pattern in main.rs.
        let mut cfg = RuntimeConfig::default();
        cfg.trading_mode = TradingMode::Paused;
        cfg.account_mode = AccountMode::Demo;
        assert_eq!(cfg.trading_mode, TradingMode::Paused);
        assert_eq!(cfg.account_mode, AccountMode::Demo);
    }
}
