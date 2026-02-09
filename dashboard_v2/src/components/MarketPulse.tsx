/**
 * Market Pulse — Real-time Market Heartbeat
 * =============================================
 * Inspired by Bloomberg Terminal's "Market Activity" indicator.
 * Shows the RHYTHM of the market as a pulse line.
 * Traders use this to "feel" the market tempo — fast pulse = high activity.
 * The pulse rate + amplitude tell you if the market is alive or dead.
 */
import { useEffect, useRef, useState } from 'react'
import { useTruthState } from '../hooks/useTruthState'
import { Heart } from 'lucide-react'

interface PulsePoint {
  time: number
  score: number
  vpin: number
  spread: number
}

export default function MarketPulse() {
  const { state } = useTruthState()
  const [history, setHistory] = useState<PulsePoint[]>([])
  const canvasRef = useRef<HTMLCanvasElement>(null)
  const prevVersion = useRef(0)

  // Collect data points on state changes
  useEffect(() => {
    const version = state?.truth?.state_version ?? 0
    if (version <= prevVersion.current) return
    prevVersion.current = version

    const score = state?.scoring?.total_score ?? 0
    const firstSym = state?.runtime_config?.symbols?.[0] ?? ''
    const vpin = firstSym && state?.vpin?.[firstSym] ? state.vpin[firstSym].vpin : 0
    const md = firstSym && state?.market_data?.symbols?.[firstSym] ? state.market_data.symbols[firstSym] : null
    const spread = md?.spread_bps ?? 0

    setHistory(prev => {
      const next = [...prev, { time: Date.now(), score, vpin, spread }]
      return next.slice(-80) // Keep last 80 points
    })
  }, [state?.truth?.state_version])

  // Draw pulse on canvas
  useEffect(() => {
    const canvas = canvasRef.current
    if (!canvas || history.length < 2) return
    const ctx = canvas.getContext('2d')
    if (!ctx) return

    const w = canvas.width
    const h = canvas.height
    ctx.clearRect(0, 0, w, h)

    // Grid lines
    ctx.strokeStyle = 'rgba(30,58,95,0.3)'
    ctx.lineWidth = 1
    for (let y = 0; y < h; y += h / 4) {
      ctx.beginPath(); ctx.moveTo(0, y); ctx.lineTo(w, y); ctx.stroke()
    }

    // Zero line
    ctx.strokeStyle = 'rgba(30,58,95,0.6)'
    ctx.beginPath(); ctx.moveTo(0, h / 2); ctx.lineTo(w, h / 2); ctx.stroke()

    // Score pulse line
    const step = w / (history.length - 1)
    const maxAbs = Math.max(...history.map(p => Math.abs(p.score)), 0.5)

    // Gradient fill
    const grad = ctx.createLinearGradient(0, 0, 0, h)
    grad.addColorStop(0, 'rgba(16,185,129,0.15)')
    grad.addColorStop(0.5, 'rgba(16,185,129,0)')
    grad.addColorStop(1, 'rgba(239,68,68,0.15)')

    ctx.beginPath()
    ctx.moveTo(0, h / 2)
    history.forEach((p, i) => {
      const x = i * step
      const y = h / 2 - (p.score / maxAbs) * (h / 2 - 10)
      if (i === 0) ctx.moveTo(x, y)
      else ctx.lineTo(x, y)
    })
    ctx.lineTo(w, h / 2)
    ctx.lineTo(0, h / 2)
    ctx.fillStyle = grad
    ctx.fill()

    // Score line
    ctx.beginPath()
    history.forEach((p, i) => {
      const x = i * step
      const y = h / 2 - (p.score / maxAbs) * (h / 2 - 10)
      if (i === 0) ctx.moveTo(x, y)
      else ctx.lineTo(x, y)
    })
    ctx.strokeStyle = '#10b981'
    ctx.lineWidth = 2
    ctx.shadowColor = '#10b981'
    ctx.shadowBlur = 8
    ctx.stroke()
    ctx.shadowBlur = 0

    // Last point glow
    const last = history[history.length - 1]
    const lastY = h / 2 - (last.score / maxAbs) * (h / 2 - 10)
    ctx.beginPath()
    ctx.arc(w - 1, lastY, 4, 0, Math.PI * 2)
    ctx.fillStyle = '#10b981'
    ctx.shadowColor = '#10b981'
    ctx.shadowBlur = 12
    ctx.fill()
    ctx.shadowBlur = 0
  }, [history])

  const lastScore = history.length > 0 ? history[history.length - 1].score : 0
  const bpm = Math.min(history.length * 2, 120) // "Beats per minute" — data frequency

  return (
    <div className="panel p-5">
      <div className="flex items-center justify-between mb-3">
        <h3 className="section-title flex items-center gap-2">
          <Heart className="h-4 w-4 text-red animate-pulse" />
          Market Pulse
        </h3>
        <div className="flex items-center gap-3">
          <span className="text-[10px] font-mono text-text-muted">{bpm} bpm</span>
          <span className={`mono-value text-sm font-bold ${lastScore >= 0 ? 'text-green' : 'text-red'}`}>
            {lastScore >= 0 ? '+' : ''}{lastScore.toFixed(3)}
          </span>
        </div>
      </div>
      <canvas ref={canvasRef} width={400} height={80} className="w-full h-20 rounded-lg" />
    </div>
  )
}
