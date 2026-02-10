// =============================================================================
// Central Application State — Aurora Trading Engine
// =============================================================================
//
// The single source of truth for the entire engine. All subsystems hold Arc
// references to their own state; AppState ties them together and provides a
// unified snapshot for the dashboard API and WebSocket feed.
//
// Thread safety:
//   - Atomic counters for lock-free version tracking.
//   - parking_lot::RwLock for all mutable shared collections.
//   - Arc wrappers for subsystem engines that manage their own interior
//     mutability.
// =============================================================================

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use chrono::Utc;
use parking_lot::RwLock;
use serde::Serialize;

use crate::decision_envelope::DecisionEnvelope;
use crate::market_data::{CandleBuffer, OrderBookManager, TradeStreamProcessor};
use crate::position_engine::{Position, PositionManager};
use crate::regime::{RegimeDetector, RegimeState};
use crate::risk::{CircuitBreakerInfo, RiskEngine, RiskState};
use crate::runtime_config::RuntimeConfig;
use crate::signals::{ScoringResult, SignalDecayManager, VPINState, WeightedScorer};
use crate::types::BalanceInfo;

// =============================================================================
// Error Record
// =============================================================================

/// A recorded error event for the dashboard error log.
#[derive(Debug, Clone, Serialize)]
pub struct ErrorRecord {
    /// Human-readable error message.
    pub message: String,
    /// Optional machine-readable error code (e.g. Binance error code).
    pub code: Option<String>,
    /// ISO 8601 timestamp.
    pub at: String,
}

// =============================================================================
// AppState
// =============================================================================

/// Maximum number of recent errors to retain.
const MAX_RECENT_ERRORS: usize = 50;
/// Maximum number of recent decisions to retain.
const MAX_RECENT_DECISIONS: usize = 100;

/// Central application state shared across all async tasks via `Arc<AppState>`.
pub struct AppState {
    // ── Version tracking ────────────────────────────────────────────────
    /// Monotonically increasing version counter. Incremented on every
    /// meaningful state mutation. The WebSocket feed uses this to detect
    /// changes and push updates.
    pub state_version: AtomicU64,

    /// WebSocket message sequence number (incremented per message sent).
    pub ws_sequence_number: AtomicU64,

    // ── Configuration ───────────────────────────────────────────────────
    pub runtime_config: Arc<RwLock<RuntimeConfig>>,

    // ── Market Data ─────────────────────────────────────────────────────
    pub candle_buffer: Arc<CandleBuffer>,
    pub trade_processors: RwLock<HashMap<String, Arc<TradeStreamProcessor>>>,
    pub orderbook_manager: Arc<OrderBookManager>,

    // ── Risk ────────────────────────────────────────────────────────────
    pub risk_engine: Arc<RiskEngine>,

    // ── Positions ───────────────────────────────────────────────────────
    pub position_manager: Arc<PositionManager>,

    // ── Regime Detection ────────────────────────────────────────────────
    pub regime_detector: Arc<RwLock<RegimeDetector>>,

    // ── Signal Pipeline ─────────────────────────────────────────────────
    pub weighted_scorer: Arc<RwLock<WeightedScorer>>,
    pub signal_decay: Arc<SignalDecayManager>,
    pub vpin_states: RwLock<HashMap<String, VPINState>>,
    pub last_scoring: RwLock<Option<ScoringResult>>,

    // ── Account / Exchange ──────────────────────────────────────────────
    pub balances: RwLock<Vec<BalanceInfo>>,

    // ── Decision Audit Trail ────────────────────────────────────────────
    pub recent_decisions: RwLock<Vec<DecisionEnvelope>>,

    // ── Error Log ───────────────────────────────────────────────────────
    pub recent_errors: RwLock<Vec<ErrorRecord>>,

    // ── Operational Status ──────────────────────────────────────────────
    pub no_go_reason: RwLock<Option<String>>,
    pub ws_user_connected: RwLock<bool>,
    pub last_ws_user_event: RwLock<std::time::Instant>,
    pub last_reconcile_ok: RwLock<Option<std::time::Instant>>,
    pub last_reconcile_error: RwLock<Option<String>>,

