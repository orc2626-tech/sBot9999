# 📁 PHASE 4 — الداشبورد الخارق
# ═══════════════════════════════════
# متوازي مع المراحل الأخرى | الأولوية: 🔴 حرجة
# تحويل الداشبورد من أوهام إلى حقيقة حية
# ⚠️ اقرأ 00_MASTER_ROADMAP.md أولاً

---

## 🎯 هدف هذه المرحلة
```
الداشبورد حالياً = 90% بيانات وهمية hardcoded
بعد هذه المرحلة = داشبورد حي بمستوى صناديق التحوط

ما سنبنيه:
✅ WebSocket Push — بدل polling كل 3 ثوانٍ
✅ ربط كل المكونات الحالية بالـ API الحقيقي
✅ 7 لوحات مراقبة جديدة خارقة
✅ Market Regime Panel — نوع السوق + مؤشرات
✅ VPIN & Toxic Flow Monitor — إنذار مبكر
✅ Signal Heatmap (Weighted) — شفافية كاملة
✅ Futures Intelligence Panel — عين على 70% من السوق
✅ Trade Journal Analytics — تعلم بصري
✅ Triple Barrier Visualizer — مراقبة الصفقة الحية
✅ System Health & Circuit Breakers — صحة النظام الشاملة
```

---

## الجزء 1: البنية التحتية — WebSocket Push + API Layer

---

## 📦 الملف 1: تحديث `src/api/ws.rs` — Backend WebSocket Push

### المواصفات:
- يرسل state snapshot فوراً عند الاتصال
- ثم يرسل diffs كل 500ms (لا polling)
- يرسل أحداث مهمة فوراً (trade, regime change, circuit breaker)
- يدعم subscribe/unsubscribe لقنوات محددة

```rust
//! WebSocket Push — بث حي بدل polling
//! يستبدل الملف الحالي بالكامل
//! يرسل: state snapshots + diffs + events

use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    response::Response,
};
use futures_util::{SinkExt, StreamExt};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tokio::sync::broadcast;
use serde::Serialize;

/// أنواع الرسائل المرسلة للداشبورد
#[derive(Clone, Serialize)]
#[serde(tag = "type")]
pub enum WsMessage {
    /// Snapshot كامل — يُرسل عند الاتصال
    #[serde(rename = "snapshot")]
    Snapshot {
        state_version: u64,
        ws_sequence_number: u64,
        timestamp: u64,
        payload: crate::app_state::StateSnapshot,
    },
    /// تحديث دوري — كل 500ms
    #[serde(rename = "tick")]
    Tick {
        state_version: u64,
        ws_sequence_number: u64,
        timestamp: u64,
        payload: crate::app_state::StateSnapshot,
    },
    /// حدث مهم — فوري
    #[serde(rename = "event")]
    Event {
        event_type: String,   // "trade_opened", "trade_closed", "regime_change", "circuit_breaker", "signal_alert"
        state_version: u64,
        timestamp: u64,
        data: serde_json::Value,
    },
}

/// إنشاء broadcast channel للأحداث المهمة
pub fn create_event_broadcaster() -> broadcast::Sender<WsMessage> {
    let (tx, _) = broadcast::channel(256);
    tx
}

pub async fn ws_handler(
    State((state, ws_sequence)): State<(
        Arc<crate::app_state::AppState>,
        Arc<std::sync::atomic::AtomicU64>,
    )>,
    ws: WebSocketUpgrade,
) -> Response {
    ws.on_upgrade(move |socket| handle_socket(state, ws_sequence, socket))
}

async fn handle_socket(
    state: Arc<crate::app_state::AppState>,
    ws_sequence: Arc<std::sync::atomic::AtomicU64>,
    socket: WebSocket,
) {
    let (mut sender, mut receiver) = socket.split();

    // 1. أرسل snapshot كامل فوراً
    let seq = ws_sequence.fetch_add(1, Ordering::SeqCst);
    let snapshot = state.build_snapshot(seq);
    let server_time = crate::runtime_config::server_time_secs();

    let init_msg = WsMessage::Snapshot {
        state_version: snapshot.state_version,
        ws_sequence_number: seq,
        timestamp: server_time,
        payload: snapshot,
    };

    if let Ok(text) = serde_json::to_string(&init_msg) {
        if sender.send(Message::Text(text)).await.is_err() {
            return;
        }
    }

    // 2. Tick كل 500ms + استقبال رسائل العميل
    let mut tick_interval = tokio::time::interval(tokio::time::Duration::from_millis(500));
    let mut last_version = state.current_state_version();

    loop {
        tokio::select! {
            // Tick — أرسل فقط إذا تغيرت الحالة
            _ = tick_interval.tick() => {
                let current_version = state.current_state_version();
                if current_version != last_version {
                    let seq = ws_sequence.fetch_add(1, Ordering::SeqCst);
                    let snapshot = state.build_snapshot(seq);
                    let msg = WsMessage::Tick {
                        state_version: snapshot.state_version,
                        ws_sequence_number: seq,
                        timestamp: crate::runtime_config::server_time_secs(),
                        payload: snapshot,
                    };
                    if let Ok(text) = serde_json::to_string(&msg) {
                        if sender.send(Message::Text(text)).await.is_err() {
                            break;
                        }
                    }
                    last_version = current_version;
                }
            }
            // رسائل العميل — ping/pong + subscribe
            msg = receiver.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        // يمكن إضافة subscribe/unsubscribe لاحقاً
                        if text == "ping" {
                            let _ = sender.send(Message::Text("pong".into())).await;
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Err(_)) => break,
                    _ => {}
                }
            }
        }
    }
}
```

---

## 📦 الملف 2: `dashboard_v2/src/hooks/useWebSocket.ts` — WebSocket Hook

```typescript
/**
 * useWebSocket — Real-time WebSocket hook
 * يستبدل polling بـ WebSocket push
 * يدعم reconnection تلقائي مع exponential backoff
 */

import { useEffect, useRef, useState, useCallback } from 'react'
import { stateWebSocketUrl, type StateSnapshot } from '../lib/api'

interface WsMessage {
  type: 'snapshot' | 'tick' | 'event'
  state_version: number
  ws_sequence_number?: number
  timestamp: number
  payload?: StateSnapshot
  event_type?: string
  data?: unknown
}

interface UseWebSocketResult {
  state: StateSnapshot | null
  connected: boolean
  error: string | null
  lastEvent: WsMessage | null
  reconnect: () => void
}

const MAX_RECONNECT_DELAY = 30000  // 30 ثانية كحد أقصى
const INITIAL_DELAY = 1000          // ثانية واحدة

export function useWebSocket(): UseWebSocketResult {
  const [state, setState] = useState<StateSnapshot | null>(null)
  const [connected, setConnected] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [lastEvent, setLastEvent] = useState<WsMessage | null>(null)
  const wsRef = useRef<WebSocket | null>(null)
  const reconnectDelay = useRef(INITIAL_DELAY)
  const reconnectTimer = useRef<NodeJS.Timeout | null>(null)

  const connect = useCallback(() => {
    // تنظيف الاتصال السابق
    if (wsRef.current) {
      wsRef.current.close()
    }
    if (reconnectTimer.current) {
      clearTimeout(reconnectTimer.current)
    }

    const url = stateWebSocketUrl()
    const ws = new WebSocket(url)
    wsRef.current = ws

    ws.onopen = () => {
      setConnected(true)
      setError(null)
      reconnectDelay.current = INITIAL_DELAY  // إعادة تعيين delay
    }

    ws.onmessage = (event) => {
      try {
        const msg: WsMessage = JSON.parse(event.data)

        if (msg.type === 'snapshot' || msg.type === 'tick') {
          if (msg.payload) {
            setState(msg.payload)
          }
        }

        if (msg.type === 'event') {
          setLastEvent(msg)
        }
      } catch (e) {
        // تجاهل pong والرسائل غير JSON
      }
    }

    ws.onclose = () => {
      setConnected(false)
      // Exponential backoff reconnect
      reconnectTimer.current = setTimeout(() => {
        reconnectDelay.current = Math.min(
          reconnectDelay.current * 2,
          MAX_RECONNECT_DELAY
        )
        connect()
      }, reconnectDelay.current)
    }

    ws.onerror = () => {
      setError('WebSocket connection error')
      ws.close()
    }
  }, [])

  useEffect(() => {
    connect()
    return () => {
      if (wsRef.current) wsRef.current.close()
      if (reconnectTimer.current) clearTimeout(reconnectTimer.current)
    }
  }, [connect])

  // Ping keepalive كل 30 ثانية
  useEffect(() => {
    const interval = setInterval(() => {
      if (wsRef.current?.readyState === WebSocket.OPEN) {
        wsRef.current.send('ping')
      }
    }, 30000)
    return () => clearInterval(interval)
  }, [])

  return { state, connected, error, lastEvent, reconnect: connect }
}
```

