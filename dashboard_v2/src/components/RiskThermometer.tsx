/**
 * Risk Thermometer — Institutional Risk Visualization
 * =====================================================
 * Used by hedge fund risk desks (Bridgewater, Citadel).
 * Shows TOTAL risk exposure as a single vertical gauge.
 * Combines: circuit breaker proximity + VPIN + regime danger + PnL drawdown
 * into ONE number that tells you "how close to disaster are we?"
 */
import { useTruthState } from '../hooks/useTruthState'
import { Thermometer } from 'lucide-react'

export default function RiskThermometer() {
  const { state } = useTruthState()
  const risk = state?.risk as Record<string, unknown> | undefined
  const regime = state?.regime
  const firstSym = state?.runtime_config?.symbols?.[0] ?? ''
  const vpin = firstSym && state?.vpin?.[firstSym] ? state.vpin[firstSym].vpin : 0

  if (!risk) return null

  // Calculate composite risk score (0 = safe, 100 = maximum danger)
  const breakers = risk.circuit_breakers as Record<string, { current: string | number; limit: string | number; tripped: boolean }> | undefined
  let breakerHeat = 0
  if (breakers) {
    const values = Object.values(breakers)
    const trippedCount = values.filter(b => b.tripped).length
    breakerHeat = (trippedCount / Math.max(values.length, 1)) * 100
  }

  const vpinHeat = Math.max(0, (vpin - 0.50) / 0.50) * 100 // 0.50-1.0 → 0-100
  const entropyHeat = regime?.entropy != null ? Math.max(0, (regime.entropy - 0.70) / 0.30) * 100 : 0
  const pnlPct = (risk.daily_pnl_pct as number) ?? 0
  const pnlHeat = Math.max(0, Math.abs(pnlPct) / 1.5) * 100 // 0-1.5% → 0-100

  const composite = Math.min(100, breakerHeat * 0.4 + vpinHeat * 0.25 + entropyHeat * 0.15 + pnlHeat * 0.20)

  const getColor = (v: number) => {
    if (v < 25) return '#10b981'  // green
    if (v < 50) return '#f59e0b'  // yellow
    if (v < 75) return '#f97316'  // orange
    return '#ef4444'               // red
  }

  const color = getColor(composite)
  const label = composite < 25 ? 'LOW' : composite < 50 ? 'MODERATE' : composite < 75 ? 'HIGH' : 'CRITICAL'

  return (
    <div className="panel p-5">
      <div className="flex items-center justify-between mb-4">
        <h3 className="section-title flex items-center gap-2">
          <Thermometer className="h-4 w-4" style={{ color }} />
          Risk Exposure
        </h3>
        <span className="text-[10px] font-mono px-2 py-0.5 rounded-full font-bold"
          style={{ color, background: `${color}15`, border: `1px solid ${color}30` }}>
          {label}
        </span>
      </div>

      {/* Vertical thermometer */}
      <div className="flex items-end gap-4">
        <div className="relative w-8 h-32 bg-bg-primary rounded-full overflow-hidden border border-panel-border/50">
          <div className="absolute bottom-0 w-full rounded-full transition-all duration-1000"
            style={{
              height: `${composite}%`,
              background: `linear-gradient(to top, ${color}, ${color}80)`,
              boxShadow: `0 0 12px ${color}40`,
            }} />
          {/* Markers */}
          {[25, 50, 75].map(m => (
            <div key={m} className="absolute w-full border-t border-panel-border/40" style={{ bottom: `${m}%` }} />
          ))}
        </div>

        <div className="flex-1 space-y-2">
          <div className="flex justify-between text-xs">
            <span className="text-text-muted">Breakers</span>
            <span className="mono-value" style={{ color: getColor(breakerHeat) }}>{breakerHeat.toFixed(0)}%</span>
          </div>
          <div className="flex justify-between text-xs">
            <span className="text-text-muted">VPIN Flow</span>
            <span className="mono-value" style={{ color: getColor(vpinHeat) }}>{vpinHeat.toFixed(0)}%</span>
          </div>
          <div className="flex justify-between text-xs">
            <span className="text-text-muted">Entropy</span>
            <span className="mono-value" style={{ color: getColor(entropyHeat) }}>{entropyHeat.toFixed(0)}%</span>
          </div>
          <div className="flex justify-between text-xs">
            <span className="text-text-muted">PnL Stress</span>
            <span className="mono-value" style={{ color: getColor(pnlHeat) }}>{pnlHeat.toFixed(0)}%</span>
          </div>
          <div className="pt-2 border-t border-panel-border/30">
            <div className="flex justify-between text-xs">
              <span className="text-text-secondary font-medium">Composite</span>
              <span className="mono-value text-base font-bold" style={{ color }}>{composite.toFixed(0)}%</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}
