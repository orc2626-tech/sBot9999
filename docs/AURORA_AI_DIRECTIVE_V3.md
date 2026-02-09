# AURORA SPOT NEXUS — AI Engineering Directive
# ═══════════════════════════════════════════════
# Version: 3.0 | Date: 2026-02-07
# Classification: PRODUCTION SYSTEM — REAL MONEY AT RISK
# Target: Any AI Engineer working on this codebase

---

## ⛔ PRIME DIRECTIVE — READ BEFORE ANY ACTION

```
┌─────────────────────────────────────────────────────────────────┐
│  THIS BOT TRADES REAL MONEY ON BINANCE.                        │
│  ANY BUG = REAL FINANCIAL LOSS.                                │
│                                                                 │
│  RULES:                                                         │
│  1. Read main.rs + AGENTS.md FULLY before ANY suggestion        │
│  2. Create checklist before ANY code change                     │
│  3. Triple-check every change — ask owner for approval          │
│  4. Never assume — always verify in actual code                 │
│  5. No theoretical proposals — only production-ready code       │
│  6. Every function must have error handling + logging            │
│  7. Test with Demo mode FIRST — never skip to Live              │
│  8. Backward compatible by default                              │
└─────────────────────────────────────────────────────────────────┘
```

---

## 📋 PROJECT CURRENT STATE (Audit Summary)

### What EXISTS and WORKS (Do NOT break these):
- ✅ Axum REST API with 15+ endpoints + Bearer auth middleware
- ✅ RuntimeConfig atomic persist (tmp + rename pattern)
- ✅ Binance User Data Stream (listen key create + keepalive + WS reconnect)
- ✅ AppState with RwLock + AtomicU64 state versioning
- ✅ Truth Header in every API response (state_version, server_time, ws_sequence)
- ✅ Dashboard: GlobalTruthHeader showing live API data
- ✅ Dashboard: Control Panel (Pause/Resume/Kill + Account Mode Demo↔Live)
- ✅ CORS, health endpoint, config schema endpoint
- ✅ Comprehensive documentation (AGENTS.md, RUNBOOK.md)

### What is PLACEHOLDER / EMPTY (Needs implementation):
- 🔴 `execution.rs` — 6 lines, no actual order placement
- 🔴 `strategy.rs` — returns empty Vec, no signal generation
- 🔴 `risk.rs` — struct only, no calculation logic
- 🔴 `position_engine.rs` — struct only, no state management
- 🔴 `reconcile.rs` — `reconcile_once()` returns `Ok(())` (empty)
- 🔴 `trade_insurance.rs` — only checks `confidence < 0.5`
- 🔴 BinanceClient `secret` field is NEVER used (no HMAC signing)
- 🔴 User Stream events are received but NEVER parsed
- 🔴 Dashboard: 80% of components show hardcoded mock data

### Tech Stack:
- **Backend:** Rust (tokio, axum 0.7, reqwest, tokio-tungstenite, parking_lot, serde, chrono, uuid, anyhow)
- **Dashboard:** React 18 + TypeScript + Vite + Tailwind CSS + Recharts + Lucide icons
- **Target:** Binance Spot API (REST + WebSocket)

---

## 🔧 IMPLEMENTATION TASKS — ORDERED BY PRIORITY

> Execute tasks in this EXACT order. Each task depends on the previous one.
> After each task: compile, test with curl, verify no regressions.

---

### PHASE 1: CORE TRADING FOUNDATION (P0 — Without these, the bot cannot trade)

#### TASK 1.1: HMAC-SHA256 Request Signing

**File:** `backend/src/binance/client.rs`

**What to build:**
- Add method `sign_request(&self, params: &str) -> String` using HMAC-SHA256
- Add `timestamp` (server time or local time) to every signed request
- Add `signature` param to query string
- The `secret` field already exists but is unused — USE IT

**Requirements:**
```rust
// The signing function must:
// 1. Take query string params (without signature)
// 2. Compute HMAC-SHA256(secret, query_string)
// 3. Return hex-encoded signature
// 4. NEVER log the secret or signature

// Example: GET /api/v3/account?timestamp=1234&signature=abc...
// Signature = HMAC-SHA256(secret_key, "timestamp=1234")
```

**Dependencies already in Cargo.toml:** `hmac = "0.12"`, `sha2 = "0.10"`, `hex = "0.4"` — all present but unused.

**Test:** `GET /api/v3/account` with signature should return account balances.

**Critical:** Use `recvWindow=5000` to handle clock drift. Add Binance server time sync if local clock is off.

