import { useTruthState } from '../hooks/useTruthState'

export function PanelRejectionReasons() {
  const { state } = useTruthState()
  const decisions = state?.recent_decisions ?? []
  const blocked = decisions.find((d) => d.blocking_layer != null && d.blocking_layer !== '')
  const last = decisions[0]

  return (
    <div className="panel p-4">
      <h3 className="text-sm font-semibold text-slate-400 uppercase tracking-wide mb-3">Rejection Reasons (Strategy)</h3>
      <div className="space-y-2 text-sm text-slate-300">
        <p>When a proposal is blocked, the blocking layer and reason appear here.</p>
        <div className="rounded border border-panelBorder p-2 font-mono text-xs text-slate-500 bg-black/20">
          {blocked ? (
            <>
              <span className="text-yellow">Blocked: {blocked.blocking_layer}</span>
              {blocked.reason && <span className="block mt-1 text-slate-400">{blocked.reason}</span>}
            </>
          ) : last ? (
            <>No active rejections — last: {last.final_decision} · {last.symbol}</>
          ) : (
            'No recent decisions — run strategy for live rejections.'
          )}
        </div>
      </div>
    </div>
  )
}

export function PanelTwoStageEntry() {
  const { state } = useTruthState()
  const last = state?.recent_decisions?.[0]
  const allowed = last ? (last.final_decision ?? '').toUpperCase().includes('ALLOW') : null
  const insurance = last?.insurance_verdict ?? null
  const risk = last?.risk_verdict ?? null

  return (
    <div className="panel p-4">
      <h3 className="text-sm font-semibold text-slate-400 uppercase tracking-wide mb-3">Two-Stage Entry (Insurance)</h3>
      <div className="space-y-2">
        <div className="flex justify-between text-sm">
          <span className="text-slate-400">Setup</span>
          <span className={`font-mono ${insurance && (insurance.toUpperCase().includes('PASS') || insurance.toUpperCase().includes('OK')) ? 'text-green' : 'text-slate-500'}`}>
            {insurance ?? '—'}
          </span>
        </div>
        <div className="flex justify-between text-sm">
          <span className="text-slate-400">Confirm</span>
          <span className={`font-mono ${risk && (risk.toUpperCase().includes('PASS') || risk.toUpperCase().includes('OK')) ? 'text-green' : 'text-slate-500'}`}>
            {risk ?? '—'}
          </span>
        </div>
        <div className="flex justify-between text-sm">
          <span className="text-slate-400">Enter</span>
          <span className={`font-mono ${allowed ? 'text-green' : 'text-slate-500'}`}>
            {allowed != null ? (allowed ? 'Allowed' : 'Blocked') : '—'}
          </span>
        </div>
      </div>
    </div>
  )
}

export function PanelRiskLimits() {
  const { state } = useTruthState()
  const rc = state?.runtime_config
  const risk = state?.risk
  const maxDaily = rc?.max_daily_loss_pct ?? 1.5
  const maxConsec = rc?.max_consecutive_losses ?? 3
  const remaining = risk?.remaining_daily_loss_pct ?? maxDaily

  return (
    <div className="panel p-4">
      <h3 className="text-sm font-semibold text-slate-400 uppercase tracking-wide mb-3">Limits &amp; Breakers</h3>
      <div className="space-y-2 text-sm">
        <div className="flex justify-between"><span className="text-slate-400">Max Daily Loss</span><span className="font-mono text-slate-200">{maxDaily}%</span></div>
        <div className="flex justify-between"><span className="text-slate-400">Max Consecutive Losses</span><span className="font-mono text-slate-200">{maxConsec}</span></div>
        <div className="flex justify-between"><span className="text-slate-400">Remaining Daily</span><span className="font-mono text-green">{remaining.toFixed(2)}%</span></div>
      </div>
    </div>
  )
}