    // ── Futures Intelligence ────────────────────────────────────────────
    pub futures_intel: RwLock<HashMap<String, serde_json::Value>>,

    // ── Timing ──────────────────────────────────────────────────────────
    /// Instant when the engine was started. Used for uptime calculations.
    pub start_time: std::time::Instant,
}

impl AppState {
    /// Construct a new `AppState` from the given runtime configuration.
    ///
    /// All subsystems are initialised with sensible defaults derived from
    /// `config`. The returned value is typically wrapped in `Arc` immediately.
    pub fn new(config: RuntimeConfig) -> Self {
        // Pre-create trade processors for each configured symbol.
        let mut trade_processors = HashMap::new();
        for symbol in &config.symbols {
            trade_processors.insert(
                symbol.clone(),
                Arc::new(TradeStreamProcessor::new(symbol.clone())),
            );
        }

        // Derive starting capital from a sensible default. In production this
        // would be fetched from the exchange balance.
        let starting_capital = 1000.0;

        // Construct the risk engine from the config's risk parameters.
        let risk_engine = RiskEngine::new(
            starting_capital,
            config.max_daily_loss_pct / 100.0, // convert pct to fraction
            config.max_consecutive_losses,
            0.05, // max drawdown pct as fraction (5%)
            config.max_trades_per_day,
        );

        Self {
            state_version: AtomicU64::new(1),
            ws_sequence_number: AtomicU64::new(0),

            runtime_config: Arc::new(RwLock::new(config)),
            candle_buffer: Arc::new(CandleBuffer::new(500)),
            trade_processors: RwLock::new(trade_processors),
            orderbook_manager: Arc::new(OrderBookManager::new()),

            risk_engine: Arc::new(risk_engine),
            position_manager: Arc::new(PositionManager::new()),

            regime_detector: Arc::new(RwLock::new(RegimeDetector::default())),
            weighted_scorer: Arc::new(RwLock::new(WeightedScorer::default())),
            signal_decay: Arc::new(SignalDecayManager::default()),
            vpin_states: RwLock::new(HashMap::new()),
            last_scoring: RwLock::new(None),

            balances: RwLock::new(Vec::new()),
            recent_decisions: RwLock::new(Vec::new()),
            recent_errors: RwLock::new(Vec::new()),

            no_go_reason: RwLock::new(None),
            ws_user_connected: RwLock::new(false),
            last_ws_user_event: RwLock::new(std::time::Instant::now()),
            last_reconcile_ok: RwLock::new(None),
            last_reconcile_error: RwLock::new(None),

            futures_intel: RwLock::new(HashMap::new()),
            start_time: std::time::Instant::now(),
        }
    }

    // ── Version Management ──────────────────────────────────────────────

    /// Atomically increment the state version. Call this after every
    /// meaningful mutation to signal WebSocket clients that fresh data is
    /// available.
    pub fn increment_version(&self) -> u64 {
        self.state_version.fetch_add(1, Ordering::SeqCst)
    }

    /// Read the current state version without modifying it.
    pub fn current_state_version(&self) -> u64 {
        self.state_version.load(Ordering::SeqCst)
    }

    // ── Error Logging ───────────────────────────────────────────────────

    /// Record an error message. The ring buffer is capped at
    /// [`MAX_RECENT_ERRORS`]; oldest entries are evicted when the limit is
    /// reached.
    pub fn push_error(&self, msg: String) {
        self.push_error_with_code(msg, None);
    }

    /// Record an error with an optional machine-readable code.
    pub fn push_error_with_code(&self, msg: String, code: Option<String>) {
        let record = ErrorRecord {
            message: msg,
            code,
            at: Utc::now().to_rfc3339(),
        };

        let mut errors = self.recent_errors.write();
        errors.push(record);
        while errors.len() > MAX_RECENT_ERRORS {
            errors.remove(0);
        }

        self.increment_version();
    }

