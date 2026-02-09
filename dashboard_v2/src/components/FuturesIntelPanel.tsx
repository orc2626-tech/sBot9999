/**
 * Futures Intelligence — Funding + OI + Long/Short
 */
import { TrendingUp, TrendingDown, Minus, Activity } from 'lucide-react'
import { useTruthState } from '../hooks/useTruthState'

function SignalIcon({ signal }: { signal: string }) {
  if (signal?.includes('BULLISH')) return <TrendingUp className="h-4 w-4 text-green" />
  if (signal?.includes('BEARISH')) return <TrendingDown className="h-4 w-4 text-red" />
  return <Minus className="h-4 w-4 text-slate-500" />
}

export default function FuturesIntelPanel() {
  const { state } = useTruthState()
  const fiMap = state?.futures_intel ?? {}
  const firstSymbol = state?.runtime_config?.symbols?.[0]
  const fi = firstSymbol ? fiMap[firstSymbol] : null

  if (!fi) {
    return (
      <div className="panel p-4">
        <h3 className="text-sm font-semibold text-slate-400 uppercase tracking-wide">Futures Intelligence</h3>
        <p className="text-sm text-slate-500 mt-2">Waiting for futures data...</p>
      </div>
    )
  }

  const fundingRate = (fi as { funding_rate?: number }).funding_rate ?? 0
  const fundingSignal = (fi as { funding_signal?: string }).funding_signal ?? 'NEUTRAL'
  const oi = (fi as { open_interest?: number }).open_interest ?? 0
  const oiChange = (fi as { oi_change_1h_pct?: number }).oi_change_1h_pct ?? 0
  const lsRatio = (fi as { long_short_ratio?: number }).long_short_ratio ?? 1
  const lsSignal = (fi as { ls_signal?: string }).ls_signal ?? 'NEUTRAL'

  return (
    <div className="panel p-4">
      <h3 className="text-sm font-semibold text-slate-400 uppercase tracking-wide mb-3 flex items-center gap-2">
        <Activity className="h-4 w-4" />
        Futures Intelligence
      </h3>

      <div className="space-y-4">
        <div>
          <div className="flex items-center justify-between text-sm mb-1">
            <span className="text-slate-400">Funding Rate</span>
            <span
              className={`font-mono font-medium ${
                fundingRate > 0.0003 ? 'text-red' : fundingRate < -0.0003 ? 'text-green' : 'text-slate-300'
              }`}
            >
              {(fundingRate * 100).toFixed(4)}%
            </span>
          </div>
          <div className="flex items-center gap-2">
            <SignalIcon signal={fundingSignal} />
            <span className="text-xs text-slate-400">{fundingSignal}</span>
          </div>
        </div>

        <div>
          <div className="flex items-center justify-between text-sm mb-1">
            <span className="text-slate-400">Open Interest</span>
            <span className="font-mono text-slate-300">${(oi / 1e6).toFixed(1)}M</span>
          </div>
          <div className="flex items-center gap-2">
            <span className={`text-xs font-mono ${oiChange > 0 ? 'text-green' : 'text-red'}`}>
              {oiChange > 0 ? '+' : ''}
              {oiChange.toFixed(1)}% (1h)
            </span>
          </div>
        </div>

        <div>
          <div className="flex items-center justify-between text-sm mb-1">
            <span className="text-slate-400">Long/Short Ratio</span>
            <span className="font-mono text-slate-300">{lsRatio.toFixed(2)}</span>
          </div>
          <div className="h-3 bg-slate-700 rounded-full overflow-hidden flex">
            <div
              className="h-full bg-green"
              style={{ width: `${(lsRatio / (lsRatio + 1)) * 100}%` }}
            />
            <div className="h-full bg-red flex-1" />
          </div>
          <div className="flex items-center gap-2 mt-1">
            <SignalIcon signal={lsSignal} />
            <span className="text-xs text-slate-400">{lsSignal}</span>
          </div>
        </div>
      </div>
    </div>
  )
}