---

## 📦 الملف 3: تحديث `dashboard_v2/src/hooks/useTruthState.ts`

```typescript
/**
 * useTruthState v2 — يستخدم WebSocket أولاً ثم fallback إلى polling
 * يستبدل الملف الحالي بالكامل
 */

import { useEffect, useState } from 'react'
import { fetchState, type StateSnapshot } from '../lib/api'
import { useWebSocket } from './useWebSocket'

const POLL_FALLBACK_MS = 5000

export function useTruthState(): {
  state: StateSnapshot | null
  error: string | null
  loading: boolean
  connected: boolean
  refetch: () => void
} {
  const ws = useWebSocket()
  const [pollState, setPollState] = useState<StateSnapshot | null>(null)
  const [pollError, setPollError] = useState<string | null>(null)
  const [loading, setLoading] = useState(true)

  // Fallback polling — فقط إذا WebSocket غير متصل
  useEffect(() => {
    if (ws.connected) {
      setLoading(false)
      return
    }

    const poll = () => {
      fetchState()
        .then(s => { setPollState(s); setPollError(null) })
        .catch(e => setPollError(e instanceof Error ? e.message : String(e)))
        .finally(() => setLoading(false))
    }

    poll()
    const t = setInterval(poll, POLL_FALLBACK_MS)
    return () => clearInterval(t)
  }, [ws.connected])

  // WebSocket state له الأولوية
  const state = ws.connected ? ws.state : (ws.state ?? pollState)
  const error = ws.error ?? pollError

  const refetch = () => {
    fetchState()
      .then(s => setPollState(s))
      .catch(() => {})
  }

  return {
    state,
    error,
    loading: loading && !ws.state,
    connected: ws.connected,
    refetch,
  }
}
```

---

## الجزء 2: إضافات REST API — Backend Endpoints

---

## 📦 الملف 4: إضافات `src/api/rest.rs`

### Endpoints جديدة مطلوبة:

```rust
// ═══ أضف هذه الـ routes إلى router() ═══

.route("/market-data", get(market_data_snapshot))
.route("/market-data/:symbol", get(market_data_symbol))
.route("/signals", get(signals_snapshot))
.route("/regime", get(regime_snapshot))
.route("/vpin", get(vpin_snapshot))
.route("/futures-intel", get(futures_intel_snapshot))
.route("/trade-journal", get(trade_journal_snapshot))
.route("/trade-journal/stats", get(trade_journal_stats))

// ═══ Handlers ═══

async fn market_data_snapshot(
    State((state, _ws_seq)): State<AppStateTuple>,
) -> impl IntoResponse {
    // يجمع بيانات السوق لكل symbol:
    // last_price, RSI, EMA, ADX, ATR, spread, CVD, OB imbalance
    let v = state.current_state_version();
    let symbols = state.runtime_config.read().symbols.clone();
    let mut data = serde_json::Map::new();

    for symbol in &symbols {
        let key = crate::market_data::CandleKey {
            symbol: symbol.clone(),
            interval: "1m".into(),
        };
        let candles = state.candle_buffer.get_closed(&key, 100);
        let closes: Vec<f64> = candles.iter().map(|c| c.close).collect();

        let rsi = crate::indicators::rsi::current_rsi(&closes, 14);
        let adx = crate::indicators::adx::calculate_adx(&candles, 14);
        let atr = crate::indicators::atr::calculate_atr(&candles, 14);
        let bb = crate::indicators::bollinger::calculate_bollinger(&closes, 20, 2.0);
        let ema9 = crate::indicators::ema::calculate_ema(&closes, 9).last().copied();
        let ema21 = crate::indicators::ema::calculate_ema(&closes, 21).last().copied();

        let ob_metrics = state.orderbook_manager.get_metrics(symbol);
        let cvd = state.trade_processors.read()
            .get(symbol)
            .map(|p| p.get_cvd());

        let symbol_data = serde_json::json!({
            "last_price": closes.last().unwrap_or(&0.0),
            "rsi_14": rsi.map(|(v, _)| v),
            "rsi_zone": rsi.map(|(_, z)| z),
            "adx": adx,
            "atr": atr,
            "ema_9": ema9,
            "ema_21": ema21,
            "bollinger_width": bb.as_ref().map(|b| b.width),
            "bollinger_percent_b": bb.as_ref().map(|b| b.percent_b),
            "spread_bps": ob_metrics.as_ref().map(|m| m.spread_bps),
            "bid_ask_imbalance": ob_metrics.as_ref().map(|m| m.imbalance),
            "mid_price": ob_metrics.as_ref().map(|m| m.mid_price),
            "cvd": cvd.as_ref().map(|c| c.cvd).unwrap_or(0.0),
            "buy_volume": cvd.as_ref().map(|c| c.buy_volume).unwrap_or(0.0),
            "sell_volume": cvd.as_ref().map(|c| c.sell_volume).unwrap_or(0.0),
        });

        data.insert(symbol.clone(), serde_json::Value::Object(
            symbol_data.as_object().unwrap().clone()
        ));
    }

    Json(serde_json::json!({
        "market_data": data,
        "state_version": v,
        "server_time": server_time(),
    }))
}

async fn regime_snapshot(
    State((state, _ws_seq)): State<AppStateTuple>,
) -> impl IntoResponse {
    // يقرأ حالة الـ Regime من app_state
    // يتوفر بعد تفعيل المرحلة 1
    let v = state.current_state_version();
    Json(serde_json::json!({
        "regime": state.regime_state.read().clone(),
        "state_version": v,
        "server_time": server_time(),
    }))
}

async fn signals_snapshot(
    State((state, _ws_seq)): State<AppStateTuple>,
) -> impl IntoResponse {
    // يقرأ حالة الإشارات المرجحة
    // يتوفر بعد تفعيل المرحلة 1
    let v = state.current_state_version();
    Json(serde_json::json!({
        "signals": state.signal_state.read().clone(),
        "state_version": v,
        "server_time": server_time(),
    }))
}

async fn vpin_snapshot(
    State((state, _ws_seq)): State<AppStateTuple>,
) -> impl IntoResponse {
    let v = state.current_state_version();
    Json(serde_json::json!({
        "vpin": state.vpin_state.read().clone(),
        "state_version": v,
        "server_time": server_time(),
    }))
}

async fn futures_intel_snapshot(
    State((state, _ws_seq)): State<AppStateTuple>,
) -> impl IntoResponse {
    let v = state.current_state_version();
    Json(serde_json::json!({
        "futures_intel": state.futures_intel_state.read().clone(),
        "state_version": v,
        "server_time": server_time(),
    }))
}

async fn trade_journal_snapshot(
    State((state, _ws_seq)): State<AppStateTuple>,
) -> impl IntoResponse {
    let v = state.current_state_version();
    Json(serde_json::json!({
        "trades": state.trade_journal.read().recent_trades(50),
        "state_version": v,
        "server_time": server_time(),
    }))
}

async fn trade_journal_stats(
    State((state, _ws_seq)): State<AppStateTuple>,
) -> impl IntoResponse {
    let v = state.current_state_version();
    Json(serde_json::json!({
        "stats": state.trade_journal.read().get_stats(),
        "state_version": v,
        "server_time": server_time(),
    }))
}
```