    // ── Decision Audit ──────────────────────────────────────────────────

    /// Record a decision envelope. The ring buffer is capped at
    /// [`MAX_RECENT_DECISIONS`]; oldest entries are evicted when the limit
    /// is reached.
    pub fn push_decision(&self, envelope: DecisionEnvelope) {
        let mut decisions = self.recent_decisions.write();
        decisions.push(envelope);
        while decisions.len() > MAX_RECENT_DECISIONS {
            decisions.remove(0);
        }

        self.increment_version();
    }

    // ── Snapshot Builder ────────────────────────────────────────────────

    /// Build a complete, serialisable snapshot of the entire engine state.
    ///
    /// This is the payload sent to the dashboard via the REST
    /// `GET /api/v1/state` endpoint and the WebSocket push feed.
    pub fn build_snapshot(&self) -> StateSnapshot {
        let now = Utc::now();
        let config = self.runtime_config.read();
        let version = self.current_state_version();

        // ── Truth header ────────────────────────────────────────────
        let ws_user_event_age_ms = self
            .last_ws_user_event
            .read()
            .elapsed()
            .as_millis() as u64;

        let reconcile_last_ok_age_s = self.last_reconcile_ok.read().map(|t| t.elapsed().as_secs());

        // Get risk state to extract the risk mode.
        let risk_state = self.risk_engine.get_state();

        let truth = TruthHeader {
            ws_mode: "combined".to_string(),
            ws_user_connected: *self.ws_user_connected.read(),
            last_ws_user_event_age_ms: ws_user_event_age_ms,
            reconcile_last_ok_age_s,
            reconcile_last_error: self.last_reconcile_error.read().clone(),
            no_go_reason: self.no_go_reason.read().clone(),
            state_version: version,
            ws_sequence_number: self.ws_sequence_number.load(Ordering::Relaxed),
            trading_mode: config.trading_mode.to_string(),
            risk_mode: risk_state.risk_mode.clone(),
            server_time: now.timestamp_millis(),
        };

        // ── Positions ───────────────────────────────────────────────
        let positions = self.position_manager.get_open_positions();

        // ── Decisions ───────────────────────────────────────────────
        let recent_decisions = self.recent_decisions.read().clone();

        // ── Risk ────────────────────────────────────────────────────
        let risk = RiskSnapshot {
            risk_mode: risk_state.risk_mode.clone(),
            daily_pnl: Some(risk_state.daily_pnl),
            daily_pnl_pct: Some(risk_state.daily_pnl_pct),
            remaining_daily_loss_pct: Some(risk_state.remaining_daily_loss_pct),
            circuit_breakers: Some(risk_state.circuit_breakers.clone()),
        };

        // ── Runtime config summary ──────────────────────────────────
        let runtime_config_summary = RuntimeConfigSummary {
            trading_mode: config.trading_mode.to_string(),
            account_mode: Some(config.account_mode.to_string()),
            symbols: Some(config.symbols.clone()),
            max_concurrent_positions: Some(config.max_concurrent_positions as u32),
            max_daily_loss_pct: Some(config.max_daily_loss_pct),
            max_consecutive_losses: Some(config.max_consecutive_losses),
            max_trades_per_day: Some(config.max_trades_per_day),
        };

        // ── Balances ────────────────────────────────────────────────
        let balances = self.balances.read().clone();

        // ── Errors ──────────────────────────────────────────────────
        let recent_errors = self.recent_errors.read().clone();

        // ── Market data ─────────────────────────────────────────────
        let market_data = self.build_market_data_snapshot(&config.symbols);

        // ── Regime ──────────────────────────────────────────────────
        let regime = self.regime_detector.read().current_regime().map(|rs| {
            RegimeSnapshot {
                regime: rs.regime.to_string(),
                adx: Some(rs.adx),
                bbw: Some(rs.bbw),
                hurst: Some(rs.hurst),
                entropy: Some(rs.entropy),
                regime_age_seconds: Some(rs.regime_age_secs),
            }
        });

        // ── Scoring ─────────────────────────────────────────────────
        let scoring = self.last_scoring.read().clone();

        // ── VPIN ────────────────────────────────────────────────────
        let vpin = {
            let states = self.vpin_states.read();
            if states.is_empty() {
                None
            } else {
                Some(states.clone())
            }
        };

        // ── Futures intel ───────────────────────────────────────────
        let futures_intel = {
            let intel = self.futures_intel.read();
            if intel.is_empty() {
                None
            } else {
                Some(intel.clone())
            }
        };

        // ── Journal stats ───────────────────────────────────────────
        let closed_positions = self.position_manager.get_closed_positions(500);
        let journal_stats = if !closed_positions.is_empty() {
            let total_trades = closed_positions.len();
            let wins = closed_positions
                .iter()
                .filter(|p| p.realized_pnl > 0.0)
                .count();
            let win_rate = wins as f64 / total_trades as f64;
            let total_net_pnl: f64 = closed_positions.iter().map(|p| p.realized_pnl).sum();
            let gross_profit: f64 = closed_positions
                .iter()
                .map(|p| p.realized_pnl)
                .filter(|&pnl| pnl > 0.0)
                .sum();
            let gross_loss: f64 = closed_positions
                .iter()
                .map(|p| p.realized_pnl)
                .filter(|&pnl| pnl < 0.0)
                .map(|pnl| pnl.abs())
                .sum();
            let profit_factor = if gross_loss > 0.0 {
                gross_profit / gross_loss
            } else if gross_profit > 0.0 {
                f64::INFINITY
            } else {
                0.0
            };

            Some(JournalStats {
                total_trades,
                win_rate,
                total_net_pnl,
                profit_factor,
            })
        } else {
            None
        };

        // ── Heartbeat ───────────────────────────────────────────────
        let last_heartbeat_age_s = Some(ws_user_event_age_ms / 1000);

        // ── Feature flags ───────────────────────────────────────────
        let feature_flags = Some(FeatureFlagsSnapshot {
            htf_gate: config.enable_htf_gate,
            score_momentum: config.enable_score_momentum,
            ofip: config.enable_ofip,
            adaptive_threshold: config.enable_adaptive_threshold,
            entropy_graduated: config.enable_entropy_graduated,
            cusum: config.enable_cusum,
            absorption: config.enable_absorption,
            entropy_valley: config.enable_entropy_valley,
        });

        StateSnapshot {
            state_version: version,
            server_time: now.timestamp_millis(),
            truth,
            positions,
            recent_decisions,
            risk,
            runtime_config: runtime_config_summary,
            balances: Some(balances),
            recent_errors: Some(recent_errors),
            market_data: Some(market_data),
            regime,
            scoring,
            vpin,
            futures_intel,
            journal_stats,
            last_heartbeat_age_s,
            feature_flags,
        }
    }

