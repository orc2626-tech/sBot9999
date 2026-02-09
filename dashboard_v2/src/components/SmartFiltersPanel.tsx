/**
 * SmartFiltersPanel — Phase 1 Advanced Gates Dashboard.
 *
 * Displays:
 * 1. HTF Trend Direction (15M + 1H structural direction per symbol)
 * 2. Feature Flag toggles (enable/disable each smart filter)
 * 3. Smart filter verdicts from recent decisions
 */

import { useTruthState } from '../hooks/useTruthState'
import { setFeatureFlags, type HtfAnalysis, type FeatureFlags, type SmartFilterVerdicts, type CusumState, type AbsorptionState } from '../lib/api'
import { useState } from 'react'

function HtfDirectionBadge({ direction, confidence }: { direction: string; confidence: number }) {
  const color =
    direction === 'BULLISH'
      ? 'text-green bg-green/10 border-green/30'
      : direction === 'BEARISH'
        ? 'text-red bg-red/10 border-red/30'
        : 'text-yellow bg-yellow/10 border-yellow/30'
  const arrow =
    direction === 'BULLISH' ? '▲' : direction === 'BEARISH' ? '▼' : '◆'

  return (
    <span className={`inline-flex items-center gap-1 px-2 py-0.5 rounded border text-xs font-mono ${color}`}>
      {arrow} {direction} ({(confidence * 100).toFixed(0)}%)
    </span>
  )
}

function HtfSymbolRow({ symbol, htf }: { symbol: string; htf: HtfAnalysis }) {
  return (
    <div className="flex items-center justify-between py-1.5 px-2 rounded bg-bg-secondary/50">
      <span className="text-xs font-mono text-text-secondary">{symbol}</span>
      <div className="flex items-center gap-2">
        <div className="flex items-center gap-1 text-[10px] text-text-muted">
          <span>15M: {htf.trend_15m > 0 ? '▲' : htf.trend_15m < 0 ? '▼' : '—'}</span>
          <span>1H: {htf.trend_1h > 0 ? '▲' : htf.trend_1h < 0 ? '▼' : '—'}</span>
        </div>
        <HtfDirectionBadge direction={htf.direction} confidence={htf.confidence} />
        {!htf.buy_allowed && (
          <span className="text-[10px] px-1.5 py-0.5 rounded bg-red/20 text-red border border-red/30">
            BUY BLOCKED
          </span>
        )}
      </div>
    </div>
  )
}

function FeatureFlagToggle({
  label,
  description,
  enabled,
  flagKey,
  onToggle,
}: {
  label: string
  description: string
  enabled: boolean
  flagKey: keyof FeatureFlags
  onToggle: (key: keyof FeatureFlags, value: boolean) => void
}) {
  return (
    <label className="flex items-start gap-3 py-1.5 cursor-pointer group">
      <div className="relative mt-0.5">
        <input
          type="checkbox"
          checked={enabled}
          onChange={() => onToggle(flagKey, !enabled)}
          className="sr-only"
        />
        <div
          className={`w-8 h-4 rounded-full transition-colors ${
            enabled ? 'bg-green' : 'bg-panel-border'
          }`}
        >
          <div
            className={`w-3.5 h-3.5 rounded-full bg-white shadow transition-transform ${
              enabled ? 'translate-x-4' : 'translate-x-0.5'
            } mt-[1px]`}
          />
        </div>
      </div>
      <div className="flex-1 min-w-0">
        <div className="text-xs font-medium text-text-primary group-hover:text-blue transition-colors">
          {label}
        </div>
        <div className="text-[10px] text-text-muted leading-tight">{description}</div>
      </div>
      <span
        className={`text-[10px] px-1.5 py-0.5 rounded ${
          enabled
            ? 'bg-green/10 text-green border border-green/30'
            : 'bg-panel-border/30 text-text-muted border border-panel-border'
        }`}
      >
        {enabled ? 'ON' : 'OFF'}
      </span>
    </label>
  )
}

function CusumBreakBadge({ cusum }: { cusum: CusumState }) {
  if (cusum.bullish_break) {
    return (
      <span className="inline-flex items-center gap-1 px-1.5 py-0.5 rounded bg-green/10 text-green border border-green/30 text-[10px] font-mono">
        ▲ BULLISH BREAK ({(cusum.break_confidence * 100).toFixed(0)}%)
      </span>
    )
  }
  if (cusum.bearish_break) {
    return (
      <span className="inline-flex items-center gap-1 px-1.5 py-0.5 rounded bg-red/10 text-red border border-red/30 text-[10px] font-mono">
        ▼ BEARISH BREAK ({(cusum.break_confidence * 100).toFixed(0)}%)
      </span>
    )
  }
  return (
    <span className="text-[10px] text-text-muted font-mono">
      S+={cusum.s_plus.toFixed(5)} S-={cusum.s_minus.toFixed(5)}
    </span>
  )
}

