import { useTruthState } from '../hooks/useTruthState'
import { TrendingUp, BarChart3, Target, Clock } from 'lucide-react'
import SymbolsTickerRow from '../components/SymbolsTickerRow'
import SignalEnsembleCard from '../components/SignalEnsembleCard'
import RegimePanel from '../components/RegimePanel'
import TradeEligibilityCard from '../components/TradeEligibilityCard'
import PositionManagementGrid from '../components/PositionManagementGrid'
import AggregatePositionsPanel from '../components/AggregatePositionsPanel'
import TripleBarrierViz from '../components/TripleBarrierViz'
import SystemHealthCard from '../components/SystemHealthCard'
import VPINMonitor from '../components/VPINMonitor'
import FuturesIntelPanel from '../components/FuturesIntelPanel'
import TradeJournalDash from '../components/TradeJournalDash'
import AiObserverPanel from '../components/AiObserverPanel'
import TwinStatePanel from '../components/TwinStatePanel'
import PositionTimelinesList from '../components/PositionTimelinesList'
import RecentErrorsPanel from '../components/RecentErrorsPanel'
import ActionLogTable from '../components/ActionLogTable'
import DecisionPipelineVisual from '../components/DecisionPipelineVisual'
import PnlEquityCurve from '../components/PnlEquityCurve'

function QuickStat({ icon: Icon, label, value, color }: {
  icon: React.ElementType; label: string; value: string; color: string
}) {
  return (
    <div className={`panel p-4 flex items-center gap-3 group hover:border-${color}/30`}>
      <div className={`p-2 rounded-lg bg-${color}/10`}>
        <Icon className={`h-5 w-5 text-${color}`} />
      </div>
      <div>
        <div className="mono-value text-lg font-bold text-text-primary">{value}</div>
        <div className="text-[11px] text-text-muted uppercase tracking-wider">{label}</div>
      </div>
    </div>
  )
}

export default function Overview() {
  const { state } = useTruthState()
  const risk = state?.risk
  const pnl = risk?.daily_pnl ?? 0
  const trades = (risk as Record<string, unknown>)?.daily_trades_count as number ?? 0
  const maxTrades = 10
  const winRate = (state?.journal_stats as Record<string, number> | undefined)?.win_rate
  const winRateStr = winRate != null ? `${(winRate * 100).toFixed(0)}%` : '—'
  const serverTime = state?.truth?.server_time ?? 0
  const uptimeH = serverTime > 0 ? Math.floor((Date.now() / 1000 - serverTime) / 3600) : 0
  const uptimeM = serverTime > 0 ? Math.floor(((Date.now() / 1000 - serverTime) % 3600) / 60) : 0

  return (
    <div className="space-y-4 animate-fade-in">
      {/* Quick Stats Row */}
      <div className="grid grid-cols-4 gap-3">
        <QuickStat icon={TrendingUp} label="Daily PnL"
          value={`${pnl >= 0 ? '+' : ''}$${pnl.toFixed(4)}`}
          color={pnl >= 0 ? 'green' : 'red'} />
        <QuickStat icon={BarChart3} label="Trades Today"
          value={`${trades}/${maxTrades}`} color="blue" />
        <QuickStat icon={Target} label="Win Rate"
          value={winRateStr} color="purple" />
        <QuickStat icon={Clock} label="Session"
          value={serverTime > 0 ? `${uptimeH}h ${uptimeM}m` : '—'} color="cyan" />
      </div>

      <SymbolsTickerRow />

      {/* PnL Equity Curve — full width */}
      <PnlEquityCurve />

      <div className="grid grid-cols-12 gap-4">
        <div className="col-span-3 space-y-4">
          <RegimePanel />
          <SignalEnsembleCard />
          <TradeEligibilityCard />
          <DecisionPipelineVisual />
        </div>
        <div className="col-span-5 space-y-4">
          <AggregatePositionsPanel />
          <TripleBarrierViz />
          <PositionManagementGrid />
        </div>
        <div className="col-span-4 space-y-4">
          <SystemHealthCard />
          <VPINMonitor />
          <FuturesIntelPanel />
          <TradeJournalDash />
          <AiObserverPanel />
          <TwinStatePanel />
          <PositionTimelinesList />
          <RecentErrorsPanel />
          <ActionLogTable />
        </div>
      </div>
    </div>
  )
}
