/**
 * API client for Aurora Bot backend.
 * Base URL: VITE_API_URL or http://127.0.0.1:3000
 */

const BASE = (import.meta as unknown as { env: { VITE_API_URL?: string } }).env?.VITE_API_URL ?? 'http://127.0.0.1:3001';

function getAdminToken(): string | null {
  return (import.meta as unknown as { env: { VITE_ADMIN_TOKEN?: string } }).env?.VITE_ADMIN_TOKEN ?? null;
}

export interface TruthHeader {
  ws_mode: string;
  ws_user_connected: boolean;
  last_ws_user_event_age_ms: number;
  reconcile_last_ok_age_s: number | null;
  reconcile_last_error: string | null;
  no_go_reason: string | null;
  state_version: number;
  ws_sequence_number: number;
  trading_mode: string;
  risk_mode: string;
  server_time: number;
  // Wire latency (WebSocket → Binance)
  ws_wire_latency_ms?: number;
  ws_wire_latency_ema?: number;
  ws_wire_latency_peak?: number;
}

export interface BalanceInfo {
  asset: string;
  free: string;
  locked: string;
}

export interface PositionSnapshot {
  symbol: string;
  base_asset: string;
  quote_asset: string;
  free: string;
  locked: string;
  total: string;
}

export interface DecisionEnvelope {
  id: string;
  symbol: string;
  side: string;
  strategy_name: string;
  data_quality_verdict?: string;
  insurance_verdict?: string;
  risk_verdict?: string;
  execution_quality_verdict?: string;
  final_decision: string;
  blocking_layer?: string | null;
  reason?: string | null;
  created_at: string;
  arena_bridge?: BridgeInfluence | null;
  smart_filters?: SmartFilterVerdicts | null;
}

export interface OrderInfo {
  order_id: number;
  symbol: string;
  status: string;
  client_order_id: string;
  executed_qty: string;
  cummulative_quote_qty: string;
}

export interface ErrorRecord {
  message: string;
  code: string | null;
  at: string;
}

export interface RateLimitSnapshot {
  used_weight_1m: number;
  order_count_10s: number;
  order_count_1d: number;
}

export interface BinanceAccountFlags {
  can_trade: boolean;
  can_withdraw: boolean;
}

export interface SymbolMarketData {
  last_price: number;
  rsi_14?: number | null;
  ema_9?: number | null;
  ema_21?: number | null;
  ema_55?: number | null;
  adx?: number | null;
  atr_14?: number | null;
  bollinger_width?: number | null;
  roc_14?: number | null;
  spread_bps?: number | null;
  cvd: number;
  orderbook_imbalance: number;
  buy_volume_ratio: number;
}

export interface StateSnapshot {
  state_version: number;
  server_time: number;
  truth: TruthHeader;
  positions: PositionSnapshot[];
  recent_decisions: DecisionEnvelope[];
  risk: {
    risk_mode: string;
    daily_pnl?: number;
    daily_pnl_pct?: number;
    remaining_daily_loss_pct?: number;
    circuit_breakers?: Record<string, { name: string; current: number | string; limit: number | string; tripped: boolean }>;
  };
  runtime_config: {
    trading_mode: string;
    account_mode?: string;
    symbols?: string[];
    max_concurrent_positions?: number;
    max_daily_loss_pct?: number;
    max_consecutive_losses?: number;
    max_trades_per_day?: number;
  };
  balances?: BalanceInfo[];
  open_orders?: OrderInfo[];
  rate_limits?: RateLimitSnapshot;
  binance_account?: BinanceAccountFlags | null;
  recent_errors?: ErrorRecord[];
  last_heartbeat_age_s?: number | null;
  market_data?: { symbols: Record<string, SymbolMarketData> };
  regime?: { regime: string; adx?: number; bbw?: number; hurst?: number; entropy?: number; regime_age_seconds?: number } | null;
  scoring?: { total_score: number; decision: string; signal_contributions: Array<{ name: string; weight: number; confidence: number; direction: number; contribution: number }> } | null;
  vpin?: Record<string, { vpin: number; zone?: string; buy_volume?: number; sell_volume?: number }>;
  futures_intel?: Record<string, { funding_rate: number; funding_signal?: string; open_interest?: number; oi_change_1h_pct?: number; long_short_ratio?: number; ls_signal?: string }>;
  journal_stats?: { total_trades: number; win_rate: number; total_net_pnl: number; profit_factor: number } | null;
  barrier_states?: Record<string, unknown>;
  arena?: ArenaState;
  ladder?: {
    current_rung: number;
    quote_amount: number;
    consecutive_wins: number;
    equity_curve_hot: boolean;
    max_allowed_rung: number;
    ema_fast: number;
    ema_slow: number;
    total_trades: number;
    total_wins: number;
    total_losses: number;
    rung_history: Array<{
      timestamp: string;
      from_rung: number;
      to_rung: number;
      reason: string;
      quote_amount: number;
    }>;
    equity_points: number[];
  };
  // Phase 1: HTF analysis per symbol
  htf_analysis?: Record<string, HtfAnalysis>;
  // Phase 1: Feature flags state
  feature_flags?: FeatureFlags;
}

