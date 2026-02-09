# 📁 PHASE 2 — بيانات Futures الخارقة
## الأسبوع 5 | الأولوية: ⚡ خارقة
## يفتح عين البوت على 70% من السوق المخفي
## ⚠️ المراحل 0+1 يجب أن تعملا 100% قبل البدء

---

## 🎯 لماذا Futures؟

```
حقيقة: 70%+ من حجم تداول BTC يحدث في Futures وليس Spot
حقيقة: Futures يقود Spot — التحركات تبدأ هناك أولاً
حقيقة: Funding Rate + OI + L/S Ratio = أقوى إشارات contrarian

بدون هذه البيانات = كأنك تقود سيارة وترى 30% فقط من الطريق
```

---

## 📦 ملف 1: `src/futures_intel/mod.rs`

```rust
pub mod funding_rate;
pub mod open_interest;
pub mod long_short_ratio;

pub use funding_rate::{FundingRateMonitor, FundingState};
pub use open_interest::{OpenInterestTracker, OIState};
pub use long_short_ratio::{LongShortMonitor, LSState};

/// حالة Futures المجمّعة لكل symbol
#[derive(Debug, Clone, Serialize)]
pub struct FuturesIntelState {
    pub symbol: String,
    pub funding: Option<FundingState>,
    pub oi: Option<OIState>,
    pub ls_ratio: Option<LSState>,
    pub composite_signal: f64,       // -1 إلى +1
    pub composite_bias: String,      // "BULLISH", "BEARISH", "NEUTRAL"
    pub last_update: String,
}
```

---

## 📦 ملف 2: `src/futures_intel/funding_rate.rs`

**لماذا**: Funding Rate = كم يدفع longs لـ shorts (أو العكس).
- Funding مرتفع إيجابي (+0.05%+) = الكل long = contrarian SELL
- Funding سلبي (-0.03%-) = الكل short = contrarian BUY (short squeeze محتمل)

**Binance API**: `GET /fapi/v1/fundingRate?symbol=BTCUSDT&limit=1`
⚠️ هذا Futures API (fapi) وليس Spot API (api)
**Base URL**: `https://fapi.binance.com`

**Specification**:
```rust
#[derive(Debug, Clone, Serialize)]
pub struct FundingState {
    pub rate: f64,              // القيمة الحالية (مثل 0.0001 = 0.01%)
    pub rate_pct: f64,          // بالنسبة المئوية
    pub signal: f64,            // -1 إلى +1 (contrarian)
    pub bias: String,           // "SELL_BIAS", "BUY_BIAS", "NEUTRAL"
    pub next_funding_time: i64, // متى الـ funding القادم
    pub interpretation: String,
}

pub struct FundingRateMonitor {
    client: reqwest::Client,
    cache: Arc<RwLock<HashMap<String, FundingState>>>,
}

impl FundingRateMonitor {
    pub fn new() -> Self;

    /// جلب funding rate لـ symbol
    pub async fn fetch(&self, symbol: &str) -> anyhow::Result<FundingState>;

    /// جلب لكل symbols (batch)
    pub async fn fetch_all(&self, symbols: &[String]) -> HashMap<String, FundingState>;

    /// تفسير:
    /// rate > +0.05% → signal = -0.8 (SELL_BIAS — overleveraged longs)
    /// rate > +0.03% → signal = -0.4
    /// rate < -0.03% → signal = +0.5 (BUY_BIAS — short squeeze)
    /// rate < -0.05% → signal = +0.9
    /// else → signal = 0.0 (NEUTRAL)
}
```

**⚠️ Rate Limit**: Binance Futures API = 2400 requests/min. جلب كل 60 ثانية كافٍ.

---

## 📦 ملف 3: `src/futures_intel/open_interest.rs`

**لماذا**: Open Interest = عدد العقود المفتوحة.
- OI ↑ + Price ↑ = أموال جديدة تدخل = BULLISH (اتجاه قوي)
- OI ↑ + Price ↓ = أموال جديدة تدخل short = BEARISH
- OI ↓ > 10%/hr = تصفيات جماعية = ⛔ BLOCK (سوق خطير)
- OI ↓ + Price ↑ = short covering = اتجاه ضعيف (احذر)

**Binance API**: `GET /fapi/v1/openInterest?symbol=BTCUSDT`
**تاريخي**: `GET /futures/data/openInterestHist?symbol=BTCUSDT&period=5m&limit=30`

