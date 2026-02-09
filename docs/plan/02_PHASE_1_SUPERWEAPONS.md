# 📁 PHASE 1 — الأسلحة الخارقة
## الأسبوع 3-4 | الأولوية: ⚡ خارقة
## تحوّل البوت من "يتداول" إلى "يتداول بذكاء"
## ⚠️ المرحلة 0 يجب أن تعمل 100% قبل البدء هنا

---

## 🎯 هدف المرحلة

```
الآن (بعد Phase 0): البوت يرى السوق ويستطيع التداول
المشكلة: يتداول في كل الظروف بنفس الطريقة = خسائر كثيرة
بعد هذه المرحلة: يعرف نوع السوق + يرفض الضوضاء + يكشف التدفق السام

5 أسلحة:
⚡ Regime Detection — يعرف: trending أم ranging أم volatile أم squeeze
⚡ Shannon Entropy — يرفض التداول في الضوضاء العشوائية
⚡ VPIN — يكشف "الأموال الذكية" تتحرك قبل الانهيار
⚡ Weighted Score — بديل "4 من 6" بمجموع مرجّح ذكي
⚡ Signal Decay — كل إشارة لها عمر افتراضي
```

---

## 📦 ملف 1: `src/regime/mod.rs`

```rust
pub mod detector;
pub mod entropy;
pub mod hurst;

pub use detector::{RegimeDetector, MarketRegime, RegimeState};
pub use entropy::ShannonEntropyFilter;
pub use hurst::calculate_hurst_exponent;
```

---

## 📦 ملف 2: `src/regime/hurst.rs` — ⚡ السلاح السري

**لماذا خارق**: Hurst Exponent يكشف "ذاكرة السوق". 99% من المتداولين لا يعرفونه.
- H > 0.55 = السوق له ذاكرة → Momentum strategies تعمل
- H < 0.45 = السوق يعكس نفسه → Mean Reversion تعمل
- H ≈ 0.50 = عشوائي تماماً → ⛔ لا تتداول!

**الخوارزمية — Rescaled Range (R/S) Analysis**:
```
المدخل: آخر N سعر إغلاق (N = 100)
1. قسّم السلسلة إلى نوافذ بأحجام مختلفة: [8, 16, 32, 64]
2. لكل حجم نافذة n:
   a. قسّم السلسلة إلى chunks بحجم n
   b. لكل chunk:
      - احسب المتوسط mean
      - احسب الانحرافات عن المتوسط: Y[i] = X[i] - mean
      - احسب التراكمي: Z[i] = sum(Y[0..i])
      - R = max(Z) - min(Z)  (Range)
      - S = std_dev(chunk)   (Standard deviation)
      - R/S = R / S (إذا S > 0)
   c. R/S لهذا الحجم = متوسط R/S لكل الـ chunks
3. Linear regression: log(R/S) vs log(n)
4. Hurst Exponent = slope of regression line
```

**Specification**:
```rust
/// Hurst Exponent via Rescaled Range (R/S) Analysis
/// closes: آخر 100+ سعر إغلاق
/// يرجع H في [0, 1]
pub fn calculate_hurst_exponent(closes: &[f64]) -> Option<f64> {
    // minimum 64 closes needed
    // window_sizes = [8, 16, 32, 64]
    // for each window_size:
    //   split closes into chunks
    //   for each chunk: compute R/S
    //   average R/S for this window_size
    // linear regression: log(avg_RS) vs log(window_size)
    // slope = Hurst exponent
    // clamp result to [0.0, 1.0]
}
```

**⚠️ Edge cases**:
- إذا S = 0 (كل القيم متساوية) → تخطَّ هذا الـ chunk
- إذا chunks < 2 → تخطَّ هذا الحجم
- إذا نتيجة الـ regression غير منطقية → None
- Clamp النتيجة إلى [0.0, 1.0]

---

## 📦 ملف 3: `src/regime/detector.rs` — كاشف نوع السوق

**المدخلات**: ADX (من indicators/adx.rs), BBW (من indicators/bollinger.rs), Hurst

