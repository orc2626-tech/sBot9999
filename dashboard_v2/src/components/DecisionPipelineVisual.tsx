/**
 * Live Decision Pipeline — from last state.recent_decisions (real data).
 */
import { CheckCircle, XCircle, Minus } from 'lucide-react'
import { useTruthState } from '../hooks/useTruthState'
import type { DecisionEnvelope } from '../lib/api'

function statusFromVerdict(v?: string): 'pass' | 'allow' | 'block' | 'pending' {
  if (!v) return 'pending'
  const u = v.toUpperCase()
  if (u.includes('PASS') || u.includes('OK') || u.includes('ALLOW')) return 'pass'
  if (u.includes('BLOCK') || u.includes('FAIL')) return 'block'
  return 'pending'
}

export default function DecisionPipelineVisual() {
  const { state } = useTruthState()
  const decisions = state?.recent_decisions ?? []
  const last = decisions[0] as DecisionEnvelope | undefined

  if (!last) {
    return (
      <div className="panel p-4">
        <h3 className="text-sm font-semibold text-slate-400 uppercase tracking-wide mb-1">Live Decision Pipeline</h3>
        <p className="text-xs text-slate-500 mb-3">Current flow — last decision</p>
        <p className="text-sm text-slate-500">No recent decision — run strategy and connect Binance for live pipeline.</p>
      </div>
    )
  }

  const dq = statusFromVerdict(last.data_quality_verdict)
  const ins = statusFromVerdict(last.insurance_verdict)
  const risk = statusFromVerdict(last.risk_verdict)
  const exec = statusFromVerdict(last.execution_quality_verdict)
  const finalAllow = (last.final_decision ?? '').toUpperCase().includes('ALLOW')

  const stages = [
    { id: 'dq', name: 'Data Quality', status: dq, detail: last.data_quality_verdict ?? '—' },
    { id: 'regime', name: 'Regime & Session', status: 'pass' as const, detail: last.strategy_name ? `${last.strategy_name}` : '—' },
    { id: 'strategy', name: 'Strategy Proposals', status: 'pass' as const, detail: last.symbol ? `${last.symbol} ${last.side}` : '—' },
    { id: 'insurance', name: 'Trade Insurance', status: ins, detail: last.insurance_verdict ?? '—' },
    { id: 'risk', name: 'Risk Pre-Check', status: risk, detail: last.risk_verdict ?? '—' },
    { id: 'tca', name: 'Execution Quality (TCA)', status: exec, detail: last.execution_quality_verdict ?? '—' },
    { id: 'final', name: 'Final', status: finalAllow ? ('allow' as const) : ('block' as const), detail: last.final_decision ?? '—' },
  ]

  return (
    <div className="panel p-4">
      <h3 className="text-sm font-semibold text-slate-400 uppercase tracking-wide mb-1">Live Decision Pipeline</h3>
      <p className="text-xs text-slate-500 mb-3">Last decision — {last.symbol} · {last.created_at ? new Date(last.created_at).toLocaleTimeString() : '—'}</p>
      <div className="space-y-1">
        {stages.map((s, i) => (
          <div
            key={s.id}
            className="flex items-center gap-3 py-1.5 px-2 rounded border border-panelBorder/70 bg-black/20"
          >
            <div className="shrink-0 w-6 h-6 rounded-full flex items-center justify-center bg-slate-700/50 text-xs font-mono text-slate-400">
              {i + 1}
            </div>
            <div className="flex-1 min-w-0">
              <span className="text-slate-200 font-medium">{s.name}</span>
              <span className="text-slate-500 text-xs ml-2 font-mono truncate block">{s.detail}</span>
            </div>
            {(s.status === 'pass' || s.status === 'allow') && <CheckCircle className="h-4 w-4 text-green shrink-0" />}
            {s.status === 'block' && <XCircle className="h-4 w-4 text-red shrink-0" />}
            {s.status === 'pending' && <Minus className="h-4 w-4 text-slate-500 shrink-0" />}
          </div>
        ))}
      </div>
      <div className="mt-2 text-xs text-slate-500 font-mono">Pipeline from backend (last decision)</div>
    </div>
  )
}
