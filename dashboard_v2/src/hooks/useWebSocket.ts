/**
 * useWebSocket — Singleton WebSocket connection
 * 
 * CRITICAL FIX: This module ensures only ONE WebSocket connection exists
 * for the entire application, regardless of how many components use it.
 * 
 * Previous bug: Each component calling useTruthState() → useWebSocket()
 * created its own WebSocket = 30+ simultaneous connections flooding the server.
 * 
 * Now: A single shared connection is managed at module scope.
 * All hook consumers share the same state via listeners.
 */

import { useEffect, useState, useCallback } from 'react'
import { stateWebSocketUrl, type StateSnapshot } from '../lib/api'

export interface WsMessage {
  type: 'snapshot' | 'tick' | 'event'
  state_version: number
  ws_sequence_number?: number
  timestamp: number
  payload?: StateSnapshot
  event_type?: string
  data?: unknown
}

export interface UseWebSocketResult {
  state: StateSnapshot | null
  connected: boolean
  error: string | null
  lastEvent: WsMessage | null
  reconnect: () => void
}

// ═══ Module-level singleton state ═══
// Only ONE WebSocket connection exists regardless of how many components use this hook

type Listener = () => void

let sharedWs: WebSocket | null = null
let sharedState: StateSnapshot | null = null
let sharedConnected = false
let sharedError: string | null = null
let sharedLastEvent: WsMessage | null = null
let reconnectTimer: ReturnType<typeof setTimeout> | null = null
let reconnectDelay = 1000
let subscriberCount = 0
let listeners: Set<Listener> = new Set()
let intentionalClose = false

const MAX_RECONNECT_DELAY = 30000
const INITIAL_DELAY = 1000
const PING_INTERVAL_MS = 25000 // Send ping every 25s to keep alive

let pingTimer: ReturnType<typeof setInterval> | null = null

function notifyListeners() {
  listeners.forEach(fn => fn())
}

function cleanupWs() {
  if (pingTimer) {
    clearInterval(pingTimer)
    pingTimer = null
  }
  if (sharedWs) {
    // Remove all handlers BEFORE closing to prevent onclose from triggering reconnect
    sharedWs.onopen = null
    sharedWs.onmessage = null
    sharedWs.onclose = null
    sharedWs.onerror = null
    sharedWs.close()
    sharedWs = null
  }
  if (reconnectTimer) {
    clearTimeout(reconnectTimer)
    reconnectTimer = null
  }
}

function scheduleReconnect() {
  if (reconnectTimer) return // Already scheduled
  if (subscriberCount <= 0) return // No subscribers, don't reconnect

  reconnectTimer = setTimeout(() => {
    reconnectTimer = null
    reconnectDelay = Math.min(reconnectDelay * 2, MAX_RECONNECT_DELAY)
    doConnect()
  }, reconnectDelay)
}

function doConnect() {
  // Clean up any existing connection
  cleanupWs()
  intentionalClose = false

  const url = stateWebSocketUrl()
  const ws = new WebSocket(url)
  sharedWs = ws

  ws.onopen = () => {
    sharedConnected = true
    sharedError = null
    reconnectDelay = INITIAL_DELAY
    notifyListeners()

    // Start ping timer to keep connection alive
    if (pingTimer) clearInterval(pingTimer)
    pingTimer = setInterval(() => {
      if (ws.readyState === WebSocket.OPEN) {
        ws.send('ping')
      }
    }, PING_INTERVAL_MS)
  }

  ws.onmessage = (event) => {
    try {
      const data = event.data as string
      if (data === 'pong') return // Server pong response, ignore

      const msg: WsMessage = JSON.parse(data)
      if ((msg.type === 'snapshot' || msg.type === 'tick') && msg.payload) {
        sharedState = msg.payload
      }
      if (msg.type === 'event') {
        sharedLastEvent = msg
      }
      notifyListeners()
    } catch {
      // Ignore non-JSON messages
    }
  }

  ws.onclose = () => {
    sharedConnected = false
    sharedWs = null
    if (pingTimer) {
      clearInterval(pingTimer)
      pingTimer = null
    }
    notifyListeners()

    // Only reconnect if not intentionally closed and there are subscribers
    if (!intentionalClose && subscriberCount > 0) {
      scheduleReconnect()
    }
  }

  ws.onerror = () => {
    sharedError = 'WebSocket connection error'
    notifyListeners()
    // Don't call ws.close() here — onclose will fire automatically after onerror
  }
}

function manualReconnect() {
  reconnectDelay = INITIAL_DELAY
  doConnect()
}

function subscribe(listener: Listener) {
  listeners.add(listener)
  subscriberCount++

  // First subscriber starts the connection
  if (subscriberCount === 1 && !sharedWs) {
    doConnect()
  }

  return () => {
    listeners.delete(listener)
    subscriberCount--

    // Last subscriber — close the connection
    if (subscriberCount <= 0) {
      subscriberCount = 0
      intentionalClose = true
      cleanupWs()
      sharedState = null
      sharedConnected = false
      sharedError = null
      sharedLastEvent = null
    }
  }
}

// ═══ React Hook ═══

export function useWebSocket(): UseWebSocketResult {
  const [, forceUpdate] = useState(0)

  useEffect(() => {
    const unsubscribe = subscribe(() => {
      forceUpdate(n => n + 1)
    })
    return unsubscribe
  }, [])

  const reconnect = useCallback(() => {
    manualReconnect()
  }, [])

  return {
    state: sharedState,
    connected: sharedConnected,
    error: sharedError,
    lastEvent: sharedLastEvent,
    reconnect,
  }
}
