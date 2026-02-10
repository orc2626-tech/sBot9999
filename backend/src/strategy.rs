// =============================================================================
// Strategy Engine — The Brain of Aurora
// =============================================================================
//
// Evaluates each symbol on every 5M candle close and produces trade Proposals.
//
// Pipeline:
//   1. Compute all indicators (EMA, RSI, ADX, Bollinger, ATR, ROC)
//   2. Detect market regime
//   3. Build signal inputs
//   4. Run weighted ensemble scorer
//   5. Apply insurance gates
//   6. Apply smart filters
//   7. Compute SL/TP using **5M ATR** (CRITICAL: never use 1M ATR)
//   8. Enforce minimum floors: SL >= 0.4%, TP1 >= 0.6%, TP2 >= 1.0%
//   9. Output DecisionEnvelope + optional Proposal
// =============================================================================

use std::sync::Arc;
use tracing::{debug, info};

use crate::app_state::AppState;
use crate::decision_envelope::DecisionEnvelope;
use crate::indicators::atr::calculate_atr;
use crate::indicators::ema::calculate_ema;
use crate::indicators::rsi::calculate_rsi;
use crate::market_data::CandleKey;
use crate::signals::SignalInput;
use crate::trade_insurance::InsuranceGate;

// =============================================================================
// Trade Proposal
// =============================================================================

/// A fully validated trade proposal ready for execution.
#[derive(Debug, Clone)]
pub struct TradeProposal {
    pub symbol: String,
    pub side: String,
    pub entry_price: f64,
    pub quantity: f64,
    pub stop_loss: f64,
    pub take_profit_1: f64,
    pub take_profit_2: f64,
    pub confidence: f64,
    pub regime: String,
    pub score: f64,
}

// =============================================================================
// Strategy Engine
// =============================================================================

pub struct StrategyEngine;

