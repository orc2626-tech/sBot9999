/**
 * useTruthState v3 — Uses SHARED singleton WebSocket (no duplicate connections)
 * 
 * All components calling useTruthState() share the SAME WebSocket connection.
 * Previous versions created a new WebSocket per component (30+ connections!).
 * 
 * Fallback: polls REST API when WebSocket is disconnected.
 */

import { useEffect, useState } from 'react'
import { fetchState, sendHeartbeat, type StateSnapshot } from '../lib/api'
import { useWebSocket } from './useWebSocket'

const POLL_FALLBACK_MS = 5000
const HEARTBEAT_MS = 5 * 60 * 1000 // 5 min — Dead Man's Switch

export function useTruthState(): {
  state: StateSnapshot | null
  error: string | null
  loading: boolean
  connected: boolean
  refetch: () => void
} {
  const ws = useWebSocket()
  const [pollState, setPollState] = useState<StateSnapshot | null>(null)
  const [pollError, setPollError] = useState<string | null>(null)
  const [loading, setLoading] = useState(true)

  // Polling fallback only when WS is disconnected
  useEffect(() => {
    if (ws.connected) {
      setLoading(false)
      return
    }
    let cancelled = false
    const poll = () => {
      if (cancelled) return
      fetchState()
        .then((s) => {
          if (!cancelled) {
            setPollState(s)
            setPollError(null)
          }
        })
        .catch((e) => {
          if (!cancelled) setPollError(e instanceof Error ? e.message : String(e))
        })
        .finally(() => {
          if (!cancelled) setLoading(false)
        })
    }
    poll()
    const t = setInterval(poll, POLL_FALLBACK_MS)
    return () => {
      cancelled = true
      clearInterval(t)
    }
  }, [ws.connected])

  // Use WS state when available, fallback to poll
  const state = ws.state ?? pollState
  const error = ws.error ?? pollError

  const refetch = () => {
    fetchState()
      .then((s) => setPollState(s))
      .catch(() => {})
  }

  // Dead Man's Switch heartbeat
  useEffect(() => {
    const t = setInterval(() => sendHeartbeat().catch(() => {}), HEARTBEAT_MS)
    return () => clearInterval(t)
  }, [])

  return {
    state,
    error,
    loading: loading && !ws.state,
    connected: ws.connected,
    refetch,
  }
}