    /// Build market data snapshots for each tracked symbol.
    fn build_market_data_snapshot(&self, symbols: &[String]) -> MarketDataSnapshot {
        let mut symbol_data = HashMap::new();
        let trade_procs = self.trade_processors.read();

        for symbol in symbols {
            let last_price = trade_procs
                .get(symbol)
                .map(|tp| tp.last_price())
                .unwrap_or(0.0);

            let cvd = trade_procs
                .get(symbol)
                .map(|tp| tp.cvd())
                .unwrap_or(0.0);

            let buy_volume_ratio = trade_procs
                .get(symbol)
                .map(|tp| tp.buy_volume_ratio())
                .unwrap_or(0.5);

            let orderbook_imbalance = self
                .orderbook_manager
                .imbalance(symbol)
                .unwrap_or(0.0);

            let spread_bps = self.orderbook_manager.spread_bps(symbol);

            symbol_data.insert(
                symbol.clone(),
                SymbolMarketData {
                    last_price,
                    rsi_14: None,
                    ema_9: None,
                    ema_21: None,
                    ema_55: None,
                    adx: None,
                    atr_14: None,
                    bollinger_width: None,
                    roc_14: None,
                    spread_bps,
                    cvd,
                    orderbook_imbalance,
                    buy_volume_ratio,
                },
            );
        }

        MarketDataSnapshot {
            symbols: symbol_data,
        }
    }
}

