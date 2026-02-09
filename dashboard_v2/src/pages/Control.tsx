import { useState } from 'react'
import { Shield, AlertTriangle, Wallet } from 'lucide-react'
import SymbolsTickerRow from '../components/SymbolsTickerRow'
import { useTruthState } from '../hooks/useTruthState'
import { controlPause, controlResume, controlKill, setAccountMode } from '../lib/api'

export default function Control() {
  const { state, refetch } = useTruthState()
  const [busy, setBusy] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [showLiveConfirm, setShowLiveConfirm] = useState(false)

  const accountMode = (state?.runtime_config?.account_mode ?? 'demo').toLowerCase()
  const isDemo = accountMode === 'demo'

  const run = async (label: string, fn: () => Promise<unknown>) => {
    setBusy(label)
    setError(null)
    try {
      await fn()
      refetch()
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e))
    } finally {
      setBusy(null)
    }
  }

  const handleSwitchToDemo = () => {
    run('Switch to Demo', () => setAccountMode('demo'))
  }

  const handleSwitchToLive = () => {
    setShowLiveConfirm(true)
  }

  const confirmSwitchToLive = () => {
    setShowLiveConfirm(false)
    run('Switch to Live', () => setAccountMode('live', true))
  }

  return (
    <div className="space-y-4">
      <div className="panel p-3 border-yellow/30 bg-yellow/5 flex items-start gap-3">
        <Shield className="h-5 w-5 text-yellow shrink-0 mt-0.5" />
        <div>
          <h2 className="text-lg font-semibold text-slate-200">Control Panel (Admin)</h2>
          <p className="text-sm text-slate-400 mt-0.5">
            Runtime config, Pause/Resume/Kill, and <strong>Account Mode</strong> (Demo vs Live).
          </p>
        </div>
      </div>

      {/* Account Mode — تجريبي / حقيقي */}
      <div className="panel p-4 border-blue/30 bg-blue/5">
        <h3 className="text-sm font-semibold text-slate-300 uppercase tracking-wide mb-3 flex items-center gap-2">
          <Wallet className="h-4 w-4" />
          Account Mode — وضع الحساب
        </h3>
        <p className="text-sm text-slate-400 mb-4">
          <strong>Demo (تجريبي):</strong> أوامر ورقية — لا أموال حقيقية. &nbsp;
          <strong>Live (حقيقي):</strong> حساب Binance الحقيقي — أموال حقيقية.
        </p>
        <div className="flex items-center gap-4 flex-wrap">
          <div className="flex items-center gap-2">
            <span className={`font-mono font-semibold ${isDemo ? 'text-green' : 'text-slate-400'}`}>
              {isDemo ? '●' : '○'} Demo (Paper)
            </span>
            <button
              disabled={isDemo || busy !== null}
              onClick={handleSwitchToDemo}
              className="px-3 py-1.5 rounded bg-slate-600 text-slate-200 text-sm font-medium hover:bg-slate-500 disabled:opacity-50 disabled:cursor-not-allowed"
            >
              Switch to Demo
            </button>
          </div>
          <div className="flex items-center gap-2">
            <span className={`font-mono font-semibold ${!isDemo ? 'text-red' : 'text-slate-400'}`}>
              {!isDemo ? '●' : '○'} Live (Real)
            </span>
            <button
              disabled={!isDemo || busy !== null}
              onClick={handleSwitchToLive}
              className="px-3 py-1.5 rounded bg-red/20 text-red border border-red/50 text-sm font-medium hover:bg-red/30 disabled:opacity-50 disabled:cursor-not-allowed"
            >
              Switch to Live
            </button>
          </div>
        </div>
        {busy === 'Switch to Live' && <p className="text-xs text-slate-500 mt-2">Processing…</p>}
      </div>

      {/* Confirmation modal for Live */}
      {showLiveConfirm && (
        <div className="fixed inset-0 z-[100] flex items-center justify-center bg-black/70 p-4">
          <div className="panel p-6 max-w-md w-full border-red/50">
            <div className="flex items-center gap-2 text-red mb-3">
              <AlertTriangle className="h-5 w-5" />
              <span className="font-semibold">Switch to REAL account?</span>
            </div>
            <p className="text-sm text-slate-300 mb-4">
              You are about to switch to <strong>Live (Real)</strong>. The bot will use your real Binance account. Real money will be at risk. Make sure you have set API keys and understand the risks.
            </p>
            <div className="flex gap-2">
              <button
                onClick={confirmSwitchToLive}
                className="px-4 py-2 rounded bg-red/30 text-red border border-red/50 font-medium hover:bg-red/40"
              >
                Yes, switch to Live
              </button>
              <button
                onClick={() => setShowLiveConfirm(false)}
                className="px-4 py-2 rounded bg-slate-600 text-slate-200 font-medium hover:bg-slate-500"
              >
                Cancel
              </button>
            </div>
          </div>
        </div>
      )}

      {error && (
        <div className="panel p-3 border-red/50 bg-red/10 flex items-center gap-2 text-red text-sm">
          <AlertTriangle className="h-4 w-4 shrink-0" />
          {error}
        </div>
      )}

      <SymbolsTickerRow />
      <div className="grid grid-cols-12 gap-4">
        <div className="col-span-6 space-y-4">
          <div className="panel p-4">
            <h3 className="text-sm font-semibold text-slate-400 uppercase tracking-wide mb-3">Control Actions</h3>
            <div className="flex gap-2 flex-wrap">
              <button
                disabled={busy !== null}
                onClick={() => run('Pause', controlPause)}
                className="px-3 py-2 rounded bg-yellow/20 text-yellow border border-yellow/50 hover:bg-yellow/30 font-medium text-sm disabled:opacity-50"
              >
                {busy === 'Pause' ? '…' : 'Pause'}
              </button>
              <button
                disabled={busy !== null}
                onClick={() => run('Resume', controlResume)}
                className="px-3 py-2 rounded bg-green/20 text-green border border-green/50 hover:bg-green/30 font-medium text-sm disabled:opacity-50"
              >
                {busy === 'Resume' ? '…' : 'Resume'}
              </button>
              <button
                disabled={busy !== null}
                onClick={() => run('Kill', controlKill)}
                className="px-3 py-2 rounded bg-red/20 text-red border border-red/50 hover:bg-red/30 font-medium text-sm disabled:opacity-50"
              >
                {busy === 'Kill' ? '…' : 'Kill Switch'}
              </button>
            </div>
            <p className="text-xs text-slate-500 mt-2">Kill: double confirm in LIVE. Shows reason banner.</p>
          </div>
        </div>
        <div className="col-span-6 space-y-4">
          <div className="panel p-4">
            <h3 className="text-sm font-semibold text-slate-400 uppercase tracking-wide mb-3">Feature Flags</h3>
            <div className="space-y-2 text-sm">
              <div className="flex justify-between items-center"><span className="text-slate-400">Shadow Mode</span><span className="font-mono text-slate-500">OFF</span></div>
              <div className="flex justify-between items-center"><span className="text-slate-400">AI Observer</span><span className="font-mono text-green">ON</span></div>
              <div className="flex justify-between items-center"><span className="text-slate-400">Account</span><span className={`font-mono ${isDemo ? 'text-green' : 'text-red'}`}>{isDemo ? 'Demo' : 'Live'}</span></div>
            </div>
          </div>
          <div className="panel p-4 border-yellow/30 bg-yellow/5">
            <div className="flex items-center gap-2 text-yellow mb-2">
              <AlertTriangle className="h-4 w-4" />
              <span className="font-semibold">Auth</span>
            </div>
            <p className="text-sm text-slate-400">Control requires valid admin token. 403 shows banner if missing.</p>
          </div>
        </div>
      </div>
    </div>
  )
}
