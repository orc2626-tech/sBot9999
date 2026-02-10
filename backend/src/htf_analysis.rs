// =============================================================================
// Higher Time Frame (HTF) Trend Analysis
// =============================================================================
//
// Evaluates the directional alignment of 15M and 1H EMA(9)/EMA(21) stacks to
// determine whether the higher time frames support a trade direction.
//
// Decision rule:
//   buy_allowed  = 15M bullish AND 1H bullish
//   sell_signal  = 15M bearish AND 1H bearish

use crate::indicators::ema::calculate_ema;
use crate::market_data::candle_buffer::{CandleBuffer, CandleKey};
use serde::{Deserialize, Serialize};
use tracing::debug;

/// Full snapshot of the HTF analysis for a single symbol.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HtfAnalysis {
    pub direction: String,
    pub confidence: f64,
    pub buy_allowed: bool,
    pub sell_signal: bool,
    pub trend_15m: String,
    pub trend_1h: String,
    pub ema_sep_15m: f64,
    pub ema_sep_1h: f64,
    pub momentum_1h: f64,
    pub candles_15m: usize,
    pub candles_1h: usize,
    pub reason: String,
}

/// Perform HTF analysis for `symbol` using data from `candle_buffer`.
///
/// Returns `None` when there are fewer than 21 closed candles on either the
/// 15M or 1H timeframe.
pub fn analyze(candle_buffer: &CandleBuffer, symbol: &str) -> Option<HtfAnalysis> {
    let key_15m = CandleKey {
        symbol: symbol.to_string(),
        interval: "15m".to_string(),
    };
    let key_1h = CandleKey {
        symbol: symbol.to_string(),
        interval: "1h".to_string(),
    };

    let closes_15m = candle_buffer.get_closes(&key_15m, 100);
    let closes_1h = candle_buffer.get_closes(&key_1h, 100);

    if closes_15m.len() < 21 || closes_1h.len() < 21 {
        debug!(
            symbol,
            candles_15m = closes_15m.len(),
            candles_1h = closes_1h.len(),
            "HTF analysis: insufficient data (need >= 21 candles per TF)"
        );
        return None;
    }

    // --- 15M EMA stack ---
    let ema9_15m = calculate_ema(&closes_15m, 9);
    let ema21_15m = calculate_ema(&closes_15m, 21);
    let e9_15m = *ema9_15m.last()?;
    let e21_15m = *ema21_15m.last()?;
    let bullish_15m = e9_15m > e21_15m;
    let ema_sep_15m = if e21_15m.abs() > f64::EPSILON {
        ((e9_15m - e21_15m) / e21_15m) * 100.0
    } else {
        0.0
    };
    let trend_15m = if bullish_15m { "BULLISH" } else { "BEARISH" };

    // --- 1H EMA stack ---
    let ema9_1h = calculate_ema(&closes_1h, 9);
    let ema21_1h = calculate_ema(&closes_1h, 21);
    let e9_1h = *ema9_1h.last()?;
    let e21_1h = *ema21_1h.last()?;
    let bullish_1h = e9_1h > e21_1h;
    let ema_sep_1h = if e21_1h.abs() > f64::EPSILON {
        ((e9_1h - e21_1h) / e21_1h) * 100.0
    } else {
        0.0
    };
    let trend_1h = if bullish_1h { "BULLISH" } else { "BEARISH" };

    // --- 1H momentum ---
    let momentum_1h = if ema9_1h.len() >= 2 {
        let prev = ema9_1h[ema9_1h.len() - 2];
        let curr = ema9_1h[ema9_1h.len() - 1];
        if prev.abs() > f64::EPSILON {
            ((curr - prev) / prev) * 100.0
        } else {
            0.0
        }
    } else {
        0.0
    };

    let buy_allowed = bullish_15m && bullish_1h;
    let sell_signal = !bullish_15m && !bullish_1h;

    let direction = if buy_allowed {
        "BULLISH"
    } else if sell_signal {
        "BEARISH"
    } else {
        "MIXED"
    };

    let confidence = {
        let sep_score = (ema_sep_15m.abs() + ema_sep_1h.abs()).min(2.0) / 2.0;
        let alignment_score = if buy_allowed || sell_signal {
            0.6
        } else {
            0.2
        };
        (sep_score * 0.4 + alignment_score).clamp(0.0, 1.0)
    };

    let reason = format!(
        "15M: {} (sep={:.3}%), 1H: {} (sep={:.3}%), momentum_1h={:.4}%",
        trend_15m, ema_sep_15m, trend_1h, ema_sep_1h, momentum_1h
    );

    debug!(
        symbol,
        direction,
        buy_allowed,
        sell_signal,
        confidence = format!("{:.3}", confidence),
        "HTF analysis complete"
    );

    Some(HtfAnalysis {
        direction: direction.to_string(),
        confidence,
        buy_allowed,
        sell_signal,
        trend_15m: trend_15m.to_string(),
        trend_1h: trend_1h.to_string(),
        ema_sep_15m,
        ema_sep_1h,
        momentum_1h,
        candles_15m: closes_15m.len(),
        candles_1h: closes_1h.len(),
        reason,
    })
}
