# 📁 PHASE 3 — الذكاء الذاتي
## الأسبوع 6 | الأولوية: ⚡ خارقة
## البوت يتعلم من أخطائه ويتحسن تلقائياً
## ⚠️ المراحل 0+1+2 يجب أن تعمل 100% قبل البدء

---

## 🎯 هدف المرحلة

```
المشكلة: البوت يكرر نفس الأخطاء — لا يتعلم
المشكلة: لا يعرف أداءه الحقيقي — PnL وهمي
المشكلة: صفقات "ميتة" تحتجز رأس المال

بعد هذه المرحلة:
✅ يسجّل كل صفقة بتفاصيلها الكاملة
✅ يخرج من الصفقات الميتة (Triple Barrier)
✅ يعدّل أوزانه تلقائياً بعد كل 50 صفقة
✅ يحسب PnL الحقيقي (مع fees + slippage)
✅ يحاكي أسوأ السيناريوهات (Monte Carlo)
```

---

## 📦 ملف 1: `src/analytics/mod.rs`

```rust
pub mod trade_journal;
pub mod pnl_calculator;
pub mod learning_engine;
pub mod monte_carlo;

pub use trade_journal::{TradeJournal, TradeRecord};
pub use pnl_calculator::{PnLCalculator, PnLReport};
pub use learning_engine::LearningEngine;
pub use monte_carlo::MonteCarloSimulator;
```

---

## 📦 ملف 2: `src/analytics/trade_journal.rs`

