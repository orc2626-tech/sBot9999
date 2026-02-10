/**
 * Trade Eligibility — from state.scoring (real data).
 */
import { CheckCircle } from 'lucide-react'
import { useTruthState } from '../hooks/useTruthState'

export default function TradeEligibilityCard() {
  const { state } = useTruthState()
  const scoring = state?.scoring
  const risk = state?.risk

  if (!scoring) {
    return (
      <div className="panel p-4">
        <h3 className="text-sm font-semibold text-slate-400 uppercase tracking-wide mb-3">Trade Eligibility</h3>
        <p className="text-sm text-slate-500">No signal data yet.</p>
        <p className="text-xs text-slate-600 mt-1">Requires market data and strategy pipeline (connect Binance for live).</p>
      </div>
    )
  }

  const totalScore = scoring.total_score ?? 0
  const decision = scoring.decision ?? 'NEUTRAL'
  const probability = Math.min(100, Math.max(0, (totalScore + 1) * 50))
  const riskMode = risk?.risk_mode ?? 'NORMAL'
  const allowed = decision !== 'NEUTRAL' && riskMode === 'NORMAL'

  return (
    <div className="panel p-4">
      <h3 className="text-sm font-semibold text-slate-400 uppercase tracking-wide mb-3">Trade Eligibility</h3>
      <div className="space-y-3">
        <div>
          <div className="flex justify-between text-sm mb-0.5">
            <span className="text-slate-300">Probability</span>
            <span className="font-mono text-green">{probability.toFixed(0)}%</span>
          </div>
          <div className="h-1.5 bg-slate-700 rounded-full overflow-hidden">
            <div className="h-full bg-green rounded-full" style={{ width: `${probability}%` }} />
          </div>
        </div>
        <div className="flex justify-between text-sm">
          <span className="text-slate-300">Decision</span>
          <span
            className={`font-mono ${
              decision === 'BUY' ? 'text-green' : decision === 'SELL' ? 'text-red' : 'text-slate-400'
            }`}
          >
            {decision}
          </span>
        </div>
        <div className="flex justify-between text-sm">
          <span className="text-slate-300">Risk Mode</span>
          <span
            className={`font-mono ${
              riskMode === 'NORMAL' ? 'text-green' : riskMode === 'CAUTION' ? 'text-yellow' : 'text-red'
            }`}
          >
            {riskMode}
          </span>
        </div>
      </div>
      <div className="mt-4 pt-3 border-t border-panelBorder flex items-center gap-2">
        <CheckCircle className={`h-5 w-5 shrink-0 ${allowed ? 'text-green' : 'text-slate-500'}`} />
        <span className="text-slate-400 text-sm">Final Decision:</span>
        <span className={`font-mono font-semibold ${allowed ? 'text-green' : 'text-slate-500'}`}>
          {allowed ? 'ALLOWED' : '—'}
        </span>
      </div>
    </div>
  )
}
