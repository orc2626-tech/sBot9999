/**
 * Strategy Arena — عرض حلبة الاستراتيجيات (بيانات حقيقية فقط)
 *
 * مصدر البيانات: WebSocket (state.arena) أو REST fallback (fetchArenaState).
 *
 * أخطاء موثقة (Error codes):
 * - ARENA_NO_TOKEN   : VITE_ADMIN_TOKEN غير معرّف → 403
 * - ARENA_404        : مسار /api/v1/arena غير موجود (Backend قديم)
 * - ARENA_NETWORK    : فشل الشبكة أو CORS / انقطع الاتصال
 * - ARENA_EMPTY      : الاستجابة فارغة (profiles = [])
 * - ARENA_OVERRIDE   : فشل تغيير الوضع اليدوي
 * - ARENA_AUTO       : فشل تفعيل/إلغاء Auto-Select
 */

import { useState, useEffect, useCallback } from 'react'
import type { ArenaState, ProfileState, ShadowTrade, RegimeStats } from '../../lib/api'
import { fetchArenaState, arenaManualOverride, arenaSetAutoSelect } from '../../lib/api'

const REGIMES = ['TRENDING', 'RANGING', 'VOLATILE', 'SQUEEZE'] as const
const REGIME_COLORS: Record<string, string> = {
  TRENDING: '#10B981',
  RANGING: '#3B82F6',
  VOLATILE: '#EF4444',
  SQUEEZE: '#F59E0B',
}

export type ArenaErrorCode =
  | 'ARENA_NO_TOKEN'
  | 'ARENA_404'
  | 'ARENA_NETWORK'
  | 'ARENA_EMPTY'
  | 'ARENA_OVERRIDE'
  | 'ARENA_AUTO'
  | null

interface Props {
  /** من WebSocket عند الاتصال؛ إن وُجد لا نستخدم REST */
  wsArenaState?: ArenaState | null
}

