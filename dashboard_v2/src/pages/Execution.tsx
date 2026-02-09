import SymbolsTickerRow from '../components/SymbolsTickerRow'
import SystemHealthCard from '../components/SystemHealthCard'
import ActionLogTable from '../components/ActionLogTable'

export default function Execution() {
  return (
    <div className="space-y-4">
      <div className="panel p-3 border-blue/30 bg-blue/5">
        <h2 className="text-lg font-semibold text-slate-200">Execution Quality</h2>
        <p className="text-sm text-slate-400 mt-0.5">
          Slippage, fill quality, retries, TCA metrics, abort reasons.
        </p>
      </div>
      <SymbolsTickerRow />
      <div className="grid grid-cols-12 gap-4">
        <div className="col-span-4 space-y-4">
          <div className="panel p-4">
            <h3 className="text-sm font-semibold text-slate-400 uppercase tracking-wide mb-3">TCA Summary</h3>
            <div className="space-y-2 text-sm">
              <div className="text-slate-500">No execution data yet</div>
            </div>
          </div>
        </div>
        <div className="col-span-4 space-y-4">
          <SystemHealthCard />
        </div>
        <div className="col-span-4 space-y-4">
          <ActionLogTable />
        </div>
      </div>
    </div>
  )
}
