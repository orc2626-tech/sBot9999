import { BarChart, Bar, XAxis, YAxis, ResponsiveContainer, Tooltip } from 'recharts'
import { useTruthState } from '../hooks/useTruthState'

function formatUsd(n: number): string {
  if (n >= 1_000_000) return (n / 1_000_000).toFixed(2) + 'M'
  if (n >= 1_000) return (n / 1_000).toFixed(2) + 'K'
  return n.toFixed(2)
}

export default function AggregatePositionsPanel() {
  const { state } = useTruthState()
  const balances = state?.balances ?? []
  const md = state?.market_data?.symbols ?? {}
  const risk = state?.risk

  const holdings = balances
    .map((b) => {
      const free = parseFloat(b.free) || 0
      const locked = parseFloat(b.locked) || 0
      const total = free + locked
      const price = b.asset === 'USDT' ? 1 : md[`${b.asset}USDT`]?.last_price
      const value = price ? total * price : 0
      return { asset: b.asset, total, value }
    })
    .filter((h) => h.total > 0)
    .sort((a, b) => b.value - a.value)

  const totalEquity = holdings.reduce((s, h) => s + h.value, 0)
  const dailyPct = typeof risk?.daily_pnl_pct === 'number' ? risk.daily_pnl_pct : null
  const pctStr = dailyPct == null ? null : (dailyPct >= 0 ? `+${dailyPct.toFixed(2)}%` : `${dailyPct.toFixed(2)}%`)

  const chartData = holdings.slice(0, 8).map((h) => ({
    name: h.asset,
    value: h.value,
  }))

  return (
    <div className="panel p-4">
      <h3 className="text-sm font-semibold text-slate-400 uppercase tracking-wide mb-2">Aggregate Positions</h3>
      <div className="flex items-baseline gap-2 mb-3">
        <span className="font-mono text-2xl font-bold text-white">
          {totalEquity > 0 ? `$${formatUsd(totalEquity)}` : '—'}
        </span>
        {pctStr && (
          <span className={`font-mono text-lg ${dailyPct != null && dailyPct >= 0 ? 'text-green' : 'text-red'}`}>{pctStr}</span>
        )}
      </div>
      <div className="h-40">
        <ResponsiveContainer width="100%" height="100%">
          <BarChart data={chartData}>
            <XAxis dataKey="name" tick={{ fill: '#94a3b8', fontSize: 10 }} />
            <YAxis tick={{ fill: '#94a3b8', fontSize: 10 }} hide />
            <Tooltip contentStyle={{ background: '#1a1d24', border: '1px solid #2d323b' }} />
            <Bar dataKey="value" fill="#22c55e" radius={[4, 4, 0, 0]} />
          </BarChart>
        </ResponsiveContainer>
      </div>
      <div className="mt-2 flex justify-between text-xs text-slate-500 font-mono">
        <span>
          {holdings.length > 0 ? `Portfolio value (est.) · ${holdings.length} asset${holdings.length !== 1 ? 's' : ''}` : 'No balance data yet'}
        </span>
      </div>
    </div>
  )
}
