/**
 * Signal Waterfall Chart — Alpha Attribution
 * =============================================
 * This is what Goldman Sachs and Two Sigma use internally.
 * Shows how each signal CONTRIBUTES to the final score as a waterfall/bridge.
 * Green bars push the score up, red bars push it down.
 * The visual flow tells you instantly WHY the bot decided what it decided.
 */
import { BarChart, Bar, XAxis, YAxis, Tooltip, ResponsiveContainer, Cell, ReferenceLine } from 'recharts'
import { useTruthState } from '../hooks/useTruthState'
import { GitBranch } from 'lucide-react'

export default function SignalWaterfall() {
  const { state } = useTruthState()
  const scoring = state?.scoring
  if (!scoring?.signal_contributions?.length) return null

  const signals = scoring.signal_contributions as Array<{
    name: string; contribution: number; weight: number; confidence: number; direction: number
  }>

  // Build waterfall data: each bar starts where the previous ended
  let cumulative = 0
  const data = signals
    .filter(s => Math.abs(s.contribution) > 0.001)
    .sort((a, b) => Math.abs(b.contribution) - Math.abs(a.contribution))
    .map(s => {
      const start = cumulative
      cumulative += s.contribution
      return {
        name: s.name,
        contribution: s.contribution,
        start,
        end: cumulative,
        // For stacked bar: invisible base + visible bar
        base: Math.min(start, cumulative),
        height: Math.abs(s.contribution),
      }
    })

  // Add total bar
  data.push({
    name: 'TOTAL',
    contribution: scoring.total_score,
    start: 0,
    end: scoring.total_score,
    base: Math.min(0, scoring.total_score),
    height: Math.abs(scoring.total_score),
  })

  const decision = scoring.decision ?? 'NEUTRAL'
  const decColor = decision === 'BUY' ? '#10b981' : decision === 'SELL' ? '#ef4444' : '#64748b'

  return (
    <div className="panel p-5">
      <div className="flex items-center justify-between mb-4">
        <h3 className="section-title flex items-center gap-2">
          <GitBranch className="h-4 w-4 text-purple" />
          Alpha Attribution
        </h3>
        <div className="flex items-center gap-2">
          <span className="mono-value text-lg font-bold" style={{ color: decColor }}>
            {scoring.total_score > 0 ? '+' : ''}{scoring.total_score.toFixed(3)}
          </span>
          <span className="text-[10px] font-mono px-2 py-0.5 rounded-full font-bold"
            style={{ color: decColor, background: `${decColor}15`, border: `1px solid ${decColor}30` }}>
            {decision}
          </span>
        </div>
      </div>

      <ResponsiveContainer width="100%" height={160}>
        <BarChart data={data} margin={{ top: 5, right: 5, bottom: 5, left: -10 }}>
          <XAxis dataKey="name" tick={{ fill: '#64748b', fontSize: 9 }} axisLine={{ stroke: '#1e3a5f' }} />
          <YAxis tick={{ fill: '#64748b', fontSize: 9 }} axisLine={{ stroke: '#1e3a5f' }}
            tickFormatter={(v: number) => v.toFixed(2)} domain={['auto', 'auto']} />
          <Tooltip
            contentStyle={{ background: '#111827', border: '1px solid #1e3a5f', borderRadius: 8, fontSize: 11, fontFamily: 'JetBrains Mono' }}
            formatter={(value: number, name: string) => {
              if (name === 'base') return [null, null]
              return [`${value > 0 ? '+' : ''}${value.toFixed(4)}`, 'Contribution']
            }}
          />
          <ReferenceLine y={0} stroke="#1e3a5f" strokeDasharray="3 3" />
          {/* Invisible base bar */}
          <Bar dataKey="base" stackId="a" fill="transparent" />
          {/* Visible contribution bar */}
          <Bar dataKey="height" stackId="a" radius={[3, 3, 0, 0]}>
            {data.map((d, i) => (
              <Cell key={i} fill={d.name === 'TOTAL' ? decColor : d.contribution >= 0 ? '#10b981' : '#ef4444'}
                fillOpacity={d.name === 'TOTAL' ? 1 : 0.8} />
            ))}
          </Bar>
        </BarChart>
      </ResponsiveContainer>
    </div>
  )
}