impl StrategyEngine {
    /// Evaluate a single symbol and return an optional trade proposal with
    /// its decision envelope.
    pub fn evaluate_symbol(
        state: &Arc<AppState>,
        symbol: &str,
    ) -> (DecisionEnvelope, Option<TradeProposal>) {
        let config = state.runtime_config.read().clone();
        let strategy_name = "AuroraV3";

        // ── 1. Gather 5M candles ─────────────────────────────────────────
        let key_5m = CandleKey {
            symbol: symbol.to_string(),
            interval: "5m".to_string(),
        };
        let candles_5m = state.candle_buffer.get_closed_candles(&key_5m, 100);

        if candles_5m.len() < 30 {
            let envelope = DecisionEnvelope::blocked(
                symbol,
                "BUY",
                strategy_name,
                "DataQuality",
                format!("Insufficient 5M candles: {} < 30", candles_5m.len()),
            );
            return (envelope, None);
        }

        // ── 2. Compute indicators on 5M ──────────────────────────────────
        let closes: Vec<f64> = candles_5m.iter().map(|c| c.close).collect();

        let ema_9 = calculate_ema(&closes, 9).last().copied();
        let ema_21 = calculate_ema(&closes, 21).last().copied();
        let ema_55 = calculate_ema(&closes, 55).last().copied();
        let rsi_14 = calculate_rsi(&closes, 14).last().copied();

        // CRITICAL: ATR from 5M candles ONLY (never 1M)
        let atr_14 = calculate_atr(&candles_5m, 14);

        let adx_val = crate::indicators::adx::calculate_adx(&candles_5m, 14);
        let bb = crate::indicators::bollinger::calculate_bollinger(&closes, 20, 2.0);
        let roc_14 = crate::indicators::roc::calculate_roc(&closes, 14).last().copied();

        let current_price = candles_5m.last().map(|c| c.close).unwrap_or(0.0);

        if current_price <= 0.0 || atr_14.is_none() {
            let envelope = DecisionEnvelope::blocked(
                symbol,
                "BUY",
                strategy_name,
                "DataQuality",
                "Invalid price or ATR not ready",
            );
            return (envelope, None);
        }

        let atr = atr_14.unwrap();

        // ── 3. Detect regime ─────────────────────────────────────────────
        let regime_state = state.regime_detector.read().current_regime();
        let regime_label = regime_state
            .as_ref()
            .map(|r| r.regime.to_string())
            .unwrap_or_else(|| "Ranging".to_string());

        // ── 4. Build signal inputs ───────────────────────────────────────
        let mut signals = Vec::new();

        // RSI signal
        if let Some(rsi) = rsi_14 {
            let (direction, confidence) = if rsi < 30.0 {
                (1.0, (30.0 - rsi) / 30.0)
            } else if rsi > 70.0 {
                (-1.0, (rsi - 70.0) / 30.0)
            } else {
                (0.0, 0.0)
            };
            signals.push(SignalInput {
                name: "rsi".to_string(),
                weight: 0.15,
                confidence: confidence.min(1.0),
                direction,
            });
        }

        // EMA trend alignment signal
        if let (Some(e9), Some(e21), Some(e55)) = (ema_9, ema_21, ema_55) {
            let bullish = e9 > e21 && e21 > e55 && current_price > e9;
            let bearish = e9 < e21 && e21 < e55 && current_price < e9;
            let (direction, confidence) = if bullish {
                (1.0, 0.8)
            } else if bearish {
                (-1.0, 0.8)
            } else {
                (0.0, 0.3)
            };
            signals.push(SignalInput {
                name: "ema_trend".to_string(),
                weight: 0.20,
                confidence,
                direction,
            });
        }

        // ADX signal (trend strength)
        if let Some(adx) = adx_val {
            let confidence = (adx / 50.0).min(1.0);
            signals.push(SignalInput {
                name: "adx".to_string(),
                weight: 0.15,
                confidence,
                direction: if adx > 25.0 { 1.0 } else { 0.0 },
            });
        }

        // Bollinger Band width (volatility)
        if let Some(ref bands) = bb {
            let bbw = if bands.middle > 0.0 {
                (bands.upper - bands.lower) / bands.middle * 100.0
            } else {
                0.0
            };
            let direction = if current_price < bands.lower {
                1.0
            } else if current_price > bands.upper {
                -1.0
            } else {
                0.0
            };
            signals.push(SignalInput {
                name: "bbw".to_string(),
                weight: 0.10,
                confidence: (bbw / 5.0).min(1.0),
                direction,
            });
        }

        // ROC (momentum)
        if let Some(roc) = roc_14 {
            let direction = if roc > 0.0 { 1.0 } else if roc < 0.0 { -1.0 } else { 0.0 };
            let confidence = (roc.abs() / 5.0).min(1.0);
            signals.push(SignalInput {
                name: "roc".to_string(),
                weight: 0.10,
                confidence,
                direction,
            });
        }

        // Orderbook imbalance
        if let Some(imbalance) = state.orderbook_manager.imbalance(symbol) {
            let direction = if imbalance > 0.1 {
                1.0
            } else if imbalance < -0.1 {
                -1.0
            } else {
                0.0
            };
            signals.push(SignalInput {
                name: "orderbook".to_string(),
                weight: 0.10,
                confidence: imbalance.abs().min(1.0),
                direction,
            });
        }

        // CVD (cumulative volume delta)
        {
            let trade_procs = state.trade_processors.read();
            if let Some(tp) = trade_procs.get(symbol) {
                let buy_ratio = tp.buy_volume_ratio();
                let direction = if buy_ratio > 0.55 {
                    1.0
                } else if buy_ratio < 0.45 {
                    -1.0
                } else {
                    0.0
                };
                signals.push(SignalInput {
                    name: "cvd".to_string(),
                    weight: 0.10,
                    confidence: ((buy_ratio - 0.5).abs() * 4.0).min(1.0),
                    direction,
                });
            }
        }

        // VPIN signal
        {
            let vpin_states = state.vpin_states.read();
            if let Some(vpin_state) = vpin_states.get(symbol) {
                let vpin_val = vpin_state.vpin;
                let direction = if vpin_val > 0.7 {
                    -1.0
                } else {
                    0.0
                };
                signals.push(SignalInput {
                    name: "vpin".to_string(),
                    weight: 0.10,
                    confidence: vpin_val.min(1.0),
                    direction,
                });
            }
        }

        // ── 5. Score ─────────────────────────────────────────────────────
        let scoring = state.weighted_scorer.read().score(&signals, &regime_label);

        // Store for dashboard
        *state.last_scoring.write() = Some(scoring.clone());

        debug!(
            symbol,
            score = scoring.total_score,
            decision = %scoring.decision,
            regime = %regime_label,
            "strategy scoring complete"
        );

        if scoring.decision == "HOLD" {
            let envelope = DecisionEnvelope::blocked(
                symbol,
                "HOLD",
                strategy_name,
                "Strategy",
                format!("Score {:.3} below threshold (regime: {})", scoring.total_score, regime_label),
            );
            return (envelope, None);
        }

        let side = scoring.decision.clone();

        // ── 6. Insurance gates ───────────────────────────────────────────
        let insurance_result = InsuranceGate::check_all(state, symbol, &side);
        if let Some(block_reason) = insurance_result {
            let envelope = DecisionEnvelope::blocked(
                symbol, &side, strategy_name, "Insurance", &block_reason,
            );
            return (envelope, None);
        }

        // ── 7. Smart filters ─────────────────────────────────────────────
        let smart_filter_result = crate::smart_filters::SmartFilterEngine::evaluate(
            state, symbol, &side, &regime_label, scoring.total_score,
        );
        if let Some(block_reason) = smart_filter_result {
            let envelope = DecisionEnvelope::blocked(
                symbol, &side, strategy_name, "SmartFilter", &block_reason,
            );
            return (envelope, None);
        }

        // ── 8. Compute SL/TP using 5M ATR with minimum floors ───────────
        let params = &config.strategy_params;
        let sl_distance = atr * params.sl_atr_multiplier;
        let tp1_distance = atr * params.tp1_atr_multiplier;
        let tp2_distance = atr * params.tp2_atr_multiplier;

        let min_sl = current_price * (params.min_sl_pct / 100.0);
        let min_tp1 = current_price * (params.min_tp1_pct / 100.0);
        let min_tp2 = current_price * (params.min_tp2_pct / 100.0);

        let sl_dist = sl_distance.max(min_sl);
        let tp1_dist = tp1_distance.max(min_tp1);
        let tp2_dist = tp2_distance.max(min_tp2);

        let (stop_loss, take_profit_1, take_profit_2) = if side == "BUY" {
            (
                current_price - sl_dist,
                current_price + tp1_dist,
                current_price + tp2_dist,
            )
        } else {
            (
                current_price + sl_dist,
                current_price - tp1_dist,
                current_price - tp2_dist,
            )
        };

        // ── 9. Position sizing ───────────────────────────────────────────
        let balances = state.balances.read();
        let usdt_balance = balances
            .iter()
            .find(|b| b.asset == "USDT")
            .map(|b| b.free)
            .unwrap_or(1000.0);

        let position_value = usdt_balance * (params.base_position_pct / 100.0);
        let quantity = if current_price > 0.0 {
            position_value / current_price
        } else {
            0.0
        };

        if quantity <= 0.0 {
            let envelope = DecisionEnvelope::blocked(
                symbol, &side, strategy_name, "PositionSizing", "Computed quantity is zero",
            );
            return (envelope, None);
        }

        // ── 10. Build proposal ───────────────────────────────────────────
        let proposal = TradeProposal {
            symbol: symbol.to_string(),
            side: side.clone(),
            entry_price: current_price,
            quantity,
            stop_loss,
            take_profit_1,
            take_profit_2,
            confidence: scoring.total_score.abs(),
            regime: regime_label.clone(),
            score: scoring.total_score,
        };

        let mut envelope = DecisionEnvelope::allow(symbol, &side, strategy_name);
        envelope.reason = Some(format!(
            "Score {:.3} | Regime {} | ATR {:.4} | SL {:.2} | TP1 {:.2} | TP2 {:.2}",
            scoring.total_score, regime_label, atr, stop_loss, take_profit_1, take_profit_2
        ));

        info!(
            symbol,
            side = %side,
            score = scoring.total_score,
            regime = %regime_label,
            atr,
            stop_loss,
            take_profit_1,
            take_profit_2,
            quantity,
            "trade proposal generated"
        );

        (envelope, Some(proposal))
    }
}