---

#### TASK 1.2: Core Trading Methods on BinanceClient

**File:** `backend/src/binance/client.rs`

**Add these methods:**

```
1. get_account() -> AccountInfo          // GET /api/v3/account (signed)
2. get_exchange_info() -> ExchangeInfo   // GET /api/v3/exchangeInfo (public)
3. place_order(params) -> OrderResponse  // POST /api/v3/order (signed)
4. cancel_order(symbol, order_id)        // DELETE /api/v3/order (signed)
5. get_open_orders(symbol) -> Vec        // GET /api/v3/openOrders (signed)
6. get_order(symbol, order_id) -> Order  // GET /api/v3/order (signed)
7. get_all_orders(symbol, start) -> Vec  // GET /api/v3/allOrders (signed)
```

**Requirements for place_order:**
- Support `newClientOrderId` (for idempotency — use trade_intent_id as UUID)
- Support order types: LIMIT, MARKET, STOP_LOSS_LIMIT, TAKE_PROFIT_LIMIT
- Support timeInForce: GTC, IOC, FOK
- Support `newOrderRespType: FULL` to get fills in response
- Return strongly typed OrderResponse (not raw JSON)

**Requirements for get_exchange_info:**
- Parse and cache `symbols[].filters` (LOT_SIZE, PRICE_FILTER, MIN_NOTIONAL, NOTIONAL)
- Cache must refresh every 24 hours or on startup
- Provide helper: `adjust_quantity(symbol, qty) -> f64` that rounds to valid LOT_SIZE
- Provide helper: `adjust_price(symbol, price) -> f64` that rounds to valid PRICE_FILTER
- Provide helper: `check_min_notional(symbol, qty, price) -> bool`

**Critical edge cases:**
- Binance returns `code: -1021` (Timestamp outside recvWindow) → sync server time
- Binance returns `code: -2010` (Insufficient balance) → do NOT retry, propagate error
- Binance returns `code: -1015` (Too many orders) → backoff, set rate limit flag

---

#### TASK 1.3: Rate Limit Tracker

**New file:** `backend/src/binance/rate_limit.rs`

**What to build:**
- Parse `X-MBX-USED-WEIGHT-1M` header from every Binance REST response
- Track current weight usage (atomic counter, resets every minute)
- Hard limit: if weight > 1000 (out of 1200) → delay requests
- Expose to AppState for dashboard display
- Log warning at 800 weight

**Structure:**
```rust
pub struct RateLimitTracker {
    used_weight_1m: AtomicU32,
    order_count_10s: AtomicU32,
    order_count_1d: AtomicU32,
    last_reset: RwLock<Instant>,
}

impl RateLimitTracker {
    pub fn update_from_headers(&self, headers: &reqwest::header::HeaderMap);
    pub fn can_send_request(&self, weight: u32) -> bool;
    pub fn can_place_order(&self) -> bool;
    pub fn snapshot(&self) -> RateLimitSnapshot; // for API/dashboard
}
```

---

#### TASK 1.4: Parse User Stream Events

**File:** `backend/src/binance/mod.rs`

**Currently:** The WebSocket receives messages but ignores content (just calls `touch_ws_user_event()`).

**What to build — parse these event types:**

```json
// 1. executionReport — order state change
{
  "e": "executionReport",
  "s": "BTCUSDT",           // symbol
  "S": "BUY",               // side
  "o": "LIMIT",             // order type
  "X": "FILLED",            // current order status
  "i": 123456,              // orderId
  "C": "intent_uuid_here",  // clientOrderId (our intent_id)
  "l": "0.001",             // last executed qty
  "L": "50000.00",          // last executed price
  "n": "0.00001",           // commission
  "N": "BNB",               // commission asset
  "z": "0.001",             // cumulative filled qty
  "Z": "50.00"              // cumulative quote qty
}

// 2. outboundAccountPosition — balance update
{
  "e": "outboundAccountPosition",
  "B": [
    { "a": "BTC", "f": "0.05", "l": "0.00" },
    { "a": "USDT", "f": "1000.00", "l": "0.00" }
  ]
}

// 3. balanceUpdate — deposit/withdrawal
{
  "e": "balanceUpdate",
  "a": "USDT",
  "d": "100.00"  // balance delta
}
```

**On executionReport:**
1. Match by `clientOrderId` (our trade_intent_id) to find the internal order
2. Update position state (qty filled, avg price, status)
3. If status == FILLED → calculate entry price, update PositionEngine
4. If status == CANCELED or REJECTED → clean up
5. Push state change to WebSocket subscribers
6. Log: `info!(symbol=%s, side=%S, status=%X, filled_qty=%z, "Order update")`

