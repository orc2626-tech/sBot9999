# AURORA Spot Nexus — Claude Code Project Guide

## CRITICAL: THIS IS A LIVE TRADING SYSTEM WITH REAL MONEY
Every code change can cause financial loss. Triple-check everything.
Never assume — always verify in the actual code first.

---

## Project Overview

AURORA Spot Nexus is a professional-grade cryptocurrency spot trading bot for Binance.
- **Language:** Rust (backend), TypeScript/React (dashboard)
- **Architecture:** Async event-driven with WebSocket streams
- **Pairs:** BTCUSDT, ETHUSDT, BNBUSDT, XRPUSDT, SOLUSDT
- **Timeframe:** 5M strategy with 1M data collection
- **Risk:** Real USDT capital — every bug = real financial loss

---

## Mandatory Rules — NEVER Skip These

### Before ANY Code Change:
1. **Read the ENTIRE file** you're about to modify — not just the function
2. **Read all files that import from** the file you're modifying
3. **Understand the full pipeline**: WebSocket -> Trade Stream -> Scoring -> Insurance Gates -> Smart Filters -> Decision Envelope -> Order Execution
4. **Create a checklist** of every file that will be affected
5. **Ask for approval** before executing changes to order/risk logic

### Code Quality Requirements:
- Run `cargo check` after EVERY change — fix ALL warnings
- Run `cargo test` after EVERY change — ALL tests must pass (currently 91/91)
- Run `cargo clippy` — zero warnings allowed
- Never use `unwrap()` in production code — use proper error handling
- Every new feature MUST have a Feature Flag (default: OFF)
- Every new module MUST have unit tests
- Log every important decision with `tracing::info!` or `tracing::warn!`

### What You Must NEVER Do:
- Never modify order execution logic without explicit approval
- Never change risk management parameters without explicit approval
- Never remove or weaken any existing Insurance Gate
- Never change WebSocket connection handling without explicit approval
- Never use `unsafe` blocks
- Never add external crates without asking first

---

## Project Structure

```
backend/src/
├── main.rs                # Entry point, WebSocket connections, app initialization
├── strategy.rs            # Core trading strategy — THE BRAIN (read this first!)
├── decision_envelope.rs   # Trade decision with full audit trail + SmartFilterVerdicts
├── scoring.rs             # 6-signal ensemble scoring system
├── insurance.rs           # 11 Insurance Gates (NEVER weaken these)
├── smart_filters.rs       # Phase 1+2: Score Momentum, OFIP, Adaptive Threshold,
│                          #   Entropy Graduated, Entropy Valley
├── htf_analysis.rs        # Phase 1: HTF Trend Gate (15M + 1H EMA analysis)
├── cusum_detector.rs      # Phase 2A: CUSUM structural break detection
├── absorption_detector.rs # Phase 2B: Institutional absorption detection
├── runtime_config.rs      # Feature flags + hot-reload configuration
├── risk.rs                # 4-layer risk management + circuit breakers
├── regime.rs              # Market regime detection (TRENDING/RANGING/VOLATILE/SQUEEZE)
├── order.rs               # Binance order execution (CRITICAL — be very careful)
├── trade_stream.rs        # Real-time trade stream processing + CVD
├── candle_buffer.rs       # Candle aggregation and storage (1M, 5M, 15M, 1H)
├── execution.rs           # Execution proposal + paper/live order routing
├── app_state.rs           # Shared application state + all trackers
├── exit/
│   ├── triple_barrier.rs  # Triple Barrier exit management (TP/SL/Time)
│   ├── micro_trail.rs     # Micro-Trail + Order Flow Adaptive Exit (Phase 3)
│   └── monitor.rs         # Exit monitor loop (every 5s, barrier + micro-trail)
├── arena/
│   ├── bridge.rs          # Shadow-to-Live bridge (DTS profile selection)
│   ├── profile.rs         # Strategy profiles (Momentum/MeanRevert/Breakout/Scalp)
│   ├── shadow_engine.rs   # Shadow trade tracking
│   └── thompson.rs        # Discounted Thompson Sampling
├── api/
│   └── rest.rs            # REST API endpoints (feature flags, control panel)
├── indicators/
│   ├── atr.rs             # ATR calculation (Wilder's method)
│   ├── ema.rs             # EMA calculation
│   └── adx.rs             # ADX indicator
└── types.rs               # Shared types and structs

dashboard_v2/src/
├── components/
│   ├── SmartFiltersPanel.tsx  # Phase 1+2 filters display + controls
│   └── ...
├── pages/
├── hooks/
│   └── useTruthState.ts   # Real-time WebSocket state
├── lib/
│   └── api.ts             # TypeScript API types + fetch helpers
└── App.tsx
```

