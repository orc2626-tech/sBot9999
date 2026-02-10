/**
 * Triple Barrier Visualizer — مراقبة الصفقة النشطة
 */
import { useTruthState } from '../hooks/useTruthState'

export default function TripleBarrierViz() {
  const { state } = useTruthState()
  const positions = (state?.positions ?? []) as Array<{
    symbol?: string
    entry_price?: number
    current_price?: number
    stop_loss?: number
    take_profit_1?: number
    take_profit_2?: number
    mfe?: number
    mae?: number
  }>

  if (positions.length === 0) {
    return (
      <div className="panel p-4">
        <h3 className="text-sm font-semibold text-slate-400 uppercase tracking-wide">Triple Barrier</h3>
        <p className="text-sm text-slate-500 mt-2">No active positions</p>
      </div>
    )
  }

  return (
    <div className="panel p-4">
      <h3 className="text-sm font-semibold text-slate-400 uppercase tracking-wide mb-3">Triple Barrier — Active Trades</h3>
      <div className="space-y-4">
        {positions.map((pos, i) => {
          const currentPrice = pos.current_price ?? pos.entry_price ?? 0
          const entry = pos.entry_price ?? 0
          const tp2 = pos.take_profit_2 ?? entry * 1.02
          const sl = pos.stop_loss ?? entry * 0.99
          const range = tp2 - sl
          const pricePct = range > 0 ? ((currentPrice - sl) / range) * 100 : 50
          const isProfitable = currentPrice > entry
          const sym = pos.symbol ?? `Position ${i + 1}`

          return (
            <div key={sym} className="border border-panelBorder rounded p-3">
              <div className="flex items-center justify-between mb-2">
                <span className="font-mono font-semibold text-slate-200">{sym}</span>
                <span className={`font-mono text-sm ${isProfitable ? 'text-green' : 'text-red'}`}>
                  {currentPrice.toFixed(2)} (
                  {entry > 0 ? ((currentPrice - entry) / entry * 100).toFixed(2) : '0'}%)
                </span>
              </div>
              <div className="relative h-8 bg-slate-800 rounded mb-2">
                <div className="absolute left-0 top-0 bottom-0 w-px bg-red" />
                <div className="absolute right-0 top-0 bottom-0 w-px bg-green" />
                <div
                  className={`absolute top-1 bottom-1 w-2 rounded ${isProfitable ? 'bg-green' : 'bg-red'} transition-all duration-300`}
                  style={{ left: `${Math.max(0, Math.min(pricePct, 100))}%` }}
                />
              </div>
              <div className="flex gap-4 text-[10px] font-mono">
                <span className="text-green">MFE: +{(pos.mfe ?? 0).toFixed(2)}%</span>
                <span className="text-red">MAE: -{(pos.mae ?? 0).toFixed(2)}%</span>
              </div>
            </div>
          )
        })}
      </div>
    </div>
  )
}
