/**
 * Binance Twin State — Exchange vs Local sync from reconcile (real data).
 */
import { Copy, CheckCircle, AlertCircle } from 'lucide-react'
import { useTruthState } from '../hooks/useTruthState'

export default function TwinStatePanel() {
  const { state } = useTruthState()
  const truth = state?.truth
  const hasError = truth?.reconcile_last_error != null && truth.reconcile_last_error !== ''
  const balances = state?.balances ?? []
  const hasBalanceData = balances.length > 0
  const acct = state?.binance_account ?? null
  const md = state?.market_data?.symbols ?? {}

  const rows = balances
    .map((b) => {
      const free = parseFloat(b.free) || 0
      const locked = parseFloat(b.locked) || 0
      const total = free + locked
      const price = b.asset === 'USDT' ? 1 : md[`${b.asset}USDT`]?.last_price
      const value = price ? total * price : null
      return { asset: b.asset, free, locked, total, value }
    })
    .filter((r) => r.total > 0)
    .sort((a, b) => (b.value ?? 0) - (a.value ?? 0))
    .slice(0, 12)

  return (
    <div className="panel p-4">
      <h3 className="text-sm font-semibold text-slate-400 uppercase tracking-wide mb-3">Binance Twin State</h3>
      <div className="space-y-2 text-sm">
        <div className="flex items-center gap-2 text-slate-300">
          <Copy className="h-4 w-4" />
          Exchange vs Local:{' '}
          <span className={`font-mono ${hasError ? 'text-red' : 'text-green'}`}>
            {hasError ? 'Drift / Error' : 'Synced'}
          </span>
        </div>
        <div className="flex items-center gap-2 pt-2 border-t border-panelBorder">
          {hasError ? (
            <>
              <AlertCircle className="h-4 w-4 text-red shrink-0" />
              <div>
                <span className="text-slate-300">Drift / Last error:</span>
                <span className="font-mono text-red ml-1 block truncate max-w-[200px]" title={truth?.reconcile_last_error ?? ''}>
                  {truth?.reconcile_last_error ?? '—'}
                </span>
              </div>
            </>
          ) : (
            <>
              <CheckCircle className="h-4 w-4 text-green shrink-0" />
              <span className="text-slate-300">Drift Status:</span>
              <span className="font-mono text-green font-medium">NONE DETECTED</span>
            </>
          )}
        </div>
      </div>
      {acct && (
        <div className="mt-3 pt-2 border-t border-panelBorder text-xs font-mono text-slate-400 flex gap-4 flex-wrap">
          <span>
            canTrade: <span className={acct.can_trade ? 'text-green' : 'text-red'}>{acct.can_trade ? 'true' : 'false'}</span>
          </span>
          <span>
            canWithdraw: <span className={acct.can_withdraw ? 'text-green' : 'text-red'}>{acct.can_withdraw ? 'true' : 'false'}</span>
          </span>
        </div>
      )}
      {rows.length > 0 && (
        <div className="mt-3 pt-2 border-t border-panelBorder">
          <div className="text-xs text-slate-500 font-mono mb-2">Top balances (from Binance)</div>
          <div className="overflow-x-auto">
            <table className="w-full text-xs font-mono">
              <thead>
                <tr className="text-slate-500 border-b border-panelBorder/70">
                  <th className="text-left py-1 font-medium">Asset</th>
                  <th className="text-right py-1 font-medium">Free</th>
                  <th className="text-right py-1 font-medium">Locked</th>
                  <th className="text-right py-1 font-medium">Value USDT</th>
                </tr>
              </thead>
              <tbody>
                {rows.map((r) => (
                  <tr key={r.asset} className="border-b border-panelBorder/40">
                    <td className="py-1 text-slate-200">{r.asset}</td>
                    <td className="py-1 text-right text-slate-300">{r.free.toFixed(8)}</td>
                    <td className="py-1 text-right text-slate-400">{r.locked.toFixed(8)}</td>
                    <td className="py-1 text-right text-slate-200">
                      {r.value == null ? '—' : r.value.toFixed(2)}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}
      <div className="mt-3 pt-2 border-t border-panelBorder text-xs text-slate-500">
        {hasBalanceData
          ? 'Balance data from Binance (reconcile + user stream).'
          : 'No balance data — set BINANCE_API_KEY and BINANCE_SECRET in backend .env and restart bot to sync from Binance.'}
      </div>
    </div>
  )
}
