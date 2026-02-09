import SymbolsTickerRow from '../components/SymbolsTickerRow'
import SignalEnsembleCard from '../components/SignalEnsembleCard'
import SignalWaterfall from '../components/SignalWaterfall'
import MarketPulse from '../components/MarketPulse'
import RegimePanel from '../components/RegimePanel'
import VPINMonitor from '../components/VPINMonitor'
import RiskThermometer from '../components/RiskThermometer'
import TradeEligibilityCard from '../components/TradeEligibilityCard'
import AggregatePositionsPanel from '../components/AggregatePositionsPanel'
import TripleBarrierViz from '../components/TripleBarrierViz'
import PositionManagementGrid from '../components/PositionManagementGrid'
import SystemHealthCard from '../components/SystemHealthCard'
import TwinStatePanel from '../components/TwinStatePanel'
import ActionLogTable from '../components/ActionLogTable'
import DecisionPipelineVisual from '../components/DecisionPipelineVisual'
import SmartFiltersPanel from '../components/SmartFiltersPanel'

export default function Live() {
  return (
    <div className="space-y-4 animate-fade-in">
      <div className="panel p-4 border-blue/30 bg-blue/5">
        <div className="flex items-center gap-3">
          <div className="h-3 w-3 rounded-full bg-blue animate-pulse" />
          <div>
            <h2 className="text-lg font-display font-bold text-text-primary">Live Trading</h2>
            <p className="text-xs text-text-muted mt-0.5">
              Real-time positions, signals, risk, and execution. Press <kbd className="px-1 py-0.5 rounded bg-bg-primary border border-panel-border text-[10px]">?</kbd> for keyboard shortcuts.
            </p>
          </div>
        </div>
      </div>

      <SymbolsTickerRow />

      {/* Phase 1: Smart Filters — structural direction + advanced gates */}
      <SmartFiltersPanel />

      {/* Market Pulse — full width */}
      <MarketPulse />

      <div className="grid grid-cols-12 gap-4">
        <div className="col-span-3 space-y-4">
          <RegimePanel />
          <VPINMonitor />
          <RiskThermometer />
          <TradeEligibilityCard />
          <DecisionPipelineVisual />
        </div>
        <div className="col-span-5 space-y-4">
          <SignalWaterfall />
          <AggregatePositionsPanel />
          <TripleBarrierViz />
          <PositionManagementGrid />
        </div>
        <div className="col-span-4 space-y-4">
          <SignalEnsembleCard />
          <SystemHealthCard />
          <TwinStatePanel />
          <ActionLogTable />
        </div>
      </div>
    </div>
  )
}
