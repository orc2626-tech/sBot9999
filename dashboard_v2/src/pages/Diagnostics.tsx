import SymbolsTickerRow from '../components/SymbolsTickerRow'
import SystemHealthCard from '../components/SystemHealthCard'
import RecentErrorsPanel from '../components/RecentErrorsPanel'
import ActionLogTable from '../components/ActionLogTable'
import { useTruthState } from '../hooks/useTruthState'

export default function Diagnostics() {
  const { state } = useTruthState()
  const truth = state?.truth
  const lastOkAgeS = truth?.reconcile_last_ok_age_s ?? null
  const lastError = truth?.reconcile_last_error ?? null

  return (
    <div className="space-y-4">
      <div className="panel p-3 border-blue/30 bg-blue/5">
        <h2 className="text-lg font-semibold text-slate-200">Diagnostics</h2>
        <p className="text-sm text-slate-400 mt-0.5">
          Errors timeline, WS ages, reconcile history, health degradation.
        </p>
      </div>
      <SymbolsTickerRow />
      <div className="grid grid-cols-12 gap-4">
        <div className="col-span-4 space-y-4">
          <SystemHealthCard />
          <RecentErrorsPanel />
        </div>
        <div className="col-span-4 space-y-4">
          <div className="panel p-4">
            <h3 className="text-sm font-semibold text-slate-400 uppercase tracking-wide mb-3">Reconcile History</h3>
            <div className="space-y-2 text-sm font-mono text-slate-400">
              <div>
                Last OK: {lastOkAgeS != null ? `${Math.round(lastOkAgeS)}s ago` : '—'}
              </div>
              <div className={lastError ? 'text-red' : 'text-slate-500'}>
                Last error: {lastError || 'No recent error'}
              </div>
            </div>
          </div>
        </div>
        <div className="col-span-4 space-y-4">
          <ActionLogTable />
        </div>
      </div>
    </div>
  )
}