---

## 📦 الملف 5: تحديث `dashboard_v2/src/lib/api.ts`

### إضافة API functions + TypeScript interfaces:

```typescript
// ═══ أضف هذه الـ interfaces ═══

export interface MarketDataSymbol {
  last_price: number
  rsi_14: number | null
  rsi_zone: string | null
  adx: number | null
  atr: number | null
  ema_9: number | null
  ema_21: number | null
  bollinger_width: number | null
  bollinger_percent_b: number | null
  spread_bps: number | null
  bid_ask_imbalance: number | null
  mid_price: number | null
  cvd: number
  buy_volume: number
  sell_volume: number
}

export interface RegimeState {
  regime: string          // "TRENDING" | "RANGING" | "VOLATILE" | "SQUEEZE" | "DEAD"
  adx: number
  bbw: number
  hurst: number
  entropy: number
  regime_age_seconds: number
  strategy_params: {
    rr_target: string
    time_limit_min: number
    preferred_strategy: string
  }
}

export interface SignalState {
  total_score: number
  threshold: number
  decision: string         // "BUY" | "SELL" | "NEUTRAL"
  signals: SignalDetail[]
}

export interface SignalDetail {
  name: string
  weight: number
  confidence: number
  direction: number        // +1 = BUY, -1 = SELL, 0 = NEUTRAL
  contribution: number     // weight × confidence × direction
  age_seconds: number
  half_life_seconds: number
  effective_value: number  // بعد التحلل
}

export interface VpinState {
  vpin: number
  zone: string             // "SAFE" | "CAUTION" | "DANGER"
  bucket_count: number
  avg_imbalance: number
  spread_rising: boolean
  combo_alert: boolean     // VPIN↑ + Spread↑
  history: { time: number; value: number }[]
}

export interface FuturesIntelState {
  funding_rate: number
  funding_signal: string   // "BULLISH_CONTRARIAN" | "BEARISH_CONTRARIAN" | "NEUTRAL"
  open_interest: number
  oi_change_1h_pct: number
  oi_interpretation: string
  long_short_ratio: number
  ls_signal: string
  last_update: number
}

export interface TradeRecord {
  id: string
  symbol: string
  side: string
  entry_price: number
  exit_price: number | null
  quantity: number
  pnl: number | null
  pnl_pct: number | null
  regime_at_entry: string
  signals_snapshot: SignalDetail[]
  entry_time: string
  exit_time: string | null
  status: string           // "OPEN" | "CLOSED_TP" | "CLOSED_SL" | "CLOSED_TIME" | "CLOSED_MANUAL"
  mfe: number              // Max Favorable Excursion
  mae: number              // Max Adverse Excursion
}

export interface TradeStats {
  total_trades: number
  wins: number
  losses: number
  win_rate: number
  avg_win: number
  avg_loss: number
  profit_factor: number
  total_pnl: number
  total_pnl_pct: number
  avg_duration_minutes: number
  best_trade: number
  worst_trade: number
  sharpe_ratio: number | null
  max_drawdown_pct: number
  signal_accuracy: Record<string, { correct: number; total: number; pct: number }>
  regime_performance: Record<string, { trades: number; win_rate: number; pnl: number }>
}

export interface RiskState {
  risk_mode: string
  daily_pnl: number
  daily_pnl_pct: number
  remaining_daily_loss_pct: number
  consecutive_losses: number
  daily_trades_count: number
  daily_wins: number
  daily_losses: number
  max_drawdown_today: number
  circuit_breakers: {
    daily_loss_breaker: BreakerState
    consecutive_loss_breaker: BreakerState
    drawdown_breaker: BreakerState
    trade_limit_breaker: BreakerState
  }
}

export interface BreakerState {
  name: string
  current: number
  limit: number
  tripped: boolean
  tripped_at: string | null
}

// ═══ أضف هذه الـ fetch functions ═══

export async function fetchMarketData(): Promise<Record<string, MarketDataSymbol>> {
  const token = getAdminToken()
  const headers: HeadersInit = {}
  if (token) headers['Authorization'] = `Bearer ${token}`
  const r = await fetch(`${BASE}/api/v1/market-data`, { headers })
  if (!r.ok) throw new Error(`MarketData ${r.status}`)
  const data = await r.json()
  return data.market_data
}

export async function fetchRegime(): Promise<RegimeState> {
  const token = getAdminToken()
  const headers: HeadersInit = {}
  if (token) headers['Authorization'] = `Bearer ${token}`
  const r = await fetch(`${BASE}/api/v1/regime`, { headers })
  if (!r.ok) throw new Error(`Regime ${r.status}`)
  const data = await r.json()
  return data.regime
}

export async function fetchSignals(): Promise<SignalState> {
  const token = getAdminToken()
  const headers: HeadersInit = {}
  if (token) headers['Authorization'] = `Bearer ${token}`
  const r = await fetch(`${BASE}/api/v1/signals`, { headers })
  if (!r.ok) throw new Error(`Signals ${r.status}`)
  const data = await r.json()
  return data.signals
}

export async function fetchVpin(): Promise<VpinState> {
  const token = getAdminToken()
  const headers: HeadersInit = {}
  if (token) headers['Authorization'] = `Bearer ${token}`
  const r = await fetch(`${BASE}/api/v1/vpin`, { headers })
  if (!r.ok) throw new Error(`VPIN ${r.status}`)
  const data = await r.json()
  return data.vpin
}

export async function fetchFuturesIntel(): Promise<FuturesIntelState> {
  const token = getAdminToken()
  const headers: HeadersInit = {}
  if (token) headers['Authorization'] = `Bearer ${token}`
  const r = await fetch(`${BASE}/api/v1/futures-intel`, { headers })
  if (!r.ok) throw new Error(`FuturesIntel ${r.status}`)
  const data = await r.json()
  return data.futures_intel
}

export async function fetchTradeJournal(): Promise<TradeRecord[]> {
  const token = getAdminToken()
  const headers: HeadersInit = {}
  if (token) headers['Authorization'] = `Bearer ${token}`
  const r = await fetch(`${BASE}/api/v1/trade-journal`, { headers })
  if (!r.ok) throw new Error(`Journal ${r.status}`)
  const data = await r.json()
  return data.trades
}

export async function fetchTradeStats(): Promise<TradeStats> {
  const token = getAdminToken()
  const headers: HeadersInit = {}
  if (token) headers['Authorization'] = `Bearer ${token}`
  const r = await fetch(`${BASE}/api/v1/trade-journal/stats`, { headers })
  if (!r.ok) throw new Error(`Stats ${r.status}`)
  const data = await r.json()
  return data.stats
}
```