// =============================================================================
// Serialisable snapshot types (match the TypeScript StateSnapshot interface)
// =============================================================================

/// Full engine state snapshot sent to the dashboard.
#[derive(Debug, Clone, Serialize)]
pub struct StateSnapshot {
    pub state_version: u64,
    pub server_time: i64,
    pub truth: TruthHeader,
    pub positions: Vec<Position>,
    pub recent_decisions: Vec<DecisionEnvelope>,
    pub risk: RiskSnapshot,
    pub runtime_config: RuntimeConfigSummary,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub balances: Option<Vec<BalanceInfo>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub recent_errors: Option<Vec<ErrorRecord>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub market_data: Option<MarketDataSnapshot>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub regime: Option<RegimeSnapshot>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub scoring: Option<ScoringResult>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub vpin: Option<HashMap<String, VPINState>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub futures_intel: Option<HashMap<String, serde_json::Value>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub journal_stats: Option<JournalStats>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_heartbeat_age_s: Option<u64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub feature_flags: Option<FeatureFlagsSnapshot>,
}

/// Operational truth header — the dashboard's primary status banner.
#[derive(Debug, Clone, Serialize)]
pub struct TruthHeader {
    pub ws_mode: String,
    pub ws_user_connected: bool,
    pub last_ws_user_event_age_ms: u64,
    pub reconcile_last_ok_age_s: Option<u64>,
    pub reconcile_last_error: Option<String>,
    pub no_go_reason: Option<String>,
    pub state_version: u64,
    pub ws_sequence_number: u64,
    pub trading_mode: String,
    pub risk_mode: String,
    pub server_time: i64,
}

/// Risk engine snapshot for the dashboard.
#[derive(Debug, Clone, Serialize)]
pub struct RiskSnapshot {
    pub risk_mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub daily_pnl: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub daily_pnl_pct: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remaining_daily_loss_pct: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub circuit_breakers: Option<Vec<CircuitBreakerInfo>>,
}

/// Summary of runtime config for the dashboard.
#[derive(Debug, Clone, Serialize)]
pub struct RuntimeConfigSummary {
    pub trading_mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbols: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_concurrent_positions: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_daily_loss_pct: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_consecutive_losses: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_trades_per_day: Option<u32>,
}

/// Market data for all tracked symbols.
#[derive(Debug, Clone, Serialize)]
pub struct MarketDataSnapshot {
    pub symbols: HashMap<String, SymbolMarketData>,
}

/// Per-symbol market data indicators.
#[derive(Debug, Clone, Serialize)]
pub struct SymbolMarketData {
    pub last_price: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rsi_14: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ema_9: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ema_21: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ema_55: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adx: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub atr_14: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bollinger_width: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roc_14: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spread_bps: Option<f64>,
    pub cvd: f64,
    pub orderbook_imbalance: f64,
    pub buy_volume_ratio: f64,
}

/// Regime detection snapshot.
#[derive(Debug, Clone, Serialize)]
pub struct RegimeSnapshot {
    pub regime: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adx: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bbw: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hurst: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entropy: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub regime_age_seconds: Option<f64>,
}

/// Feature flags snapshot for the dashboard.
#[derive(Debug, Clone, Serialize)]
pub struct FeatureFlagsSnapshot {
    pub htf_gate: bool,
    pub score_momentum: bool,
    pub ofip: bool,
    pub adaptive_threshold: bool,
    pub entropy_graduated: bool,
    pub cusum: bool,
    pub absorption: bool,
    pub entropy_valley: bool,
}

/// Trade journal aggregate statistics.
#[derive(Debug, Clone, Serialize)]
pub struct JournalStats {
    pub total_trades: usize,
    pub win_rate: f64,
    pub total_net_pnl: f64,
    pub profit_factor: f64,
}
