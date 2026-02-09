/**
 * VPIN & Toxic Flow Monitor — with visual gauge
 */
import { Bell, Activity } from 'lucide-react'
import { useTruthState } from '../hooks/useTruthState'

function VpinGauge({ value, zone }: { value: number; zone: string }) {
  const angle = Math.min(value, 1) * 180
  const color = zone === 'DANGER' ? '#ef4444' : zone === 'CAUTION' ? '#f59e0b' : '#10b981'
  return (
    <div className="relative w-36 h-[72px] mx-auto mb-1">
      <svg viewBox="0 0 200 100" className="w-full h-full">
        <path d="M 10 95 A 90 90 0 0 1 190 95" fill="none" stroke="#1e3a5f" strokeWidth="14" strokeLinecap="round" />
        <path d="M 10 95 A 90 90 0 0 1 190 95" fill="none" stroke={color} strokeWidth="14" strokeLinecap="round"
          strokeDasharray={`${angle * 1.57} 999`} className="transition-all duration-700"
          style={{ filter: `drop-shadow(0 0 8px ${color})` }} />
      </svg>
      <div className="absolute inset-0 flex items-end justify-center pb-0.5">
        <span className="mono-value text-2xl font-bold" style={{ color }}>{value.toFixed(3)}</span>
      </div>
    </div>
  )
}

export default function VPINMonitor() {
  const { state } = useTruthState()
  const vpinMap = state?.vpin ?? {}
  const firstSymbol = state?.runtime_config?.symbols?.[0]
  const vpin = firstSymbol ? vpinMap[firstSymbol] : null

  if (!vpin) {
    return (
      <div className="panel p-5">
        <h3 className="section-title flex items-center gap-2"><Activity className="h-4 w-4 text-cyan" />VPIN Monitor</h3>
        <p className="text-sm text-text-muted mt-3">Waiting for VPIN data...</p>
      </div>
    )
  }

  const vpinValue = vpin.vpin ?? 0
  const status = (vpin as Record<string, unknown>).status as string ?? ''
  const zone = status === 'DANGER' ? 'DANGER' : (status === 'CAUTION' || vpinValue > 0.80) ? 'CAUTION' : 'SAFE'
  const buyVol = (vpin as { buy_volume?: number }).buy_volume ?? 50
  const sellVol = (vpin as { sell_volume?: number }).sell_volume ?? 50
  const total = buyVol + sellVol
  const buyPct = total > 0 ? (buyVol / total) * 100 : 50

  const glowClass = zone === 'DANGER' ? 'glow-red' : zone === 'CAUTION' ? 'glow-yellow' : ''

  return (
    <div className={`panel p-5 transition-all duration-500 ${glowClass}`}>
      <div className="flex items-center justify-between mb-3">
        <h3 className="section-title flex items-center gap-2">
          <Activity className="h-4 w-4 text-cyan" />VPIN — Toxic Flow
        </h3>
        <span className={`text-[10px] font-mono px-2 py-0.5 rounded-full font-bold ${
          zone === 'DANGER' ? 'bg-red/15 text-red border border-red/30' :
          zone === 'CAUTION' ? 'bg-yellow/15 text-yellow border border-yellow/30' :
          'bg-green/10 text-green border border-green/20'
        }`}>{zone}</span>
      </div>

      <VpinGauge value={vpinValue} zone={zone} />

      <div className="mt-3">
        <div className="flex items-center gap-2 text-xs mb-1.5">
          <div className="flex-1 h-2.5 bg-bg-primary rounded-full overflow-hidden flex">
            <div className="h-full bg-green rounded-l-full transition-all duration-500" style={{ width: `${buyPct}%` }} />
            <div className="h-full bg-red rounded-r-full flex-1" />
          </div>
        </div>
        <div className="flex justify-between text-[10px] text-text-muted font-mono">
          <span className="text-green">Buy: {buyPct.toFixed(0)}%</span>
          <span className="text-red">Sell: {(100 - buyPct).toFixed(0)}%</span>
        </div>
      </div>

      {zone === 'DANGER' && (
        <div className="mt-3 p-2.5 rounded-lg bg-red/10 border border-red/30 flex items-center gap-2 animate-fade-in">
          <Bell className="h-4 w-4 text-red shrink-0 animate-pulse" />
          <span className="text-xs text-red font-medium">Toxic flow detected — BLOCK active</span>
        </div>
      )}
    </div>
  )
}