// ═══ Arena API types ═══
export interface RegimeStats {
  trades: number;
  wins: number;
  win_rate: number;
  total_pnl_pct: number;
  avg_pnl_pct: number;
}

export interface ProfileState {
  id: string;
  name: string;
  icon: string;
  color: string;
  description: string;
  enabled: boolean;
  is_active: boolean;
  min_confidence: number;
  max_daily_trades: number;
  today_trade_count: number;
  open_trades: number;
  total_trades: number;
  wins: number;
  losses: number;
  win_rate: number;
  total_pnl_pct: number;
  today_pnl_pct: number;
  today_trades: number;
  today_wins: number;
  profit_factor: number;
  current_streak: number;
  thompson_score: number;
  regime_performance: Record<string, RegimeStats>;
}

export interface ShadowTrade {
  id: string;
  profile_id: string;
  symbol: string;
  direction: string;
  entry_price: number;
  stop_loss: number;
  take_profit_1: number;
  take_profit_2: number;
  quantity_pct: number;
  signal_score: number;
  regime_at_entry: string;
  opened_at: string;
  max_hold_minutes: number;
  status: 'Open' | 'ClosedTP1' | 'ClosedTP2' | 'ClosedSL' | 'ClosedTime';
  current_price: number;
  unrealized_pnl_pct: number;
  mfe_pct: number;
  mae_pct: number;
  closed_at: string | null;
  exit_reason: string | null;
  realized_pnl_pct: number;
}

// ═══ Phase 1: Smart Filters types ═══
export interface HtfAnalysis {
  direction: string;
  confidence: number;
  buy_allowed: boolean;
  sell_signal: boolean;
  trend_15m: number;
  trend_1h: number;
  ema_sep_15m: number;
  ema_sep_1h: number;
  momentum_1h: number;
  candles_15m: number;
  candles_1h: number;
  reason: string;
}

export interface ScoreMomentumState {
  avg_score: number;
  positive_ratio: number;
  score_trend: number;
  buy_allowed: boolean;
  reason: string;
  history_len: number;
}

export interface OfipState {
  avg_imbalance: number;
  persistence: number;
  buy_blocked: boolean;
  buy_boost: boolean;
  reason: string;
  history_len: number;
}

// ═══ Phase 2: CUSUM + Absorption + Entropy Valley types ═══

export interface CusumState {
  s_plus: number;
  s_minus: number;
  threshold: number;
  rolling_mean: number;
  rolling_std: number;
  bullish_break: boolean;
  bearish_break: boolean;
  break_confidence: number;
  candles_since_break: number;
  candles_available: number;
  reason: string;
}

export interface CusumHtfInteraction {
  soft_block: boolean;
  position_factor: number;
  reason: string;
}

export interface AbsorptionState {
  detected: boolean;
  direction: string;
  strength: number;
  cvd_confirmed: boolean;
  effective_weight: number;
  best_volume_ratio: number;
  best_range_ratio: number;
  taker_buy_ratio: number;
  absorption_count: number;
  candles_available: number;
  reason: string;
}

export interface EntropyValleyState {
  valley_detected: boolean;
  recent_max_entropy: number;
  current_entropy: number;
  drop_magnitude: number;
  confidence_boost: number;
  history_len: number;
  reason: string;
}

export interface MicroTrailSnapshot {
  phase: string;
  best_price: number;
  trail_price: number;
  raw_trail_distance: number;
  adjusted_trail_distance: number;
  of_tighten_mult: number;
  velocity_triggered: boolean;
  atr_5m: number;
  adjustment_reason: string;
}

export interface SmartFilterVerdicts {
  htf?: HtfAnalysis | null;
  score_momentum?: ScoreMomentumState | null;
  ofip?: OfipState | null;
  adaptive_threshold?: number | null;
  entropy_multiplier?: number | null;
  // Phase 2
  cusum?: CusumState | null;
  cusum_htf_interaction?: CusumHtfInteraction | null;
  absorption?: AbsorptionState | null;
  entropy_valley?: EntropyValleyState | null;
  // Phase 3 — Exit Intelligence
  micro_trail?: MicroTrailSnapshot | null;
}

export interface FeatureFlags {
  htf_gate: boolean;
  score_momentum: boolean;
  ofip: boolean;
  adaptive_threshold: boolean;
  entropy_graduated: boolean;
  // Phase 2
  cusum: boolean;
  absorption: boolean;
  entropy_valley: boolean;
  // Phase 3 — Exit Intelligence
  micro_trail: boolean;
}

export interface BridgeInfluence {
  active: boolean;
  reason: string;
  profile_id: string;
  thompson_confidence: number;
  sl_atr_multiplier: number;
  tp1_atr_multiplier: number;
  tp2_atr_multiplier: number;
  position_size_factor: number;
  confidence_adjustment: number;
}

export interface ArenaState {
  profiles: ProfileState[];
  active_profile_id: string;
  auto_select: boolean;
  open_shadow_trades: ShadowTrade[];
  recent_closed_trades: ShadowTrade[];
  next_evaluation_secs: number;
  total_evaluations: number;
}

