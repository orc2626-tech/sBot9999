import SymbolsTickerRow from '../components/SymbolsTickerRow'
import SystemHealthCard from '../components/SystemHealthCard'
import { PanelRiskLimits } from '../components/PagePanels'
import { useTruthState } from '../hooks/useTruthState'

export default function Risk() {
  const { state } = useTruthState()
  const positions = state?.positions ?? []
  const balances = state?.balances ?? []
  const usdt = balances.find((b) => b.asset === 'USDT')
  const cashUsd = usdt ? parseFloat(usdt.free) + parseFloat(usdt.locked) : 0
  const exposureFromPositions = positions.reduce((sum, p) => sum + parseFloat(p.total || '0'), 0)
  const totalExposure = cashUsd + exposureFromPositions
  const perSymbolCap = state?.runtime_config?.max_concurrent_positions ?? 5

  return (
    <div className="space-y-4">
      <div className="panel p-3 border-blue/30 bg-blue/5">
        <h2 className="text-lg font-semibold text-slate-200">Risk Monitor</h2>
        <p className="text-sm text-slate-400 mt-0.5">
          Limits, drawdown, circuit breakers, exposure, trade budget. Live warnings + confirmation policy.
        </p>
      </div>
      <SymbolsTickerRow />
      <div className="grid grid-cols-12 gap-4">
        <div className="col-span-4 space-y-4">
          <PanelRiskLimits />
          <div className="panel p-4">
            <h3 className="text-sm font-semibold text-slate-400 uppercase tracking-wide mb-3">Exposure</h3>
            <div className="space-y-2 text-sm font-mono text-slate-300">
              <div className="flex justify-between"><span>Total</span><span className="text-green">{totalExposure > 0 ? `$${totalExposure.toLocaleString(undefined, { maximumFractionDigits: 0 })}` : '—'}</span></div>
              <div className="flex justify-between"><span>Per-symbol cap</span><span>{perSymbolCap} positions</span></div>
            </div>
          </div>
        </div>
        <div className="col-span-4 space-y-4">
          <div className="panel p-4">
            <h3 className="text-sm font-semibold text-slate-400 uppercase tracking-wide mb-3">Circuit Breakers</h3>
            <div className="space-y-2 text-sm text-slate-300">
              {state?.risk?.circuit_breakers && Object.keys(state.risk.circuit_breakers).length > 0 ? (
                Object.values(state.risk.circuit_breakers).map((b) => {
                  const tripped = b.tripped ?? false
                  const current = typeof b.current === 'number' ? b.current : parseFloat(String(b.current)) || 0
                  const limit = typeof b.limit === 'number' ? b.limit : parseFloat(String(b.limit).replace('%', '')) || 1
                  const pct = limit > 0 ? (current / limit) * 100 : 0
                  const statusClass = tripped ? 'status-red' : pct >= 80 ? 'status-yellow' : 'status-green'
                  const statusText = tripped ? 'TRIPPED' : pct >= 80 ? 'WARNING' : 'OK'
                  return (
                    <div key={b.name} className="flex items-center gap-2">
                      <span className={`status-dot ${statusClass}`} />
                      {b.name} — {statusText}
                    </div>
                  )
                })
              ) : (
                <div className="text-slate-500">No circuit breaker data available</div>
              )}
            </div>
          </div>
        </div>
        <div className="col-span-4 space-y-4">
          <SystemHealthCard />
        </div>
      </div>
    </div>
  )
}
