import type { ProfileState, RegimeStats } from '../../lib/api'

const REGIMES = ['TRENDING', 'RANGING', 'VOLATILE', 'SQUEEZE']
const REGIME_COLORS: Record<string, string> = {
  TRENDING: '#10B981',
  RANGING: '#3B82F6',
  VOLATILE: '#EF4444',
  SQUEEZE: '#F59E0B',
}

interface Props {
  profiles: ProfileState[]
}

export default function RegimeMatrix({ profiles }: Props) {
  return (
    <div className="bg-gray-900 rounded-xl border border-gray-700 p-6">
      <div className="flex items-center gap-2 mb-4">
        <span className="text-lg">📊</span>
        <h3 className="text-white font-semibold">Regime × Profile Matrix</h3>
      </div>

      <div className="overflow-x-auto">
        <table className="w-full text-sm">
          <thead>
            <tr>
              <th className="text-left text-gray-500 pb-2">Profile</th>
              {REGIMES.map((r) => (
                <th key={r} className="text-center text-gray-500 pb-2 px-3">
                  <span
                    className="px-2 py-0.5 rounded text-xs"
                    style={{ backgroundColor: REGIME_COLORS[r] + '20', color: REGIME_COLORS[r] }}
                  >
                    {r}
                  </span>
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {profiles.map((profile) => (
              <tr key={profile.name} className="border-t border-gray-800">
                <td className="py-3">
                  <span className="flex items-center gap-2">
                    <span>{profile.icon}</span>
                    <span className="text-white">{profile.name}</span>
                    {profile.is_active && <span className="w-2 h-2 rounded-full bg-emerald-500" />}
                  </span>
                </td>
                {REGIMES.map((regime) => {
                  const stats = profile.regime_performance[regime]
                  return (
                    <td key={regime} className="text-center px-3 py-3">
                      {stats && stats.trades > 0 ? (
                        <RegimeCell stats={stats} />
                      ) : (
                        <span className="text-gray-600 text-xs">—</span>
                      )}
                    </td>
                  )
                })}
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      <div className="mt-4 flex items-center gap-4 text-xs text-gray-500">
        <span>🟢 Win Rate &gt;60%</span>
        <span>🟡 Win Rate 45–60%</span>
        <span>🔴 Win Rate &lt;45%</span>
        <span>📊 Min 5 trades for color</span>
      </div>
    </div>
  )
}

function RegimeCell({ stats }: { stats: RegimeStats; profileColor?: string }) {
  const wr = stats.win_rate * 100
  const hasSufficientData = stats.trades >= 5

  let bgColor = 'bg-gray-800'
  if (hasSufficientData) {
    if (wr >= 60) bgColor = 'bg-emerald-900/40'
    else if (wr >= 45) bgColor = 'bg-yellow-900/30'
    else bgColor = 'bg-red-900/30'
  }

  const pnlColor = stats.total_pnl_pct >= 0 ? 'text-emerald-400' : 'text-red-400'

  return (
    <div className={`${bgColor} rounded-lg px-2 py-1.5`}>
      <div className="text-white font-mono text-sm">{wr.toFixed(0)}%</div>
      <div className="text-gray-400 text-xs">
        {stats.wins}/{stats.trades}
      </div>
      <div className={`text-xs font-mono ${pnlColor}`}>
        {stats.total_pnl_pct >= 0 ? '+' : ''}{stats.total_pnl_pct.toFixed(1)}%
      </div>
    </div>
  )
}