function AbsorptionBadge({ abs }: { abs: AbsorptionState }) {
  if (!abs.detected) return <span className="text-[10px] text-text-muted">None</span>
  const color = abs.direction === 'BUY_ABSORPTION'
    ? 'text-green bg-green/10 border-green/30'
    : abs.direction === 'SELL_ABSORPTION'
      ? 'text-red bg-red/10 border-red/30'
      : 'text-yellow bg-yellow/10 border-yellow/30'
  return (
    <span className={`inline-flex items-center gap-1 px-1.5 py-0.5 rounded border text-[10px] font-mono ${color}`}>
      {abs.direction.replace('_ABSORPTION', '')} vol={abs.best_volume_ratio.toFixed(1)}x
      {abs.cvd_confirmed && ' CVD✓'}
      {!abs.cvd_confirmed && ' CVD?'}
    </span>
  )
}

function SmartFilterVerdictsDisplay({ sf }: { sf: SmartFilterVerdicts }) {
  return (
    <div className="space-y-1 text-[10px]">
      {sf.htf && (
        <div className="flex items-center justify-between">
          <span className="text-text-muted">HTF:</span>
          <HtfDirectionBadge direction={sf.htf.direction} confidence={sf.htf.confidence} />
        </div>
      )}
      {sf.cusum && (
        <div className="flex items-center justify-between">
          <span className="text-text-muted">CUSUM:</span>
          <CusumBreakBadge cusum={sf.cusum} />
        </div>
      )}
      {sf.cusum_htf_interaction && sf.cusum_htf_interaction.soft_block && (
        <div className="flex items-center justify-between">
          <span className="text-text-muted">CUSUM×HTF:</span>
          <span className="text-yellow font-mono">
            SOFT BLOCK {(sf.cusum_htf_interaction.position_factor * 100).toFixed(0)}%
          </span>
        </div>
      )}
      {sf.score_momentum && (
        <div className="flex items-center justify-between">
          <span className="text-text-muted">Score Mom:</span>
          <span className={sf.score_momentum.buy_allowed ? 'text-green' : 'text-red'}>
            avg={sf.score_momentum.avg_score.toFixed(3)} ratio={sf.score_momentum.positive_ratio.toFixed(2)}
            {sf.score_momentum.buy_allowed ? ' PASS' : ' BLOCK'}
          </span>
        </div>
      )}
      {sf.ofip && (
        <div className="flex items-center justify-between">
          <span className="text-text-muted">OFIP:</span>
          <span className={sf.ofip.buy_blocked ? 'text-red' : sf.ofip.buy_boost ? 'text-green' : 'text-text-secondary'}>
            imb={sf.ofip.avg_imbalance.toFixed(3)} per={sf.ofip.persistence.toFixed(2)}
            {sf.ofip.buy_blocked ? ' BLOCKED' : sf.ofip.buy_boost ? ' BOOST' : ''}
          </span>
        </div>
      )}
      {sf.absorption && (
        <div className="flex items-center justify-between">
          <span className="text-text-muted">Absorption:</span>
          <AbsorptionBadge abs={sf.absorption} />
        </div>
      )}
      {sf.adaptive_threshold != null && (
        <div className="flex items-center justify-between">
          <span className="text-text-muted">Threshold:</span>
          <span className="text-text-secondary font-mono">{sf.adaptive_threshold.toFixed(3)}</span>
        </div>
      )}
      {sf.entropy_multiplier != null && (
        <div className="flex items-center justify-between">
          <span className="text-text-muted">Entropy Mult:</span>
          <span className={`font-mono ${sf.entropy_multiplier < 0.7 ? 'text-yellow' : 'text-text-secondary'}`}>
            {(sf.entropy_multiplier * 100).toFixed(0)}%
          </span>
        </div>
      )}
      {sf.entropy_valley && sf.entropy_valley.valley_detected && (
        <div className="flex items-center justify-between">
          <span className="text-text-muted">Entropy Valley:</span>
          <span className="text-green font-mono">
            DETECTED +{(sf.entropy_valley.confidence_boost * 100).toFixed(1)}% boost
          </span>
        </div>
      )}
    </div>
  )
}