**On outboundAccountPosition:**
1. Update internal balance cache
2. Use for risk calculations (remaining capital)

---

#### TASK 1.5: Position State Machine

**File:** `backend/src/position_engine.rs` (currently 13 lines — needs full rewrite)

**What to build:**

```rust
#[derive(Clone, Debug, Serialize)]
pub enum PositionStatus {
    Pending,          // Order sent, not confirmed
    PartiallyFilled,  // Partially filled
    Open,             // Fully entered
    Closing,          // Exit order sent
    Closed,           // Fully exited
    Cancelled,        // Entry cancelled
    Error(String),    // Something went wrong
}

#[derive(Clone, Debug, Serialize)]
pub struct Position {
    pub id: String,                    // UUID
    pub trade_intent_id: String,       // Idempotency key
    pub symbol: String,
    pub side: String,                  // BUY or SELL
    pub status: PositionStatus,
    pub entry_order_id: Option<u64>,
    pub entry_client_order_id: String,
    pub requested_qty: f64,
    pub filled_qty: f64,
    pub avg_entry_price: f64,
    pub exit_order_id: Option<u64>,
    pub avg_exit_price: f64,
    pub realized_pnl: f64,
    pub commission_total: f64,
    pub commission_asset: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
    pub tags: Vec<String>,            // strategy name, setup type, etc.
}

pub struct PositionEngine {
    positions: RwLock<HashMap<String, Position>>,  // by id
    by_order_id: RwLock<HashMap<u64, String>>,     // order_id → position_id
    by_client_id: RwLock<HashMap<String, String>>, // client_order_id → position_id
}

impl PositionEngine {
    pub fn open_position(&self, params: OpenPositionParams) -> String; // returns position_id
    pub fn on_execution_report(&self, report: &ExecutionReport);       // update from WS
    pub fn get_open_positions(&self) -> Vec<Position>;
    pub fn get_position(&self, id: &str) -> Option<Position>;
    pub fn close_position(&self, id: &str) -> Result<()>;
    pub fn get_total_exposure(&self) -> f64;                           // sum of open position values
    pub fn get_pnl_today(&self) -> f64;                                // realized PnL today
}
```

**PnL Calculation (critical — get this right):**
```
For BUY positions:
  realized_pnl = (avg_exit_price - avg_entry_price) × filled_qty - total_commission

For SELL positions:
  realized_pnl = (avg_entry_price - avg_exit_price) × filled_qty - total_commission

Commission must be converted to quote currency if paid in BNB.
```

---

#### TASK 1.6: Risk Engine (Real Implementation)

**File:** `backend/src/risk.rs` (currently 26 lines — needs full rewrite)

**What to build:**

```rust
pub struct RiskEngine {
    config: SharedRuntimeConfig,
    position_engine: Arc<PositionEngine>,
    state: RwLock<RiskState>,
}

#[derive(Clone, Serialize)]
pub struct RiskState {
    pub risk_mode: RiskMode,                    // Normal, Elevated, Critical, CircuitBroken
    pub daily_pnl: f64,
    pub daily_pnl_pct: f64,
    pub remaining_daily_loss_pct: f64,
    pub consecutive_losses: u32,
    pub daily_trades_count: u32,
    pub max_daily_trades: u32,
    pub open_positions_count: u32,
    pub total_exposure_usdt: f64,
    pub circuit_breaker: CircuitBreakerState,
    pub last_check_at: String,
}

#[derive(Clone, Serialize)]
pub enum CircuitBreakerState {
    Closed,                          // Normal operation
    Open { tripped_at: String, reason: String },   // Trading halted
    HalfOpen { next_attempt_at: String },          // Testing with single trade
}

impl RiskEngine {
    // Called BEFORE every trade attempt
    pub fn pre_trade_check(&self, proposal: &Proposal) -> RiskVerdict {
        // 1. Check daily loss limit
        // 2. Check consecutive losses
        // 3. Check max concurrent positions
        // 4. Check max daily trades
        // 5. Check circuit breaker state
        // 6. Check correlation with existing positions (see ADVANCED section)
        // 7. Return ALLOW or BLOCK(reason)
    }

    // Called AFTER every trade closes
    pub fn on_trade_closed(&self, pnl: f64) {
        // 1. Update daily PnL
        // 2. Update consecutive losses counter
        // 3. Check if circuit breaker should trip
        // 4. If tripped → set no_go_reason on AppState
    }

    // Circuit Breaker Half-Open logic
    pub fn check_half_open(&self) -> bool {
        // If Open for > 1 hour → transition to HalfOpen
        // HalfOpen: allow ONE trade with 25% size
        // If that trade profits → Closed
        // If that trade loses → back to Open for 2 hours
    }
}
```

