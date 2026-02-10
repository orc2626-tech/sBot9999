/**
 * System Health & Circuit Breakers
 */
import { Activity, Wifi, AlertTriangle, Shield } from 'lucide-react'
import { useTruthState } from '../hooks/useTruthState'

function formatAgeMs(ms: number): string {
  if (ms >= 86400000) return '—'
  if (ms >= 60000) return `${Math.floor(ms / 60000)}m ago`
  if (ms >= 1000) return `${Math.floor(ms / 1000)}s ago`
  return `${ms}ms ago`
}

function formatAgeS(s: number): string {
  if (s >= 3600) return `${Math.floor(s / 3600)}h ago`
  if (s >= 60) return `${Math.floor(s / 60)}m ago`
  return `${s}s ago`
}

export default function SystemHealthCard() {
  const { state, connected } = useTruthState()
  const risk = state?.risk as { risk_mode?: string; circuit_breakers?: Record<string, { name: string; current: number; limit: number; tripped: boolean }> } | undefined
  const truth = state?.truth
  const breakers = risk?.circuit_breakers
  const wsAgeMs = truth?.last_ws_user_event_age_ms ?? 0
  const reconAgeS = truth?.reconcile_last_ok_age_s ?? null
  const rateLimits = state?.rate_limits

  return (
    <div className="panel p-4">
      <h3 className="text-sm font-semibold text-slate-400 uppercase tracking-wide mb-3 flex items-center gap-2">
        <Shield className="h-4 w-4" />
        System Health & Circuit Breakers
      </h3>

      <div className="space-y-2 text-sm mb-3">
        <div className="flex items-center gap-2">
          <Wifi className={`h-4 w-4 ${connected ? 'text-green' : 'text-red'}`} />
          <span className="text-slate-300">WebSocket:</span>
          <span className={`font-mono ${connected ? 'text-green' : 'text-red'}`}>
            {connected ? 'SYNCED' : 'OFF'}
          </span>
        </div>
        <div className="flex items-center gap-2">
          <Activity className="h-4 w-4 text-slate-500" />
          <span className="text-slate-300">WS Age:</span>
          <span className="font-mono text-slate-200">{formatAgeMs(wsAgeMs)}</span>
        </div>
        <div className="flex items-center gap-2">
          <Wifi className="h-4 w-4 text-slate-500" />
          <span className="text-slate-300">Last Reconcile:</span>
          <span className={`font-mono ${reconAgeS != null && reconAgeS > 120 ? 'text-yellow' : 'text-green'}`}>
            {reconAgeS != null ? formatAgeS(reconAgeS) : '—'}
          </span>
        </div>
        {rateLimits != null && (
          <div className="flex items-center gap-2">
            <Activity className="h-4 w-4 text-slate-500" />
            <span className="text-slate-300">Rate (1m):</span>
            <span className={`font-mono ${rateLimits.used_weight_1m >= 800 ? 'text-yellow' : 'text-green'}`}>
              {rateLimits.used_weight_1m} / 1200
            </span>
          </div>
        )}
      </div>

      {breakers && Object.keys(breakers).length > 0 && (
        <div className="space-y-2 pt-3 border-t border-panelBorder">
          <div className="text-xs text-slate-500 mb-2">Circuit Breakers</div>
          {Object.values(breakers).map((b) => {
            const current = typeof b.current === 'number' ? b.current : parseFloat(String(b.current)) || 0
            const limit = typeof b.limit === 'number' ? b.limit : parseFloat(String(b.limit).replace('%', '')) || 1
            const pct = limit > 0 ? (current / limit) * 100 : 0
            const color = b.tripped ? 'bg-red' : pct >= 80 ? 'bg-yellow' : 'bg-green'
            return (
              <div key={b.name}>
                <div className="flex justify-between text-xs mb-0.5">
                  <span className={b.tripped ? 'text-red font-medium' : 'text-slate-400'}>
                    {b.tripped ? '⛔ ' : ''}
                    {b.name}
                  </span>
                  <span className="font-mono text-slate-300">
                    {typeof b.current === 'number' ? b.current.toFixed(1) : b.current} / {typeof b.limit === 'number' ? b.limit : b.limit}
                  </span>
                </div>
                <div className="h-1.5 bg-slate-700 rounded-full overflow-hidden">
                  <div className={`h-full ${color} rounded-full transition-all`} style={{ width: `${Math.min(pct, 100)}%` }} />
                </div>
              </div>
            )
          })}
        </div>
      )}

      {risk && (
        <div className="mt-3 pt-3 border-t border-panelBorder flex items-center justify-between">
          <span className="text-sm text-slate-400">Risk Mode:</span>
          <span
            className={`font-mono font-bold ${
              risk.risk_mode === 'NORMAL' ? 'text-green' : risk.risk_mode === 'CAUTION' ? 'text-yellow' : 'text-red'
            }`}
          >
            {risk.risk_mode ?? '—'}
          </span>
        </div>
      )}

      {truth?.no_go_reason && (
        <div className="mt-2 p-2 rounded bg-red/10 border border-red/30 flex items-center gap-2">
          <AlertTriangle className="h-4 w-4 text-red shrink-0" />
          <span className="text-xs text-red font-mono">{truth.no_go_reason}</span>
        </div>
      )}

      {state?.recent_errors != null && state.recent_errors.length > 0 && (
        <div className="mt-2 flex items-center gap-2">
          <AlertTriangle className="h-4 w-4 text-yellow" />
          <span className="text-slate-300">Recent errors:</span>
          <span className="font-mono text-yellow">{state.recent_errors.length}</span>
        </div>
      )}
    </div>
  )
}
