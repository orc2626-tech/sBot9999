import SymbolsTickerRow from '../components/SymbolsTickerRow'
import SignalEnsembleCard from '../components/SignalEnsembleCard'
import RegimePanel from '../components/RegimePanel'
import FuturesIntelPanel from '../components/FuturesIntelPanel'
import TradeEligibilityCard from '../components/TradeEligibilityCard'
import SystemHealthCard from '../components/SystemHealthCard'
import DecisionPipelineVisual from '../components/DecisionPipelineVisual'
import SmartFiltersPanel from '../components/SmartFiltersPanel'
import { PanelRejectionReasons } from '../components/PagePanels'
import StrategyArena from '../components/arena/StrategyArena'
import { useTruthState } from '../hooks/useTruthState'

export default function Strategy() {
  const { state } = useTruthState()

  return (
    <div className="space-y-4">
      <div className="panel p-3 border-blue/30 bg-blue/5">
        <h2 className="text-lg font-semibold text-slate-200">Strategy</h2>
        <p className="text-sm text-slate-400 mt-0.5">
          Signals, ensemble, confidence, smart filters, context, rejection reasons.
        </p>
      </div>
      <SymbolsTickerRow />

      {/* Phase 1: Smart Filters — HTF + Momentum + OFIP + Adaptive + Entropy */}
      <SmartFiltersPanel />

      {/* Strategy Arena — حلبة الاستراتيجيات (تشمل Regime Matrix داخلياً) */}
      <StrategyArena wsArenaState={state?.arena ?? undefined} />

      <div className="grid grid-cols-12 gap-4">
        <div className="col-span-4 space-y-4">
          <RegimePanel />
          <FuturesIntelPanel />
          <SignalEnsembleCard />
          <TradeEligibilityCard />
          <DecisionPipelineVisual />
        </div>
        <div className="col-span-4 space-y-4">
          <PanelRejectionReasons />
        </div>
        <div className="col-span-4 space-y-4">
          <SystemHealthCard />
        </div>
      </div>
    </div>
  )
}