export default function StrategyArena({ wsArenaState }: Props) {
  const [arena, setArena] = useState<ArenaState | null>(null)
  const [loading, setLoading] = useState(true)
  const [errorCode, setErrorCode] = useState<ArenaErrorCode>(null)
  const [errorDetail, setErrorDetail] = useState<string | null>(null)
  const [showOverrideConfirm, setShowOverrideConfirm] = useState<string | null>(null)
  const [dataSource, setDataSource] = useState<'ws' | 'rest'>('rest')

  const clearError = useCallback(() => {
    setErrorCode(null)
    setErrorDetail(null)
  }, [])

  useEffect(() => {
    if (wsArenaState != null) {
      setArena(wsArenaState)
      setDataSource('ws')
      setLoading(false)
      setErrorCode(null)
      setErrorDetail(null)
      return
    }
    setDataSource('rest')
    const load = async () => {
      try {
        const data = await fetchArenaState()
        if (!data || !Array.isArray(data.profiles) || data.profiles.length === 0) {
          setErrorCode('ARENA_EMPTY')
          setErrorDetail('Backend returned no profiles')
          setArena(data)
        } else {
          setArena(data)
          setErrorCode(null)
          setErrorDetail(null)
        }
      } catch (err: unknown) {
        const msg = err instanceof Error ? err.message : String(err)
        if (msg.includes('token') || msg.includes('403')) {
          setErrorCode('ARENA_NO_TOKEN')
          setErrorDetail('Set VITE_ADMIN_TOKEN in .env')
        } else if (msg.includes('404') || msg.includes('not found')) {
          setErrorCode('ARENA_404')
          setErrorDetail('Backend may not expose /api/v1/arena. Rebuild backend.')
        } else {
          setErrorCode('ARENA_NETWORK')
          setErrorDetail(msg)
        }
        setArena(null)
      } finally {
        setLoading(false)
      }
    }
    load()
    const interval = setInterval(load, 5000)
    return () => clearInterval(interval)
  }, [wsArenaState])

  const handleOverride = useCallback(async (profileId: string) => {
    try {
      await arenaManualOverride(profileId)
      setShowOverrideConfirm(null)
      const data = await fetchArenaState()
      setArena(data)
      setErrorCode(null)
    } catch (err: unknown) {
      setErrorCode('ARENA_OVERRIDE')
      setErrorDetail(err instanceof Error ? err.message : 'Override failed')
    }
  }, [])

  const handleAutoSelect = useCallback(async (enabled: boolean) => {
    try {
      await arenaSetAutoSelect(enabled)
      const data = await fetchArenaState()
      setArena(data)
      setErrorCode(null)
    } catch (err: unknown) {
      setErrorCode('ARENA_AUTO')
      setErrorDetail(err instanceof Error ? err.message : 'Auto-select failed')
    }
  }, [])

  if (loading && !arena) {
    return <ArenaSkeleton />
  }

  if (errorCode && !arena) {
    return (
      <ArenaErrorPanel
        code={errorCode}
        detail={errorDetail}
        onRetry={() => {
          clearError()
          setLoading(true)
          fetchArenaState()
            .then((data) => {
              setArena(data)
              setErrorCode(null)
              setErrorDetail(null)
            })
            .catch(() => setErrorCode('ARENA_NETWORK'))
            .finally(() => setLoading(false))
        }}
      />
    )
  }

  if (!arena) {
    return null
  }

  const activeProfile = arena.profiles.find((p) => p.is_active)
  const hasOpenShadowTrades = arena.open_shadow_trades.length > 0
  const hasRecentClosed = arena.recent_closed_trades.length > 0

  return (
    <div className="rounded-xl overflow-hidden border border-[#1c2538] bg-[#0f1520] font-sans text-[#e8edf5]">
      {/* Header */}
      <div className="px-5 py-4 border-b border-[#1c2538] flex flex-wrap items-center justify-between gap-3">
        <div className="flex items-center gap-3">
          <span className="text-2xl">🏟️</span>
          <span className="text-lg font-bold">Strategy Arena</span>
          <span className="text-xs text-[#546380] bg-[#141c2b] px-2.5 py-1 rounded-full">
            {arena.total_evaluations.toLocaleString()} evals
          </span>
          {dataSource === 'ws' && (
            <span className="text-[10px] uppercase tracking-wider text-emerald-500/90 bg-emerald-500/10 px-2 py-0.5 rounded">
              Live
            </span>
          )}
        </div>
        <div className="flex items-center gap-4">
          <span className="text-xs text-[#7a889c]">Auto-Select</span>
          <button
            type="button"
            onClick={() => handleAutoSelect(!arena.auto_select)}
            className={`relative w-10 h-6 rounded-full transition-colors ${arena.auto_select ? 'bg-emerald-500' : 'bg-[#1c2538]'}`}
            aria-label={arena.auto_select ? 'Disable auto-select' : 'Enable auto-select'}
          >
            <span
              className={`absolute top-1 w-5 h-5 bg-white rounded-full transition-transform ${arena.auto_select ? 'left-5' : 'left-1'}`}
            />
          </button>
          {arena.auto_select && (
            <span className="text-xs text-[#546380]">
              Next: {Math.floor(arena.next_evaluation_secs / 60)}m {arena.next_evaluation_secs % 60}s
            </span>
          )}
        </div>
      </div>

      {/* Active profile banner */}
      {activeProfile && (
        <div
          className="px-5 py-3 flex flex-wrap items-center gap-3"
          style={{ background: `linear-gradient(90deg, ${activeProfile.color}08, transparent)` }}
        >
          <span className="text-2xl">{activeProfile.icon}</span>
          <span className="font-bold text-[#e8edf5]">{activeProfile.name}</span>
          <span className="text-sm text-[#546380]">
            — Active • Thompson selected for current regime
          </span>
          <span
            className="ml-auto font-mono font-bold text-sm"
            style={{ color: activeProfile.color }}
          >
            {(activeProfile.thompson_score * 100).toFixed(1)}%
          </span>
        </div>
      )}

      {/* Profile cards — 4 columns */}
      <div className="px-5 py-4 grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-4 gap-3">
        {arena.profiles.map((p) => (
          <ProfileCard
            key={p.id}
            profile={p}
            onSelect={() => setShowOverrideConfirm(p.name)}
            totalEvaluations={arena.total_evaluations}
          />
        ))}
      </div>

      {/* Open shadow trades */}
      {hasOpenShadowTrades && (
        <div className="px-5 pb-4">
          <div className="text-xs font-bold text-[#546380] uppercase tracking-wider mb-2">
            Open Shadow Trades ({arena.open_shadow_trades.length})
          </div>
          <div className="overflow-x-auto rounded-lg border border-[#141c2b]">
            <table className="w-full text-sm border-collapse">
              <thead>
                <tr className="text-[10px] text-[#3d4f6a] uppercase tracking-wider">
                  <th className="text-left py-2 px-2 font-semibold">Profile</th>
                  <th className="text-left py-2 px-2 font-semibold">Symbol</th>
                  <th className="text-right py-2 px-2 font-semibold">Entry</th>
                  <th className="text-right py-2 px-2 font-semibold">Current</th>
                  <th className="text-right py-2 px-2 font-semibold">PnL</th>
                  <th className="text-right py-2 px-2 font-semibold">MFE</th>
                  <th className="text-right py-2 px-2 font-semibold">MAE</th>
                  <th className="text-center py-2 px-2 font-semibold">Regime</th>
                </tr>
              </thead>
              <tbody>
                {arena.open_shadow_trades.map((t) => (
                  <ShadowTradeRow key={t.id} trade={t} profiles={arena.profiles} />
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}

      {/* Empty state: no shadow trades */}
      {!hasOpenShadowTrades && (
        <div className="px-5 py-6 text-center">
          <div className="rounded-xl border border-dashed border-[#1c2538] bg-[#080c14] p-6">
            <div className="text-4xl mb-2">⏳</div>
            <p className="text-[#7a889c] text-sm leading-relaxed">
              السوق في حالة <span className="text-amber-400 font-bold">SQUEEZE</span> أو هادئ —{' '}
              <span className="text-[#546380]">ADX منخفض</span>
            </p>
            <p className="text-[#546380] text-xs mt-2">
              عندما يتحول السوق إلى <span className="text-emerald-400">TRENDING</span> مع ADX &gt; 25
              ستظهر Shadow Trades وأرقام حقيقية
            </p>
          </div>
        </div>
      )}

      {/* Recent W/L */}
      {hasRecentClosed && (
        <div className="px-5 pb-4">
          <div className="text-xs font-bold text-[#546380] uppercase tracking-wider mb-2">
            Recent Results ({arena.recent_closed_trades.length})
          </div>
          <div className="flex flex-wrap gap-1">
            {arena.recent_closed_trades.slice(-30).map((t) => {
              const isWin = t.realized_pnl_pct > 0
              return (
                <div
                  key={t.id}
                  className={`w-7 h-7 rounded-md text-xs font-bold font-mono flex items-center justify-center ${
                    isWin ? 'bg-emerald-500/15 text-emerald-400 border border-emerald-500/30' : 'bg-red-500/15 text-red-400 border border-red-500/30'
                  }`}
                  title={`${t.profile_id} | ${t.symbol} | ${t.realized_pnl_pct >= 0 ? '+' : ''}${t.realized_pnl_pct.toFixed(2)}%`}
                >
                  {isWin ? 'W' : 'L'}
                </div>
              )
            })}
          </div>
        </div>
      )}

      {/* Regime × Profile matrix */}
      {arena.profiles.length > 0 && (
        <div className="px-5 pb-5">
          <div className="flex items-center gap-2 mb-3">
            <span className="text-lg">📊</span>
            <span className="text-sm font-bold">Regime × Profile Matrix</span>
          </div>
          <div className="overflow-x-auto rounded-lg border border-[#141c2b]">
            <table className="w-full text-sm border-collapse">
              <thead>
                <tr>
                  <th className="text-left text-[#546380] py-2 px-2 text-xs">Profile</th>
                  {REGIMES.map((r) => (
                    <th key={r} className="text-center py-2 px-2">
                      <span
                        className="text-[10px] px-2 py-1 rounded-md font-bold"
                        style={{ background: REGIME_COLORS[r] + '18', color: REGIME_COLORS[r] }}
                      >
                        {r}
                      </span>
                    </th>
                  ))}
                </tr>
              </thead>
              <tbody>
                {arena.profiles.map((profile) => (
                  <tr key={profile.id} className="border-t border-[#141c2b]">
                    <td className="py-2 px-2">
                      <span className="flex items-center gap-1.5">
                        <span>{profile.icon}</span>
                        <span className="text-[#c8d0dc] font-medium">{profile.name}</span>
                        {profile.is_active && (
                          <span className="w-1.5 h-1.5 rounded-full bg-emerald-500 shadow-[0_0_6px_#10B981]" />
                        )}
                      </span>
                    </td>
                    {REGIMES.map((regime) => {
                      const s = profile.regime_performance?.[regime]
                      return (
                        <td key={regime} className="text-center py-2 px-1">
                          {s && s.trades > 0 ? (
                            <RegimeCell stats={s} color={profile.color} />
                          ) : (
                            <span className="text-[#1c2538]">—</span>
                          )}
                        </td>
                      )
                    })}
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          <div className="flex gap-4 mt-2 text-xs text-[#546380]">
            <span>🟢 &gt;60%</span>
            <span>🟡 45–60%</span>
            <span>🔴 &lt;45%</span>
          </div>
        </div>
      )}

      {/* Override confirm modal */}
      {showOverrideConfirm && (
        <div className="fixed inset-0 bg-black/60 flex items-center justify-center z-50">
          <div className="bg-[#0f1520] rounded-xl p-6 max-w-sm w-full border border-[#1c2538]">
            <h3 className="text-[#e8edf5] font-semibold mb-2">Manual Override</h3>
            <p className="text-[#7a889c] text-sm mb-4">
              Switch active profile to <strong className="text-white">{showOverrideConfirm}</strong>?
              This will disable Auto-Select.
            </p>
            <div className="flex gap-3">
              <button
                type="button"
                onClick={() => setShowOverrideConfirm(null)}
                className="flex-1 px-4 py-2 bg-[#1c2538] rounded-lg text-[#c8d0dc] hover:bg-[#141c2b]"
              >
                Cancel
              </button>
              <button
                type="button"
                onClick={() => handleOverride(showOverrideConfirm)}
                className="flex-1 px-4 py-2 bg-blue-500 rounded-lg text-white hover:bg-blue-600"
              >
                Confirm
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  )
}

function ArenaSkeleton() {
  return (
    <div className="rounded-xl border border-[#1c2538] bg-[#0f1520] p-6 animate-pulse">
      <div className="h-6 bg-[#1c2538] rounded w-48 mb-4" />
      <div className="grid grid-cols-2 xl:grid-cols-4 gap-3">
        {[1, 2, 3, 4].map((i) => (
          <div key={i} className="h-44 bg-[#141c2b] rounded-xl" />
        ))}
      </div>
    </div>
  )
}

function ArenaErrorPanel({
  code,
  detail,
  onRetry,
}: {
  code: ArenaErrorCode
  detail: string | null
  onRetry: () => void
}) {
  const is404 = code === 'ARENA_404'
  const isToken = code === 'ARENA_NO_TOKEN'

  return (
    <div
      className={`rounded-xl border p-6 ${
        is404 ? 'border-amber-500/50 bg-amber-500/5' : 'border-red-500/50 bg-red-500/5'
      }`}
    >
      <p className="font-mono text-sm text-amber-400/90 mb-1">
        {code ?? 'ARENA_ERROR'}
      </p>
      <p className="text-[#c8d0dc] text-sm mb-3">{detail ?? 'No details'}</p>
      {is404 && (
        <p className="text-xs text-[#7a889c] mb-3">
          Ensure backend is running and built with Arena (VITE_API_URL same port).
        </p>
      )}
      {isToken && (
        <p className="text-xs text-[#7a889c] mb-3">
          Add VITE_ADMIN_TOKEN to dashboard_v2/.env (same as AURORA_ADMIN_TOKEN on backend).
        </p>
      )}
      <button
        type="button"
        onClick={onRetry}
        className="px-4 py-2 rounded-lg bg-[#1c2538] text-[#e8edf5] hover:bg-[#141c2b] text-sm font-medium"
      >
        Retry
      </button>
    </div>
  )
}

function ProfileCard({
  profile,
  onSelect,
  totalEvaluations = 0,
}: {
  profile: ProfileState
  onSelect: () => void
  totalEvaluations?: number
}) {
  const wrPct = Math.max(profile.win_rate * 100, 2)
  const streakText =
    profile.current_streak > 0
      ? `+${profile.current_streak}W`
      : profile.current_streak < 0
        ? `${profile.current_streak}L`
        : '—'

  return (
    <div
      role="button"
      tabIndex={0}
      onClick={onSelect}
      onKeyDown={(e) => e.key === 'Enter' && onSelect()}
      className={`relative rounded-xl border p-4 transition-all cursor-pointer ${
        profile.is_active
          ? 'opacity-100 border-2 shadow-lg'
          : 'opacity-80 border-[#1c2538] hover:opacity-100'
      }`}
      style={{
        background: profile.is_active
          ? `linear-gradient(160deg, ${profile.color}06, ${profile.color}12)`
          : '#0f1520',
        borderColor: profile.is_active ? profile.color : undefined,
        boxShadow: profile.is_active ? `0 0 24px ${profile.color}15` : undefined,
      }}
    >
      {profile.is_active && (
        <div
          className="absolute top-0 right-0 text-[9px] font-extrabold uppercase tracking-widest px-2.5 py-1 rounded-bl-md text-white"
          style={{ background: profile.color }}
        >
          Active
        </div>
      )}
      <div className="flex items-center gap-2 mb-3">
        <span className="text-2xl">{profile.icon}</span>
        <div>
          <div className="text-[#e8edf5] font-bold text-sm">{profile.name}</div>
          <div className="text-[#546380] text-[10px]">{profile.description}</div>
        </div>
      </div>
      <div className="text-xs space-y-1">
        <Row label="Today" value={<span className="font-mono">{profile.today_wins}/{profile.today_trades} {pnlSpan(profile.today_pnl_pct)}</span>} />
        <Row label="Total" value={<span className="font-mono">{profile.wins}/{profile.total_trades} {pnlSpan(profile.total_pnl_pct)}</span>} />
      </div>
      <div className="mt-3">
        <div className="flex justify-between text-[10px] text-[#546380] mb-1">
          <span>Win Rate</span>
          <span className="text-[#c8d0dc] font-mono">{(profile.win_rate * 100).toFixed(1)}%</span>
        </div>
        <div className="h-1.5 bg-[#141c2b] rounded-full overflow-hidden">
          <div
            className="h-full rounded-full transition-[width] duration-300"
            style={{
              width: `${wrPct}%`,
              background: `linear-gradient(90deg, ${profile.color}70, ${profile.color})`,
              boxShadow: `0 0 6px ${profile.color}30`,
            }}
          />
        </div>
      </div>
      <div className="grid grid-cols-2 gap-x-3 gap-y-1 mt-3 text-[11px]">
        <Row label="Thompson" value={<span style={{ color: profile.color }} className="font-bold font-mono">{(profile.thompson_score * 100).toFixed(1)}%</span>} />
        <Row
          label="Streak"
          value={
            <span
              className={`font-mono ${
                profile.current_streak > 0
                  ? 'text-emerald-400'
                  : profile.current_streak < 0
                    ? 'text-red-400'
                    : 'text-[#546380]'
              }`}
            >
              {streakText}
            </span>
          }
        />
        <Row label="Daily" value={<span className="font-mono text-[#7a889c]">{profile.today_trade_count}/{profile.max_daily_trades}</span>} />
        <Row label="Shadow" value={<span className={`font-mono ${profile.open_trades > 0 ? 'text-cyan-400' : 'text-[#546380]'}`}>{profile.open_trades} open</span>} />
      </div>
      {/* Arena Bridge Status */}
      {profile.is_active && (
        <div className="mt-2 pt-2 border-t border-slate-700/50">
          <div className="flex items-center gap-1.5 text-xs">
            <span className="text-[#546380]">Bridge:</span>
            {totalEvaluations >= 20 ? (
              <span className="text-emerald-400">● Active — Influencing SL/TP</span>
            ) : (
              <span className="text-[#546380]">○ Learning ({totalEvaluations}/20 samples)</span>
            )}
          </div>
        </div>
      )}
    </div>
  )
}

function Row({
  label,
  value,
}: {
  label: string
  value: React.ReactNode
}) {
  return (
    <div className="flex justify-between items-center">
      <span className="text-[#546380]">{label}</span>
      <span className="text-[#c8d0dc]">{value}</span>
    </div>
  )
}

function pnlSpan(v: number) {
  const color = v >= 0 ? 'text-emerald-400' : 'text-red-400'
  const sign = v >= 0 ? '+' : ''
  return <span className={`${color} font-mono font-semibold ml-1`}>{sign}{v.toFixed(2)}%</span>
}

function ShadowTradeRow({ trade, profiles }: { trade: ShadowTrade; profiles: ProfileState[] }) {
  const profile = profiles.find((p) => p.id === trade.profile_id || p.name === trade.profile_id)
  const pnl = trade.unrealized_pnl_pct ?? 0
  const color = pnl >= 0 ? 'text-emerald-400' : 'text-red-400'

  return (
    <tr className="border-t border-[#141c2b]">
      <td className="py-2 px-2">
        <span className="flex items-center gap-1.5 text-sm">
          <span>{profile?.icon ?? '•'}</span>
          <span className="text-[#c8d0dc]">{profile?.name ?? trade.profile_id}</span>
          {profile?.is_active && (
            <span className="w-1.5 h-1.5 rounded-full bg-emerald-500 shadow-[0_0_5px_#10B981]" />
          )}
        </span>
      </td>
      <td className="text-[#7a889c] text-sm">{trade.symbol}</td>
      <td className="text-right text-[#546380] font-mono text-xs">{trade.entry_price.toFixed(2)}</td>
      <td className="text-right text-[#e8edf5] font-mono text-xs">{trade.current_price.toFixed(2)}</td>
      <td className={`text-right font-mono text-xs font-bold ${color}`}>
        {pnl >= 0 ? '+' : ''}{pnl.toFixed(3)}%
      </td>
      <td className="text-right text-emerald-400 font-mono text-[10px]">+{(trade.mfe_pct ?? 0).toFixed(2)}%</td>
      <td className="text-right text-red-400 font-mono text-[10px]">{(trade.mae_pct ?? 0).toFixed(2)}%</td>
      <td className="text-center">
        <span
          className="text-[10px] px-1.5 py-0.5 rounded-md"
          style={{ background: (REGIME_COLORS[trade.regime_at_entry] ?? '#546380') + '18', color: REGIME_COLORS[trade.regime_at_entry] ?? '#7a889c' }}
        >
          {trade.regime_at_entry}
        </span>
      </td>
    </tr>
  )
}

function RegimeCell({ stats }: { stats: RegimeStats; color?: string }) {
  const wr = stats.win_rate * 100
  const c = wr >= 60 ? '#10B981' : wr >= 45 ? '#F59E0B' : '#EF4444'
  return (
    <div className="inline-block min-w-[70px] rounded-lg px-1.5 py-1 text-center" style={{ background: c + '12' }}>
      <div className="font-bold font-mono text-sm" style={{ color: c }}>{wr.toFixed(0)}%</div>
      <div className="text-[9px] text-[#546380] mt-0.5">
        {stats.wins}/{stats.trades} • {stats.total_pnl_pct >= 0 ? '+' : ''}{stats.total_pnl_pct.toFixed(1)}%
      </div>
    </div>
  )
}