**4 أنظمة سوق**:
```
TRENDING:  ADX > 25 AND Hurst > 0.55
RANGING:   ADX < 20 AND Hurst < 0.45
VOLATILE:  BBW > 5.0  (Bollinger Width واسع)
SQUEEZE:   BBW < 1.5 AND ADX < 20  (ضغط → انفجار قادم)
DEAD:      Entropy ≥ 0.95  (عشوائي تماماً → ⛔ لا تتداول)
```

**Struct**:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub enum MarketRegime {
    Trending,    // اتجاه واضح — momentum strategies
    Ranging,     // نطاق — mean reversion strategies
    Volatile,    // تقلب عالٍ — reduce position size
    Squeeze,     // ضغط — استعد للانفجار
    Dead,        // عشوائي — ⛔ لا تتداول
}

#[derive(Debug, Clone, Serialize)]
pub struct RegimeState {
    pub regime: MarketRegime,
    pub adx: f64,
    pub bbw: f64,           // Bollinger Band Width
    pub hurst: f64,
    pub entropy: f64,
    pub confidence: f64,     // 0-1 — كم نحن واثقون من التصنيف
    pub regime_age_secs: u64,  // منذ متى في هذا النظام
    pub recommended_rr: (f64, f64),  // (risk, reward) ratio
    pub max_position_pct: f64,  // نسبة الحجم المسموح
}
```

**Functions**:
```rust
impl RegimeDetector {
    pub fn new() -> Self;

    /// يحسب النظام الحالي من الشموع
    /// يحتاج minimum 100 شمعة مغلقة
    pub fn detect(&self, candles: &[Candle], closes: &[f64]) -> Option<RegimeState>;

    /// آخر نظام مكتشف
    pub fn current_regime(&self) -> Option<RegimeState>;
}
```

**Decision Matrix — ماذا يفعل كل نظام**:
```
TRENDING:   R:R = 1:3,  max_position = 100%, time_limit = 30min
RANGING:    R:R = 1:1.5, max_position = 70%,  time_limit = 12min
VOLATILE:   R:R = 1:2,  max_position = 50%,  time_limit = 15min
SQUEEZE:    R:R = 1:4,  max_position = 80%,  time_limit = 20min
DEAD:       ⛔ BLOCK ALL TRADES
```

---

## 📦 ملف 4: `src/regime/entropy.rs` — فلتر Shannon

**لماذا خارق**: من نظرية المعلومات (Claude Shannon, 1948). يقيس "عشوائية" السوق.
- إذا السوق عشوائي تماماً = لا يمكن التنبؤ = لا تتداول
- إذا السوق له نمط = يمكن استغلاله = تداول

**الخوارزمية**:
```
المدخل: آخر 30 شمعة مغلقة
1. صنّف كل شمعة: UP (close > open) أو DOWN (close <= open)
2. احسب الاحتمالات:
   p_up = count(UP) / 30
   p_down = count(DOWN) / 30
3. Shannon Entropy = -p_up × log₂(p_up) - p_down × log₂(p_down)
   (إذا p = 0 → p × log₂(p) = 0 بالاصطلاح)
4. النتيجة في [0, 1]:
   0 = كل الشموع اتجاه واحد = predictable تماماً
   1 = 50/50 تماماً = عشوائي تماماً
```

**القرار**:
```
Entropy ≥ 0.95  → BLOCK — ⛔ سوق عشوائي تماماً
Entropy 0.80-0.95 → REDUCE — حجم 50%
Entropy < 0.80  → CLEAR — ✅ يوجد نمط قابل للاستغلال
```

**Specification**:
```rust
pub struct ShannonEntropyFilter;

impl ShannonEntropyFilter {
    /// حساب entropy من آخر window شمعة
    pub fn calculate(candles: &[Candle], window: usize) -> Option<f64>;

    /// هل مسموح التداول؟
    /// returns (allowed, entropy_value, adjustment_factor)
    /// adjustment_factor: 1.0 = full size, 0.5 = half, 0.0 = blocked
    pub fn check(candles: &[Candle]) -> (bool, f64, f64);
}
```

---

## 📦 ملف 5: `src/signals/mod.rs`

```rust
pub mod weighted_score;
pub mod signal_decay;
pub mod vpin;

