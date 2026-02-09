# 📁 PHASE 0 — الأساسيات المفقودة
## الأسبوع 1-2 | الأولوية: 🔴🔴🔴 حرجة
## بدون هذه المرحلة = لا فائدة من أي شيء آخر
## ⚠️ اقرأ 00_MASTER_ROADMAP.md أولاً

---

## 🎯 هدف المرحلة

```
الآن: البوت أعمى + أصم + مشلول
بعدها: يرى السوق + يحلله + يتداول + يحمي رأس المال

الملفات الجديدة: ~15 ملف
الأسطر المقدّرة: ~3,500-4,500 سطر Rust
```

---

## ⚙️ الخطوة 0: تحديث Cargo.toml

أضف في `[dependencies]`:
```toml
hmac = "0.12"
sha2 = "0.10"
hex = "0.4"
```
**تأكد** أن هذه موجودة أصلاً (وإلا أضفها):
```toml
tokio = { version = "1", features = ["full"] }
tokio-tungstenite = { version = "0.21", features = ["native-tls"] }
futures-util = "0.3"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
chrono = { version = "0.4", features = ["serde"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
anyhow = "1"
uuid = { version = "1", features = ["v4"] }
parking_lot = "0.12"
reqwest = { version = "0.12", features = ["json", "native-tls"] }
axum = { version = "0.7", features = ["ws"] }
tower-http = { version = "0.5", features = ["cors"] }
```

---

## 📦 ملف 1: `src/market_data/mod.rs` — تعريف Module

**المسار**: `backend/src/market_data/mod.rs`
**الوظيفة**: يجمع modules بيانات السوق

```rust
//! Market Data Module — عيون البوت على السوق
pub mod candle_buffer;
pub mod orderbook;
pub mod trade_stream;

pub use candle_buffer::{CandleBuffer, Candle, CandleKey};
pub use orderbook::{OrderBookManager, OrderBookSnapshot, OrderBookMetrics};
pub use trade_stream::{TradeStreamProcessor, ClassifiedTrade, CVDState, TradeSide};
```

---

## 📦 ملف 2: `src/market_data/candle_buffer.rs`

**المسار**: `backend/src/market_data/candle_buffer.rs`
**الوظيفة**: Ring buffer للشموع الحية من Binance Kline WS
**المواصفات**:
- يتصل بـ combined WS stream (عدة symbols × عدة timeframes)
- يحفظ آخر 500 شمعة لكل symbol/timeframe
- يُحدّث الشمعة المفتوحة ويضيف الجديدة عند الإغلاق
- Thread-safe (parking_lot::RwLock)
- يدعم: 1m, 5m, 15m, 1h