**Circuit Breaker Triggers:**
- `daily_pnl_pct < -max_daily_loss_pct` → **OPEN** (reason: MAX_DAILY_LOSS)
- `consecutive_losses >= max_consecutive_losses` → **OPEN** (reason: MAX_CONSECUTIVE_LOSSES)
- Manual Kill → **OPEN** (reason: MANUAL_KILL)

**When circuit breaker is OPEN:**
- `no_go_reason` MUST be set on AppState
- Dashboard MUST show red banner
- NO orders can be placed
- Existing positions can still be closed (safety exits)

---

#### TASK 1.7: Reconciliation Engine (Real Implementation)

**File:** `backend/src/reconcile.rs` (currently empty logic)

**What to build:**

```rust
async fn reconcile_once(state: &AppState, client: &BinanceClient) -> Result<()> {
    // STEP 1: Get account info from Binance
    let account = client.get_account().await?;

    // STEP 2: Get open orders from Binance
    let exchange_orders = client.get_open_orders(None).await?;

    // STEP 3: Compare with internal state
    let internal_positions = state.position_engine.get_open_positions();

    // STEP 4: Detect drift
    for pos in &internal_positions {
        // Check if position exists on exchange
        // Check if quantities match
        // Check if orders still exist
    }

    // STEP 5: Detect orphan orders (on exchange but not in our state)
    for order in &exchange_orders {
        if !state.position_engine.has_order(order.order_id) {
            warn!("Orphan order detected: {} {}", order.symbol, order.order_id);
            // Option: cancel orphan orders or track them
        }
    }

    // STEP 6: Update balances
    state.update_balances(account.balances);

    // STEP 7: Log reconciliation result
    info!(
        positions = internal_positions.len(),
        exchange_orders = exchange_orders.len(),
        "Reconciliation complete"
    );

    Ok(())
}
```

**Reconciliation must also handle:**
- Phantom orders (sent but no response received) — check by `newClientOrderId`
- Balance drift (internal tracking vs actual balance)
- Stale positions (Open for > 24h without activity → flag for review)

---

### PHASE 2: EXECUTION LAYER (P1 — Makes the bot actually trade)

#### TASK 2.1: Execution Engine

**File:** `backend/src/execution.rs` (currently 6 lines — needs full rewrite)

**What to build:**

```rust
pub struct ExecutionEngine {
    client: Arc<BinanceClient>,
    position_engine: Arc<PositionEngine>,
    risk_engine: Arc<RiskEngine>,
    exchange_info: Arc<ExchangeInfoCache>,
    intent_store: RwLock<HashMap<String, IntentRecord>>,  // idempotency
}

struct IntentRecord {
    intent_id: String,
    order_id: Option<u64>,
    status: IntentStatus,
    created_at: Instant,
}

impl ExecutionEngine {
    pub async fn execute_proposal(&self, proposal: &Proposal) -> ExecutionResult {
        // 1. Generate trade_intent_id (UUID)
        let intent_id = Uuid::new_v4().to_string();

        // 2. Check idempotency (was this proposal already executed?)
        if let Some(existing) = self.intent_store.read().get(&intent_id) {
            return ExecutionResult::AlreadyExecuted(existing.order_id);
        }

        // 3. Risk pre-check
        let risk_verdict = self.risk_engine.pre_trade_check(proposal);
        if risk_verdict.is_blocked() {
            return ExecutionResult::Blocked(risk_verdict.reason);
        }

        // 4. Check account mode (Demo vs Live)
        let config = self.config.read();
        if config.account_mode == AccountMode::Demo {
            return self.simulate_execution(proposal, &intent_id);
        }

        // 5. Adjust quantity and price for exchange filters
        let qty = self.exchange_info.adjust_quantity(&proposal.symbol, proposal.quantity);
        let price = self.exchange_info.adjust_price(&proposal.symbol, proposal.price);

        // 6. Check MIN_NOTIONAL
        if !self.exchange_info.check_min_notional(&proposal.symbol, qty, price) {
            return ExecutionResult::Rejected("Below MIN_NOTIONAL".into());
        }

        // 7. Place order on Binance
        let order_result = self.client.place_order(OrderParams {
            symbol: proposal.symbol.clone(),
            side: proposal.direction.clone(),
            order_type: "LIMIT".into(),
            time_in_force: "GTC".into(),
            quantity: qty,
            price: Some(price),
            new_client_order_id: Some(intent_id.clone()),
            new_order_resp_type: "FULL".into(),
        }).await;

        // 8. Handle result
        match order_result {
            Ok(response) => {
                self.intent_store.write().insert(intent_id.clone(), IntentRecord {
                    intent_id: intent_id.clone(),
                    order_id: Some(response.order_id),
                    status: IntentStatus::Placed,
                    created_at: Instant::now(),
                });
                self.position_engine.open_position(/* ... */);
                ExecutionResult::Placed(response)
            }
            Err(e) => {
                // Check if it's a timeout (phantom order risk)
                if e.is_timeout() {
                    self.handle_phantom_order(&intent_id, &proposal.symbol).await;
                }
                ExecutionResult::Error(e.to_string())
            }
        }
    }

    // Phantom Order Recovery
    async fn handle_phantom_order(&self, intent_id: &str, symbol: &str) {
        // Mark as PHANTOM in intent store
        // Schedule check: GET /api/v3/allOrders with newClientOrderId
        // If found → sync state
        // If not found after 30s → mark as failed
        warn!(intent_id, symbol, "Phantom order detected — scheduling recovery");
    }
}
```