---

## الجزء 3: ربط المكونات الحالية بالبيانات الحقيقية

---

## 📦 الملف 6: تحديث `SymbolsTickerRow.tsx` — أسعار حية

```tsx
/**
 * SymbolsTickerRow — أسعار حية من API
 * يستبدل البيانات الوهمية الحالية
 */
import { LineChart, Line, ResponsiveContainer } from 'recharts'
import { useTruthState } from '../hooks/useTruthState'

export default function SymbolsTickerRow() {
  const { state } = useTruthState()
  const symbols = state?.runtime_config?.symbols ?? ['BTCUSDT', 'ETHUSDT', 'SOLUSDT', 'BNBUSDT', 'XRPUSDT']
  const marketData = (state as any)?.market_data ?? {}

  return (
    <div className="grid grid-cols-5 gap-3">
      {symbols.map((sym: string) => {
        const data = marketData[sym]
        const price = data?.last_price ?? 0
        const rsi = data?.rsi_14
        const spreadBps = data?.spread_bps

        return (
          <div key={sym} className="panel p-3">
            <div className="flex justify-between items-start">
              <span className="font-mono font-semibold text-slate-200">
                {sym.replace('USDT', '/USDT')}
              </span>
              {rsi != null && (
                <span className={`font-mono text-xs px-1 rounded ${
                  rsi > 70 ? 'text-red bg-red/20' :
                  rsi < 30 ? 'text-green bg-green/20' :
                  'text-slate-400'
                }`}>
                  RSI {rsi.toFixed(0)}
                </span>
              )}
            </div>
            <div className="font-mono text-lg text-white mt-0.5">
              {price > 0 ? price.toLocaleString(undefined, { maximumFractionDigits: 2 }) : '—'}
            </div>
            <div className="flex justify-between text-xs mt-1">
              {spreadBps != null && (
                <span className={`font-mono ${spreadBps < 5 ? 'text-green' : spreadBps < 15 ? 'text-yellow' : 'text-red'}`}>
                  Spread: {spreadBps.toFixed(1)}bps
                </span>
              )}
            </div>
          </div>
        )
      })}
    </div>
  )
}
```

---

## 📦 الملف 7: تحديث `SignalEnsembleCard.tsx` — إشارات حقيقية

```tsx
/**
 * SignalEnsembleCard — إشارات مرجحة حقيقية من API
 * يعرض: الاسم، الوزن، الثقة، الاتجاه، المساهمة، العمر
 */
import { useTruthState } from '../hooks/useTruthState'

export default function SignalEnsembleCard() {
  const { state } = useTruthState()
  const signalState = (state as any)?.signals

  if (!signalState) {
    return (
      <div className="panel p-4">
        <h3 className="text-sm font-semibold text-slate-400 uppercase tracking-wide mb-3">Signal Ensemble</h3>
        <p className="text-sm text-slate-500">Waiting for signal data...</p>
      </div>
    )
  }

  const { total_score, threshold, decision, signals } = signalState

  return (
    <div className="panel p-4">
      <h3 className="text-sm font-semibold text-slate-400 uppercase tracking-wide mb-3">Signal Ensemble (Weighted)</h3>
      <div className="space-y-2">
        {(signals ?? []).map((s: any) => {
          const barWidth = Math.abs(s.contribution) * 100
          const isPositive = s.contribution > 0
          const opacity = Math.max(0.3, s.effective_value / (s.confidence || 1))

          return (
            <div key={s.name} className="space-y-0.5">
              <div className="flex justify-between text-xs">
                <span className="text-slate-300">{s.name}</span>
                <span className="font-mono text-slate-400">
                  w:{s.weight.toFixed(2)} × c:{s.confidence.toFixed(2)} = {s.contribution > 0 ? '+' : ''}{s.contribution.toFixed(3)}
                </span>
              </div>
              <div className="h-2 bg-slate-700 rounded-full overflow-hidden relative">
                <div
                  className={`h-full rounded-full ${isPositive ? 'bg-green' : 'bg-red'}`}
                  style={{
                    width: `${Math.min(barWidth * 3, 100)}%`,
                    opacity,
                  }}
                />
              </div>
              <div className="flex justify-between text-[10px] text-slate-500">
                <span>Age: {s.age_seconds}s / HL: {s.half_life_seconds}s</span>
                <span style={{ opacity }}>{(opacity * 100).toFixed(0)}% fresh</span>
              </div>
            </div>
          )
        })}
      </div>

      <div className="mt-4 pt-3 border-t border-panelBorder">
        <div className="flex items-center justify-between">
          <span className="text-slate-400 text-sm">Score:</span>
          <span className={`font-mono text-lg font-bold ${
            total_score > threshold ? 'text-green' :
            total_score < -threshold ? 'text-red' :
            'text-slate-400'
          }`}>
            {total_score > 0 ? '+' : ''}{total_score?.toFixed(3) ?? '—'}
          </span>
        </div>
        <div className="flex items-center justify-between mt-1">
          <span className="text-slate-400 text-sm">Decision:</span>
          <span className={`font-mono font-semibold ${
            decision === 'BUY' ? 'text-green' :
            decision === 'SELL' ? 'text-red' :
            'text-slate-500'
          }`}>
            {decision ?? 'NEUTRAL'}
          </span>
        </div>
      </div>
    </div>
  )
}
```

---

## الجزء 4: اللوحات الجديدة الخارقة (7 لوحات)

---

## 📦 الملف 8: `RegimePanel.tsx` — لوحة نوع السوق