export async function fetchArenaState(): Promise<ArenaState> {
  const token = getAdminToken();
  if (!token) throw new Error('Admin token required');
  const r = await fetch(`${BASE}/api/v1/arena`, { headers: { Authorization: `Bearer ${token}` } });
  if (!r.ok) throw new Error(`Arena API ${r.status}`);
  return r.json();
}

export async function arenaManualOverride(profileId: string): Promise<void> {
  const token = getAdminToken();
  if (!token) throw new Error('Admin token required');
  const r = await fetch(`${BASE}/api/v1/arena/override`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', Authorization: `Bearer ${token}` },
    body: JSON.stringify({ profile_id: profileId }),
  });
  if (!r.ok) throw new Error(`Arena override ${r.status}`);
}

export async function arenaSetAutoSelect(enabled: boolean): Promise<void> {
  const token = getAdminToken();
  if (!token) throw new Error('Admin token required');
  const r = await fetch(`${BASE}/api/v1/arena/auto-select`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', Authorization: `Bearer ${token}` },
    body: JSON.stringify({ enabled }),
  });
  if (!r.ok) throw new Error(`Arena auto-select ${r.status}`);
}

export async function setAccountMode(mode: 'demo' | 'live', confirmLive = false): Promise<{ account_mode: string }> {
  const token = getAdminToken();
  if (!token) throw new Error('Admin token required');
  const r = await fetch(`${BASE}/api/v1/control/account-mode`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', Authorization: `Bearer ${token}` },
    body: JSON.stringify({ account_mode: mode, confirm_live: confirmLive }),
  });
  if (!r.ok) throw new Error(`Account mode ${r.status}`);
  return r.json();
}

export async function fetchHealth(): Promise<{ status: string; state_version: number; server_time: number }> {
  const r = await fetch(`${BASE}/api/v1/health`);
  if (!r.ok) throw new Error(`Health ${r.status}`);
  return r.json();
}

export async function fetchState(): Promise<StateSnapshot> {
  const token = getAdminToken();
  const headers: HeadersInit = {};
  if (token) headers['Authorization'] = `Bearer ${token}`;
  const r = await fetch(`${BASE}/api/v1/state`, { headers });
  if (r.status === 403) throw new Error('403: Missing or invalid admin token');
  if (!r.ok) throw new Error(`State ${r.status}`);
  return r.json();
}

export async function controlPause(): Promise<void> {
  const token = getAdminToken();
  if (!token) throw new Error('Admin token required');
  const r = await fetch(`${BASE}/api/v1/control/pause`, { method: 'POST', headers: { Authorization: `Bearer ${token}` } });
  if (!r.ok) throw new Error(`Pause ${r.status}`);
}

export async function controlResume(): Promise<void> {
  const token = getAdminToken();
  if (!token) throw new Error('Admin token required');
  const r = await fetch(`${BASE}/api/v1/control/resume`, { method: 'POST', headers: { Authorization: `Bearer ${token}` } });
  if (!r.ok) throw new Error(`Resume ${r.status}`);
}

export async function controlKill(): Promise<void> {
  const token = getAdminToken();
  if (!token) throw new Error('Admin token required');
  const r = await fetch(`${BASE}/api/v1/control/kill`, { method: 'POST', headers: { Authorization: `Bearer ${token}` } });
  if (!r.ok) throw new Error(`Kill ${r.status}`);
}

export function stateWebSocketUrl(): string {
  const u = new URL(BASE);
  const wsProto = u.protocol === 'https:' ? 'wss:' : 'ws:';
  const token = getAdminToken();
  const tokenParam = token ? `?token=${encodeURIComponent(token)}` : '';
  return `${wsProto}//${u.host}/api/v1/ws${tokenParam}`;
}

// ═══ Phase 1: Feature Flags API ═══

export async function fetchFeatureFlags(): Promise<FeatureFlags> {
  const token = getAdminToken();
  if (!token) throw new Error('Admin token required');
  const r = await fetch(`${BASE}/api/v1/feature-flags`, { headers: { Authorization: `Bearer ${token}` } });
  if (!r.ok) throw new Error(`Feature flags ${r.status}`);
  return r.json();
}

export async function setFeatureFlags(flags: Partial<FeatureFlags>): Promise<FeatureFlags & { changes: string[] }> {
  const token = getAdminToken();
  if (!token) throw new Error('Admin token required');
  const r = await fetch(`${BASE}/api/v1/feature-flags`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', Authorization: `Bearer ${token}` },
    body: JSON.stringify(flags),
  });
  if (!r.ok) throw new Error(`Feature flags update ${r.status}`);
  return r.json();
}

/** Dead Man's Switch: call every 5 min while tab is active so bot does not auto-pause. */
export async function sendHeartbeat(): Promise<void> {
  const token = getAdminToken();
  const headers: HeadersInit = {};
  if (token) headers['Authorization'] = `Bearer ${token}`;
  const r = await fetch(`${BASE}/api/v1/heartbeat`, { method: 'POST', headers });
  if (!r.ok) throw new Error(`Heartbeat ${r.status}`);
}