---

#### TASK 2.2: Decision Envelope Pipeline (Real Implementation)

**File:** `backend/src/decision_envelope.rs`

**The struct exists but is never populated from a real pipeline. Build the pipeline:**

```rust
pub struct DecisionPipeline {
    risk_engine: Arc<RiskEngine>,
    position_engine: Arc<PositionEngine>,
    execution_engine: Arc<ExecutionEngine>,
}

impl DecisionPipeline {
    pub async fn evaluate(&self, proposal: Proposal) -> DecisionEnvelope {
        let mut envelope = DecisionEnvelope::new(&proposal);

        // Gate 1: Data Quality
        envelope.data_quality_verdict = self.check_data_quality();
        if envelope.data_quality_verdict != "PASS" {
            envelope.final_decision = "BLOCK".into();
            envelope.blocking_layer = Some("DATA_QUALITY".into());
            return envelope;
        }

        // Gate 2: Trade Insurance
        let (insured, reason) = trade_insurance::check_insurance(&proposal);
        envelope.insurance_verdict = if insured { "PASS".into() } else { "BLOCK".into() };
        if !insured {
            envelope.final_decision = "BLOCK".into();
            envelope.blocking_layer = Some("INSURANCE".into());
            envelope.reason = reason;
            return envelope;
        }

        // Gate 3: Risk Pre-Check
        let risk_verdict = self.risk_engine.pre_trade_check(&proposal);
        envelope.risk_verdict = risk_verdict.label.clone();
        if risk_verdict.is_blocked() {
            envelope.final_decision = "BLOCK".into();
            envelope.blocking_layer = Some("RISK".into());
            envelope.reason = Some(risk_verdict.reason);
            return envelope;
        }

        // Gate 4: Execution Quality (spread, depth)
        envelope.execution_quality_verdict = "PASS".into(); // TODO: TCA

        // All gates passed
        envelope.final_decision = "ALLOW".into();
        envelope
    }
}
```

**Store last N decisions in AppState (already exists: `recent_decisions`)**
**Dashboard must read from this — remove hardcoded pipeline data**

---

### PHASE 3: DASHBOARD — CONNECT TO REAL DATA (P1)

> **Rule:** Every component that currently shows hardcoded data must be connected to real API data.

#### TASK 3.1: Extend /api/v1/state Response

**File:** `backend/src/app_state.rs` + `backend/src/api/rest.rs`

**Add to StateSnapshot:**
```rust
pub struct StateSnapshot {
    // ... existing fields ...
    pub balances: Vec<BalanceInfo>,              // NEW
    pub open_orders: Vec<OrderInfo>,             // NEW
    pub risk_details: RiskState,                 // NEW (replace basic risk)
    pub recent_trades: Vec<TradeRecord>,         // NEW
    pub rate_limits: RateLimitSnapshot,          // NEW
    pub exchange_info_loaded: bool,              // NEW
    pub uptime_secs: u64,                        // NEW
    pub ws_latency_ms: Option<u64>,              // NEW
}
```

#### TASK 3.2: Connect Dashboard Components

**For EACH component below, remove hardcoded data and fetch from API:**

