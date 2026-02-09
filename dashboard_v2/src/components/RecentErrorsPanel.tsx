import { AlertTriangle } from 'lucide-react'
import { useTruthState } from '../hooks/useTruthState'

export default function RecentErrorsPanel() {
  const { state } = useTruthState()
  const errors = state?.recent_errors ?? []

  if (errors.length === 0) {
    return (
      <div className="panel p-4">
        <h3 className="text-sm font-semibold text-slate-400 uppercase tracking-wide mb-3">Recent Errors</h3>
        <div className="text-sm text-slate-500">No errors recorded.</div>
      </div>
    )
  }

  return (
    <div className="panel p-4">
      <h3 className="text-sm font-semibold text-slate-400 uppercase tracking-wide mb-3">Recent Errors</h3>
      <div className="space-y-2">
        {errors.slice(0, 10).map((e, i) => (
          <div key={i} className="flex items-start gap-2 text-sm py-1.5 border-b border-panelBorder/50 last:border-0">
            <AlertTriangle className="h-4 w-4 text-yellow shrink-0 mt-0.5" />
            <div>
              <div className="text-slate-200">{e.message}</div>
              <div className="text-slate-500 font-mono text-xs">{e.at}{e.code ? ` · ${e.code}` : ''}</div>
            </div>
          </div>
        ))}
      </div>
    </div>
  )
}
