# AURORA Advanced Upgrade Roadmap

> Created: 2026-02-08
> Updated: 2026-02-09
> Status: Phase 1 IMPLEMENTED + Phase 2 IMPLEMENTED. Phase 3 (remaining) planned.

---

## Phase 1 — Structural Direction + Smart Filters (DONE)

**Goal:** Prevent buying against the market's structural direction.
**Status:** Implemented and tested (91/91 tests pass).

### Components Implemented:

| Component | File | Feature Flag | Description |
|-----------|------|-------------|-------------|
| **HTF Trend Gate** | `htf_analysis.rs` | `enable_htf_gate` | Analyzes 15M + 1H EMA alignment. Blocks BUY when structure is bearish. |
| **Score Momentum** | `smart_filters.rs` | `enable_score_momentum` | Tracks rolling score history (15 samples ~30s). Requires sustained positive readings. |
| **OFIP** | `smart_filters.rs` | `enable_ofip` | Order Flow Imbalance Persistence (45 samples ~90s). Blocks BUY on persistent sell-side. |
| **Adaptive Threshold** | `smart_filters.rs` | `enable_adaptive_threshold` | Adjusts BUY threshold based on regime + volatility + entropy. |
| **Entropy Graduated** | `smart_filters.rs` | `enable_entropy_graduated` | Graduated position sizing reduction as entropy increases. |

---

## Phase 2 — CUSUM + Absorption + Entropy Valley (DONE)

**Goal:** Detect structural trend changes before EMAs, identify institutional activity, and use entropy transitions as entry signals.
**Status:** Implemented and tested (91/91 tests pass). All flags OFF by default.

### Components Implemented:

| Component | File | Feature Flag | Description |
|-----------|------|-------------|-------------|
| **CUSUM Detector** | `cusum_detector.rs` | `enable_cusum` | Page's CUSUM Test (1954) on 5M candle returns. Dynamic threshold = 2.0 × rolling_std. Detects reversals 5-15 min before EMA. |
| **CUSUM×HTF Interaction** | `cusum_detector.rs` | (part of CUSUM) | When CUSUM detects bullish break but HTF is bearish → SOFT BLOCK (50% position, not 0%). Solves EMA lag problem. |
| **Absorption Detector** | `absorption_detector.rs` | `enable_absorption` | Detects heavy volume + compressed range (institutional accumulation). Uses `range_ratio = (high-low)/ATR` (not body-based). CVD flip confirmation: 100% weight if confirmed, 50% if not. |
| **Entropy Valley** | `smart_filters.rs` | `enable_entropy_valley` | Detects entropy drop from >0.85 to <0.65 in 30-min window. Chaos-to-order transition = confidence booster (up to +15%). |

### Key Innovations:

1. **CUSUM×HTF Soft Block:** Instead of binary HTF hard-block, when CUSUM detects early bullish reversal while HTF is still bearish, the bot trades with 50% position size. This catches trend transitions EMA would miss by 15+ minutes.

2. **CVD Flip Confirmation:** Absorption detection differentiates between true institutional accumulation (CVD flips from selling to buying) and noise (doji candles). Without CVD flip = 50% weight. With CVD flip = 100% weight.

3. **Dynamic CUSUM Threshold:** `threshold = 2.0 × rolling_std` of 5M returns. Self-calibrates to each symbol's volatility — no backtesting needed.

4. **Multi-Factor Confidence Boosting:** Entropy Valley (+15% max) and BUY Absorption (+10% max) boost confidence additively. These are offensive signals, not gates.

### Activation Order (Recommended):
1. Deploy with Phase 2 flags OFF (default) — data already collecting
2. Monitor CUSUM S+/S- values in dashboard for 24-48h
3. Enable `enable_cusum` first — observe soft-block behavior
4. Enable `enable_entropy_valley` — low risk, confidence booster only
5. Enable `enable_absorption` last — needs most observation
6. Run with all 3 enabled for 48h before Phase 3

### Rollback:
- Each flag independent:
  ```
  POST /api/v1/feature-flags
  {"cusum": false, "absorption": false, "entropy_valley": false}
  ```

---

## Phase 3 — Advanced Quantitative Techniques (Week 3-5)

**Goal:** Techniques from quantitative finance research that most practitioners don't implement.

