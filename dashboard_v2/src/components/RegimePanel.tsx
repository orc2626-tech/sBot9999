/**
 * Market Regime Panel — upgraded visual with glow effects
 */
import { TrendingUp, ArrowRightLeft, Zap, Minimize2, Skull } from 'lucide-react'
import { useTruthState } from '../hooks/useTruthState'

const REGIME_CONFIG: Record<string, { icon: React.ElementType; bg: string; text: string; border: string; glow: string }> = {
  TRENDING:  { icon: TrendingUp,    bg: 'bg-green/8',     text: 'text-green',    border: 'border-green/30',    glow: 'glow-green' },
  RANGING:   { icon: ArrowRightLeft,bg: 'bg-blue/8',      text: 'text-blue',     border: 'border-blue/30',     glow: 'glow-blue' },
  VOLATILE:  { icon: Zap,           bg: 'bg-red/8',       text: 'text-red',      border: 'border-red/30',      glow: 'glow-red' },
  SQUEEZE:   { icon: Minimize2,     bg: 'bg-yellow/8',    text: 'text-yellow',   border: 'border-yellow/30',   glow: 'glow-yellow' },
  DEAD:      { icon: Skull,         bg: 'bg-slate-600/8', text: 'text-slate-500',border: 'border-slate-600/30', glow: '' },
}

function MetricBar({ label, value, maxVal, color, suffix = '' }: {
  label: string; value: number; maxVal: number; color: string; suffix?: string
}) {
  const pct = Math.min((value / maxVal) * 100, 100)
  return (
    <div className="space-y-1">
      <div className="flex justify-between text-xs">
        <span className="text-text-muted">{label}</span>
        <span className="mono-value text-text-primary">{value.toFixed(2)}{suffix}</span>
      </div>
      <div className="h-1.5 bg-bg-primary rounded-full overflow-hidden">
        <div className={`h-full ${color} rounded-full transition-all duration-700`} style={{ width: `${pct}%` }} />
      </div>
    </div>
  )
}

export default function RegimePanel() {
  const { state } = useTruthState()
  const regime = state?.regime

  if (!regime) {
    return (
      <div className="panel p-5">
        <h3 className="section-title">Market Regime</h3>
        <p className="text-sm text-text-muted mt-3">Waiting for regime data...</p>
      </div>
    )
  }

  const r = regime.regime ?? 'DEAD'
  const cfg = REGIME_CONFIG[r] ?? REGIME_CONFIG.DEAD
  const Icon = cfg.icon
  const adx = regime.adx ?? 0
  const bbw = regime.bbw ?? 0
  const hurst = regime.hurst ?? 0.5
  const entropy = regime.entropy ?? 1

  return (
    <div className={`panel p-5 ${cfg.glow} transition-all duration-500`}>
      <div className="flex items-center justify-between mb-4">
        <h3 className="section-title">Market Regime</h3>
        <div className={`flex items-center gap-1.5 px-3 py-1 rounded-lg font-mono font-bold text-sm ${cfg.text} ${cfg.bg} border ${cfg.border}`}>
          <Icon className="h-4 w-4" />
          {r}
        </div>
      </div>

      <div className="space-y-3">
        <MetricBar label="ADX (Trend)" value={adx} maxVal={60} color="bg-blue" />
        <MetricBar label="BBW (Volatility)" value={bbw} maxVal={10} color="bg-yellow" suffix="%" />

        <div className="grid grid-cols-2 gap-3 pt-1">
          <div className="space-y-0.5">
            <span className="text-[10px] text-text-muted uppercase tracking-wider">Hurst</span>
            <div className={`mono-value text-lg font-bold ${hurst > 0.55 ? 'text-green' : hurst < 0.45 ? 'text-yellow' : 'text-red'}`}>
              {hurst.toFixed(3)}
            </div>
            <span className="text-[9px] text-text-muted">
              {hurst > 0.55 ? 'Momentum' : hurst < 0.45 ? 'Mean-revert' : 'Random'}
            </span>
          </div>
          <div className="space-y-0.5">
            <span className="text-[10px] text-text-muted uppercase tracking-wider">Entropy</span>
            <div className={`mono-value text-lg font-bold ${entropy < 0.8 ? 'text-green' : entropy < 0.95 ? 'text-yellow' : 'text-red'}`}>
              {entropy.toFixed(3)}
            </div>
            <span className="text-[9px] text-text-muted">
              {entropy >= 0.95 ? 'Noise' : entropy >= 0.8 ? 'Reduce 50%' : 'Clear'}
            </span>
          </div>
        </div>
      </div>

      <div className="mt-3 pt-2 border-t border-panel-border/30 text-[10px] text-text-muted font-mono">
        Age: {regime.regime_age_seconds ?? 0}s
      </div>
    </div>
  )
}
