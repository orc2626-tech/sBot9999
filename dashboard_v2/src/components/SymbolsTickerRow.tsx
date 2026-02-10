/**
 * SymbolsTickerRow — أسعار حية من API
 */
import { useTruthState } from '../hooks/useTruthState'

export default function SymbolsTickerRow() {
  const { state } = useTruthState()
  const symbols = state?.runtime_config?.symbols ?? ['BTCUSDT', 'ETHUSDT', 'SOLUSDT', 'BNBUSDT', 'XRPUSDT']
  const marketData = state?.market_data?.symbols ?? {}

  return (
    <div className="grid grid-cols-5 gap-3">
      {symbols.map((sym: string) => {
        const data = marketData[sym]
        const price = data?.last_price ?? 0
        const rsi = data?.rsi_14
        const spreadBps = data?.spread_bps

        return (
          <div key={sym} className="panel p-3">
            <div className="flex justify-between items-start">
              <span className="font-mono font-semibold text-slate-200">
                {sym.replace('USDT', '/USDT')}
              </span>
              {rsi != null && (
                <span
                  className={`font-mono text-xs px-1 rounded ${
                    rsi > 70 ? 'text-red bg-red/20' : rsi < 30 ? 'text-green bg-green/20' : 'text-slate-400'
                  }`}
                >
                  RSI {rsi.toFixed(0)}
                </span>
              )}
            </div>
            <div className="font-mono text-lg text-white mt-0.5">
              {price > 0 ? price.toLocaleString(undefined, { maximumFractionDigits: 2 }) : '—'}
            </div>
            <div className="flex justify-between text-xs mt-1">
              {spreadBps != null && (
                <span
                  className={`font-mono ${
                    spreadBps < 5 ? 'text-green' : spreadBps < 15 ? 'text-yellow' : 'text-red'
                  }`}
                >
                  Spread: {spreadBps.toFixed(1)}bps
                </span>
              )}
            </div>
          </div>
        )
      })}
    </div>
  )
}
