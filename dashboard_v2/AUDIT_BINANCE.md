# تدقيق ربط المنصة بباينانس والبيانات الحقيقية

## مصدر الرصيد من باينانس

- **الباكند:** عند وجود `BINANCE_API_KEY` و `BINANCE_SECRET` في `backend/.env`:
  - حلقة **Reconcile** تستدعي `GET /api/v3/account` من باينانس وتحدّث `state.balances`.
  - **User Stream** (WebSocket) يحدّث الرصيد فوراً عند أحداث `outboundAccountPosition`.
- **الداشبورد:** يعرض `state.balances` و `state.positions` من الـ API. الرصيد المعروض هو رصيد باينانس عند تفعيل المفاتيح وإعادة تشغيل البوت.

---

## كل عنصر في المنصة — هل مربوط ببيانات حقيقية؟

| العنصر في المنصة | المصدر | الحالة |
|------------------|--------|--------|
| **الهيدر:** System, Account, State, WS, Recon, Risk | `state.truth` + `runtime_config` | ✓ مربوط |
| **Symbols Ticker Row** (أسعار / RSI / Spread) | `state.market_data.symbols` + `runtime_config.symbols` | ✓ مربوط |
| **Aggregate Positions** (إجمالي + رسم) | `state.positions` + `state.balances` | ✓ مربوط |
| **Position Timelines** | `state.positions` | ✓ مربوط |
| **Position Management** (BTC/ETH/…) | `state.balances` | ✓ مربوط |
| **Binance Twin State** (Synced / Drift) | `state.truth.reconcile_last_error` + `reconcile_last_ok_age_s` | ✓ مربوط |
| **System Health & Circuit Breakers** | `state.truth` + `state.risk` + `state.rate_limits` | ✓ مربوط |
| **Market Regime** | `state.regime` | ✓ مربوط (يحتاج شموع من باينانس) |
| **Signal Ensemble** | `state.scoring` | ✓ مربوط (يحتاج استراتيجية + بيانات) |
| **VPIN Monitor** | `state.vpin` | ✓ مربوط (يحتاج تدفق تداول) |
| **Futures Intelligence** | `state.futures_intel` | ✓ مربوط (يستخدم Binance Futures API) |
| **Trade Eligibility** | `state.scoring` + `state.risk` | ✓ مربوط |
| **Live Decision Pipeline** | `state.recent_decisions[0]` | ✓ مربوط |
| **Rejection Reasons** | `state.recent_decisions` (blocking_layer / reason) | ✓ مربوط |
| **Two-Stage Entry (Insurance)** | `state.recent_decisions[0]` (insurance_verdict, risk_verdict, final_decision) | ✓ مربوط |
| **Limits & Breakers** (Risk page) | `state.runtime_config` + `state.risk.remaining_daily_loss_pct` | ✓ مربوط |
| **Exposure (Total / Per-symbol cap)** | `state.balances` + `state.positions` + `runtime_config.max_concurrent_positions` | ✓ مربوط |
| **Reconcile History** (Diagnostics) | `state.truth.reconcile_last_ok_age_s` + `reconcile_last_error` | ✓ مربوط |
| **Recent Errors** | `state.recent_errors` | ✓ مربوط |
| **Action Log** | `state.recent_decisions` | ✓ مربوط |
| **AI Observer** (Anomalies / Last Audit) | `state.recent_errors` + `state.scoring` | ✓ مربوط |
| **Trade Journal** | `state.journal_stats` | ✓ مربوط |
| **Triple Barrier** | `state.positions` | ✓ مربوط |

---

## رسائل "لا توجد بيانات" (عند عدم الربط)

- **No regime data yet** — تحتاج مفاتيح API وشموع حية من باينانس.
- **No signal data yet** — تحتاج بيانات سوق وخط أنابيب استراتيجية.
- **Waiting for VPIN data** — تحتاج تدفق تداول (Trade Stream).
- **Waiting for futures data** — تحتاج تفعيل Futures Intelligence في الباكند.
- **No balance data** — ضع `BINANCE_API_KEY` و `BINANCE_SECRET` في `backend/.env` وأعد تشغيل البوت.

كل ما له اسم في المنصة الآن يعتمد على `state` أو الـ API؛ لا توجد قيم ثابتة وهمية للرصيد أو المراكز أو التوأمة أو المؤشرات.
