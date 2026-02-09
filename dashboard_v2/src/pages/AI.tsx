import SymbolsTickerRow from '../components/SymbolsTickerRow'
import AiObserverPanel from '../components/AiObserverPanel'
import SystemHealthCard from '../components/SystemHealthCard'

export default function AI() {
  return (
    <div className="space-y-4">
      <div className="panel p-3 border-blue/30 bg-blue/5">
        <h2 className="text-lg font-semibold text-slate-200">AI Observer</h2>
        <p className="text-sm text-slate-400 mt-0.5">
          Last audits, anomalies, recommendations. Deep-audit button + audit history view.
        </p>
      </div>
      <SymbolsTickerRow />
      <div className="grid grid-cols-12 gap-4">
        <div className="col-span-4 space-y-4">
          <AiObserverPanel />
          <div className="panel p-4">
            <h3 className="text-sm font-semibold text-slate-400 uppercase tracking-wide mb-3">Audit History</h3>
            <div className="space-y-2 text-sm text-slate-400">
              <div>No audit data yet — Learning Engine analyzing...</div>
            </div>
          </div>
        </div>
        <div className="col-span-4 space-y-4">
          <div className="panel p-4">
            <h3 className="text-sm font-semibold text-slate-400 uppercase tracking-wide mb-3">Run Deep Audit</h3>
            <p className="text-sm text-slate-500 mb-3">Trigger AI observer audit job (audit-only, no trading). Allowed when PAUSED.</p>
            <button disabled className="px-3 py-2 rounded bg-slate-700/50 text-slate-500 border border-slate-600/50 font-medium text-sm cursor-not-allowed opacity-50">
              Run Deep Audit Now
            </button>
          </div>
        </div>
        <div className="col-span-4 space-y-4">
          <SystemHealthCard />
        </div>
      </div>
    </div>
  )
}
