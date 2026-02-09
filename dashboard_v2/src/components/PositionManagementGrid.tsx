import { LineChart, Line, ResponsiveContainer } from 'recharts'
import { useTruthState } from '../hooks/useTruthState'

const SYMBOLS = ['BTC', 'ETH', 'SOL', 'BNB', 'XRP']

export default function PositionManagementGrid() {
  const { state } = useTruthState()
  const balances = state?.balances ?? []
  const byAsset = new Map(balances.map((b) => [b.asset, b]))

  const rows = SYMBOLS.map((asset) => {
    const b = byAsset.get(asset)
    const free = b ? parseFloat(b.free) || 0 : 0
    const locked = b ? parseFloat(b.locked) || 0 : 0
    const total = free + locked
    return { pair: `${asset}/USDT`, total, free, locked, hasData: total > 0 }
  })

  return (
    <div className="panel p-4">
      <h3 className="text-sm font-semibold text-slate-400 uppercase tracking-wide mb-3">Position Management</h3>
      <div className="grid grid-cols-5 gap-2">
        {rows.map((p) => (
          <div key={p.pair} className="rounded border border-panelBorder p-2 bg-black/20">
            <div className="font-mono text-xs text-slate-400">{p.pair}</div>
            <div className="font-mono text-slate-200 text-sm mt-0.5">
              {p.hasData ? `${p.total.toFixed(4)}` : '—'}
            </div>
            <div className="h-10 mt-1">
              <ResponsiveContainer width="100%" height="100%">
                <LineChart data={p.hasData ? [{ v: 0 }, { v: p.total }] : [{ v: 0 }]}>
                  <Line type="monotone" dataKey="v" stroke="#22c55e" strokeWidth={1} dot={false} />
                </LineChart>
              </ResponsiveContainer>
            </div>
          </div>
        ))}
      </div>
    </div>
  )
}
