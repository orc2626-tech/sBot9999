import { Outlet } from 'react-router-dom'
import GlobalTruthHeader from './GlobalTruthHeader'
import { useTruthState } from '../hooks/useTruthState'

export default function Layout() {
  const { state, connected } = useTruthState()
  const stateVer = state?.truth?.state_version ?? 0
  const riskMode = state?.truth?.risk_mode ?? '—'
  const tradingMode = state?.truth?.trading_mode ?? '—'

  return (
    <div className="min-h-screen flex flex-col bg-[#0a0e17] bg-grid-pattern">
      <GlobalTruthHeader />
      <main className="flex-1 p-4 overflow-auto animate-fade-in">
        <Outlet />
      </main>
      <footer className="border-t border-panel-border/50 bg-panel/60 backdrop-blur-md
                         px-4 py-2 flex items-center justify-between text-xs font-mono">
        <div className="flex items-center gap-4 text-text-muted">
          <span className="flex items-center gap-1.5">
            <span className={`status-dot ${connected ? 'text-green animate-glow-pulse' : 'text-red'}`} />
            {connected ? 'Live' : 'Disconnected'}
          </span>
          <span>v{stateVer}</span>
          <span>Risk: <span className={riskMode === 'NORMAL' ? 'text-green' : 'text-yellow'}>{riskMode}</span></span>
        </div>
        <div className="flex items-center gap-4 text-text-muted">
          <span className="text-text-secondary">Aurora Spot Nexus</span>
          <span>Mode: <span className={tradingMode === 'Live' ? 'text-green' : 'text-text-secondary'}>{tradingMode}</span></span>
        </div>
      </footer>
    </div>
  )
}
