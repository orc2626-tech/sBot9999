/**
 * PnL Equity Curve — real-time equity line chart from trade journal.
 * 
 * Shows cumulative PnL over time with color-coded zones:
 * - Green zone: profit territory
 * - Red zone: drawdown territory  
 * - Peak equity line (dotted)
 * 
 * Dashboard audit fix: "No PnL curve" → now included.
 */

import { useEffect, useState, useMemo } from 'react'
import {
  AreaChart, Area, XAxis, YAxis, Tooltip, ResponsiveContainer,
  ReferenceLine, CartesianGrid,
} from 'recharts'
import { TrendingUp, TrendingDown, Award, BarChart3 } from 'lucide-react'
import { useTruthState } from '../hooks/useTruthState'

interface TradeRecord {
  id: string
  symbol: string
  net_pnl: number
  net_pnl_pct: number
  close_reason: string
  closed_at: string
  opened_at: string
  duration_secs: number
}

interface EquityPoint {
  time: string
  pnl: number
  cumPnl: number
  peak: number
  drawdown: number
  trade: string
}

const BASE = (import.meta as unknown as { env: { VITE_API_URL?: string } }).env?.VITE_API_URL ?? 'http://127.0.0.1:3001'

function getAdminToken(): string | null {
  return (import.meta as unknown as { env: { VITE_ADMIN_TOKEN?: string } }).env?.VITE_ADMIN_TOKEN ?? null
}