---

## Trading Pipeline (execution order)

```
 1. WebSocket receives 1M kline + trade stream
 2. candle_buffer.rs aggregates candles (1M, 5M, 15M, 1H)
 3. trade_stream.rs calculates CVD, VPIN, delta
 4. scoring.rs computes 6-signal weighted ensemble score
 5. regime.rs determines market regime (TRENDING/RANGING/VOLATILE/SQUEEZE)
 6. insurance.rs runs 11 Insurance Gates (PASS/FAIL)
 7. smart_filters.rs runs Smart Filters (all gated by Feature Flags):
    a. HTF Trend Gate — 15M + 1H EMA structural direction
    b. Score Momentum — rolling score history, sustained conviction
    c. OFIP — Order Flow Imbalance Persistence (90s window)
    d. Adaptive Threshold — dynamic BUY threshold by regime/entropy/ATR
    e. Entropy Graduated — graduated position sizing by entropy
    f. CUSUM — structural break detection (5M candles)
    g. CUSUM×HTF soft-block — EMA lag compensation
    h. Absorption — institutional volume detection + CVD flip
    i. Entropy Valley — chaos-to-order confidence boost
 8. decision_envelope.rs packages everything with full audit trail
 9. strategy.rs makes final BUY/SELL/HOLD decision
10. execution.rs executes on Binance (paper or live)
11. exit/monitor.rs manages exits: Profit Lock, Triple Barrier, Micro-Trail
12. exit/micro_trail.rs: phased ATR trail + CVD/VPIN/OB adaptive tightening
13. risk.rs monitors position and enforces circuit breakers
```

---

## CRITICAL: ATR and SL/TP Calculation Rules

### ALWAYS use 5M ATR for SL/TP (NEVER 1M ATR)

**History:** On 2026-02-09, we discovered the root cause of a 13.6% win rate:
SL/TP were calculated from 1M ATR which is 5-10x too small. A 1M ATR for
XRPUSDT at $1.44 might be ~$0.002, producing a SL of only 0.05%. Normal
crypto noise is 0.3-0.5% per minute, so SL was guaranteed to get hit.

**The fix (strategy.rs):**
- Use `key_5m` candles for ATR calculation
- Fallback: `atr_1m * 3.0` if 5M not yet available
- Minimum SL floor: 0.4% of entry price (40 bps)
- Minimum TP1 floor: 0.6% of entry price (60 bps)
- Minimum TP2 floor: 1.0% of entry price (100 bps)

**Triple Barrier (exit/triple_barrier.rs) also has minimum floors:**
- SL floor: 0.4%
- TP floor: 0.6%

**NEVER reduce these floors.** They prevent the noise-triggered exits
that caused the 86.4% loss rate.

---

## Feature Flags System

All features use feature flags in `runtime_config.rs`. Persisted in `runtime_config.json`.

```
Current flags (Phase 1 — Defensive):
  enable_htf_gate:            true  — HTF Trend Gate (structural direction)
  enable_score_momentum:      true  — Score Momentum (sustained conviction)
  enable_ofip:                true  — Order Flow Imbalance Persistence
  enable_adaptive_threshold:  true  — Adaptive BUY threshold by regime
  enable_entropy_graduated:   true  — Graduated position sizing by entropy

Current flags (Phase 2 — Offensive):
  enable_cusum:               true  — CUSUM structural break detection
  enable_absorption:          true  — Institutional absorption detection
  enable_entropy_valley:      true  — Entropy Valley confidence booster

Current flags (Phase 3 — Exit Intelligence):
  enable_micro_trail:         false — Micro-Trail + Order Flow Adaptive Exit
```

New features MUST follow this pattern:
1. Add flag to `RuntimeConfig` struct (default: false via `#[serde(default)]`)
2. Add to REST API GET/POST `/api/v1/feature-flags`
3. Add dashboard toggle in SmartFiltersPanel.tsx
4. Gate ALL new logic behind `if config.enable_new_feature {}`
5. Collect data even when flag is OFF (for observation)

---

## Key Technical Details

### ATR Timeframe Rules:
- **SL/TP calculations:** Always use 5M ATR (strategy.rs, triple_barrier.rs)
- **Scoring signals:** Use 1M candles (scoring.rs)
- **Regime detection:** Use 1M candles (regime.rs)
- **CUSUM:** Uses 5M candles (cusum_detector.rs)
- **Absorption:** Uses 1M candles (absorption_detector.rs)
- **HTF Gate:** Uses 15M + 1H candles (htf_analysis.rs)

### Position Sizing Pipeline:
```
base_pct = AFPL_ladder.quote_amount / capital
ladder_pct = base_pct × bridge.position_size_factor
                      × entropy_graduated_multiplier
                      × cusum_factor
Final: clamped to [0.5%, 10%] of capital
```