| Component | Current State | Connect To |
|-----------|--------------|------------|
| `SignalEnsembleCard.tsx` | Hardcoded signals | `state.recent_decisions[0].signals` |
| `TradeEligibilityCard.tsx` | Hardcoded 82% | `state.recent_decisions[0]` verdict data |
| `DecisionPipelineVisual.tsx` | Hardcoded stages | `state.recent_decisions` — map each gate verdict |
| `SystemHealthCard.tsx` | "128ms", "32s ago" | `state.truth.last_ws_user_event_age_ms`, `state.truth.reconcile_last_ok_age_s` |
| `ActionLogTable.tsx` | Hardcoded entries | `state.recent_trades` + control actions |
| `AiObserverPanel.tsx` | Hardcoded chart | `state.risk_details` anomaly data |
| `PositionManagementGrid.tsx` | Empty | `state.positions` — real open positions |
| `AggregatePositionsPanel.tsx` | Empty | Sum of positions + total PnL |
| `RecentErrorsPanel.tsx` | Empty | `state.recent_errors` from backend |
| `PagePanels.tsx` (RiskLimits) | Hardcoded "1.5%" | `state.risk_details` + `state.runtime_config` |
| `PagePanels.tsx` (CircuitBreakers) | All "OK" | `state.risk_details.circuit_breaker` |

**Pattern for connecting a component:**
```tsx
// BEFORE (hardcoded):
const signals = [
  { name: 'Momentum', value: 82 },
]

// AFTER (from API):
import { useTruthState } from '../hooks/useTruthState'

export default function SignalEnsembleCard() {
  const { state } = useTruthState()
  const signals = state?.signals ?? []
  // ... render real data with loading/empty states
}
```

**Every component MUST handle 3 states:**
1. **Loading** — show skeleton/spinner
2. **Empty** — show "No data yet" message
3. **Error** — show error with retry button

---

#### TASK 3.3: WebSocket Push (Replace Polling)

**File:** `backend/src/api/ws.rs`

**Currently:** Dashboard polls `/api/v1/state` every 3 seconds.
**Problem:** Wastes bandwidth, adds latency, hammers API.

**What to build:**
```rust
// Backend: push state changes to all WebSocket subscribers
async fn handle_socket(state: Arc<AppState>, ws_seq: Arc<AtomicU64>, socket: WebSocket) {
    let (mut sender, mut receiver) = socket.split();

    // Send initial snapshot
    send_snapshot(&mut sender, &state, &ws_seq).await;

    // Watch for state changes
    let mut last_version = state.current_state_version();
    let mut interval = tokio::time::interval(Duration::from_millis(500));

    loop {
        tokio::select! {
            _ = interval.tick() => {
                let current = state.current_state_version();
                if current != last_version {
                    send_snapshot(&mut sender, &state, &ws_seq).await;
                    last_version = current;
                }
            }
            msg = receiver.next() => {
                match msg {
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(Message::Ping(d))) => { let _ = sender.send(Message::Pong(d)).await; }
                    _ => {}
                }
            }
        }
    }
}
```

**Dashboard side:**
```typescript
// Replace polling with WebSocket in useTruthState.ts
export function useTruthState() {
  const [state, setState] = useState<StateSnapshot | null>(null)
  const wsRef = useRef<WebSocket | null>(null)

  useEffect(() => {
    const ws = new WebSocket(stateWebSocketUrl())
    ws.onmessage = (e) => {
      const data = JSON.parse(e.data)
      if (data.type === 'state') setState(data.payload)
    }
    ws.onclose = () => {
      // Reconnect with exponential backoff
      setTimeout(() => { /* reconnect */ }, 2000)
    }
    wsRef.current = ws
    return () => ws.close()
  }, [])

  return { state, error: null, loading: !state }
}
```

---

### PHASE 4: ADVANCED IMPROVEMENTS (P2 — Elevates to professional grade)

#### TASK 4.1: Idempotency System

**New file:** `backend/src/idempotency.rs`

```rust
// Every trade attempt gets a unique intent_id (UUID).
// If same intent_id is sent twice (retry, network issue), return same result.
// Uses Binance newClientOrderId as external idempotency key.

pub struct IdempotencyStore {
    store: RwLock<HashMap<String, IdempotencyRecord>>,
    ttl: Duration,  // auto-cleanup after 24h
}

pub struct IdempotencyRecord {
    pub intent_id: String,
    pub binance_order_id: Option<u64>,
    pub client_order_id: String,
    pub status: IntentStatus,
    pub created_at: Instant,
    pub result: Option<ExecutionResult>,
}
```

#### TASK 4.2: Dead Man's Switch