export default function PnlEquityCurve() {
  const { state } = useTruthState()
  const [trades, setTrades] = useState<TradeRecord[]>([])

  // Fetch trade journal
  useEffect(() => {
    const token = getAdminToken()
    if (!token) return
    const fetchTrades = async () => {
      try {
        const r = await fetch(`${BASE}/api/v1/trade-journal`, {
          headers: { Authorization: `Bearer ${token}` },
        })
        if (r.ok) {
          const data = await r.json()
          if (Array.isArray(data)) setTrades(data)
          else if (data?.trades) setTrades(data.trades)
        }
      } catch { /* ignore */ }
    }
    fetchTrades()
    const interval = setInterval(fetchTrades, 30_000) // refresh every 30s
    return () => clearInterval(interval)
  }, [])

  // Build equity curve from trades
  const { equityData, stats } = useMemo(() => {
    // Filter only closed trades (has exit event)
    const closed = trades
      .filter(t => t.close_reason !== 'OPEN' && t.close_reason !== 'OPENED_DEMO' && t.close_reason !== 'OPENED_LIVE')
      .sort((a, b) => new Date(a.closed_at).getTime() - new Date(b.closed_at).getTime())

    if (closed.length === 0) {
      return {
        equityData: [] as EquityPoint[],
        stats: { totalPnl: 0, maxDrawdown: 0, winRate: 0, profitFactor: 0, bestTrade: 0, worstTrade: 0, tradeCount: 0 },
      }
    }

    let cumPnl = 0
    let peak = 0
    let maxDrawdown = 0
    let wins = 0
    let losses = 0
    let totalWin = 0
    let totalLoss = 0
    let bestTrade = -Infinity
    let worstTrade = Infinity

    const data: EquityPoint[] = [{ time: '', pnl: 0, cumPnl: 0, peak: 0, drawdown: 0, trade: 'Start' }]

    for (const t of closed) {
      cumPnl += t.net_pnl
      if (cumPnl > peak) peak = cumPnl
      const dd = peak - cumPnl
      if (dd > maxDrawdown) maxDrawdown = dd

      if (t.net_pnl > 0) { wins++; totalWin += t.net_pnl }
      else { losses++; totalLoss += Math.abs(t.net_pnl) }
      if (t.net_pnl > bestTrade) bestTrade = t.net_pnl
      if (t.net_pnl < worstTrade) worstTrade = t.net_pnl

      const timeStr = t.closed_at ? new Date(t.closed_at).toLocaleTimeString('en', { hour: '2-digit', minute: '2-digit' }) : ''
      data.push({
        time: timeStr,
        pnl: t.net_pnl,
        cumPnl: parseFloat(cumPnl.toFixed(4)),
        peak: parseFloat(peak.toFixed(4)),
        drawdown: parseFloat(dd.toFixed(4)),
        trade: `${t.symbol} ${t.net_pnl >= 0 ? '+' : ''}${t.net_pnl.toFixed(4)}`,
      })
    }

    return {
      equityData: data,
      stats: {
        totalPnl: cumPnl,
        maxDrawdown,
        winRate: closed.length > 0 ? wins / closed.length : 0,
        profitFactor: totalLoss > 0 ? totalWin / totalLoss : totalWin > 0 ? Infinity : 0,
        bestTrade: bestTrade === -Infinity ? 0 : bestTrade,
        worstTrade: worstTrade === Infinity ? 0 : worstTrade,
        tradeCount: closed.length,
      },
    }
  }, [trades])

  // Also include daily PnL from state
  const dailyPnl = state?.risk?.daily_pnl ?? 0

  return (
    <div className="panel p-4 space-y-3">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <BarChart3 className="h-5 w-5 text-green" />
          <h3 className="text-sm font-display font-semibold text-text-primary">PnL Equity Curve</h3>
        </div>
        <div className="flex items-center gap-3 text-xs font-mono">
          <span className={dailyPnl >= 0 ? 'text-green' : 'text-red'}>
            Day: {dailyPnl >= 0 ? '+' : ''}{dailyPnl.toFixed(4)} USDT
          </span>
        </div>
      </div>

      {/* Stats Row */}
      <div className="grid grid-cols-5 gap-2">
        <StatBox
          icon={TrendingUp}
          label="Total PnL"
          value={`${stats.totalPnl >= 0 ? '+' : ''}${stats.totalPnl.toFixed(4)}`}
          color={stats.totalPnl >= 0 ? 'green' : 'red'}
        />
        <StatBox
          icon={TrendingDown}
          label="Max Drawdown"
          value={`-${stats.maxDrawdown.toFixed(4)}`}
          color="red"
        />
        <StatBox
          icon={Award}
          label="Win Rate"
          value={`${(stats.winRate * 100).toFixed(0)}%`}
          color={stats.winRate >= 0.5 ? 'green' : 'yellow'}
        />
        <StatBox
          icon={BarChart3}
          label="Profit Factor"
          value={stats.profitFactor === Infinity ? '∞' : stats.profitFactor.toFixed(2)}
          color={stats.profitFactor >= 1 ? 'green' : 'red'}
        />
        <StatBox
          icon={BarChart3}
          label="Trades"
          value={String(stats.tradeCount)}
          color="blue"
        />
      </div>

      {/* Chart */}
      {equityData.length > 1 ? (
        <div className="h-48">
          <ResponsiveContainer width="100%" height="100%">
            <AreaChart data={equityData} margin={{ top: 5, right: 10, left: 0, bottom: 0 }}>
              <defs>
                <linearGradient id="pnlGradient" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="5%" stopColor="#22c55e" stopOpacity={0.3} />
                  <stop offset="95%" stopColor="#22c55e" stopOpacity={0} />
                </linearGradient>
                <linearGradient id="pnlGradientNeg" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="5%" stopColor="#ef4444" stopOpacity={0} />
                  <stop offset="95%" stopColor="#ef4444" stopOpacity={0.3} />
                </linearGradient>
              </defs>
              <CartesianGrid strokeDasharray="3 3" stroke="#1e293b" />
              <XAxis
                dataKey="time"
                tick={{ fontSize: 10, fill: '#64748b' }}
                axisLine={{ stroke: '#1e293b' }}
              />
              <YAxis
                tick={{ fontSize: 10, fill: '#64748b' }}
                axisLine={{ stroke: '#1e293b' }}
                tickFormatter={(v: number) => v.toFixed(2)}
              />
              <Tooltip
                contentStyle={{
                  backgroundColor: '#0f172a',
                  border: '1px solid #1e293b',
                  borderRadius: '8px',
                  fontSize: '12px',
                }}
                formatter={(value: number, name: string) => [
                  `${value >= 0 ? '+' : ''}${value.toFixed(4)} USDT`,
                  name === 'cumPnl' ? 'Equity' : name,
                ]}
              />
              <ReferenceLine y={0} stroke="#475569" strokeDasharray="3 3" />
              <Area
                type="monotone"
                dataKey="cumPnl"
                stroke="#22c55e"
                strokeWidth={2}
                fill="url(#pnlGradient)"
                dot={false}
                activeDot={{ r: 4, fill: '#22c55e', stroke: '#0a0e17', strokeWidth: 2 }}
              />
              <Area
                type="monotone"
                dataKey="peak"
                stroke="#3b82f6"
                strokeWidth={1}
                strokeDasharray="4 4"
                fill="none"
                dot={false}
              />
            </AreaChart>
          </ResponsiveContainer>
        </div>
      ) : (
        <div className="h-48 flex items-center justify-center border border-dashed border-panel-border/50 rounded-lg">
          <div className="text-center text-text-muted text-sm">
            <BarChart3 className="h-8 w-8 mx-auto mb-2 opacity-30" />
            <p>No closed trades yet</p>
            <p className="text-xs mt-1">Equity curve will appear after the first closed trade</p>
          </div>
        </div>
      )}
    </div>
  )
}

function StatBox({ icon: Icon, label, value, color }: {
  icon: React.ElementType; label: string; value: string; color: string
}) {
  return (
    <div className="p-2 rounded-lg bg-panel-hover/50 text-center">
      <Icon className={`h-3.5 w-3.5 text-${color} mx-auto mb-0.5`} />
      <div className={`font-mono text-sm font-bold text-${color}`}>{value}</div>
      <div className="text-[9px] text-text-muted uppercase tracking-wider">{label}</div>
    </div>
  )
}