pub use weighted_score::{WeightedScorer, SignalInput, ScoringResult};
pub use signal_decay::SignalDecayManager;
pub use vpin::{VPINCalculator, VPINState};
```

---

## 📦 ملف 6: `src/signals/vpin.rs` — ⚡ كاشف التدفق السام

**لماذا خارق**: VPIN (Volume-Synchronized Probability of Informed Trading) — من أبحاث Marcos López de Prado. مستخدم في NASDAQ الرسمي. تنبّأ بـ Flash Crash 2010 قبل حدوثه.

**الخوارزمية**:
```
المدخل: آخر 1000 صفقة مصنّفة (من TradeStreamProcessor)
1. قسّم الصفقات إلى 50 "دلو" (bucket) بحجم متساوي
   - حجم الدلو = total_volume / 50
   - كل دلو يمتلئ حتى يصل إلى الحجم المطلوب
2. لكل دلو:
   - imbalance = |buy_volume - sell_volume| / bucket_volume
3. VPIN = mean(all bucket imbalances)
4. النتيجة في [0, 1]:
   - 0 = متوازن تماماً = آمن
   - 1 = كل التدفق في اتجاه واحد = خطر!
```

**القرار**:
```
VPIN < 0.25  → SAFE ✅ — تداول عادي
VPIN 0.25-0.45 → CAUTION ⚠️ — حجم 50%
VPIN > 0.45  → BLOCK ⛔ — لا تتداول!

⚠️ COMBO الخطير: VPIN > 0.45 + Spread ↑ في نفس الوقت = خطر كارثي
   → IMMEDIATE BLOCK + alert
```

**Specification**:
```rust
#[derive(Debug, Clone, Serialize)]
pub struct VPINState {
    pub vpin: f64,
    pub status: String,     // "SAFE", "CAUTION", "DANGER"
    pub bucket_count: usize,
    pub avg_imbalance: f64,
    pub last_update_ms: i64,
    pub spread_increasing: bool,  // combo check
}

pub struct VPINCalculator {
    num_buckets: usize,  // 50
}

impl VPINCalculator {
    pub fn new(num_buckets: usize) -> Self;

    /// حساب VPIN من صفقات مصنّفة
    pub fn calculate(&self, trades: &[ClassifiedTrade]) -> Option<VPINState>;

    /// combo check: VPIN + spread
    pub fn check_toxic_combo(&self, vpin: f64, current_spread_bps: f64,
                              avg_spread_bps: f64) -> bool;
}
```

---

## 📦 ملف 7: `src/signals/weighted_score.rs` — بديل "4 من 6"

**المشكلة مع "4 من 6"**: كل الإشارات لها نفس الوزن. Momentum و Trend متلازمتان (correlation ~0.75).

**الحل — مجموع مرجّح**:
```
Score = Σ(weight × confidence × direction × freshness)
direction: +1 (buy) أو -1 (sell) أو 0 (neutral)
freshness: e^(-0.693 × age / half_life)  ← signal decay
```

**الأوزان الأولية** (تتغيّر حسب Regime):
```
TRENDING regime:
  CVD = 0.22, OrderBook = 0.18, Trend(EMA) = 0.20, RSI = 0.13,
  Momentum(ROC) = 0.12, Volatility(BBW) = 0.08, Hurst = 0.07

RANGING regime:
  RSI = 0.25, OrderBook = 0.20, CVD = 0.18, Volatility = 0.15,
  Trend = 0.08, Momentum = 0.07, Hurst = 0.07

VOLATILE regime:
  OrderBook = 0.25, CVD = 0.22, Volatility = 0.20, RSI = 0.13,
  Trend = 0.08, Momentum = 0.07, Hurst = 0.05
```

**القرار**:
```
score > +0.35  → BUY signal
score < -0.35  → SELL signal
between        → NEUTRAL — لا تفعل شيء
```

**Specification**:
```rust
#[derive(Debug, Clone, Serialize)]
pub struct SignalInput {
    pub name: String,       // "RSI", "EMA", "CVD", etc.
    pub direction: f64,     // +1, -1, 0
    pub confidence: f64,    // 0-1
    pub age_secs: f64,      // عمر الإشارة بالثواني
}