```tsx
/**
 * Market Regime Panel — يعرض نوع السوق الحالي
 * اللون يتغير: أخضر=TRENDING، أزرق=RANGING، أحمر=VOLATILE، أصفر=SQUEEZE، رمادي=DEAD
 * يعرض: ADX, BBW, Hurst, Entropy + المعلمات المفعّلة
 */
import { useTruthState } from '../hooks/useTruthState'

const REGIME_COLORS: Record<string, { bg: string; text: string; border: string }> = {
  TRENDING: { bg: 'bg-green/10', text: 'text-green', border: 'border-green/50' },
  RANGING: { bg: 'bg-blue-400/10', text: 'text-blue-400', border: 'border-blue-400/50' },
  VOLATILE: { bg: 'bg-red/10', text: 'text-red', border: 'border-red/50' },
  SQUEEZE: { bg: 'bg-yellow/10', text: 'text-yellow', border: 'border-yellow/50' },
  DEAD: { bg: 'bg-slate-600/10', text: 'text-slate-500', border: 'border-slate-600/50' },
}

export default function RegimePanel() {
  const { state } = useTruthState()
  const regime = (state as any)?.regime

  if (!regime) {
    return (
      <div className="panel p-4">
        <h3 className="text-sm font-semibold text-slate-400 uppercase tracking-wide mb-3">Market Regime</h3>
        <p className="text-sm text-slate-500">Waiting for regime data...</p>
      </div>
    )
  }

  const colors = REGIME_COLORS[regime.regime] ?? REGIME_COLORS.DEAD
  const params = regime.strategy_params ?? {}

  return (
    <div className={`panel p-4 ${colors.border} ${colors.bg}`}>
      <div className="flex items-center justify-between mb-3">
        <h3 className="text-sm font-semibold text-slate-400 uppercase tracking-wide">Market Regime</h3>
        <span className={`px-3 py-1 rounded-full font-mono font-bold text-sm ${colors.text} ${colors.bg} border ${colors.border} animate-pulse`}>
          {regime.regime}
        </span>
      </div>

      <div className="grid grid-cols-2 gap-3 text-sm">
        <div>
          <span className="text-slate-500">ADX (Trend)</span>
          <div className="font-mono text-slate-200">{regime.adx?.toFixed(1) ?? '—'}</div>
          <div className="h-1.5 bg-slate-700 rounded-full mt-1">
            <div className="h-full bg-blue-400 rounded-full" style={{ width: `${Math.min((regime.adx ?? 0) / 60 * 100, 100)}%` }} />
          </div>
        </div>
        <div>
          <span className="text-slate-500">BBW (Volatility)</span>
          <div className="font-mono text-slate-200">{regime.bbw?.toFixed(2) ?? '—'}%</div>
          <div className="h-1.5 bg-slate-700 rounded-full mt-1">
            <div className="h-full bg-orange-400 rounded-full" style={{ width: `${Math.min((regime.bbw ?? 0) / 10 * 100, 100)}%` }} />
          </div>
        </div>
        <div>
          <span className="text-slate-500">Hurst (Memory)</span>
          <div className={`font-mono ${(regime.hurst ?? 0.5) > 0.55 ? 'text-green' : (regime.hurst ?? 0.5) < 0.45 ? 'text-yellow' : 'text-red'}`}>
            {regime.hurst?.toFixed(3) ?? '—'}
          </div>
          <span className="text-[10px] text-slate-500">
            {(regime.hurst ?? 0.5) > 0.55 ? 'Momentum works' :
             (regime.hurst ?? 0.5) < 0.45 ? 'Mean-revert works' : 'Random — BLOCK'}
          </span>
        </div>
        <div>
          <span className="text-slate-500">Entropy</span>
          <div className={`font-mono ${(regime.entropy ?? 1) < 0.8 ? 'text-green' : (regime.entropy ?? 1) < 0.95 ? 'text-yellow' : 'text-red'}`}>
            {regime.entropy?.toFixed(3) ?? '—'}
          </div>
          <span className="text-[10px] text-slate-500">
            {(regime.entropy ?? 1) >= 0.95 ? '⛔ Noise — BLOCK' :
             (regime.entropy ?? 1) >= 0.8 ? '⚠️ Reduce size 50%' : '✅ Clear signal'}
          </span>
        </div>
      </div>

      <div className="mt-3 pt-3 border-t border-panelBorder/50 grid grid-cols-3 gap-2 text-xs">
        <div>
          <span className="text-slate-500">R:R Target</span>
          <div className="font-mono text-slate-200">{params.rr_target ?? '—'}</div>
        </div>
        <div>
          <span className="text-slate-500">Time Limit</span>
          <div className="font-mono text-slate-200">{params.time_limit_min ?? '—'}min</div>
        </div>
        <div>
          <span className="text-slate-500">Strategy</span>
          <div className="font-mono text-slate-200">{params.preferred_strategy ?? '—'}</div>
        </div>
      </div>

      <div className="mt-2 text-[10px] text-slate-500 font-mono">
        Regime age: {regime.regime_age_seconds ?? 0}s
      </div>
    </div>
  )
}
```

---

## 📦 الملف 9: `VPINMonitor.tsx` — مراقب التدفق السام

```tsx
/**
 * VPIN & Toxic Flow Monitor — إنذار مبكر للانهيارات
 * الخلفية تتغير: أخضر→أصفر→أحمر حسب مستوى VPIN
 * ⚡ هذا المؤشر تنبأ بـ Flash Crash 2010
 */
import { LineChart, Line, ResponsiveContainer, ReferenceLine } from 'recharts'
import { AlertTriangle, Bell } from 'lucide-react'
import { useTruthState } from '../hooks/useTruthState'

export default function VPINMonitor() {
  const { state } = useTruthState()
  const vpin = (state as any)?.vpin

  if (!vpin) {
    return (
      <div className="panel p-4">
        <h3 className="text-sm font-semibold text-slate-400 uppercase tracking-wide">VPIN Monitor</h3>
        <p className="text-sm text-slate-500 mt-2">Waiting for VPIN data...</p>
      </div>
    )
  }

  const vpinValue = vpin.vpin ?? 0
  const zone = vpin.zone ?? 'SAFE'
  const comboAlert = vpin.combo_alert ?? false
  const history = vpin.history ?? []

  const bgClass = zone === 'DANGER' ? 'bg-red/10 border-red/50' :
                  zone === 'CAUTION' ? 'bg-yellow/10 border-yellow/50' :
                  'bg-green/10 border-green/30'

  return (
    <div className={`panel p-4 ${bgClass} transition-colors duration-500`}>
      <div className="flex items-center justify-between mb-2">
        <h3 className="text-sm font-semibold text-slate-400 uppercase tracking-wide">VPIN — Toxic Flow</h3>
        {comboAlert && (
          <div className="flex items-center gap-1 text-red animate-pulse">
            <Bell className="h-4 w-4" />
            <span className="text-xs font-bold">COMBO ALERT</span>
          </div>
        )}
      </div>

      <div className="flex items-baseline gap-3 mb-3">
        <span className={`font-mono text-3xl font-bold ${
          zone === 'DANGER' ? 'text-red' :
          zone === 'CAUTION' ? 'text-yellow' : 'text-green'
        }`}>
          {vpinValue.toFixed(3)}
        </span>
        <span className={`text-sm font-mono px-2 py-0.5 rounded ${
          zone === 'DANGER' ? 'bg-red/20 text-red' :
          zone === 'CAUTION' ? 'bg-yellow/20 text-yellow' :
          'bg-green/20 text-green'
        }`}>
          {zone}
        </span>
      </div>

      {/* VPIN Chart */}
      {history.length > 0 && (
        <div className="h-24 mb-2">
          <ResponsiveContainer width="100%" height="100%">
            <LineChart data={history}>
              <ReferenceLine y={0.45} stroke="#ef4444" strokeDasharray="3 3" label="DANGER" />
              <ReferenceLine y={0.25} stroke="#eab308" strokeDasharray="3 3" label="CAUTION" />
              <Line
                type="monotone"
                dataKey="value"
                stroke={zone === 'DANGER' ? '#ef4444' : zone === 'CAUTION' ? '#eab308' : '#22c55e'}
                strokeWidth={2}
                dot={false}
              />
            </LineChart>
          </ResponsiveContainer>
        </div>
      )}

      {/* Buy/Sell Volume Split */}
      <div className="flex items-center gap-2 text-xs mb-2">
        <div className="flex-1 h-3 bg-slate-700 rounded-full overflow-hidden flex">
          <div
            className="h-full bg-green"
            style={{ width: `${((vpin.buy_vol ?? 50) / ((vpin.buy_vol ?? 50) + (vpin.sell_vol ?? 50))) * 100}%` }}
          />
          <div className="h-full bg-red flex-1" />
        </div>
      </div>
      <div className="flex justify-between text-[10px] text-slate-500 font-mono">
        <span>Buy: {((vpin.buy_vol ?? 50) / ((vpin.buy_vol ?? 50) + (vpin.sell_vol ?? 50)) * 100).toFixed(0)}%</span>
        <span>Sell: {((vpin.sell_vol ?? 50) / ((vpin.buy_vol ?? 50) + (vpin.sell_vol ?? 50)) * 100).toFixed(0)}%</span>
      </div>

      {comboAlert && (
        <div className="mt-2 p-2 rounded bg-red/20 border border-red/50 flex items-center gap-2">
          <AlertTriangle className="h-4 w-4 text-red shrink-0" />
          <span className="text-xs text-red font-medium">VPIN↑ + Spread↑ = Immediate BLOCK</span>
        </div>
      )}
    </div>
  )
}
```

