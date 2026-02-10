import { Brain, AlertTriangle } from 'lucide-react'
import { LineChart, Line, ResponsiveContainer } from 'recharts'
import { useTruthState } from '../hooks/useTruthState'

export default function AiObserverPanel() {
  const { state } = useTruthState()
  const recentErrors = state?.recent_errors ?? []
  const anomalyCount = recentErrors.length
  const hasData = anomalyCount > 0 || (state?.scoring != null)

  const auditTrend = hasData
    ? [100, 100 - Math.min(anomalyCount * 2, 10), 100 - Math.min(anomalyCount, 5), 100]
    : [100, 100, 100, 100]

  return (
    <div className="panel p-4">
      <h3 className="text-sm font-semibold text-slate-400 uppercase tracking-wide mb-3 flex items-center gap-2">
        <span className={`status-dot ${anomalyCount > 0 ? 'status-yellow' : 'status-green'}`} />
        <span className="status-dot status-green" />
        AI Observer
      </h3>
      <div className="space-y-2 text-sm">
        <div className="flex items-center gap-2">
          <AlertTriangle className="h-4 w-4 text-yellow" />
          <span className="text-slate-300">Anomalies:</span>
          <span className={`font-mono ${anomalyCount > 0 ? 'text-yellow' : 'text-green'}`}>{anomalyCount}</span>
        </div>
        <div className="flex items-center gap-2">
          <Brain className="h-4 w-4 text-slate-500" />
          <span className="text-slate-300">Last Audit:</span>
          <span className="font-mono text-green">
            {state?.scoring ? 'Data available' : 'No audit data yet — connect Binance and run strategy'}
          </span>
        </div>
      </div>
      <div className="h-12 mt-2">
        <ResponsiveContainer width="100%" height="100%">
          <LineChart data={auditTrend.map((v) => ({ v }))}>
            <Line type="monotone" dataKey="v" stroke="#22c55e" strokeWidth={1.5} dot={false} />
          </LineChart>
        </ResponsiveContainer>
      </div>
      {anomalyCount > 0 && (
        <div className="mt-2 flex items-center gap-2 text-xs text-yellow">
          <AlertTriangle className="h-3 w-3" />
          <span>AI Recommendation: Review recent errors in Diagnostics</span>
        </div>
      )}
    </div>
  )
}
