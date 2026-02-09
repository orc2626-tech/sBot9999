import SymbolsTickerRow from '../components/SymbolsTickerRow'
import AggregatePositionsPanel from '../components/AggregatePositionsPanel'
import PositionManagementGrid from '../components/PositionManagementGrid'
import PositionTimelinesList from '../components/PositionTimelinesList'
import TwinStatePanel from '../components/TwinStatePanel'
import SystemHealthCard from '../components/SystemHealthCard'

export default function Positions() {
  return (
    <div className="space-y-4">
      <div className="panel p-3 border-blue/30 bg-blue/5">
        <h2 className="text-lg font-semibold text-slate-200">Positions</h2>
        <p className="text-sm text-slate-400 mt-0.5">
          FSM state per position, timeline, linked orders. Twin status + drift details.
        </p>
      </div>
      <SymbolsTickerRow />
      <div className="grid grid-cols-12 gap-4">
        <div className="col-span-5 space-y-4">
          <AggregatePositionsPanel />
          <PositionManagementGrid />
        </div>
        <div className="col-span-4 space-y-4">
          <PositionTimelinesList />
          <TwinStatePanel />
        </div>
        <div className="col-span-3 space-y-4">
          <SystemHealthCard />
        </div>
      </div>
    </div>
  )
}