---

## 📦 الملف 10: `FuturesIntelPanel.tsx` — استخبارات Futures

```tsx
/**
 * Futures Intelligence Dashboard
 * Funding Rate + Open Interest + Long/Short Ratio
 * إشارات Contrarian — ضد القطيع
 */
import { TrendingUp, TrendingDown, Minus, Activity } from 'lucide-react'
import { useTruthState } from '../hooks/useTruthState'

export default function FuturesIntelPanel() {
  const { state } = useTruthState()
  const fi = (state as any)?.futures_intel

  if (!fi) {
    return (
      <div className="panel p-4">
        <h3 className="text-sm font-semibold text-slate-400 uppercase tracking-wide">Futures Intelligence</h3>
        <p className="text-sm text-slate-500 mt-2">Waiting for futures data...</p>
      </div>
    )
  }

  const SignalIcon = ({ signal }: { signal: string }) => {
    if (signal?.includes('BULLISH')) return <TrendingUp className="h-4 w-4 text-green" />
    if (signal?.includes('BEARISH')) return <TrendingDown className="h-4 w-4 text-red" />
    return <Minus className="h-4 w-4 text-slate-500" />
  }

  return (
    <div className="panel p-4">
      <h3 className="text-sm font-semibold text-slate-400 uppercase tracking-wide mb-3 flex items-center gap-2">
        <Activity className="h-4 w-4" />
        Futures Intelligence
      </h3>

      <div className="space-y-4">
        {/* Funding Rate */}
        <div>
          <div className="flex items-center justify-between text-sm mb-1">
            <span className="text-slate-400">Funding Rate</span>
            <span className={`font-mono font-medium ${
              (fi.funding_rate ?? 0) > 0.03 ? 'text-red' :
              (fi.funding_rate ?? 0) < -0.03 ? 'text-green' : 'text-slate-300'
            }`}>
              {((fi.funding_rate ?? 0) * 100).toFixed(4)}%
            </span>
          </div>
          <div className="flex items-center gap-2">
            <SignalIcon signal={fi.funding_signal ?? ''} />
            <span className="text-xs text-slate-400">{fi.funding_signal ?? 'NEUTRAL'}</span>
            {(fi.funding_rate ?? 0) > 0.05 && (
              <span className="text-[10px] text-red bg-red/20 px-1 rounded">Overleveraged Longs</span>
            )}
            {(fi.funding_rate ?? 0) < -0.03 && (
              <span className="text-[10px] text-green bg-green/20 px-1 rounded">Short Squeeze Likely</span>
            )}
          </div>
        </div>

        {/* Open Interest */}
        <div>
          <div className="flex items-center justify-between text-sm mb-1">
            <span className="text-slate-400">Open Interest</span>
            <span className="font-mono text-slate-300">${((fi.open_interest ?? 0) / 1e6).toFixed(1)}M</span>
          </div>
          <div className="flex items-center gap-2">
            <span className={`text-xs font-mono ${
              (fi.oi_change_1h_pct ?? 0) > 0 ? 'text-green' : 'text-red'
            }`}>
              {(fi.oi_change_1h_pct ?? 0) > 0 ? '+' : ''}{(fi.oi_change_1h_pct ?? 0).toFixed(1)}% (1h)
            </span>
            <span className="text-xs text-slate-400">{fi.oi_interpretation ?? ''}</span>
          </div>
          {(fi.oi_change_1h_pct ?? 0) < -10 && (
            <div className="text-[10px] text-red mt-1 font-medium">⚠️ Mass Liquidation — BLOCK</div>
          )}
        </div>

        {/* Long/Short Ratio */}
        <div>
          <div className="flex items-center justify-between text-sm mb-1">
            <span className="text-slate-400">Long/Short Ratio</span>
            <span className="font-mono text-slate-300">{(fi.long_short_ratio ?? 1).toFixed(2)}</span>
          </div>
          <div className="h-3 bg-slate-700 rounded-full overflow-hidden flex">
            <div
              className="h-full bg-green"
              style={{ width: `${((fi.long_short_ratio ?? 1) / ((fi.long_short_ratio ?? 1) + 1)) * 100}%` }}
            />
            <div className="h-full bg-red flex-1" />
          </div>
          <div className="flex justify-between text-[10px] text-slate-500 mt-1">
            <span>Longs {(((fi.long_short_ratio ?? 1) / ((fi.long_short_ratio ?? 1) + 1)) * 100).toFixed(0)}%</span>
            <span>Shorts {((1 / ((fi.long_short_ratio ?? 1) + 1)) * 100).toFixed(0)}%</span>
          </div>
          <div className="flex items-center gap-2 mt-1">
            <SignalIcon signal={fi.ls_signal ?? ''} />
            <span className="text-xs text-slate-400">{fi.ls_signal ?? 'NEUTRAL'}</span>
          </div>
        </div>
      </div>
    </div>
  )
}
```

---

## 📦 الملف 11: `TradeJournalDash.tsx` — تحليلات الصفقات

