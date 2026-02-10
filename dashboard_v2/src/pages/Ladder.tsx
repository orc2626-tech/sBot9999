import { TrendingUp, TrendingDown, Flame, Snowflake, ArrowUp, ArrowDown, Minus } from 'lucide-react'
import SymbolsTickerRow from '../components/SymbolsTickerRow'
import { useTruthState } from '../hooks/useTruthState'

interface RungEvent {
  timestamp: string
  from_rung: number
  to_rung: number
  reason: string
  quote_amount: number
}

interface LadderData {
  current_rung: number
  quote_amount: number
  equity_curve_hot: boolean
  max_allowed_rung: number
  ema_fast: number
  ema_slow: number
  consecutive_wins: number
  total_trades: number
  total_wins: number
  total_losses: number
  rung_history: RungEvent[]
}

const RUNGS = [
  { rung: 5, quote: 25.00, label: 'MAXIMUM' },
  { rung: 4, quote: 22.75, label: '' },
  { rung: 3, quote: 20.50, label: 'NEUTRAL' },
  { rung: 2, quote: 18.25, label: '' },
  { rung: 1, quote: 16.00, label: 'MINIMUM' },
]

function LadderVisual({ data }: { data: LadderData }) {
  return (
    <div className="panel p-4">
      <div className="flex items-center justify-between mb-4">
        <h3 className="text-sm font-semibold text-slate-400 uppercase tracking-wide">Position Ladder</h3>
        <div className={`flex items-center gap-1.5 px-2.5 py-1 rounded text-xs font-semibold ${
          data.equity_curve_hot
            ? 'bg-amber-500/10 text-amber-400 border border-amber-500/20'
            : 'bg-blue/10 text-blue border border-blue/20'
        }`}>
          {data.equity_curve_hot ? <Flame className="h-3 w-3" /> : <Snowflake className="h-3 w-3" />}
          EC: {data.equity_curve_hot ? 'HOT' : 'COLD'}
        </div>
      </div>

      {/* Ladder visualization */}
      <div className="space-y-1.5">
        {RUNGS.map(({ rung, quote, label }) => {
          const isActive = rung === data.current_rung
          const isLocked = rung > data.max_allowed_rung
          const fill = isActive ? 100 : rung < data.current_rung ? 100 : 0

          return (
            <div key={rung} className="flex items-center gap-3">
              <span className={`w-14 text-right font-mono text-xs ${
                isActive ? 'text-green font-bold' : isLocked ? 'text-slate-600' : 'text-slate-400'
              }`}>
                R{rung}
              </span>
              <span className={`w-16 text-right font-mono text-xs ${
                isActive ? 'text-green font-bold' : isLocked ? 'text-slate-600' : 'text-slate-300'
              }`}>
                ${quote.toFixed(2)}
              </span>
              <div className="flex-1 h-5 bg-slate-800 rounded overflow-hidden relative">
                <div
                  className={`h-full rounded transition-all duration-500 ${
                    isActive ? 'bg-green' : isLocked ? 'bg-slate-700' : 'bg-green/20'
                  }`}
                  style={{ width: `${fill}%` }}
                />
                {isActive && (
                  <span className="absolute inset-0 flex items-center justify-center text-[10px] font-bold text-black">
                    ACTIVE
                  </span>
                )}
                {isLocked && (
                  <span className="absolute inset-0 flex items-center justify-center text-[10px] text-slate-600">
                    LOCKED (EC COLD)
                  </span>
                )}
              </div>
              <span className={`w-16 text-xs ${
                isActive ? 'text-green font-semibold' : 'text-slate-600'
              }`}>
                {label}
              </span>
            </div>
          )
        })}
      </div>

      {/* Current state summary */}
      <div className="mt-4 pt-3 border-t border-panelBorder grid grid-cols-4 gap-4 text-center">
        <div>
          <p className="text-[10px] text-slate-500 uppercase">Quote</p>
          <p className="text-lg font-mono font-bold text-green">${data.quote_amount.toFixed(2)}</p>
        </div>
        <div>
          <p className="text-[10px] text-slate-500 uppercase">Win Streak</p>
          <p className="text-lg font-mono font-bold text-slate-200">{data.consecutive_wins}</p>
        </div>
        <div>
          <p className="text-[10px] text-slate-500 uppercase">Max Rung</p>
          <p className="text-lg font-mono font-bold text-slate-200">{data.max_allowed_rung}</p>
        </div>
        <div>
          <p className="text-[10px] text-slate-500 uppercase">Win Rate</p>
          <p className="text-lg font-mono font-bold text-slate-200">
            {data.total_trades > 0 ? `${((data.total_wins / data.total_trades) * 100).toFixed(0)}%` : '—'}
          </p>
        </div>
      </div>
    </div>
  )
}

function StatsPanel({ data }: { data: LadderData }) {
  return (
    <div className="panel p-4">
      <h3 className="text-sm font-semibold text-slate-400 uppercase tracking-wide mb-3">Ladder Stats</h3>
      <div className="space-y-2 text-sm">
        <div className="flex justify-between">
          <span className="text-slate-400">Total Trades</span>
          <span className="font-mono text-slate-200">{data.total_trades}</span>
        </div>
        <div className="flex justify-between">
          <span className="text-slate-400">Wins</span>
          <span className="font-mono text-green">{data.total_wins}</span>
        </div>
        <div className="flex justify-between">
          <span className="text-slate-400">Losses</span>
          <span className="font-mono text-red">{data.total_losses}</span>
        </div>
        <div className="flex justify-between border-t border-panelBorder pt-2">
          <span className="text-slate-400">EMA Fast (5)</span>
          <span className="font-mono text-slate-200">{data.ema_fast > 0 ? data.ema_fast.toFixed(2) : '—'}</span>
        </div>
        <div className="flex justify-between">
          <span className="text-slate-400">EMA Slow (20)</span>
          <span className="font-mono text-slate-200">{data.ema_slow > 0 ? data.ema_slow.toFixed(2) : '—'}</span>
        </div>
        <div className="flex justify-between">
          <span className="text-slate-400">EMA Cross</span>
          <span className={`font-mono font-semibold ${data.equity_curve_hot ? 'text-green' : 'text-blue'}`}>
            {data.equity_curve_hot ? 'BULLISH (Hot)' : 'BEARISH (Cold)'}
          </span>
        </div>
      </div>
    </div>
  )
}

