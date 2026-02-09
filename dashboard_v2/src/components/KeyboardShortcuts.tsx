/**
 * Keyboard Shortcuts — Professional Trading Desk Feature
 * ========================================================
 * Professional traders NEVER use mouse for critical actions.
 * This adds Bloomberg Terminal-style keyboard shortcuts.
 * 
 * Ctrl+P = Pause trading
 * Ctrl+R = Resume trading  
 * Ctrl+K = Kill switch (with confirmation)
 * Ctrl+D = Toggle Demo/Live
 * Esc    = Close any modal
 */
import { useEffect, useState, useCallback } from 'react'
import { controlPause, controlResume, controlKill } from '../lib/api'
import { useTruthState } from '../hooks/useTruthState'
import { Keyboard, X } from 'lucide-react'

export default function KeyboardShortcuts() {
  const [showHelp, setShowHelp] = useState(false)
  const [toast, setToast] = useState<{ msg: string; type: 'ok' | 'err' } | null>(null)
  const { refetch } = useTruthState()

  const showToast = useCallback((msg: string, type: 'ok' | 'err') => {
    setToast({ msg, type })
    setTimeout(() => setToast(null), 3000)
  }, [])

  useEffect(() => {
    const handler = async (e: KeyboardEvent) => {
      // Ctrl+P = Pause
      if (e.ctrlKey && e.key === 'p') {
        e.preventDefault()
        try { await controlPause(); refetch(); showToast('Trading PAUSED', 'ok') }
        catch { showToast('Pause failed', 'err') }
      }
      // Ctrl+R = Resume
      if (e.ctrlKey && e.key === 'r') {
        e.preventDefault()
        try { await controlResume(); refetch(); showToast('Trading RESUMED', 'ok') }
        catch { showToast('Resume failed', 'err') }
      }
      // Ctrl+K = Kill (with browser confirm)
      if (e.ctrlKey && e.key === 'k') {
        e.preventDefault()
        if (window.confirm('⚠️ KILL SWITCH: Stop all trading and cancel all orders?')) {
          try { await controlKill(); refetch(); showToast('KILL activated', 'err') }
          catch { showToast('Kill failed', 'err') }
        }
      }
      // ? = Show shortcuts help
      if (e.key === '?' && !e.ctrlKey) {
        setShowHelp(p => !p)
      }
      // Esc = close help
      if (e.key === 'Escape') {
        setShowHelp(false)
      }
    }

    window.addEventListener('keydown', handler)
    return () => window.removeEventListener('keydown', handler)
  }, [refetch, showToast])

  return (
    <>
      {/* Toast notification */}
      {toast && (
        <div className={`fixed top-4 right-4 z-[100] px-4 py-2.5 rounded-lg font-mono text-sm font-medium
          shadow-lg backdrop-blur-md animate-slide-up
          ${toast.type === 'ok' ? 'bg-green/20 text-green border border-green/30' : 'bg-red/20 text-red border border-red/30'}`}>
          {toast.msg}
        </div>
      )}

      {/* Help overlay */}
      {showHelp && (
        <div className="fixed inset-0 z-[90] bg-black/60 backdrop-blur-sm flex items-center justify-center animate-fade-in"
          onClick={() => setShowHelp(false)}>
          <div className="bg-panel border border-panel-border rounded-2xl p-6 shadow-2xl max-w-md w-full mx-4"
            onClick={e => e.stopPropagation()}>
            <div className="flex items-center justify-between mb-5">
              <h2 className="flex items-center gap-2 text-text-primary font-display font-bold text-lg">
                <Keyboard className="h-5 w-5 text-purple" />
                Keyboard Shortcuts
              </h2>
              <button onClick={() => setShowHelp(false)} className="text-text-muted hover:text-text-primary">
                <X className="h-5 w-5" />
              </button>
            </div>
            <div className="space-y-3">
              {[
                { keys: 'Ctrl + P', desc: 'Pause trading', color: 'yellow' },
                { keys: 'Ctrl + R', desc: 'Resume trading', color: 'green' },
                { keys: 'Ctrl + K', desc: 'Kill switch (with confirm)', color: 'red' },
                { keys: '?', desc: 'Toggle this help', color: 'blue' },
                { keys: 'Esc', desc: 'Close overlay', color: 'text-muted' },
              ].map(({ keys, desc, color }) => (
                <div key={keys} className="flex items-center justify-between py-1">
                  <kbd className={`px-2.5 py-1 rounded-lg bg-bg-primary border border-panel-border font-mono text-sm text-${color}`}>
                    {keys}
                  </kbd>
                  <span className="text-text-secondary text-sm">{desc}</span>
                </div>
              ))}
            </div>
            <p className="text-[10px] text-text-muted mt-4 text-center">Press ? anytime to toggle this panel</p>
          </div>
        </div>
      )}
    </>
  )
}