**New file:** `backend/src/dead_man_switch.rs`

```rust
// If no user heartbeat (dashboard ping or manual check-in) for X minutes,
// AND no WebSocket data flow for Y minutes,
// → auto-pause the bot.

pub struct DeadManSwitch {
    last_user_heartbeat: RwLock<Instant>,
    last_market_data: RwLock<Instant>,
    user_timeout: Duration,      // e.g., 30 minutes
    data_timeout: Duration,      // e.g., 5 minutes
}

impl DeadManSwitch {
    pub fn check(&self) -> Option<String> {
        let user_age = self.last_user_heartbeat.read().elapsed();
        let data_age = self.last_market_data.read().elapsed();

        if data_age > self.data_timeout {
            return Some("DEAD_MAN_SWITCH: No market data for 5+ minutes".into());
        }
        if user_age > self.user_timeout {
            return Some("DEAD_MAN_SWITCH: No user activity for 30+ minutes".into());
        }
        None
    }
}

// Add heartbeat endpoint:
// POST /api/v1/heartbeat → resets user heartbeat timer
// Dashboard should call this every 5 minutes while tab is active
```

#### TASK 4.3: Execution Latency Tracking

```rust
// Track P50/P95/P99 of order placement latency per symbol
// If P95 > threshold → pause trading on that symbol

pub struct LatencyTracker {
    per_symbol: RwLock<HashMap<String, LatencyHistogram>>,
}

pub struct LatencyHistogram {
    samples: VecDeque<Duration>,  // rolling window of last 100 orders
    max_samples: usize,
}

impl LatencyHistogram {
    pub fn add(&mut self, latency: Duration);
    pub fn p50(&self) -> Duration;
    pub fn p95(&self) -> Duration;
    pub fn p99(&self) -> Duration;
}
```

#### TASK 4.4: Trade Correlation Guard

```rust
// Before opening a new position, check correlation with existing positions
// If Pearson correlation > 0.85 with any open position → reject or reduce size

pub fn check_correlation(
    new_symbol: &str,
    open_positions: &[Position],
    price_cache: &PriceCache,  // last 60 1-minute candles per symbol
) -> CorrelationVerdict {
    for pos in open_positions {
        let corr = pearson_correlation(
            &price_cache.get_returns(&pos.symbol, 60),
            &price_cache.get_returns(new_symbol, 60),
        );
        if corr > 0.85 {
            return CorrelationVerdict::TooCorrelated {
                with_symbol: pos.symbol.clone(),
                correlation: corr,
            };
        }
    }
    CorrelationVerdict::Ok
}
```

#### TASK 4.5: Post-Trade TCA (Transaction Cost Analysis)

```rust
// After every trade, calculate execution quality metrics
pub struct TCARecord {
    pub trade_id: String,
    pub signal_price: f64,          // price when signal was generated
    pub arrival_price: f64,         // price when order was submitted
    pub fill_price: f64,            // actual fill price
    pub slippage_bps: f64,          // (fill - signal) / signal × 10000
    pub implementation_shortfall: f64,
    pub market_impact_5s: f64,      // price change 5s after fill
    pub spread_at_entry: f64,       // bid-ask spread when order placed
}

// If rolling average slippage > 5bps → switch from MARKET to LIMIT orders
// If rolling average slippage > 15bps → reduce position size by 50%
```

#### TASK 4.6: Graceful Shutdown Handler

```rust
// When the bot process receives SIGINT/SIGTERM:
// 1. Set trading_mode = Paused (no new trades)
// 2. Cancel all open LIMIT orders
// 3. Log all open positions (DO NOT auto-close — could be worse)
// 4. Save state to disk
// 5. Close WebSocket connections cleanly
// 6. Exit

// In main.rs:
tokio::select! {
    _ = server => {},
    _ = tokio::signal::ctrl_c() => {
        info!("Shutdown signal received");
        graceful_shutdown(&state, &client).await;
    }
}
```

---

### PHASE 5: DASHBOARD ADVANCED FEATURES (P2)

#### TASK 5.1: Real-Time PnL Chart

**File:** `dashboard_v2/src/components/PnLChart.tsx` (NEW)

```tsx
// Line chart showing:
// - Cumulative PnL over time (today)
// - Individual trade PnL as dots
// - Drawdown area (shaded red)
// Use Recharts (already installed)
// Data from: state.recent_trades
```

#### TASK 5.2: Position Details Modal

