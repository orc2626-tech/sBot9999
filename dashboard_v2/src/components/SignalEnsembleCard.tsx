/**
 * SignalEnsembleCard — إشارات مرجحة حقيقية من API
 */
import { useTruthState } from '../hooks/useTruthState'

export default function SignalEnsembleCard() {
  const { state } = useTruthState()
  const signalState = state?.scoring

  if (!signalState) {
    return (
      <div className="panel p-4">
        <h3 className="text-sm font-semibold text-slate-400 uppercase tracking-wide mb-3">Signal Ensemble</h3>
        <p className="text-sm text-slate-500">No signal data yet.</p>
        <p className="text-xs text-slate-600 mt-1">Requires market data and strategy pipeline.</p>
      </div>
    )
  }

  const { total_score, decision, signal_contributions } = signalState
  const threshold = 0.35
  const signals = signal_contributions ?? []

  return (
    <div className="panel p-4">
      <h3 className="text-sm font-semibold text-slate-400 uppercase tracking-wide mb-3">Signal Ensemble (Weighted)</h3>
      <div className="space-y-2">
        {signals.map((s: { name: string; weight: number; confidence: number; direction: number; contribution: number }) => {
          const barWidth = Math.abs(s.contribution) * 100
          const isPositive = s.contribution > 0
          const opacity = Math.max(0.3, Math.abs(s.contribution) / (s.confidence || 1))

          return (
            <div key={s.name} className="space-y-0.5">
              <div className="flex justify-between text-xs">
                <span className="text-slate-300">{s.name}</span>
                <span className="font-mono text-slate-400">
                  w:{s.weight.toFixed(2)} × c:{s.confidence.toFixed(2)} = {s.contribution > 0 ? '+' : ''}
                  {s.contribution.toFixed(3)}
                </span>
              </div>
              <div className="h-2 bg-slate-700 rounded-full overflow-hidden relative">
                <div
                  className={`h-full rounded-full ${isPositive ? 'bg-green' : 'bg-red'}`}
                  style={{
                    width: `${Math.min(barWidth * 3, 100)}%`,
                    opacity,
                  }}
                />
              </div>
            </div>
          )
        })}
      </div>

      <div className="mt-4 pt-3 border-t border-panelBorder">
        <div className="flex items-center justify-between">
          <span className="text-slate-400 text-sm">Score:</span>
          <span
            className={`font-mono text-lg font-bold ${
              total_score > threshold ? 'text-green' : total_score < -threshold ? 'text-red' : 'text-slate-400'
            }`}
          >
            {total_score > 0 ? '+' : ''}
            {total_score?.toFixed(3) ?? '—'}
          </span>
        </div>
        <div className="flex items-center justify-between mt-1">
          <span className="text-slate-400 text-sm">Decision:</span>
          <span
            className={`font-mono font-semibold ${
              decision === 'BUY' ? 'text-green' : decision === 'SELL' ? 'text-red' : 'text-slate-500'
            }`}
          >
            {decision ?? 'NEUTRAL'}
          </span>
        </div>
      </div>
    </div>
  )
}
