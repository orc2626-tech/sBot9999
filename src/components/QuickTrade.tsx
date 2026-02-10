import { useState, useEffect } from 'react';
import { ArrowRightLeft, Zap, ChevronDown } from 'lucide-react';
import { binanceService } from '../services/binance';
import { useStore } from '../store/useStore';
import './QuickTrade.css';

const QUICK_PAIRS = ['BTCUSDT', 'ETHUSDT', 'BNBUSDT', 'SOLUSDT', 'XRPUSDT'];

export default function QuickTrade() {
  const { setSelectedSymbol } = useStore();
  const [side, setSide] = useState<'BUY' | 'SELL'>('BUY');
  const [selectedPair, setSelectedPair] = useState('BTCUSDT');
  const [amount, setAmount] = useState('');
  const [currentPrice, setCurrentPrice] = useState<number>(0);
  const [showPairDropdown, setShowPairDropdown] = useState(false);

  useEffect(() => {
    binanceService.getTicker(selectedPair).then((ticker) => {
      if (ticker) {
        setCurrentPrice(parseFloat(ticker.price));
      }
    });
  }, [selectedPair]);

  const formatSymbol = (symbol: string) => symbol.replace('USDT', '/USDT');

  const estimatedTotal = amount ? parseFloat(amount) * currentPrice : 0;

  const handleTrade = () => {
    setSelectedSymbol(selectedPair);
    alert(`Demo: ${side} order for ${amount} ${selectedPair.replace('USDT', '')} at $${currentPrice.toLocaleString('en-US', { minimumFractionDigits: 2 })}`);
  };

  const QUICK_AMOUNTS = ['25%', '50%', '75%', '100%'];

  return (
    <div className="quick-trade-card">
      <div className="quick-trade-header">
        <h3>
          <Zap size={18} />
          Quick Trade
        </h3>
      </div>

      <div className="quick-trade-body">
        <div className="side-toggle">
          <button
            className={`side-btn ${side === 'BUY' ? 'active buy' : ''}`}
            onClick={() => setSide('BUY')}
          >
            Buy
          </button>
          <button
            className={`side-btn ${side === 'SELL' ? 'active sell' : ''}`}
            onClick={() => setSide('SELL')}
          >
            Sell
          </button>
        </div>

        <div className="pair-selector" onClick={() => setShowPairDropdown(!showPairDropdown)}>
          <span className="pair-label">Pair</span>
          <div className="pair-selected">
            <span>{formatSymbol(selectedPair)}</span>
            <ChevronDown size={16} />
          </div>
          {showPairDropdown && (
            <div className="pair-dropdown">
              {QUICK_PAIRS.map((pair) => (
                <button
                  key={pair}
                  className={pair === selectedPair ? 'active' : ''}
                  onClick={(e) => {
                    e.stopPropagation();
                    setSelectedPair(pair);
                    setShowPairDropdown(false);
                  }}
                >
                  {formatSymbol(pair)}
                </button>
              ))}
            </div>
          )}
        </div>

        <div className="price-display">
          <span className="price-label">Market Price</span>
          <span className="price-value">
            ${currentPrice.toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 })}
          </span>
        </div>

        <div className="amount-input-group">
          <label>Amount ({selectedPair.replace('USDT', '')})</label>
          <div className="amount-input-wrapper">
            <input
              type="number"
              placeholder="0.00"
              value={amount}
              onChange={(e) => setAmount(e.target.value)}
            />
          </div>
          <div className="quick-amounts">
            {QUICK_AMOUNTS.map((pct) => (
              <button key={pct} onClick={() => setAmount('0.01')}>
                {pct}
              </button>
            ))}
          </div>
        </div>

        {amount && (
          <div className="estimated-total">
            <span>Estimated Total</span>
            <span className="total-value">
              <ArrowRightLeft size={14} />
              ${estimatedTotal.toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 })} USDT
            </span>
          </div>
        )}

        <button
          className={`trade-submit-btn ${side === 'BUY' ? 'buy' : 'sell'}`}
          onClick={handleTrade}
          disabled={!amount || parseFloat(amount) <= 0}
        >
          {side === 'BUY' ? 'Buy' : 'Sell'} {selectedPair.replace('USDT', '')}
        </button>
      </div>
    </div>
  );
}
