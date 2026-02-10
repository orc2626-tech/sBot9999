/**
 * Trade Journal & Learning Analytics
 */
import { useTruthState } from '../hooks/useTruthState'

export default function TradeJournalDash() {
  const { state } = useTruthState()
  const stats = state?.journal_stats

  if (!stats) {
    return (
      <div className="panel p-4">
        <h3 className="text-sm font-semibold text-slate-400 uppercase tracking-wide">Trade Journal</h3>
        <p className="text-sm text-slate-500 mt-2">No trade data yet...</p>
      </div>
    )
  }

  const totalTrades = stats.total_trades ?? 0
  const winRate = stats.win_rate ?? 0
  const totalPnl = stats.total_net_pnl ?? 0
  const profitFactor = stats.profit_factor ?? 0

  return (
    <div className="panel p-4">
      <h3 className="text-sm font-semibold text-slate-400 uppercase tracking-wide mb-3">Trade Journal & Analytics</h3>

      <div className="grid grid-cols-4 gap-2 mb-4">
        <div className="text-center">
          <div className="text-xs text-slate-500">Trades</div>
          <div className="font-mono text-lg text-slate-200">{totalTrades}</div>
        </div>
        <div className="text-center">
          <div className="text-xs text-slate-500">Win Rate</div>
          <div className={`font-mono text-lg ${winRate >= 50 ? 'text-green' : 'text-red'}`}>
            {winRate.toFixed(1)}%
          </div>
        </div>
        <div className="text-center">
          <div className="text-xs text-slate-500">Net PnL</div>
          <div className={`font-mono text-lg ${totalPnl >= 0 ? 'text-green' : 'text-red'}`}>
            {totalPnl >= 0 ? '+' : ''}
            {totalPnl.toFixed(2)}
          </div>
        </div>
        <div className="text-center">
          <div className="text-xs text-slate-500">Profit Factor</div>
          <div className="font-mono text-lg text-slate-200">{profitFactor.toFixed(2)}</div>
        </div>
      </div>
    </div>
  )
}
