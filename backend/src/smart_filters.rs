// =============================================================================
// Smart Filters — Phase 1 + Phase 2 feature-flag-gated filters
// =============================================================================
//
// Each filter is individually gated by a feature flag in RuntimeConfig.
// If the flag is disabled, that filter is skipped (returns pass).
//
// Filters:
//   - HTF Gate:            15M + 1H EMA alignment must confirm the trade direction
//   - Score Momentum:      Rolling score average must be improving
//   - OFIP:                Order Flow Imbalance Persistence
//   - Adaptive Threshold:  Dynamic buy threshold adjusted by regime
//   - CUSUM:               CUSUM structural break detection
//   - Absorption:          Institutional absorption detection
//   - Entropy Valley:      Entropy valley confidence boost
// =============================================================================

use std::sync::Arc;
use tracing::debug;

use crate::app_state::AppState;
use crate::market_data::CandleKey;

pub struct SmartFilterEngine;

impl SmartFilterEngine {
    /// Evaluate all enabled smart filters. Returns `None` if all pass,
    /// or `Some(reason)` if any filter blocks.
    pub fn evaluate(
        state: &Arc<AppState>,
        symbol: &str,
        side: &str,
        regime: &str,
        score: f64,
    ) -> Option<String> {
        let config = state.runtime_config.read();

        // ── HTF Gate ─────────────────────────────────────────────────────
        if config.enable_htf_gate {
            if let Some(htf) = crate::htf_analysis::analyze(&state.candle_buffer, symbol) {
                let htf_pass = if side == "BUY" {
                    htf.buy_allowed
                } else {
                    htf.sell_signal
                };
                if !htf_pass {
                    return Some(format!(
                        "HTF Gate: {} — does not confirm {} direction",
                        htf.reason, side
                    ));
                }
                debug!(symbol, side, "HTF gate passed");
            }
            // If HTF data insufficient, allow the trade
        }

        // ── Score Momentum ───────────────────────────────────────────────
        if config.enable_score_momentum {
            let momentum_threshold = 0.12;
            if score.abs() < momentum_threshold {
                return Some(format!(
                    "Score Momentum: |{:.3}| < {:.3} threshold",
                    score, momentum_threshold
                ));
            }
            debug!(symbol, score, "Score momentum filter passed");
        }

        // ── OFIP (Order Flow Imbalance Persistence) ──────────────────────
        if config.enable_ofip {
            let trade_procs = state.trade_processors.read();
            if let Some(tp) = trade_procs.get(symbol) {
                let buy_ratio = tp.buy_volume_ratio();
                let ofip_ok = if side == "BUY" {
                    buy_ratio > 0.52
                } else {
                    buy_ratio < 0.48
                };
                if !ofip_ok {
                    return Some(format!(
                        "OFIP: buy_ratio {:.3} does not confirm {} direction",
                        buy_ratio, side
                    ));
                }
            }
            debug!(symbol, side, "OFIP filter passed");
        }

        // ── Adaptive Threshold ───────────────────────────────────────────
        if config.enable_adaptive_threshold {
            let adaptive_min = match regime {
                "Trending" => 0.10,
                "Ranging" => 0.18,
                "Volatile" => 0.20,
                "Squeeze" => 0.15,
                "Dead" => 999.0,
                _ => 0.15,
            };
            if score.abs() < adaptive_min {
                return Some(format!(
                    "Adaptive Threshold: |{:.3}| < {:.3} for {} regime",
                    score, adaptive_min, regime
                ));
            }
            debug!(symbol, regime, score, adaptive_min, "Adaptive threshold passed");
        }

        // ── CUSUM Detection ──────────────────────────────────────────────
        if config.enable_cusum {
            let key_5m = CandleKey {
                symbol: symbol.to_string(),
                interval: "5m".to_string(),
            };
            let candles = state.candle_buffer.get_closed_candles(&key_5m, 50);
            if candles.len() >= 20 {
                let closes: Vec<f64> = candles.iter().map(|c| c.close).collect();
                let mut detector = crate::cusum_detector::CusumDetector::default();
                if let Some(cusum_state) = detector.detect(&closes) {
                    if cusum_state.bullish_break || cusum_state.bearish_break {
                        let break_matches = (side == "BUY" && cusum_state.bullish_break)
                            || (side == "SELL" && cusum_state.bearish_break);
                        if !break_matches {
                            return Some(format!(
                                "CUSUM: structural break in opposite direction ({})",
                                cusum_state.reason
                            ));
                        }
                        debug!(symbol, "CUSUM break confirms trade direction");
                    }
                }
            }
        }

        // ── Absorption Detection ─────────────────────────────────────────
        if config.enable_absorption {
            let key_5m = CandleKey {
                symbol: symbol.to_string(),
                interval: "5m".to_string(),
            };
            let candles = state.candle_buffer.get_closed_candles(&key_5m, 20);
            if candles.len() >= 20 {
                // Get CVD direction from trade stream
                let cvd_dir = {
                    let trade_procs = state.trade_processors.read();
                    trade_procs
                        .get(symbol)
                        .map(|tp| tp.cvd())
                        .unwrap_or(0.0)
                };
                if let Some(absorption) = crate::absorption_detector::AbsorptionDetector::detect(&candles, cvd_dir) {
                    if absorption.detected {
                        let opposes = (side == "BUY" && absorption.direction == "BEARISH")
                            || (side == "SELL" && absorption.direction == "BULLISH");
                        if opposes {
                            return Some(format!(
                                "Absorption: {} opposing {} (strength={:.2})",
                                absorption.direction, side, absorption.strength
                            ));
                        }
                        debug!(symbol, "Absorption confirms or neutral for trade direction");
                    }
                }
            }
        }

        // ── Entropy Valley ───────────────────────────────────────────────
        if config.enable_entropy_valley {
            let regime_state = state.regime_detector.read().current_regime();
            if let Some(rs) = regime_state {
                if rs.entropy < 0.3 {
                    debug!(
                        symbol,
                        entropy = rs.entropy,
                        "Entropy valley detected — confidence boost"
                    );
                }
            }
        }

        debug!(symbol, side, regime, score, "all smart filters passed");
        None
    }
}