```tsx
// Click any position row → modal showing:
// - Full Decision Envelope (which gates passed/blocked, why)
// - Entry/exit prices, PnL
// - Execution latency
// - TCA metrics (slippage, market impact)
// - Timeline: created → placed → filled → closed
```

#### TASK 5.3: Risk Dashboard Improvements

```tsx
// Risk page must show REAL data:
// - Daily PnL gauge (animated, changes color at thresholds)
// - Circuit breaker state with visual indicator (green/yellow/red)
// - Correlation matrix heatmap (if multiple positions open)
// - Exposure pie chart (by symbol)
// - Rate limit usage bar
```

#### TASK 5.4: Notification System

```tsx
// Toast notifications for critical events:
// - Trade executed (green)
// - Trade rejected by risk (yellow)
// - Circuit breaker tripped (red)
// - WebSocket disconnected (red)
// - Reconciliation drift detected (orange)
// Use: react-hot-toast or build custom
```

---

## 🔒 SECURITY CHECKLIST

Before deploying ANY change:

- [ ] `secret` is NEVER logged anywhere (grep for it)
- [ ] All new endpoints have auth middleware
- [ ] Constant-time token comparison (`subtle::ConstantTimeEq` or `ring`)
- [ ] CORS restricted to specific origins in production
- [ ] Rate limiting on control endpoints (prevent accidental rapid-fire)
- [ ] Input validation on all POST bodies (max values, allowed strings)
- [ ] Kill switch works even if backend is partially broken
- [ ] Demo mode CANNOT accidentally place real orders

---

## 📁 FILE MAP (After all tasks complete)

```
backend/src/
├── main.rs                    (entry point + graceful shutdown)
├── config.rs                  (env config — EXISTS)
├── app_state.rs               (global state — EXISTS, extend)
├── runtime_config.rs          (atomic config — EXISTS)
├── api/
│   ├── mod.rs                 (EXISTS)
│   ├── rest.rs                (EXISTS, extend with new endpoints)
│   ├── ws.rs                  (EXISTS, add push updates)
│   └── auth.rs                (EXISTS, add constant-time compare)
├── binance/
│   ├── mod.rs                 (EXISTS, add event parsing)
│   ├── client.rs              (EXISTS, add trading methods + signing)
│   └── rate_limit.rs          (NEW)
├── execution.rs               (REWRITE — full execution engine)
├── position_engine.rs         (REWRITE — position state machine)
├── risk.rs                    (REWRITE — real risk engine)
├── reconcile.rs               (REWRITE — real reconciliation)
├── strategy.rs                (EXTEND — signal generation)
├── decision_envelope.rs       (EXTEND — real pipeline)
├── trade_insurance.rs         (EXTEND — more checks)
├── health.rs                  (EXISTS, extend)
├── idempotency.rs             (NEW)
├── dead_man_switch.rs         (NEW)
└── tca.rs                     (NEW — transaction cost analysis)
```

---

## ⚠️ CRITICAL REMINDERS

1. **AccountMode::Demo MUST be the default.** Never change this.
2. **Every order MUST use `newClientOrderId`** for idempotency.
3. **Every Binance API error MUST be logged** with status code and body.
4. **Never auto-close positions on error** — could make things worse.
5. **Circuit breaker can block new trades** but MUST allow closing existing ones.
6. **Reconciliation runs even when paused** — it's a safety check.
7. **state_version MUST increment** on every state change (it does — don't break this).
8. **Dashboard must degrade gracefully** — if API is down, show last known state + error banner.
9. **Test EVERY change with Demo mode first** — verify with curl + dashboard.
10. **Backward compatible** — old runtime_config.json must still load after schema changes (use `#[serde(default)]`).

---

## 🏁 COMPLETION CRITERIA

The bot is "production ready" when ALL of these pass:

```
□ cargo build --release compiles with zero warnings
□ Bot starts with Demo mode, shows GREEN in dashboard
□ Pause/Resume/Kill work from dashboard and curl
□ Switch to Live requires confirmation dialog
□ Place a LIMIT order on Binance testnet → see it in dashboard
□ Order fills → position shows with correct PnL
□ Close position → PnL updates correctly
□ Hit max_daily_loss → circuit breaker trips → dashboard shows red
□ Kill switch → all new orders blocked → existing positions unchanged
□ Disconnect network → Dead Man's Switch triggers within 5 minutes
□ Restart bot → reconciliation detects existing positions on exchange
□ Dashboard shows all data from API (zero hardcoded values)
□ WebSocket push works (no polling)
□ Rate limits tracked and displayed
□ grep -r "secret\|api_key\|token" shows NO leaks in logs
```