```tsx
/**
 * Trade Journal & Learning Analytics
 * إحصائيات اليوم/الأسبوع + منحنى PnL + دقة الإشارات بحسب Regime
 */
import { BarChart, Bar, LineChart, Line, XAxis, YAxis, ResponsiveContainer, Tooltip, Cell } from 'recharts'
import { useTruthState } from '../hooks/useTruthState'

export default function TradeJournalDash() {
  const { state } = useTruthState()
  const stats = (state as any)?.trade_stats
  const trades = (state as any)?.recent_trades ?? []

  if (!stats) {
    return (
      <div className="panel p-4">
        <h3 className="text-sm font-semibold text-slate-400 uppercase tracking-wide">Trade Journal</h3>
        <p className="text-sm text-slate-500 mt-2">No trade data yet...</p>
      </div>
    )
  }

  // PnL Curve
  const pnlCurve = trades.filter((t: any) => t.pnl != null).map((t: any, i: number) => ({
    index: i + 1,
    pnl: t.pnl,
    cumulative: trades.slice(0, i + 1).reduce((sum: number, tr: any) => sum + (tr.pnl ?? 0), 0),
  }))

  // Signal accuracy by regime
  const regimePerf = stats.regime_performance ?? {}

  return (
    <div className="panel p-4">
      <h3 className="text-sm font-semibold text-slate-400 uppercase tracking-wide mb-3">Trade Journal & Analytics</h3>

      {/* Stats Row */}
      <div className="grid grid-cols-4 gap-2 mb-4">
        <div className="text-center">
          <div className="text-xs text-slate-500">Trades</div>
          <div className="font-mono text-lg text-slate-200">{stats.total_trades ?? 0}</div>
        </div>
        <div className="text-center">
          <div className="text-xs text-slate-500">Win Rate</div>
          <div className={`font-mono text-lg ${(stats.win_rate ?? 0) >= 50 ? 'text-green' : 'text-red'}`}>
            {(stats.win_rate ?? 0).toFixed(1)}%
          </div>
        </div>
        <div className="text-center">
          <div className="text-xs text-slate-500">Net PnL</div>
          <div className={`font-mono text-lg ${(stats.total_pnl ?? 0) >= 0 ? 'text-green' : 'text-red'}`}>
            {(stats.total_pnl ?? 0) >= 0 ? '+' : ''}{(stats.total_pnl ?? 0).toFixed(2)}
          </div>
        </div>
        <div className="text-center">
          <div className="text-xs text-slate-500">Profit Factor</div>
          <div className="font-mono text-lg text-slate-200">{(stats.profit_factor ?? 0).toFixed(2)}</div>
        </div>
      </div>

      {/* PnL Equity Curve */}
      {pnlCurve.length > 0 && (
        <div className="h-28 mb-3">
          <div className="text-xs text-slate-500 mb-1">Equity Curve</div>
          <ResponsiveContainer width="100%" height="100%">
            <LineChart data={pnlCurve}>
              <XAxis dataKey="index" hide />
              <YAxis hide />
              <Tooltip contentStyle={{ background: '#1a1d24', border: '1px solid #2d323b', fontSize: 12 }} />
              <Line type="monotone" dataKey="cumulative" stroke="#22c55e" strokeWidth={2} dot={false} />
            </LineChart>
          </ResponsiveContainer>
        </div>
      )}

      {/* Regime Performance */}
      {Object.keys(regimePerf).length > 0 && (
        <div className="mt-3 pt-3 border-t border-panelBorder">
          <div className="text-xs text-slate-500 mb-2">Performance by Regime</div>
          <div className="space-y-1">
            {Object.entries(regimePerf).map(([regime, data]: [string, any]) => (
              <div key={regime} className="flex items-center justify-between text-xs">
                <span className="text-slate-400">{regime}</span>
                <div className="flex items-center gap-3 font-mono">
                  <span className="text-slate-500">{data.trades} trades</span>
                  <span className={data.win_rate >= 50 ? 'text-green' : 'text-red'}>
                    {data.win_rate?.toFixed(0)}% WR
                  </span>
                  <span className={data.pnl >= 0 ? 'text-green' : 'text-red'}>
                    {data.pnl >= 0 ? '+' : ''}{data.pnl?.toFixed(2)}
                  </span>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Extra Stats */}
      <div className="mt-3 pt-3 border-t border-panelBorder grid grid-cols-2 gap-2 text-xs">
        <div className="flex justify-between">
          <span className="text-slate-500">Avg Duration</span>
          <span className="font-mono text-slate-300">{(stats.avg_duration_minutes ?? 0).toFixed(0)}min</span>
        </div>
        <div className="flex justify-between">
          <span className="text-slate-500">Sharpe Ratio</span>
          <span className="font-mono text-slate-300">{stats.sharpe_ratio?.toFixed(2) ?? '—'}</span>
        </div>
        <div className="flex justify-between">
          <span className="text-slate-500">Max Drawdown</span>
          <span className="font-mono text-red">{(stats.max_drawdown_pct ?? 0).toFixed(2)}%</span>
        </div>
        <div className="flex justify-between">
          <span className="text-slate-500">Best / Worst</span>
          <span className="font-mono">
            <span className="text-green">{(stats.best_trade ?? 0).toFixed(2)}</span>
            {' / '}
            <span className="text-red">{(stats.worst_trade ?? 0).toFixed(2)}</span>
          </span>
        </div>
      </div>
    </div>
  )
}
```

---

## 📦 الملف 12: `TripleBarrierViz.tsx` — مراقبة الصفقة الحية

```tsx
/**
 * Triple Barrier Visualizer — يعرض الصفقة النشطة بين حواجزها الثلاثة
 * TP2, TP1, Entry, SL + شريط الوقت + تشديد تلقائي
 */
import { useTruthState } from '../hooks/useTruthState'

export default function TripleBarrierViz() {
  const { state } = useTruthState()
  const positions = (state as any)?.active_positions ?? []

  if (positions.length === 0) {
    return (
      <div className="panel p-4">
        <h3 className="text-sm font-semibold text-slate-400 uppercase tracking-wide">Triple Barrier</h3>
        <p className="text-sm text-slate-500 mt-2">No active positions</p>
      </div>
    )
  }

  return (
    <div className="panel p-4">
      <h3 className="text-sm font-semibold text-slate-400 uppercase tracking-wide mb-3">Triple Barrier — Active Trades</h3>
      <div className="space-y-4">
        {positions.map((pos: any) => {
          const currentPrice = pos.current_price ?? pos.entry_price
          const entry = pos.entry_price
          const tp1 = pos.tp1_price
          const tp2 = pos.tp2_price
          const sl = pos.sl_price
          const timeUsed = pos.time_used_pct ?? 0

          // حساب موقع السعر الحالي بين SL و TP2
          const range = tp2 - sl
          const pricePct = range > 0 ? ((currentPrice - sl) / range) * 100 : 50

          const isProfitable = currentPrice > entry
          const priceColor = isProfitable ? 'text-green' : 'text-red'

          return (
            <div key={pos.symbol} className="border border-panelBorder rounded p-3">
              <div className="flex items-center justify-between mb-2">
                <span className="font-mono font-semibold text-slate-200">{pos.symbol}</span>
                <span className={`font-mono text-sm ${priceColor}`}>
                  {currentPrice?.toFixed(2)} ({isProfitable ? '+' : ''}{((currentPrice - entry) / entry * 100).toFixed(2)}%)
                </span>
              </div>

              {/* Price Level Visualization */}
              <div className="relative h-8 bg-slate-800 rounded mb-2">
                {/* TP2 */}
                <div className="absolute right-0 top-0 bottom-0 w-px bg-green" />
                <div className="absolute right-0 -top-4 text-[9px] text-green font-mono">TP2 {tp2?.toFixed(1)}</div>

                {/* TP1 */}
                <div className="absolute top-0 bottom-0 w-px bg-green/50" style={{ left: `${((tp1 - sl) / range * 100)}%` }} />

                {/* Entry */}
                <div className="absolute top-0 bottom-0 w-px bg-blue-400 border-dashed" style={{ left: `${((entry - sl) / range * 100)}%` }} />

                {/* SL */}
                <div className="absolute left-0 top-0 bottom-0 w-px bg-red" />
                <div className="absolute left-0 -top-4 text-[9px] text-red font-mono">SL {sl?.toFixed(1)}</div>

                {/* Current Price Marker */}
                <div
                  className={`absolute top-1 bottom-1 w-2 rounded ${isProfitable ? 'bg-green' : 'bg-red'} transition-all duration-300`}
                  style={{ left: `${Math.max(0, Math.min(pricePct, 100))}%` }}
                />
              </div>

              {/* Time Progress Bar */}
              <div className="mb-1">
                <div className="flex justify-between text-[10px] text-slate-500 mb-0.5">
                  <span>Time used</span>
                  <span>{timeUsed.toFixed(0)}%</span>
                </div>
                <div className="h-1.5 bg-slate-700 rounded-full overflow-hidden">
                  <div
                    className={`h-full rounded-full transition-all ${
                      timeUsed >= 75 ? 'bg-red' : timeUsed >= 50 ? 'bg-yellow' : 'bg-blue-400'
                    }`}
                    style={{ width: `${timeUsed}%` }}
                  />
                </div>
                {timeUsed >= 50 && (
                  <div className="text-[9px] text-yellow mt-0.5">
                    {timeUsed >= 75 ? '⚠️ SL moved to break-even' : '⚠️ SL tightened 40%'}
                  </div>
                )}
              </div>

              {/* MFE / MAE */}
              <div className="flex gap-4 text-[10px] font-mono">
                <span className="text-green">MFE: +{(pos.mfe ?? 0).toFixed(2)}%</span>
                <span className="text-red">MAE: -{(pos.mae ?? 0).toFixed(2)}%</span>
              </div>
            </div>
          )
        })}
      </div>
    </div>
  )
}
```