**Struct الأساسي**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candle {
    pub open_time: i64,      // ms timestamp
    pub close_time: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub quote_volume: f64,
    pub trades_count: u64,
    pub taker_buy_volume: f64,
    pub taker_buy_quote_volume: f64,
    pub is_closed: bool,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct CandleKey {
    pub symbol: String,       // "BTCUSDT"
    pub interval: String,     // "1m", "5m", "15m", "1h"
}
```

**Functions المطلوبة**:
```rust
impl CandleBuffer {
    pub fn new(max_candles: usize) -> Self;
    pub fn update(&self, key: &CandleKey, candle: Candle);
    pub fn get_closed(&self, key: &CandleKey, count: usize) -> Vec<Candle>;
    pub fn get_closes(&self, key: &CandleKey, count: usize) -> Vec<f64>;
    pub fn last_close(&self, key: &CandleKey) -> Option<f64>;
    pub fn count(&self, key: &CandleKey) -> usize;
}
```

**WS Stream Function**:
```rust
/// يتصل بـ wss://stream.binance.com:9443/stream?streams=btcusdt@kline_1m/...
/// Combined stream format: {"stream":"btcusdt@kline_1m","data":{kline payload}}
pub async fn run_kline_stream(
    buffer: Arc<CandleBuffer>,
    symbols: Vec<String>,
    intervals: Vec<String>,
) -> anyhow::Result<()>;
```

**Binance Kline WS payload parsing** — الحقول:
```
v["s"] = symbol, k["i"] = interval
k["t"] = open_time, k["T"] = close_time
k["o"/"h"/"l"/"c"] = OHLC (strings → parse to f64)
k["v"] = volume, k["q"] = quote_volume
k["n"] = trades_count, k["V"] = taker_buy_volume
k["Q"] = taker_buy_quote_volume, k["x"] = is_closed (bool)
```

**⚠️ مهم**: reconnect loop مع sleep 5 ثوانٍ عند الانقطاع.

---

## 📦 ملف 3: `src/market_data/trade_stream.rs`

**المسار**: `backend/src/market_data/trade_stream.rs`
**الوظيفة**: تصنيف الصفقات + حساب CVD
**لماذا**: أساس لـ VPIN (Phase 1) و Order Flow Analysis

**Struct الأساسي**:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub enum TradeSide {
    BuyerInitiated,   // المشتري أخذ من Ask (taker buy)
    SellerInitiated,  // البائع أخذ من Bid (taker sell)
}

#[derive(Debug, Clone, Serialize)]
pub struct ClassifiedTrade {
    pub price: f64,
    pub quantity: f64,
    pub quote_qty: f64,
    pub time: i64,
    pub side: TradeSide,
}

#[derive(Debug, Clone, Serialize)]
pub struct CVDState {
    pub cvd: f64,           // Cumulative Volume Delta (buy_vol - sell_vol)
    pub buy_volume: f64,
    pub sell_volume: f64,
    pub trade_count: u64,
    pub last_update_ms: i64,
}
```

**⚠️ تحذير حرج — Binance "m" field**:
```
Binance aggTrade: m = is_buyer_maker
m=true  → buyer is maker → SELLER initiated the trade (taker sell)
m=false → seller is maker → BUYER initiated the trade (taker buy)
هذا عكس ما يتوقعه الكثيرون! تحقق من التوثيق الرسمي.
```

**Functions المطلوبة**:
```rust
impl TradeStreamProcessor {
    pub fn new(symbol: String, max_trades: usize) -> Self;   // max_trades = 5000
    pub fn process_trade(&self, price: f64, qty: f64, time: i64, is_buyer_maker: bool);
    pub fn get_cvd(&self) -> CVDState;
    pub fn get_recent_trades(&self, count: usize) -> Vec<ClassifiedTrade>;
    pub fn reset_cvd(&self);  // يومياً
}

/// يتصل بـ wss://stream.binance.com:9443/ws/{symbol}@aggTrade
pub async fn run_trade_stream(processor: Arc<TradeStreamProcessor>, symbol: &str);
```

**Binance aggTrade payload**:
```
v["p"] = price (string), v["q"] = quantity (string)
v["T"] = trade time (ms), v["m"] = is_buyer_maker (bool)
```

---

## 📦 ملف 4: `src/market_data/orderbook.rs`

**المسار**: `backend/src/market_data/orderbook.rs`
**الوظيفة**: Order Book محلي + حساب Spread/Imbalance/Depth

**Struct الأساسي**:
```rust
#[derive(Debug, Clone, Serialize)]
pub struct OrderBookLevel {
    pub price: f64,
    pub quantity: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct OrderBookSnapshot {
    pub symbol: String,
    pub bids: Vec<OrderBookLevel>,  // مرتّب تنازلياً بالسعر
    pub asks: Vec<OrderBookLevel>,  // مرتّب تصاعدياً بالسعر
    pub last_update_id: u64,
    pub timestamp_ms: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct OrderBookMetrics {
    pub best_bid: f64,
    pub best_ask: f64,
    pub spread: f64,
    pub spread_bps: f64,       // basis points = (spread/mid)*10000
    pub mid_price: f64,
    pub bid_depth_5: f64,      // total volume top 5 bid levels
    pub ask_depth_5: f64,
    pub imbalance: f64,        // (bid-ask)/(bid+ask) range [-1,+1]
    pub weighted_mid: f64,     // volume-weighted mid price
}
```

**Functions المطلوبة**:
```rust
impl OrderBookManager {
    pub fn new() -> Self;
    pub fn update_from_depth(&self, symbol: &str, bids: Vec<OrderBookLevel>,
                              asks: Vec<OrderBookLevel>, update_id: u64);
    pub fn get_metrics(&self, symbol: &str) -> Option<OrderBookMetrics>;
    pub fn get_snapshot(&self, symbol: &str) -> Option<OrderBookSnapshot>;
}

/// يتصل بـ depth20@100ms stream لكل symbol
pub async fn run_depth_stream(manager: Arc<OrderBookManager>, symbols: Vec<String>);
```

**Binance depth payload**: `v["b"]` = bids array, `v["a"]` = asks array, كل عنصر = `[price_str, qty_str]`

---

## 📦 ملف 5: `src/indicators/mod.rs` + 6 ملفات مؤشرات

**المسار**: `backend/src/indicators/mod.rs`

```rust
pub mod ema;
pub mod rsi;
pub mod adx;
pub mod bollinger;
pub mod atr;
pub mod roc;
```

### `src/indicators/ema.rs`:
```rust
/// EMA — Exponential Moving Average
/// multiplier = 2/(period+1), أول قيمة = SMA
pub fn calculate_ema(closes: &[f64], period: usize) -> Vec<f64>;

/// هل EMAs مرتبة صعودياً (EMA9 > EMA21 > EMA55)?
/// يرجع (is_bullish, strength) — strength = |EMA9-EMA55|/EMA55
pub fn ema_trend_aligned(closes: &[f64]) -> Option<(bool, f64)>;
```

### `src/indicators/rsi.rs`:
```rust
/// RSI — Relative Strength Index (Wilder's smoothing, not simple MA)
/// avg_gain = (prev_avg × (period-1) + current_gain) / period
pub fn calculate_rsi(closes: &[f64], period: usize) -> Vec<f64>;

/// RSI الحالي مع تصنيف: ≥70 OVERBOUGHT, ≤30 OVERSOLD, else NEUTRAL
pub fn current_rsi(closes: &[f64], period: usize) -> Option<(f64, &'static str)>;
```

### `src/indicators/adx.rs`:
```rust
/// ADX — Average Directional Index (يحتاج OHLC candles)
/// 1. حساب +DM/-DM  2. Wilder's smooth  3. +DI/-DI  4. DX  5. ADX
/// ADX > 25 = trending, < 20 = ranging
pub fn calculate_adx(candles: &[Candle], period: usize) -> Option<f64>;
```
- يحتاج `use crate::market_data::Candle;`
- **Wilder's smoothing**: `smoothed = prev - prev/period + current`

### `src/indicators/bollinger.rs`:
```rust
pub struct BollingerResult {
    pub upper: f64,           // SMA + 2×StdDev
    pub middle: f64,          // SMA(20)
    pub lower: f64,           // SMA - 2×StdDev
    pub width: f64,           // (upper-lower)/middle × 100  ← BBW (Bollinger Band Width)
    pub percent_b: f64,       // (close-lower)/(upper-lower)
}

/// Bollinger Bands — period=20, std_dev=2.0 default
pub fn calculate_bollinger(closes: &[f64], period: usize, std_dev: f64) -> Option<BollingerResult>;
```

### `src/indicators/atr.rs`:
```rust
/// ATR — Average True Range (لحساب SL/TP)
/// TR = max(H-L, |H-prevC|, |L-prevC|) → Wilder's smooth
pub fn calculate_atr(candles: &[Candle], period: usize) -> Option<f64>;
```

### `src/indicators/roc.rs`:
```rust
/// ROC — Rate of Change (Momentum)
/// ROC = ((current - past) / past) × 100
pub fn calculate_roc(closes: &[f64], period: usize) -> Option<f64>;
```

**⚠️ Edge cases في كل مؤشر**: تحقق من `closes.len() >= period`, تحقق من `divisor != 0.0`, تحقق من `result.is_finite()`.

---

## 📦 ملف 6: تحديث `src/binance/client.rs`

**المسار**: `backend/src/binance/client.rs`
**العمل**: إضافة HMAC-SHA256 signing + وظائف جديدة

**أضف هذه الوظائف إلى `impl BinanceClient`**:

```rust
// ═══ وظائف جديدة تُضاف ═══

/// HMAC-SHA256 signing
fn sign(&self, query: &str) -> String;

/// Timestamp بالمللي ثانية
fn timestamp_ms() -> u64;

/// الأرصدة — GET /api/v3/account (SIGNED)
pub async fn get_account(&self) -> anyhow::Result<serde_json::Value>;

/// رصيد asset معيّن
pub async fn get_balance(&self, asset: &str) -> anyhow::Result<f64>;

/// وضع أمر — POST /api/v3/order (SIGNED)
/// order_type: "LIMIT" أو "MARKET"
/// price: مطلوب لـ LIMIT، None لـ MARKET
/// time_in_force: "GTC" لـ LIMIT
pub async fn place_order(
    &self, symbol: &str, side: &str, order_type: &str,
    quantity: f64, price: Option<f64>, time_in_force: Option<&str>,
) -> anyhow::Result<serde_json::Value>;

/// إلغاء أمر — DELETE /api/v3/order (SIGNED)
pub async fn cancel_order(&self, symbol: &str, order_id: u64) -> anyhow::Result<serde_json::Value>;

/// أوامر مفتوحة — GET /api/v3/openOrders (SIGNED)
pub async fn get_open_orders(&self, symbol: Option<&str>) -> anyhow::Result<Vec<serde_json::Value>>;

/// شموع تاريخية — GET /api/v3/klines (PUBLIC, لا signing)
/// لملء CandleBuffer عند البدء
pub async fn get_klines(&self, symbol: &str, interval: &str, limit: u32)
    -> anyhow::Result<Vec<crate::market_data::Candle>>;

/// معلومات symbol — GET /api/v3/exchangeInfo (PUBLIC)
/// للحصول على: step_size, tick_size, min_notional
pub async fn get_symbol_info(&self, symbol: &str) -> anyhow::Result<serde_json::Value>;
```

**HMAC-SHA256 signing**:
```rust
use hmac::{Hmac, Mac};
use sha2::Sha256;

fn sign(&self, query: &str) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(self.secret.as_bytes())
        .expect("HMAC key");
    mac.update(query.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}
```

**نمط كل طلب SIGNED**:
```
1. بناء query string مع timestamp=...
2. signature = HMAC-SHA256(query, secret)
3. URL = base + endpoint + "?" + query + "&signature=" + signature
4. Header: X-MBX-APIKEY: api_key
```

**⚠️ Klines parsing**: Binance يرجع array of arrays:
```
[openTime, open, high, low, close, volume, closeTime, quoteVol, trades, takerBuyVol, takerBuyQuoteVol, ignore]
indices:  0       1     2    3    4       5        6         7        8         9            10           11
أول وآخر = numbers, الباقي = strings (يحتاج parse)
```

---

## 📦 ملف 7: إعادة بناء `src/risk.rs`

**المسار**: `backend/src/risk.rs`
**العمل**: استبدال كامل — الملف الحالي struct فقط بلا منطق

**RiskEngine — 4 Circuit Breakers**:

| Breaker | الشرط | الفعل |
|---------|-------|-------|
| Daily Loss | `daily_pnl_pct.abs() >= max_daily_loss_pct` | HALT — إيقاف كامل |
| Consecutive Losses | `consecutive_losses >= max_consecutive` | HALT — إيقاف كامل |
| Max Drawdown | `drawdown_pct >= max_drawdown_pct` | HALT — إيقاف كامل |
| Trade Limit | `daily_trades >= max_trades_per_day` | CAUTION — لا صفقات جديدة |

**Struct المطلوب**:
```rust
pub struct RiskState {
    pub risk_mode: String,         // "NORMAL", "CAUTION", "HALT", "KILLED"
    pub daily_pnl: f64,
    pub daily_pnl_pct: f64,
    pub remaining_daily_loss_pct: f64,
    pub consecutive_losses: u32,
    pub daily_trades_count: u32,
    pub daily_wins: u32,
    pub daily_losses: u32,
    pub max_drawdown_today: f64,
    pub peak_equity_today: f64,
    pub circuit_breakers: CircuitBreakers,  // 4 breakers مع current/limit/tripped
    pub current_date: String,
}

pub struct RiskEngine {
    state: Arc<RwLock<RiskState>>,
    capital: f64,
    // limits from RuntimeConfig
}
```

**Functions المطلوبة**:
```rust
impl RiskEngine {
    pub fn new(capital: f64, max_daily_loss_pct: f64, max_consecutive: u32, max_trades: u32) -> Self;
    pub fn record_trade_result(&self, pnl: f64);  // يحدّث كل شيء + يتحقق من breakers
    pub fn can_trade(&self) -> (bool, Option<String>);  // هل مسموح التداول؟
    pub fn get_state(&self) -> RiskState;
    pub fn reset_daily(&self);  // إعادة تعيين عند تغيّر التاريخ
    pub fn kill(&self);  // Emergency kill
}
```

**⚠️ مهم**: `record_trade_result` يتحقق من التاريخ — إذا تغيّر اليوم، يعيد تعيين كل الإحصائيات اليومية.

---

## 📦 ملف 8: إعادة بناء `src/position_engine.rs`

**المسار**: `backend/src/position_engine.rs`
**العمل**: من struct فقط → محرك إدارة مراكز كامل

```rust
#[derive(Debug, Clone, Serialize)]
pub struct Position {
    pub id: String,              // UUID
    pub symbol: String,
    pub side: String,            // "BUY"
    pub entry_price: f64,
    pub quantity: f64,
    pub current_price: f64,
    pub unrealized_pnl: f64,
    pub unrealized_pnl_pct: f64,
    pub stop_loss: f64,
    pub take_profit_1: f64,      // TP1 — 60% of position
    pub take_profit_2: f64,      // TP2 — 40% of position
    pub trailing_stop: Option<f64>,
    pub highest_price: f64,       // لحساب trailing
    pub status: PositionStatus,   // Open, PartialTP, Closed
    pub opened_at: String,
    pub closed_at: Option<String>,
    pub close_reason: Option<String>,  // "TP1", "TP2", "SL", "TRAILING", "MANUAL", "TIME_EXIT"
}

pub enum PositionStatus { Open, PartialTP1, Closed }

pub struct PositionManager {
    positions: Arc<RwLock<Vec<Position>>>,
    closed_positions: Arc<RwLock<Vec<Position>>>,  // آخر 100 صفقة مغلقة
}
```

**Functions المطلوبة**:
```rust
impl PositionManager {
    pub fn new() -> Self;
    pub fn open_position(&self, symbol: &str, side: &str, entry_price: f64,
                          qty: f64, sl: f64, tp1: f64, tp2: f64) -> String;  // returns position_id
    pub fn update_price(&self, symbol: &str, current_price: f64);  // يحدّث كل المراكز لهذا الرمز
    pub fn check_exits(&self) -> Vec<(String, String)>;  // → [(position_id, reason)]
    pub fn close_position(&self, id: &str, reason: &str, close_price: f64) -> Option<f64>;  // → pnl
    pub fn get_open_positions(&self) -> Vec<Position>;
    pub fn get_position(&self, id: &str) -> Option<Position>;
    pub fn get_closed_positions(&self, count: usize) -> Vec<Position>;
}
```

**SL/TP Logic في `check_exits`**:
```
لكل Position مفتوحة:
  if current_price <= stop_loss → close (reason: "SL")
  if current_price >= take_profit_2 → close كامل (reason: "TP2")
  if current_price >= take_profit_1 && status == Open → partial close 60% (reason: "TP1")
  if trailing_stop.is_some() && current_price <= trailing_stop → close (reason: "TRAILING")

  // تحديث trailing stop
  if current_price > highest_price:
    highest_price = current_price
    trailing_stop = Some(current_price - ATR * 2)  // أو نسبة ثابتة
```

---

## 📦 ملف 9: إعادة بناء `src/reconcile.rs`

**المسار**: `backend/src/reconcile.rs`
**العمل**: من `Ok(())` فارغ → مزامنة حقيقية مع Binance

```rust
/// reconcile_once:
/// 1. client.get_account() → أرصدة حقيقية
/// 2. client.get_open_orders() → أوامر حقيقية
/// 3. قارن مع state.positions
/// 4. إذا drift → log warning + تحديث state
/// 5. تحديث PositionSnapshot في app_state
pub async fn reconcile_once(state: &AppState) -> anyhow::Result<()>;
```

**⚠️ مهم**: لا تُلغِ أوامر أو تُغلق مراكز تلقائياً — فقط كشف الانحراف وتسجيله.

---

## 📦 ملف 10: تحديث `src/api/ws.rs`

**المسار**: `backend/src/api/ws.rs`
**العمل**: من "يرسل snapshot واحد ثم يصمت" → push كل 1-2 ثانية

```rust
/// بدل الكود الحالي:
/// handle_socket يرسل snapshot ثم ينتظر — لا يفعل شيء
///
/// الكود الجديد:
/// 1. أرسل full snapshot أولاً
/// 2. loop: كل 1-2 ثانية، أرسل delta/snapshot
/// 3. استمع لرسائل العميل (subscribe/unsubscribe)
/// 4. إذا انقطع → cleanup

async fn handle_socket(state, ws_sequence, socket) {
    let (mut sender, mut receiver) = socket.split();

    // إرسال snapshot أولي
    send_snapshot(&mut sender, &state, &ws_sequence).await;

    let mut interval = tokio::time::interval(Duration::from_millis(1500));
    let mut last_version = state.current_state_version();

    loop {
        tokio::select! {
            _ = interval.tick() => {
                let current_version = state.current_state_version();
                if current_version != last_version {
                    send_snapshot(&mut sender, &state, &ws_sequence).await;
                    last_version = current_version;
                }
            }
            msg = receiver.next() => {
                match msg {
                    Some(Ok(_)) => {} // handle client messages
                    _ => break,       // disconnected
                }
            }
        }
    }
}
```

---

## 📦 ملف 11: تحديث `src/app_state.rs`

**العمل**: إضافة حقول جديدة لبيانات السوق والمحركات

أضف إلى `struct AppState`:
```rust
pub candle_buffer: Arc<CandleBuffer>,
pub trade_processors: RwLock<HashMap<String, Arc<TradeStreamProcessor>>>,
pub orderbook_manager: Arc<OrderBookManager>,
pub risk_engine: Arc<RiskEngine>,
pub position_manager: Arc<PositionManager>,
```

أضف إلى `struct StateSnapshot`:
```rust
pub market_data: MarketDataSnapshot,
```

**MarketDataSnapshot** جديد:
```rust
#[derive(Clone, Serialize)]
pub struct MarketDataSnapshot {
    pub symbols: HashMap<String, SymbolMarketData>,
}

#[derive(Clone, Serialize)]
pub struct SymbolMarketData {
    pub last_price: f64,
    pub rsi_14: Option<f64>,
    pub ema_9: Option<f64>,
    pub ema_21: Option<f64>,
    pub ema_55: Option<f64>,
    pub adx: Option<f64>,
    pub atr_14: Option<f64>,
    pub bollinger_width: Option<f64>,
    pub roc_14: Option<f64>,
    pub spread_bps: Option<f64>,
    pub cvd: f64,
    pub orderbook_imbalance: f64,
    pub buy_volume_ratio: f64,
}
```

`build_snapshot()` يحسب المؤشرات لكل symbol عند كل snapshot.

---

## 📦 ملف 12: تحديث `src/main.rs`

**العمل**: إضافة spawning لجميع المحركات الجديدة

بعد `let state = Arc::new(state)`:
```rust
// 1. ملء buffer بشموع تاريخية عند البدء
// إذا كان لدينا Binance keys:
//   لكل symbol × interval: client.get_klines(symbol, interval, 500)
//   أضفها إلى candle_buffer

// 2. Kline WS Stream
// tokio::spawn(run_kline_stream(candle_buffer, symbols, intervals))

// 3. Trade Stream لكل symbol
// لكل symbol: tokio::spawn(run_trade_stream(processor, symbol))

// 4. Depth Stream
// tokio::spawn(run_depth_stream(orderbook_manager, symbols))

// 5. Health staleness check (موجود — تأكد أنه يشمل الجديد)
```

**⚠️ ترتيب**: ملء buffer التاريخي **أولاً** (await) ثم spawn الـ streams (لا await).

---

## ✅ قائمة التحقق — المرحلة 0

```
الملفات الجديدة:
□ src/market_data/mod.rs
□ src/market_data/candle_buffer.rs
□ src/market_data/trade_stream.rs
□ src/market_data/orderbook.rs
□ src/indicators/mod.rs
□ src/indicators/ema.rs
□ src/indicators/rsi.rs
□ src/indicators/adx.rs
□ src/indicators/bollinger.rs
□ src/indicators/atr.rs
□ src/indicators/roc.rs

الملفات المُحدّثة:
□ Cargo.toml — dependencies
□ src/main.rs — mod declarations + spawning
□ src/binance/client.rs — signing + orders + klines
□ src/risk.rs — إعادة بناء كامل
□ src/position_engine.rs — إعادة بناء كامل
□ src/reconcile.rs — منطق حقيقي
□ src/api/ws.rs — push loop
□ src/app_state.rs — حقول جديدة

التحقق:
□ cargo build — صفر أخطاء
□ cargo clippy --all-targets — صفر تحذيرات
□ البوت يتصل بـ Binance ويستقبل klines
□ البوت يتصل ويستقبل trades
□ البوت يتصل ويستقبل depth
□ المؤشرات تحسب قيم معقولة
□ Risk engine يتتبع الصفقات
□ Position manager يدير SL/TP
□ WebSocket يدفع updates للداشبورد
□ Reconcile يقارن state مع البورصة
```

---

*انتهت المرحلة 0 — انتقل إلى 02_PHASE_1_SUPERWEAPONS.md*