### 3A: Kyle's Lambda Compression
- **What:** Track the price impact per unit of volume (Kyle's Lambda) to detect liquidity cycles.
- **Why:** When Lambda is low (deep liquidity), institutions are accumulating. When it suddenly spikes, they're done and price moves.
- **How:**
  - Lambda = |price_change| / volume for each trade batch
  - Track Lambda EMA(20) and standard deviation
  - "Compression" = Lambda < EMA - 1*StdDev (unusually deep liquidity)
  - "Expansion" = Lambda > EMA + 2*StdDev (liquidity withdrawal → breakout imminent)
- **Integration:** Signal booster, not a gate.
- **Data needed:** Trade stream data (already available)
- **Estimated effort:** 3 days

### 3B: Delta Acceleration
- **What:** Track the rate of change of buy/sell delta (not just delta itself).
- **Why:** ACCELERATING positive delta means exponential buying pressure — predicts rapid price increases.
- **How:**
  - Calculate delta per 10-second window from trade stream
  - Track delta_velocity = delta[t] - delta[t-1]
  - Track delta_acceleration = velocity[t] - velocity[t-1]
  - Signal when acceleration is consistently positive for 3+ windows
- **Integration:** Confidence booster for BUY signals.
- **Estimated effort:** 2 days

### 3C: Bayesian Convergence Score (Phase 3 Final)
- **What:** Instead of binary pass/fail gates, each gate outputs a probability. Combined using Bayes' theorem.
- **Why:** Current system: 15 gates × pass/fail = too restrictive. Bayesian: marginal signals add up.
- **Caution:** Requires careful handling of signal dependencies (NOT independent).
- **How:**
  - Each existing gate outputs P(profit | gate_value)
  - Use copula-adjusted Bayesian combination (handles dependencies)
  - Final score = posterior probability of profitable trade
- **Integration:** Replaces the current binary conviction system.
- **Estimated effort:** 5-7 days (major refactor)
- **Prerequisite:** Phase 1 + 2 validated with real trade data (need at least 100 trades for calibration)

---

## Dependency Chain

```
Phase 1 (DONE ✅)               Phase 2 (DONE ✅)               Phase 3 (Week 3-5)
┌─────────────────┐            ┌──────────────────┐            ┌──────────────────┐
│ HTF Trend Gate  │────────────│ CUSUM Break      │────────────│ Kyle's Lambda    │
│ Score Momentum  │            │ CUSUM×HTF Soft   │            │ Delta Accel      │
│ OFIP            │            │ Absorption+CVD   │            │ Bayesian Conv.   │
│ Adaptive Thresh │            │ Entropy Valley   │            └──────────────────┘
│ Entropy Grad    │            └──────────────────┘
└─────────────────┘
        ↓                              ↓
  All 5 flags ON ✅              3 flags OFF (default)
  Validated in live              Monitor data → Enable
```

## All Feature Flags (8 total):

| # | Flag | Phase | Default | Status |
|---|------|-------|---------|--------|
| 1 | `enable_htf_gate` | Phase 1 | OFF | Active ✅ |
| 2 | `enable_score_momentum` | Phase 1 | OFF | Ready |
| 3 | `enable_ofip` | Phase 1 | OFF | Active ✅ |
| 4 | `enable_adaptive_threshold` | Phase 1 | OFF | Active ✅ |
| 5 | `enable_entropy_graduated` | Phase 1 | OFF | Active ✅ |
| 6 | `enable_cusum` | Phase 2 | OFF | NEW — monitor first |
| 7 | `enable_absorption` | Phase 2 | OFF | NEW — monitor first |
| 8 | `enable_entropy_valley` | Phase 2 | OFF | NEW — monitor first |

## Critical Rules (Per AGENTS.md):
1. Every new feature is behind a Feature Flag, OFF by default
2. No feature proceeds to Phase N+1 until Phase N is validated
3. No Bayesian Convergence until 100+ real trades provide calibration data
4. Each phase has its own rollback plan (disable flag)
5. Dashboard shows all data BEFORE enabling the gate (observation period)

---

## Metrics to Track:
- Trades blocked by each smart filter (per day)
- Win rate WITH filter enabled vs without
- Average holding time
- False positive rate (filter blocked a trade that would have been profitable)
- False negative rate (filter allowed a trade that was a loss)
- CUSUM break detection timing vs EMA crossover timing
- Absorption detection accuracy (true institutional activity vs noise)
- Entropy Valley confidence boost effectiveness
