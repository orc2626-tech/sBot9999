import { Shield, ShieldAlert, ShieldCheck, Zap, Crown, Lock, Unlock, AlertTriangle } from 'lucide-react'
import SymbolsTickerRow from '../components/SymbolsTickerRow'
import TradeEligibilityCard from '../components/TradeEligibilityCard'
import SystemHealthCard from '../components/SystemHealthCard'
import { PanelTwoStageEntry } from '../components/PagePanels'
import { useTruthState } from '../hooks/useTruthState'

// ═══════════════════════════════════════════════
// Gate definitions matching backend Conviction Hierarchy
// ═══════════════════════════════════════════════

interface GateDef {
  name: string
  tier: 'T0:CRITICAL' | 'T1:SAFETY' | 'T2:QUALITY' | 'T3:PREF'
  label: string
  description: string
}

const GATES: GateDef[] = [
  { name: 'NAN_SAFETY',     tier: 'T0:CRITICAL', label: 'NaN Safety',        description: 'All values finite, price > 0' },
  { name: 'MIN_LIQUIDITY',  tier: 'T1:SAFETY',   label: 'Min Liquidity',     description: 'Enough depth to fill without slippage' },
  { name: 'ATR_SANITY',     tier: 'T1:SAFETY',   label: 'ATR Sanity',        description: 'Market moving but not insane' },
  { name: 'GIANT_CANDLE',   tier: 'T1:SAFETY',   label: 'Giant Candle',      description: 'Last candle < 3× ATR (no flash crash)' },
  { name: 'MIN_CONFIDENCE', tier: 'T1:SAFETY',   label: 'Min Confidence',    description: 'Signal strength ≥ 0.15' },
  { name: 'SPREAD_OK',      tier: 'T2:QUALITY',  label: 'Spread OK',         description: 'Spread ≤ 30 bps' },
  { name: 'ENTROPY_SAFE',   tier: 'T2:QUALITY',  label: 'Entropy Safe',      description: 'Market not pure noise (< 0.92)' },
  { name: 'REGIME_COMPAT',  tier: 'T2:QUALITY',  label: 'Regime Compat',     description: 'VOLATILE needs conf ≥ 0.40' },
  { name: 'DIRECTION_OK',   tier: 'T2:QUALITY',  label: 'Direction OK',      description: 'Orderbook not strongly against us' },
  { name: 'TIME_SESSION',   tier: 'T3:PREF',     label: 'Time Session',      description: 'Not midnight UTC (low liquidity)' },
  { name: 'SPREAD_SPIKE',   tier: 'T3:PREF',     label: 'Spread Spike',      description: 'Spread < 3× average (no trap)' },
]

const tierMeta: Record<string, { color: string; bg: string; border: string; icon: React.ReactNode; label: string; bypassable: string }> = {
  'T0:CRITICAL': { color: 'text-red',       bg: 'bg-red/10',    border: 'border-red/30',    icon: <Lock className="h-3.5 w-3.5" />,        label: 'CRITICAL', bypassable: 'Never' },
  'T1:SAFETY':   { color: 'text-orange-400', bg: 'bg-orange-500/10', border: 'border-orange-500/30', icon: <ShieldAlert className="h-3.5 w-3.5" />, label: 'SAFETY',   bypassable: 'Never' },
  'T2:QUALITY':  { color: 'text-yellow',     bg: 'bg-yellow/10', border: 'border-yellow/30', icon: <Shield className="h-3.5 w-3.5" />,      label: 'QUALITY',  bypassable: 'Elite only' },
  'T3:PREF':     { color: 'text-blue',       bg: 'bg-blue/10',   border: 'border-blue/30',   icon: <Unlock className="h-3.5 w-3.5" />,      label: 'PREF',     bypassable: 'High + Elite' },
}

function parseInsuranceVerdict(verdict: string | undefined): {
  passed: boolean
  conviction: string | null
  gatesPassed: number
  gatesTotal: number
  gatesBypassed: number
  blockedBy: string | null
} {
  if (!verdict) return { passed: false, conviction: null, gatesPassed: 0, gatesTotal: 11, gatesBypassed: 0, blockedBy: null }

  if (verdict.startsWith('PASS')) {
    // PASS(9/11 +2bypass Elite) or PASS(11/11 Standard)
    const conviction = verdict.match(/(Standard|High|Elite)/)?.[1] ?? null
    const counts = verdict.match(/(\d+)\/(\d+)/)
    const bypassed = verdict.match(/\+(\d+)bypass/)?.[1]
    return {
      passed: true,
      conviction,
      gatesPassed: counts ? parseInt(counts[1]) : 0,
      gatesTotal: counts ? parseInt(counts[2]) : 11,
      gatesBypassed: bypassed ? parseInt(bypassed) : 0,
      blockedBy: null,
    }
  }

  // BLOCK:GATE_NAME
  const blockedBy = verdict.replace('BLOCK:', '')
  return { passed: false, conviction: null, gatesPassed: 0, gatesTotal: 11, gatesBypassed: 0, blockedBy }
}

function ConvictionBadge({ level }: { level: string | null }) {
  if (!level) return <span className="text-slate-500 text-xs">—</span>

  const styles: Record<string, { bg: string; text: string; icon: React.ReactNode }> = {
    Standard: { bg: 'bg-slate-700/50', text: 'text-slate-300', icon: <Shield className="h-3 w-3" /> },
    High:     { bg: 'bg-blue/20',      text: 'text-blue',      icon: <Zap className="h-3 w-3" /> },
    Elite:    { bg: 'bg-amber-500/20',  text: 'text-amber-400', icon: <Crown className="h-3 w-3" /> },
  }

  const s = styles[level] ?? styles.Standard!

  return (
    <span className={`inline-flex items-center gap-1 px-2 py-0.5 rounded text-xs font-semibold ${s.bg} ${s.text}`}>
      {s.icon} {level}
    </span>
  )
}