### CUSUM×HTF Soft-Block (Key Innovation):
When CUSUM detects a bullish break but HTF is still bearish (EMA lag):
- Instead of hard-blocking the BUY, allow it at 50% position size
- This compensates for EMA's inherent lag (15-30 minute delay)
- `cusum_factor` is applied to position sizing, not to the block decision

### Triple Barrier Exit Management:
- TP/SL set from 5M ATR with regime-specific multipliers
- Progressive tightening: SL moves toward entry at 50% time elapsed
- Breakeven lock: SL moves to entry + 0.05% at 75% time elapsed
- Time exit: Close at market after time limit (varies by regime)

### Profit Lock:
When price reaches 50% of TP1 distance, SL moves to breakeven + 0.05%

### Micro-Trail + Order Flow Adaptive Exit (Phase 3):
Replaces fixed 0.5% trailing stop with ATR-calibrated, microstructure-aware trail:
- **Phased Trail Distance** (based on profit fraction of TP1):
  - Loose (0-30% of TP1): 1.5× ATR — let the trade breathe
  - Standard (30-60% of TP1): 1.0× ATR — balanced protection
  - Aggressive (60%+ of TP1): 0.5× ATR — lock maximum profit
- **Order Flow Adaptation** (multiplicative tightening):
  - CVD divergence against position → tighten 30%
  - Orderbook imbalance against position → tighten 20%
  - VPIN toxic (>0.7) → emergency tighten 50%
- **Velocity Shield**: >0.3% adverse move in 5s → snap trail to current level
- **Minimum floor**: 0.2% trail distance (never tighter than this)
- **Feature flag**: `enable_micro_trail` (default: OFF)

---

## Known Bugs Fixed (Evidence-Based)

### 2026-02-09: SL/TP Too Tight (Critical)
- **Evidence:** 66 trades, 13.6% win rate, avg duration 119 seconds
- **Root Cause:** 1M ATR used for SL/TP → 0.05-0.31% stops hit by noise
- **Fix:** 5M ATR + minimum floors (SL ≥ 0.4%, TP1 ≥ 0.6%, TP2 ≥ 1.0%)
- **Expected Impact:** Win rate should increase from ~14% to 50-65%

### 2026-02-09: Phase 1 Filters Disabled
- **Evidence:** `score_momentum=false`, `entropy_graduated=false` in config
- **Root Cause:** Config file not updated after Phase 2 deployment
- **Fix:** Enabled all Phase 1 and Phase 2 filters in runtime_config.json

---

## Testing Requirements

```bash
# After EVERY change:
cargo check                    # Must compile with zero errors
cargo test                     # All 91 tests must pass
cargo clippy -- -D warnings    # Zero warnings

# For new modules, minimum tests:
# - Unit tests for core calculations
# - Edge case tests (empty data, NaN, overflow)
# - Integration test with mock candle data
```

---

## API Endpoints (Quick Reference)

```
GET  /api/v1/health           — Health check
GET  /api/v1/state            — Full state snapshot
GET  /api/v1/regime           — Current market regime
GET  /api/v1/positions        — Open positions
GET  /api/v1/trade-journal    — All trades
GET  /api/v1/trade-journal/stats — Win rate, PnL, profit factor
GET  /api/v1/feature-flags    — Current feature flag states
POST /api/v1/feature-flags    — Update feature flags
GET  /api/v1/decisions        — Recent decision envelopes
GET  /api/v1/ladder           — AFPL ladder state

All endpoints require: Authorization: Bearer AURORA_ADMIN_TOKEN
```

---

## Upgrade Roadmap

### Phase 1 (Complete): Defensive Layer
HTF Trend Gate, Score Momentum, OFIP, Adaptive Threshold, Entropy Graduated

### Phase 2 (Complete): Offensive Layer
CUSUM Break Detection, Absorption Detection, Entropy Valley, CUSUM×HTF Soft-Block

### Phase 3 (In Progress): Exit Intelligence Layer
- **Micro-Trail + Order Flow Adaptive Exit** (Complete) — phased ATR trail
  with CVD/VPIN/orderbook adaptive tightening + velocity shield
- Bayesian Convergence Filter — probabilistic PASS/FAIL instead of binary
- Microstructure Momentum — tick-by-tick momentum analysis
- Cross-Symbol Correlation — exploit BTC→ALT lead-lag relationships
- Dynamic Regime Weighting — auto-adjust signal weights per regime

---

## Communication Style

- Arabic for discussions, English for code/comments/variables
- Be direct and concise
- Show analysis BEFORE making changes
- Ask for approval on critical components (order/risk logic)