export default function SmartFiltersPanel() {
  const { state } = useTruthState()
  const [saving, setSaving] = useState(false)

  const flags = state?.feature_flags
  const htfData = state?.htf_analysis

  const handleToggle = async (key: keyof FeatureFlags, value: boolean) => {
    setSaving(true)
    try {
      await setFeatureFlags({ [key]: value })
    } catch (e) {
      console.error('Failed to toggle feature flag:', e)
    }
    setSaving(false)
  }

  // Get the latest decision with smart_filters
  const latestWithFilters = state?.recent_decisions?.find(d => d.smart_filters)

  return (
    <div className="panel p-3 space-y-3">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h3 className="text-sm font-semibold text-text-primary flex items-center gap-2">
            Smart Filters
            <span className="text-[10px] px-1.5 py-0.5 rounded bg-blue/10 text-blue border border-blue/30">
              Phase 1+2
            </span>
          </h3>
          <p className="text-[10px] text-text-muted">
            HTF + CUSUM + Absorption + OFIP + Momentum + Adaptive + Entropy
          </p>
        </div>
        {saving && <span className="text-[10px] text-yellow animate-pulse">Saving...</span>}
      </div>

      {/* HTF Structural Direction */}
      {htfData && Object.keys(htfData).length > 0 && (
        <div className="space-y-1">
          <div className="text-[10px] font-medium text-text-muted uppercase tracking-wider">
            Structural Direction (15M + 1H)
          </div>
          <div className="space-y-1">
            {Object.entries(htfData).map(([symbol, htf]) => (
              <HtfSymbolRow key={symbol} symbol={symbol} htf={htf} />
            ))}
          </div>
        </div>
      )}

      {/* Feature Flag Toggles */}
      {flags && (
        <div className="space-y-0.5">
          <div className="text-[10px] font-medium text-text-muted uppercase tracking-wider mb-1">
            Feature Flags
          </div>
          <FeatureFlagToggle
            label="HTF Trend Gate"
            description="Block BUY when 15M+1H show bearish structure"
            enabled={flags.htf_gate}
            flagKey="htf_gate"
            onToggle={handleToggle}
          />
          <FeatureFlagToggle
            label="Score Momentum"
            description="Require sustained positive score before BUY"
            enabled={flags.score_momentum}
            flagKey="score_momentum"
            onToggle={handleToggle}
          />
          <FeatureFlagToggle
            label="OFIP (Order Flow)"
            description="Block BUY on persistent sell-side orderbook imbalance"
            enabled={flags.ofip}
            flagKey="ofip"
            onToggle={handleToggle}
          />
          <FeatureFlagToggle
            label="Adaptive Threshold"
            description="Adjust BUY threshold based on regime + volatility"
            enabled={flags.adaptive_threshold}
            flagKey="adaptive_threshold"
            onToggle={handleToggle}
          />
          <FeatureFlagToggle
            label="Entropy Graduated"
            description="Gradually reduce position size as market noise increases"
            enabled={flags.entropy_graduated}
            flagKey="entropy_graduated"
            onToggle={handleToggle}
          />
          {/* Phase 2 */}
          <div className="mt-2 pt-2 border-t border-panel-border/50">
            <div className="text-[10px] font-medium text-blue/80 uppercase tracking-wider mb-1">
              Phase 2 — Offensive
            </div>
          </div>
          <FeatureFlagToggle
            label="CUSUM Detector"
            description="Detect trend reversals 5-15 min before EMA crossover"
            enabled={flags.cusum}
            flagKey="cusum"
            onToggle={handleToggle}
          />
          <FeatureFlagToggle
            label="Absorption Detector"
            description="Detect institutional accumulation/distribution + CVD flip"
            enabled={flags.absorption}
            flagKey="absorption"
            onToggle={handleToggle}
          />
          <FeatureFlagToggle
            label="Entropy Valley"
            description="Confidence boost when entropy drops from chaos to order"
            enabled={flags.entropy_valley}
            flagKey="entropy_valley"
            onToggle={handleToggle}
          />
        </div>
      )}

      {/* Latest Smart Filter Verdicts */}
      {latestWithFilters?.smart_filters && (
        <div className="space-y-1">
          <div className="text-[10px] font-medium text-text-muted uppercase tracking-wider">
            Latest Verdicts ({latestWithFilters.symbol})
          </div>
          <div className="p-2 rounded bg-bg-secondary/50 border border-panel-border">
            <SmartFilterVerdictsDisplay sf={latestWithFilters.smart_filters} />
          </div>
        </div>
      )}
    </div>
  )
}