function MechanicsPanel() {
  return (
    <div className="panel p-4">
      <h3 className="text-sm font-semibold text-slate-400 uppercase tracking-wide mb-3">How It Works</h3>
      <div className="space-y-2 text-xs">
        <div className="flex items-start gap-2">
          <ArrowUp className="h-3.5 w-3.5 text-green shrink-0 mt-0.5" />
          <div>
            <span className="text-green font-semibold">Win</span>
            <p className="text-slate-500">2 consecutive wins = climb 1 rung (slow, earn it)</p>
          </div>
        </div>
        <div className="flex items-start gap-2">
          <ArrowDown className="h-3.5 w-3.5 text-red shrink-0 mt-0.5" />
          <div>
            <span className="text-red font-semibold">Loss</span>
            <p className="text-slate-500">1 loss = drop 2 rungs instantly (fast protection)</p>
          </div>
        </div>
        <div className="flex items-start gap-2">
          <Flame className="h-3.5 w-3.5 text-amber-400 shrink-0 mt-0.5" />
          <div>
            <span className="text-amber-400 font-semibold">Equity Curve Hot</span>
            <p className="text-slate-500">EMA(5) &gt; EMA(20) = all rungs unlocked</p>
          </div>
        </div>
        <div className="flex items-start gap-2">
          <Snowflake className="h-3.5 w-3.5 text-blue shrink-0 mt-0.5" />
          <div>
            <span className="text-blue font-semibold">Equity Curve Cold</span>
            <p className="text-slate-500">EMA(5) &lt; EMA(20) = max Rung 3 ($20.50)</p>
          </div>
        </div>
        <div className="flex items-start gap-2">
          <Minus className="h-3.5 w-3.5 text-slate-400 shrink-0 mt-0.5" />
          <div>
            <span className="text-slate-300 font-semibold">Conviction Boost</span>
            <p className="text-slate-500">Elite +1 temp rung, Standard -1 temp rung</p>
          </div>
        </div>
      </div>
    </div>
  )
}

function RungHistory({ events }: { events: RungEvent[] }) {
  if (events.length === 0) {
    return (
      <div className="panel p-4">
        <h3 className="text-sm font-semibold text-slate-400 uppercase tracking-wide mb-3">Rung History</h3>
        <p className="text-sm text-slate-500">No rung changes yet. Ladder starts at Rung 3 ($20.50).</p>
      </div>
    )
  }

  return (
    <div className="panel p-4">
      <h3 className="text-sm font-semibold text-slate-400 uppercase tracking-wide mb-3">
        Rung History <span className="text-slate-600">({events.length})</span>
      </h3>
      <div className="space-y-1 max-h-60 overflow-y-auto">
        {[...events].reverse().map((e, i) => {
          const up = e.to_rung > e.from_rung
          return (
            <div key={i} className="flex items-center gap-2 text-xs py-1 border-b border-panelBorder/50 last:border-0">
              {up ? (
                <TrendingUp className="h-3 w-3 text-green shrink-0" />
              ) : (
                <TrendingDown className="h-3 w-3 text-red shrink-0" />
              )}
              <span className={`font-mono font-semibold ${up ? 'text-green' : 'text-red'}`}>
                R{e.from_rung} → R{e.to_rung}
              </span>
              <span className="text-slate-500 font-mono">${e.quote_amount.toFixed(2)}</span>
              <span className="text-slate-500 truncate flex-1">{e.reason}</span>
              <span className="text-slate-600 text-[10px] shrink-0">
                {new Date(e.timestamp).toLocaleTimeString()}
              </span>
            </div>
          )
        })}
      </div>
    </div>
  )
}

export default function Ladder() {
  const { state } = useTruthState()
  const ladder = state?.ladder as LadderData | undefined

  if (!ladder) {
    return (
      <div className="space-y-4">
        <div className="panel p-3 border-amber-500/30 bg-amber-500/5">
          <h2 className="text-lg font-semibold text-slate-200">Anti-Fragile Position Ladder</h2>
          <p className="text-sm text-slate-400 mt-0.5">Waiting for ladder state from backend...</p>
        </div>
        <SymbolsTickerRow />
      </div>
    )
  }

  return (
    <div className="space-y-4">
      <div className="panel p-3 border-amber-500/30 bg-amber-500/5">
        <h2 className="text-lg font-semibold text-slate-200">Anti-Fragile Position Ladder (AFPL)</h2>
        <p className="text-sm text-slate-400 mt-0.5">
          Equity curve trading + asymmetric sizing. Slow up (2 wins), fast down (1 loss). $16–$25 range.
        </p>
      </div>
      <SymbolsTickerRow />
      <div className="grid grid-cols-12 gap-4">
        <div className="col-span-8 space-y-4">
          <LadderVisual data={ladder} />
          <RungHistory events={ladder.rung_history ?? []} />
        </div>
        <div className="col-span-4 space-y-4">
          <StatsPanel data={ladder} />
          <MechanicsPanel />
        </div>
      </div>
    </div>
  )
}
