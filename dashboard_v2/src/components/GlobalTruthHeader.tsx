import { NavLink } from 'react-router-dom'
import { Wifi, RefreshCw, Shield, AlertTriangle, Activity, Zap } from 'lucide-react'
import { useTruthState } from '../hooks/useTruthState'

const links = [
  { to: '/', label: 'Overview' },
  { to: '/live', label: 'Live' },
  { to: '/strategy', label: 'Strategy' },
  { to: '/insurance', label: 'Insurance' },
  { to: '/risk', label: 'Risk' },
  { to: '/positions', label: 'Positions' },
  { to: '/execution', label: 'Execution' },
  { to: '/ladder', label: 'Ladder' },
  { to: '/ai', label: 'AI' },
  { to: '/diagnostics', label: 'Diagnostics' },
  { to: '/control', label: 'Control' },
]

export default function GlobalTruthHeader() {
  const { state, error, connected } = useTruthState()
  const t = state?.truth
  const systemOk = !error && (t?.no_go_reason == null || t.no_go_reason === '')
  const wsConnected = connected
  const reconOk = t ? (t.reconcile_last_error == null || t.reconcile_last_error === '') : true
  const accountMode = (state?.runtime_config?.account_mode ?? 'demo').toLowerCase()
  const accountLabel = accountMode === 'live' ? 'LIVE' : 'DEMO'
  const tradingState = t?.trading_mode ?? 'Paused'
  const riskMode = (t?.risk_mode && t.risk_mode !== '') ? t.risk_mode : 'NORMAL'
  const stateVer = t?.state_version ?? 0
  const seq = t?.ws_sequence_number ?? 0

  return (
    <header className="sticky top-0 z-50 border-b border-panel-border/60 bg-[#0a0e17]/95 backdrop-blur-xl">
      {error && (
        <div className="px-4 py-1.5 bg-red/10 border-b border-red/30 flex items-center gap-2 text-red text-sm">
          <AlertTriangle className="h-4 w-4 shrink-0" />
          <span>Cannot connect to backend API</span>
        </div>
      )}
      {/* Status Bar */}
      <div className="flex items-center justify-between gap-3 px-4 py-2.5">
        <div className="flex items-center gap-5 flex-wrap">
          {/* Logo + System Status */}
          <div className="flex items-center gap-2.5">
            <Activity className="h-5 w-5 text-green" />
            <span className="font-display font-bold text-base tracking-tight text-text-primary">AURORA</span>
          </div>
          <div className={`flex items-center gap-1.5 px-2.5 py-1 rounded-lg text-sm font-medium ${
            systemOk ? 'bg-green/10 text-green' : 'bg-yellow/10 text-yellow'
          }`}>
            <span className={`status-dot ${systemOk ? 'text-green animate-glow-pulse' : 'text-red'}`} />
            {systemOk ? 'SYSTEM OK' : 'NO-GO'}
          </div>
          {/* Account Badge */}
          <span className={`px-2 py-0.5 rounded text-xs font-bold font-mono tracking-wide ${
            accountLabel === 'LIVE' ? 'bg-yellow/15 text-yellow border border-yellow/30' : 'bg-green/10 text-green border border-green/20'
          }`}>{accountLabel}</span>
          {/* Trading State */}
          <span className={`font-mono text-sm ${tradingState === 'Live' ? 'text-green' : 'text-text-secondary'}`}>
            {tradingState}
          </span>
          {/* WS */}
          <div className={`flex items-center gap-1.5 text-sm ${wsConnected ? 'text-green' : 'text-yellow'}`}>
            <Wifi className="h-3.5 w-3.5" />
            <span className="font-mono text-xs">{wsConnected ? 'SYNC' : 'OFF'}</span>
          </div>
          {/* Recon */}
          <div className={`flex items-center gap-1.5 text-sm ${reconOk ? 'text-green' : 'text-yellow'}`}>
            <RefreshCw className="h-3.5 w-3.5" />
            <span className="font-mono text-xs">{reconOk ? 'OK' : 'ERR'}</span>
          </div>
          {/* Risk */}
          <div className="flex items-center gap-1">
            <Shield className="h-3.5 w-3.5 text-text-muted" />
            <span className={`font-mono text-xs font-medium ${
              riskMode === 'NORMAL' ? 'text-green' : riskMode === 'HALT' ? 'text-red' : 'text-yellow'
            }`}>{riskMode}</span>
          </div>
          {/* Wire Latency */}
          {t?.ws_wire_latency_ema != null && (
            <div className={`flex items-center gap-1 text-sm ${
              (t.ws_wire_latency_ema ?? 0) < 100 ? 'text-green' :
              (t.ws_wire_latency_ema ?? 0) < 300 ? 'text-yellow' : 'text-red'
            }`}>
              <Zap className="h-3.5 w-3.5" />
              <span className="font-mono text-xs">{Math.round(t.ws_wire_latency_ema ?? 0)}ms</span>
            </div>
          )}
          {/* Version */}
          <span className="font-mono text-[10px] text-text-muted">v{stateVer} seq:{seq}</span>
          {/* NO-GO */}
          {t?.no_go_reason && (
            <span className="px-2 py-0.5 rounded bg-yellow/10 text-yellow text-xs font-mono border border-yellow/20">
              {t.no_go_reason}
            </span>
          )}
        </div>
        {/* Navigation */}
        <nav className="flex items-center gap-0.5">
          {links.map(({ to, label }) => (
            <NavLink
              key={to}
              to={to}
              end={to === '/'}
              className={({ isActive }) =>
                `px-2.5 py-1.5 rounded-lg text-xs font-medium transition-all duration-200 ${
                  isActive
                    ? 'text-green bg-green/10 shadow-glow'
                    : 'text-text-secondary hover:text-text-primary hover:bg-panel-hover'
                }`
              }
            >
              {label}
            </NavLink>
          ))}
        </nav>
      </div>
    </header>
  )
}