#[derive(Debug, Clone, Serialize)]
pub struct ScoringResult {
    pub total_score: f64,
    pub decision: String,        // "BUY", "SELL", "NEUTRAL"
    pub signal_contributions: Vec<SignalContribution>,
    pub regime_used: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SignalContribution {
    pub name: String,
    pub weight: f64,
    pub confidence: f64,
    pub direction: f64,
    pub freshness: f64,
    pub contribution: f64,  // weight × confidence × direction × freshness
}

pub struct WeightedScorer {
    weights: HashMap<String, HashMap<String, f64>>,  // regime → signal → weight
}

impl WeightedScorer {
    pub fn new() -> Self;  // يُحمّل الأوزان الأولية
    pub fn score(&self, signals: &[SignalInput], regime: &str) -> ScoringResult;
    pub fn update_weights(&mut self, regime: &str, signal: &str, new_weight: f64);
}
```

---

## 📦 ملف 8: `src/signals/signal_decay.rs` — نظام نصف العمر

**الفكرة**: إشارة Order Book عمرها 30 ثانية = ميتة. إشارة EMA عمرها 30 ثانية = طازجة.

**Half-Life لكل إشارة**:
```
OrderBook:   3 ثوانٍ    (بيانات تتغير بسرعة جنونية)
CVD:         10 ثوانٍ
Momentum:    30 ثانية
RSI:         45 ثانية
Trend (EMA): 90 ثانية   (بطيء التغيّر)
Volatility:  300 ثانية  (5 دقائق)
Hurst:       600 ثانية  (10 دقائق)
```

**المعادلة**: `freshness = e^(-0.693 × age_secs / half_life)`

**Specification**:
```rust
pub struct SignalDecayManager {
    half_lives: HashMap<String, f64>,  // signal_name → half_life_secs
    last_updates: RwLock<HashMap<String, Instant>>,
}

impl SignalDecayManager {
    pub fn new() -> Self;  // يحمّل half-lives الافتراضية
    pub fn record_update(&self, signal_name: &str);  // يسجّل وقت آخر تحديث
    pub fn get_freshness(&self, signal_name: &str) -> f64;  // 0-1
    pub fn get_age_secs(&self, signal_name: &str) -> f64;
    pub fn is_alive(&self, signal_name: &str) -> bool;  // freshness > 0.1
}
```

---

## 📦 تحديثات مطلوبة

### تحديث `src/app_state.rs`:
أضف إلى AppState:
```rust
pub regime_detector: Arc<RwLock<RegimeDetector>>,
pub weighted_scorer: Arc<RwLock<WeightedScorer>>,
pub signal_decay: Arc<SignalDecayManager>,
pub vpin_calculators: RwLock<HashMap<String, Arc<VPINCalculator>>>,
```

أضف إلى StateSnapshot:
```rust
pub regime: Option<RegimeState>,
pub scoring: Option<ScoringResult>,
pub vpin: HashMap<String, VPINState>,
```

### تحديث `src/main.rs`:
أضف `mod regime;` و `mod signals;`

---

## ✅ قائمة التحقق — المرحلة 1

```
الملفات الجديدة:
□ src/regime/mod.rs
□ src/regime/hurst.rs — Hurst Exponent
□ src/regime/detector.rs — Regime Detection
□ src/regime/entropy.rs — Shannon Entropy
□ src/signals/mod.rs
□ src/signals/vpin.rs — VPIN Calculator
□ src/signals/weighted_score.rs — Weighted Scoring
□ src/signals/signal_decay.rs — Half-Life System

التحقق:
□ Hurst على بيانات BTCUSDT يرجع 0.4-0.6 (طبيعي)
□ Entropy على 30 شمعة يرجع 0.7-1.0
□ Regime detector يصنّف السوق بشكل منطقي
□ VPIN يرجع 0-1 مع 1000+ صفقة
□ Weighted score يرجع -1 إلى +1
□ Signal decay: إشارة عمرها = half_life → freshness ≈ 0.5
□ cargo build + clippy = صفر أخطاء
```

---

*انتهت المرحلة 1 — انتقل إلى 03_PHASE_2_FUTURES_INTEL.md*