**Specification**:
```rust
#[derive(Debug, Clone, Serialize)]
pub struct OIState {
    pub current_oi: f64,
    pub oi_change_1h_pct: f64,   // تغيّر في آخر ساعة
    pub oi_change_4h_pct: f64,
    pub price_direction: f64,     // +1 صعود, -1 هبوط
    pub signal: f64,              // -1 إلى +1
    pub interpretation: String,   // "BULLISH_CONVICTION", "LIQUIDATION_CASCADE", etc.
    pub block_trading: bool,      // true إذا OI ↓ > 10%/hr
}

pub struct OpenInterestTracker {
    client: reqwest::Client,
    history: Arc<RwLock<HashMap<String, Vec<(i64, f64)>>>>,  // symbol → [(time, oi)]
}

impl OpenInterestTracker {
    pub fn new() -> Self;
    pub async fn fetch(&self, symbol: &str) -> anyhow::Result<OIState>;
    pub async fn fetch_history(&self, symbol: &str) -> anyhow::Result<Vec<(i64, f64)>>;

    /// تحديث + تحليل
    pub async fn analyze(&self, symbol: &str, current_price_direction: f64) -> anyhow::Result<OIState>;
}
```

**Decision Matrix**:
```
OI_change > +5%  AND price ↑ → signal = +0.7  "BULLISH_CONVICTION"
OI_change > +5%  AND price ↓ → signal = -0.7  "BEARISH_CONVICTION"
OI_change < -10% (1hr)       → signal = 0.0,  block = true  "LIQUIDATION_CASCADE"
OI_change < -5%  AND price ↑ → signal = +0.2  "SHORT_COVERING" (ضعيف)
else                          → signal = 0.0   "NEUTRAL"
```

---

## 📦 ملف 4: `src/futures_intel/long_short_ratio.rs`

**لماذا**: Long/Short Ratio = نسبة المتداولين الذين هم long vs short.
- استراتيجية Contrarian: الأغلبية دائماً تخسر
- > 65% long = retail overleveraged = SELL bias
- > 65% short = retail panicking = BUY bias

**Binance API**: `GET /futures/data/globalLongShortAccountRatio?symbol=BTCUSDT&period=5m&limit=1`

**Specification**:
```rust
#[derive(Debug, Clone, Serialize)]
pub struct LSState {
    pub long_pct: f64,       // نسبة Long (مثل 67.5)
    pub short_pct: f64,      // نسبة Short (مثل 32.5)
    pub ratio: f64,          // long_pct / short_pct
    pub signal: f64,         // -1 إلى +1 (contrarian)
    pub bias: String,
}

pub struct LongShortMonitor {
    client: reqwest::Client,
    cache: Arc<RwLock<HashMap<String, LSState>>>,
}

impl LongShortMonitor {
    pub fn new() -> Self;
    pub async fn fetch(&self, symbol: &str) -> anyhow::Result<LSState>;

    /// Contrarian logic:
    /// long_pct > 70% → signal = -0.9  (extreme — SELL)
    /// long_pct > 65% → signal = -0.5
    /// short_pct > 70% → signal = +0.9  (extreme — BUY)
    /// short_pct > 65% → signal = +0.5
    /// else → signal = 0.0
}
```

---

## 📦 تحديث `src/main.rs`

```rust
mod futures_intel;

// في main(), بعد المحركات الأخرى:
// spawn futures polling loop — كل 60 ثانية
// لكل symbol: fetch funding + OI + L/S ratio
// تحديث FuturesIntelState في app_state
```

**Polling Loop**:
```rust
/// كل 60 ثانية: جلب بيانات futures لكل symbol
pub async fn run_futures_intel_loop(state: Arc<AppState>) {
    let mut interval = tokio::time::interval(Duration::from_secs(60));
    let funding_monitor = FundingRateMonitor::new();
    let oi_tracker = OpenInterestTracker::new();
    let ls_monitor = LongShortMonitor::new();

    loop {
        interval.tick().await;
        let symbols = state.runtime_config.read().symbols.clone();
        for symbol in &symbols {
            // fetch all three, combine into FuturesIntelState
            // update app_state
        }
    }
}
```

## 📦 تحديث `src/app_state.rs`

أضف:
```rust
pub futures_intel: RwLock<HashMap<String, FuturesIntelState>>,
```

أضف إلى StateSnapshot:
```rust
pub futures_intel: HashMap<String, FuturesIntelState>,
```

---

## ✅ قائمة التحقق — المرحلة 2

```
الملفات الجديدة:
□ src/futures_intel/mod.rs
□ src/futures_intel/funding_rate.rs
□ src/futures_intel/open_interest.rs
□ src/futures_intel/long_short_ratio.rs

التحقق:
□ Funding Rate لـ BTCUSDT يرجع قيمة منطقية (عادة 0.0001-0.001)
□ Open Interest يرجع رقم > 0
□ L/S Ratio يرجع نسب تجمعها = 100%
□ Composite signal يرجع -1 إلى +1
□ Polling loop يعمل كل 60 ثانية بدون أخطاء
□ لا يتجاوز rate limits
□ cargo build + clippy = صفر أخطاء
```

---

*انتهت المرحلة 2 — انتقل إلى 04_PHASE_3_SELF_LEARNING.md*