**الوظيفة**: يسجّل كل صفقة بكل تفاصيلها — أساس التعلم

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeRecord {
    // === الهوية ===
    pub id: String,                  // UUID
    pub symbol: String,
    pub side: String,                // "BUY" / "SELL"

    // === الأسعار ===
    pub intended_entry: f64,         // السعر المخطط
    pub actual_entry: f64,           // السعر الفعلي (بعد slippage)
    pub exit_price: f64,
    pub stop_loss: f64,
    pub take_profit_1: f64,
    pub take_profit_2: f64,

    // === الكميات ===
    pub quantity: f64,
    pub quote_amount: f64,

    // === الأداء ===
    pub gross_pnl: f64,             // قبل fees
    pub fees: f64,                   // Binance fees
    pub slippage_bps: f64,          // basis points
    pub net_pnl: f64,               // بعد fees + slippage
    pub net_pnl_pct: f64,
    pub mfe: f64,                   // Maximum Favorable Excursion (أقصى ربح وصلت له)
    pub mae: f64,                   // Maximum Adverse Excursion (أقصى خسارة وصلت لها)

    // === السياق ===
    pub regime: String,              // TRENDING/RANGING/VOLATILE/SQUEEZE
    pub regime_adx: f64,
    pub regime_bbw: f64,
    pub regime_hurst: f64,
    pub entropy: f64,
    pub vpin: f64,

    // === الإشارات عند الدخول ===
    pub signals_snapshot: Vec<SignalSnapshot>,
    pub weighted_score: f64,
    pub funding_rate: Option<f64>,
    pub oi_change: Option<f64>,

    // === Gate Results ===
    pub gates_passed: Vec<String>,
    pub gates_blocked: Vec<String>,

    // === التوقيت ===
    pub opened_at: String,
    pub closed_at: String,
    pub duration_secs: u64,
    pub close_reason: String,       // "TP1", "TP2", "SL", "TRAILING", "TIME_EXIT", "MANUAL"

    // === Execution Quality ===
    pub execution_latency_ms: u64,
    pub fill_quality: f64,          // actual_entry / intended_entry
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalSnapshot {
    pub name: String,
    pub value: f64,
    pub direction: f64,
    pub confidence: f64,
    pub weight: f64,
    pub contribution: f64,
}
```

**Functions**:
```rust
pub struct TradeJournal {
    records: Arc<RwLock<Vec<TradeRecord>>>,
    max_records: usize,  // 1000
}

impl TradeJournal {
    pub fn new(max_records: usize) -> Self;
    pub fn record(&self, trade: TradeRecord);
    pub fn get_recent(&self, count: usize) -> Vec<TradeRecord>;
    pub fn get_by_regime(&self, regime: &str, count: usize) -> Vec<TradeRecord>;

    /// إحصائيات سريعة لآخر N صفقة
    pub fn quick_stats(&self, count: usize) -> JournalStats;

    /// إحصائيات حسب regime
    pub fn stats_by_regime(&self, count: usize) -> HashMap<String, JournalStats>;
}

#[derive(Debug, Clone, Serialize)]
pub struct JournalStats {
    pub total_trades: u32,
    pub wins: u32,
    pub losses: u32,
    pub win_rate: f64,
    pub avg_win: f64,
    pub avg_loss: f64,
    pub profit_factor: f64,    // total_wins / total_losses
    pub avg_duration_secs: f64,
    pub avg_mfe: f64,
    pub avg_mae: f64,
    pub best_trade: f64,
    pub worst_trade: f64,
    pub total_net_pnl: f64,
    pub avg_slippage_bps: f64,
    pub total_fees: f64,
}
```

---

## 📦 ملف 3: `src/exit/triple_barrier.rs` — ⚡ من López de Prado

**لماذا خارق**: Triple Barrier Method — من كتاب "Advances in Financial Machine Learning" (2018) لـ Marcos López de Prado. يُستخدم في AQR, Two Sigma, DE Shaw.

**الفكرة**: 3 حواجز — أيهما يُلمس أولاً:
```
┌─────────────────────── Take Profit (الحاجز العلوي) ───┐
│                                                         │
│          ╱╲    ╱╲                                       │
│    ─────╱──╲──╱──╲───── Price Path ────────            │
│   ╱          ╲                           ╲              │
│──╱────────────╲───────────────────────────╲─           │
│                                                         │
├─────────────────────── Entry Price ────────────────────┤
│                                                         │
└─────────────────────── Stop Loss (الحاجز السفلي) ─────┘
                    │← Time Limit (الحاجز الزمني) →│
```

**Progressive Tightening** — الحواجز تضيق مع الوقت:
```
عند 50% من الوقت:  SL يضيق 40%
عند 75% من الوقت:  SL يتحرك إلى Break-Even
عند 100% من الوقت: إغلاق قسري (TIME_EXIT)
```

**Time Limits حسب Regime**:
```
TRENDING:  30 دقيقة  (الاتجاه يحتاج وقت)
RANGING:   12 دقيقة  (سريع الانعكاس)
VOLATILE:  15 دقيقة
SQUEEZE:   20 دقيقة  (ننتظر الانفجار)
```

**Specification**:
```rust
pub mod triple_barrier;  // أضف في src/exit/mod.rs

#[derive(Debug, Clone, Serialize)]
pub struct BarrierConfig {
    pub take_profit_pct: f64,    // 0.015 = 1.5%
    pub stop_loss_pct: f64,      // 0.007 = 0.7%
    pub time_limit_secs: u64,    // حسب regime
    pub tighten_at_50pct: f64,   // 0.4 = tighten SL by 40%
    pub breakeven_at_75pct: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct BarrierState {
    pub config: BarrierConfig,
    pub entry_price: f64,
    pub current_sl: f64,         // SL الحالي (قد يتغيّر مع الوقت)
    pub current_tp: f64,
    pub elapsed_secs: u64,
    pub time_pct: f64,           // elapsed / time_limit
    pub phase: String,           // "NORMAL", "TIGHTENED", "BREAKEVEN", "EXPIRED"
}

pub struct TripleBarrierManager;

impl TripleBarrierManager {
    /// إنشاء config حسب regime
    pub fn config_for_regime(regime: &str, atr: f64, entry_price: f64) -> BarrierConfig;

    /// تحديث حالة الحواجز
    pub fn update(state: &mut BarrierState, current_price: f64, elapsed_secs: u64)
        -> Option<String>;  // None = still open, Some("TP"/"SL"/"TIME_EXIT")

    /// هل يجب الإغلاق؟
    pub fn should_exit(state: &BarrierState, current_price: f64) -> Option<String>;
}
```

**ATR-based barriers**:
```
Take Profit = entry + ATR × 3.0
Stop Loss   = entry - ATR × 1.5
(يتم ضربها بـ regime multiplier)
```

---

## 📦 ملف 4: `src/analytics/pnl_calculator.rs` — الحساب الحقيقي

**المشكلة**: الكثير يحسبون PnL بدون fees + slippage → أرباح وهمية

```rust
#[derive(Debug, Clone, Serialize)]
pub struct PnLReport {
    pub gross_pnl: f64,
    pub total_fees: f64,          // 0.1% × 2 sides = 0.2% per trade
    pub total_slippage: f64,
    pub net_pnl: f64,
    pub net_pnl_pct: f64,
    pub effective_win_rate: f64,  // بعد fees
    pub break_even_win_rate: f64, // minimum win rate to be profitable
    pub profit_factor: f64,
    pub sharpe_daily: f64,
    pub max_drawdown_pct: f64,
    pub calmar_ratio: f64,        // annual_return / max_drawdown
}

pub struct PnLCalculator;

impl PnLCalculator {
    /// Binance Spot fees (default 0.1% maker/taker)
    const FEE_RATE: f64 = 0.001;

    /// حساب PnL لصفقة واحدة
    pub fn calculate_trade_pnl(
        entry: f64, exit: f64, quantity: f64,
        slippage_bps: f64,
    ) -> (f64, f64, f64);  // (gross_pnl, fees, net_pnl)

    /// تقرير شامل لمجموعة صفقات
    pub fn report(trades: &[TradeRecord]) -> PnLReport;

    /// Break-even win rate = 1 / (1 + avg_win/avg_loss)
    /// إذا avg_win = 1.5% و avg_loss = 1% → BEP = 40%
    pub fn break_even_rate(avg_win: f64, avg_loss: f64) -> f64;
}
```

---

## 📦 ملف 5: `src/analytics/learning_engine.rs` — ⚡ التعلم الذاتي

**الفكرة**: كل 50 صفقة، البوت يحلل أداءه ويعدّل أوزانه

```rust
pub struct LearningEngine {
    journal: Arc<TradeJournal>,
    scorer: Arc<RwLock<WeightedScorer>>,
    analysis_interval: usize,  // 50 trades
    last_analyzed_at: RwLock<usize>,  // trade count
}

#[derive(Debug, Clone, Serialize)]
pub struct LearningReport {
    pub trades_analyzed: usize,
    pub signal_accuracy: HashMap<String, SignalAccuracy>,  // per signal
    pub regime_performance: HashMap<String, RegimePerf>,   // per regime
    pub weight_adjustments: Vec<WeightAdjustment>,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SignalAccuracy {
    pub signal_name: String,
    pub times_used: u32,
    pub times_correct: u32,   // direction matched outcome
    pub accuracy: f64,
    pub avg_contribution_when_correct: f64,
    pub avg_contribution_when_wrong: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct WeightAdjustment {
    pub signal: String,
    pub regime: String,
    pub old_weight: f64,
    pub new_weight: f64,
    pub reason: String,
}

impl LearningEngine {
    pub fn new(journal: Arc<TradeJournal>, scorer: Arc<RwLock<WeightedScorer>>) -> Self;

    /// هل حان وقت التحليل؟
    pub fn should_analyze(&self) -> bool;

    /// تحليل كامل + تعديل أوزان
    pub fn analyze_and_adjust(&self) -> LearningReport;

    /// تحليل دقة كل إشارة حسب regime
    fn analyze_signal_accuracy(&self, trades: &[TradeRecord]) -> HashMap<String, SignalAccuracy>;

    /// تعديل الأوزان:
    /// إشارة دقتها > 60% → زيادة وزنها 10%
    /// إشارة دقتها < 40% → تقليل وزنها 20%
    /// إعادة normalize ليصبح المجموع = 1.0
    fn adjust_weights(&self, accuracy: &HashMap<String, SignalAccuracy>,
                      regime: &str) -> Vec<WeightAdjustment>;
}
```

**خوارزمية تعديل الأوزان**:
```
لكل regime:
  1. احسب دقة كل إشارة (correct predictions / total)
  2. لكل إشارة:
     if accuracy > 0.60: new_weight = old_weight × 1.10
     if accuracy < 0.40: new_weight = old_weight × 0.80
     else: keep weight
  3. Normalize: all weights / sum(all weights) = 1.0
  4. Clamp: min 0.03, max 0.35 (لا إشارة تهيمن)
  5. سجّل التعديلات
```

---

## 📦 ملف 6: `src/analytics/monte_carlo.rs`

**الفكرة**: خذ آخر 50 صفقة → اخلطها عشوائياً 10,000 مرة → ما أسوأ سيناريو؟

```rust
#[derive(Debug, Clone, Serialize)]
pub struct MonteCarloResult {
    pub simulations: usize,        // 10,000
    pub trades_per_sim: usize,
    pub median_pnl: f64,
    pub worst_5pct_pnl: f64,      // 5th percentile
    pub worst_1pct_pnl: f64,      // 1st percentile
    pub best_5pct_pnl: f64,
    pub max_drawdown_median: f64,
    pub max_drawdown_worst_5pct: f64,
    pub probability_of_loss: f64,  // % of sims that ended negative
    pub var_95: f64,               // Value at Risk 95%
}

pub struct MonteCarloSimulator;

impl MonteCarloSimulator {
    /// Bootstrap Monte Carlo — خلط عشوائي بإعادة
    /// 1. خذ PnL values من آخر N صفقة
    /// 2. لكل simulation:
    ///    a. اختر N قيم عشوائياً (مع إعادة)
    ///    b. احسب التراكمي
    ///    c. سجّل: final_pnl, max_drawdown
    /// 3. الإحصائيات: median, percentiles, VaR
    pub fn simulate(trade_pnls: &[f64], num_simulations: usize) -> MonteCarloResult;
}
```

**⚠️ Deterministic random**: استخدم seed ثابت للـ reproducibility في الاختبارات.

---

## 📦 ملف 7: `src/exit/mod.rs`

```rust
pub mod triple_barrier;
pub use triple_barrier::{TripleBarrierManager, BarrierConfig, BarrierState};
```

---

## 📦 تحديثات

### `src/app_state.rs`:
```rust
pub trade_journal: Arc<TradeJournal>,
pub learning_engine: Arc<LearningEngine>,
pub triple_barrier_states: RwLock<HashMap<String, BarrierState>>,  // position_id → barrier
```

### `src/main.rs`:
```rust
mod analytics;
mod exit;
```

### StateSnapshot:
```rust
pub journal_stats: Option<JournalStats>,
pub learning_report: Option<LearningReport>,
pub monte_carlo: Option<MonteCarloResult>,
pub barrier_states: HashMap<String, BarrierState>,
```

---

## ✅ قائمة التحقق — المرحلة 3

```
الملفات الجديدة:
□ src/analytics/mod.rs
□ src/analytics/trade_journal.rs
□ src/analytics/pnl_calculator.rs
□ src/analytics/learning_engine.rs
□ src/analytics/monte_carlo.rs
□ src/exit/mod.rs
□ src/exit/triple_barrier.rs

التحقق:
□ Trade journal يسجّل صفقة كاملة بكل الحقول
□ PnL calculator يحسب fees (0.2% per trade round-trip)
□ Triple barrier: TP hit → close, SL hit → close, time → close
□ Triple barrier: tightening عند 50% و 75% يعمل
□ Learning engine: بعد 50 صفقة يعدّل الأوزان
□ Monte Carlo: 10,000 simulation يرجع إحصائيات معقولة
□ cargo build + clippy = صفر أخطاء
```

---

*انتهت المرحلة 3 — انتقل إلى 05_PHASE_4_DASHBOARD.md*