function ConvictionHierarchyVisual({ verdict }: { verdict: string | undefined }) {
  const parsed = parseInsuranceVerdict(verdict)

  return (
    <div className="panel p-4">
      <div className="flex items-center justify-between mb-4">
        <h3 className="text-sm font-semibold text-slate-400 uppercase tracking-wide">Conviction Hierarchy</h3>
        <ConvictionBadge level={parsed.conviction} />
      </div>

      {/* Summary bar */}
      <div className="flex items-center gap-3 mb-4 p-2 rounded border border-panelBorder bg-black/20">
        {parsed.passed ? (
          <ShieldCheck className="h-5 w-5 text-green shrink-0" />
        ) : (
          <ShieldAlert className="h-5 w-5 text-red shrink-0" />
        )}
        <div className="flex-1 min-w-0">
          <span className={`font-mono text-sm font-semibold ${parsed.passed ? 'text-green' : 'text-red'}`}>
            {parsed.passed ? 'PASS' : 'BLOCKED'}
          </span>
          <span className="text-slate-500 text-xs ml-2">
            {parsed.gatesPassed}/{parsed.gatesTotal} passed
            {parsed.gatesBypassed > 0 && (
              <span className="text-amber-400 ml-1">+{parsed.gatesBypassed} bypassed</span>
            )}
          </span>
        </div>
        {parsed.blockedBy && (
          <span className="text-xs font-mono text-red bg-red/10 px-2 py-0.5 rounded">
            {parsed.blockedBy}
          </span>
        )}
      </div>

      {/* Gate list by tier */}
      <div className="space-y-1">
        {GATES.map((gate) => {
          const meta = tierMeta[gate.tier]!
          const isBlocked = parsed.blockedBy === gate.name
          return (
            <div
              key={gate.name}
              className={`flex items-center gap-2 px-2 py-1 rounded text-xs ${
                isBlocked ? 'bg-red/10 border border-red/30' : 'hover:bg-white/[0.02]'
              }`}
            >
              <span className={`shrink-0 ${meta.color}`}>{meta.icon}</span>
              <span className={`w-16 shrink-0 font-mono ${meta.color} text-[10px]`}>{meta.label}</span>
              <span className="text-slate-300 flex-1 truncate">{gate.label}</span>
              <span className="text-slate-600 text-[10px] hidden lg:inline">{gate.description}</span>
              {isBlocked ? (
                <span className="text-red font-semibold ml-auto">FAIL</span>
              ) : (
                <span className="text-green/60 ml-auto">OK</span>
              )}
            </div>
          )
        })}
      </div>
    </div>
  )
}

function ConvictionExplainer() {
  return (
    <div className="panel p-4">
      <h3 className="text-sm font-semibold text-slate-400 uppercase tracking-wide mb-3">Conviction Levels</h3>
      <div className="space-y-2">
        <div className="flex items-start gap-2 text-xs">
          <Crown className="h-3.5 w-3.5 text-amber-400 shrink-0 mt-0.5" />
          <div>
            <span className="text-amber-400 font-semibold">Elite</span>
            <span className="text-slate-500 ml-1">(conf &gt; 0.75)</span>
            <p className="text-slate-500 mt-0.5">Bypasses Quality + Preference tiers. Only Critical and Safety gates apply.</p>
          </div>
        </div>
        <div className="flex items-start gap-2 text-xs">
          <Zap className="h-3.5 w-3.5 text-blue shrink-0 mt-0.5" />
          <div>
            <span className="text-blue font-semibold">High</span>
            <span className="text-slate-500 ml-1">(conf 0.50-0.75)</span>
            <p className="text-slate-500 mt-0.5">Bypasses Preference tier only. Must pass Quality gates.</p>
          </div>
        </div>
        <div className="flex items-start gap-2 text-xs">
          <Shield className="h-3.5 w-3.5 text-slate-400 shrink-0 mt-0.5" />
          <div>
            <span className="text-slate-300 font-semibold">Standard</span>
            <span className="text-slate-500 ml-1">(conf &lt; 0.50)</span>
            <p className="text-slate-500 mt-0.5">Must pass ALL tiers. No bypasses allowed.</p>
          </div>
        </div>
      </div>
      <div className="mt-3 pt-2 border-t border-panelBorder">
        <div className="flex items-center gap-1 text-[10px] text-slate-600">
          <AlertTriangle className="h-3 w-3" />
          <span>Critical + Safety tiers are NEVER bypassed regardless of conviction level.</span>
        </div>
      </div>
    </div>
  )
}

export default function Insurance() {
  const { state } = useTruthState()
  const lastDecision = state?.recent_decisions?.[0]

  return (
    <div className="space-y-4">
      <div className="panel p-3 border-blue/30 bg-blue/5">
        <h2 className="text-lg font-semibold text-slate-200">Trade Insurance — Conviction Hierarchy</h2>
        <p className="text-sm text-slate-400 mt-0.5">
          11 gates organized into 4 tiers. Strong signals bypass low-priority gates while safety is always enforced.
        </p>
      </div>
      <SymbolsTickerRow />
      <div className="grid grid-cols-12 gap-4">
        <div className="col-span-8 space-y-4">
          <ConvictionHierarchyVisual verdict={lastDecision?.insurance_verdict} />
        </div>
        <div className="col-span-4 space-y-4">
          <TradeEligibilityCard />
          <PanelTwoStageEntry />
          <ConvictionExplainer />
          <SystemHealthCard />
        </div>
      </div>
    </div>
  )
}
