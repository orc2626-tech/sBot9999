import { Play } from 'lucide-react'
import { useTruthState } from '../hooks/useTruthState'

export default function PositionTimelinesList() {
  const { state } = useTruthState()
  const positions = state?.positions ?? []

  return (
    <div className="panel p-4">
      <h3 className="text-sm font-semibold text-slate-400 uppercase tracking-wide mb-3">Position Timelines</h3>
      <div className="space-y-2">
        {positions.length === 0 ? (
          <div className="py-4 text-center text-slate-500 text-sm font-mono">
            No open positions — connect Binance and run strategy for live positions.
          </div>
        ) : (
          positions.map((p, i) => (
            <div key={p.symbol + i} className="flex items-center gap-2 py-1.5 border-b border-panelBorder/50 last:border-0">
              <Play className="h-3.5 w-3.5 text-green shrink-0 cursor-pointer hover:text-greenDim" />
              <span className="font-mono text-slate-200">{p.symbol?.replace('USDT', '/USD') ?? p.symbol}</span>
              <span className="font-mono text-slate-400">{p.total ?? p.free ?? '—'}</span>
              <span className="font-mono text-green ml-auto">—</span>
            </div>
          ))
        )}
      </div>
    </div>
  )
}
