import { ListChecks } from 'lucide-react'
import { useTruthState } from '../hooks/useTruthState'

function formatTimeAgo(iso: string): string {
  try {
    const d = new Date(iso)
    const s = Math.round((Date.now() - d.getTime()) / 1000)
    if (s < 60) return `${s}s ago`
    if (s < 3600) return `${Math.floor(s / 60)}m ago`
    if (s < 86400) return `${Math.floor(s / 3600)}h ago`
    return `${Math.floor(s / 86400)}d ago`
  } catch {
    return '—'
  }
}

export default function ActionLogTable() {
  const { state } = useTruthState()
  const decisions = state?.recent_decisions ?? []

  return (
    <div className="panel p-5">
      <h3 className="section-title flex items-center gap-2 mb-3">
        <ListChecks className="h-4 w-4 text-blue" />Action Log
      </h3>
      <div className="overflow-x-auto">
        <table className="w-full text-xs">
          <thead>
            <tr className="text-text-muted border-b border-panel-border/50">
              <th className="text-left py-2 font-medium">Decision</th>
              <th className="text-left py-2 font-medium">Symbol</th>
              <th className="text-left py-2 font-medium">Side</th>
              <th className="text-left py-2 font-medium">Block</th>
              <th className="text-right py-2 font-medium">Time</th>
            </tr>
          </thead>
          <tbody>
            {decisions.length === 0 ? (
              <tr>
                <td colSpan={5} className="py-6 text-center text-text-muted font-mono text-xs">
                  No recent actions — waiting for strategy decisions...
                </td>
              </tr>
            ) : (
              decisions.slice(0, 15).map((a, i) => {
                const isExec = a.final_decision?.startsWith('EXECUTED')
                const decColor = isExec ? 'text-green' : 'text-red'
                return (
                  <tr key={a.id} className={`border-b border-panel-border/30 hover:bg-panel-hover/50 transition-colors ${i === 0 ? 'animate-slide-up' : ''}`}>
                    <td className={`py-2 font-mono font-medium ${decColor}`}>
                      {a.final_decision || '—'}
                    </td>
                    <td className="py-2 font-mono text-text-primary">{a.symbol ?? '—'}</td>
                    <td className={`py-2 font-mono ${a.side === 'BUY' ? 'text-green' : a.side === 'SELL' ? 'text-red' : 'text-text-secondary'}`}>
                      {a.side ?? '—'}
                    </td>
                    <td className="py-2 font-mono text-text-secondary">{a.blocking_layer ?? '—'}</td>
                    <td className="py-2 text-right text-text-muted">{formatTimeAgo(a.created_at)}</td>
                  </tr>
                )
              })
            )}
          </tbody>
        </table>
      </div>
    </div>
  )
}