---

## 📦 الملف 13: تحديث `SystemHealthCard.tsx` — صحة النظام الشاملة

```tsx
/**
 * System Health & Circuit Breakers — نسخة متقدمة
 * يعرض: حالة كل circuit breaker + WS latency + API rate + block reasons
 */
import { Activity, Wifi, AlertTriangle, Shield } from 'lucide-react'
import { useTruthState } from '../hooks/useTruthState'

export default function SystemHealthCard() {
  const { state, connected } = useTruthState()
  const risk = (state as any)?.risk
  const truth = state?.truth
  const breakers = risk?.circuit_breakers

  return (
    <div className="panel p-4">
      <h3 className="text-sm font-semibold text-slate-400 uppercase tracking-wide mb-3 flex items-center gap-2">
        <Shield className="h-4 w-4" />
        System Health & Circuit Breakers
      </h3>

      {/* Connection Status */}
      <div className="space-y-2 text-sm mb-3">
        <div className="flex items-center gap-2">
          <Wifi className={`h-4 w-4 ${connected ? 'text-green' : 'text-red'}`} />
          <span className="text-slate-300">WebSocket:</span>
          <span className={`font-mono ${connected ? 'text-green' : 'text-red'}`}>
            {connected ? 'LIVE' : 'DISCONNECTED'}
          </span>
        </div>
        <div className="flex items-center gap-2">
          <Activity className="h-4 w-4 text-slate-500" />
          <span className="text-slate-300">WS Age:</span>
          <span className="font-mono text-slate-200">
            {truth?.last_ws_user_event_age_ms != null
              ? `${(truth.last_ws_user_event_age_ms / 1000).toFixed(1)}s`
              : '—'}
          </span>
        </div>
      </div>

      {/* Circuit Breakers */}
      {breakers && (
        <div className="space-y-2 pt-3 border-t border-panelBorder">
          <div className="text-xs text-slate-500 mb-2">Circuit Breakers</div>
          {Object.values(breakers).map((b: any) => {
            const pct = b.limit > 0 ? (b.current / b.limit) * 100 : 0
            const color = b.tripped ? 'bg-red' : pct >= 80 ? 'bg-yellow' : 'bg-green'
            return (
              <div key={b.name}>
                <div className="flex justify-between text-xs mb-0.5">
                  <span className={b.tripped ? 'text-red font-medium' : 'text-slate-400'}>
                    {b.tripped ? '⛔ ' : ''}{b.name}
                  </span>
                  <span className="font-mono text-slate-300">
                    {typeof b.current === 'number' ? b.current.toFixed(1) : b.current} / {b.limit}
                  </span>
                </div>
                <div className="h-1.5 bg-slate-700 rounded-full overflow-hidden">
                  <div
                    className={`h-full ${color} rounded-full transition-all`}
                    style={{ width: `${Math.min(pct, 100)}%` }}
                  />
                </div>
              </div>
            )
          })}
        </div>
      )}

      {/* Risk Mode */}
      {risk && (
        <div className="mt-3 pt-3 border-t border-panelBorder flex items-center justify-between">
          <span className="text-sm text-slate-400">Risk Mode:</span>
          <span className={`font-mono font-bold ${
            risk.risk_mode === 'NORMAL' ? 'text-green' :
            risk.risk_mode === 'CAUTION' ? 'text-yellow' :
            'text-red'
          }`}>
            {risk.risk_mode}
          </span>
        </div>
      )}

      {/* NO-GO Reason */}
      {truth?.no_go_reason && (
        <div className="mt-2 p-2 rounded bg-red/10 border border-red/30 flex items-center gap-2">
          <AlertTriangle className="h-4 w-4 text-red shrink-0" />
          <span className="text-xs text-red font-mono">{truth.no_go_reason}</span>
        </div>
      )}
    </div>
  )
}
```

---

## 📦 الملف 14: تحديث `App.tsx` + Pages

### إضافة اللوحات الجديدة في الصفحات المناسبة:

```tsx
// ═══ imports جديدة في الصفحات ═══
import RegimePanel from '../components/RegimePanel'
import VPINMonitor from '../components/VPINMonitor'
import FuturesIntelPanel from '../components/FuturesIntelPanel'
import TradeJournalDash from '../components/TradeJournalDash'
import TripleBarrierViz from '../components/TripleBarrierViz'

// ═══ Overview.tsx — أضف اللوحات الجديدة ═══
// في العمود الأيسر (col-span-3):
<RegimePanel />
<SignalEnsembleCard />

// في العمود الأوسط (col-span-5):
<AggregatePositionsPanel />
<TripleBarrierViz />

// في العمود الأيمن (col-span-4):
<SystemHealthCard />
<VPINMonitor />
<FuturesIntelPanel />
<TradeJournalDash />

// ═══ Live.tsx — أضف ═══
// العمود الأيسر:
<RegimePanel />
<VPINMonitor />

// الأوسط:
<TripleBarrierViz />

// ═══ Strategy.tsx — أضف ═══
<RegimePanel />
<FuturesIntelPanel />

// ═══ Risk.tsx — أضف ═══
// SystemHealthCard الجديد يعرض circuit breakers حقيقية
```

---

## ✅ قائمة التحقق — المرحلة 4

```
□ src/api/ws.rs — WebSocket Push مع snapshot + tick + events
□ dashboard_v2/src/hooks/useWebSocket.ts — WS hook مع reconnect
□ dashboard_v2/src/hooks/useTruthState.ts — v2 مع WS أولاً ثم polling fallback
□ src/api/rest.rs — 7 endpoints جديدة
□ dashboard_v2/src/lib/api.ts — TypeScript interfaces + fetch functions
□ SymbolsTickerRow.tsx — أسعار حية من API
□ SignalEnsembleCard.tsx — إشارات مرجحة حقيقية
□ RegimePanel.tsx — 🆕 نوع السوق + مؤشرات
□ VPINMonitor.tsx — 🆕 التدفق السام + إنذارات
□ FuturesIntelPanel.tsx — 🆕 استخبارات Futures
□ TradeJournalDash.tsx — 🆕 تحليلات + منحنى PnL
□ TripleBarrierViz.tsx — 🆕 مراقبة الصفقة بين الحواجز
□ SystemHealthCard.tsx — ترقية مع circuit breakers حقيقية
□ تحديث صفحات Overview, Live, Strategy, Risk
□ تحقق: npm run build بدون أخطاء
□ تحقق: WebSocket يتصل ويستقبل بيانات حية
□ تحقق: كل المكونات تعرض بيانات حقيقية
□ تحقق: لا مكونات hardcoded متبقية
```

---

*انتهت المرحلة 4 — انتقل إلى 06_PHASE_5_STRATEGY_ASSEMBLY.md*
