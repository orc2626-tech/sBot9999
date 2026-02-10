import { useState, useEffect } from 'react';
import { binanceService } from '../services/binance';
import { useStore } from '../store/useStore';
import './TradePanel.css';

type OrderType = 'LIMIT' | 'MARKET' | 'STOP_LIMIT';
type OrderSide = 'BUY' | 'SELL';

export default function TradePanel() {
  const { selectedSymbol } = useStore();
  const [orderType, setOrderType] = useState<OrderType>('LIMIT');
  const [orderSide, setOrderSide] = useState<OrderSide>('BUY');
  const [price, setPrice] = useState('');
  const [stopPrice, setStopPrice] = useState('');
  const [amount, setAmount] = useState('');
  const [total, setTotal] = useState('');
  const [currentPrice, setCurrentPrice] = useState(0);

  useEffect(() => {
    binanceService.getTicker(selectedSymbol).then((ticker) => {
      if (ticker) {
        const tickerPrice = parseFloat(ticker.price);
        setCurrentPrice(tickerPrice);
        if (!price) {
          setPrice(tickerPrice.toFixed(2));
        }
      }
    });

    const unsubscribe = binanceService.subscribeToTicker(selectedSymbol, (ticker) => {
      const tickerPrice = parseFloat(ticker.price);
      setCurrentPrice(tickerPrice);
    });

    return () => unsubscribe();
  }, [selectedSymbol]);

  useEffect(() => {
    if (price && amount) {
      setTotal((parseFloat(price) * parseFloat(amount)).toFixed(2));
    } else if (total && price) {
      setAmount((parseFloat(total) / parseFloat(price)).toFixed(6));
    }
  }, [price, amount]);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    console.log('Order submitted:', {
      symbol: selectedSymbol,
      type: orderType,
      side: orderSide,
      price,
      stopPrice,
      amount,
      total,
    });
    alert('Order placed successfully! (Demo mode)');
  };

  const setPercentage = (percent: number) => {
    const mockBalance = 10000;
    const calculatedTotal = (mockBalance * percent) / 100;
    setTotal(calculatedTotal.toFixed(2));
    if (price) {
      setAmount((calculatedTotal / parseFloat(price)).toFixed(6));
    }
  };

  return (
    <div className="trade-panel">
      <div className="trade-tabs">
        <button
          className={`trade-tab ${orderSide === 'BUY' ? 'active buy' : ''}`}
          onClick={() => setOrderSide('BUY')}
        >
          Buy
        </button>
        <button
          className={`trade-tab ${orderSide === 'SELL' ? 'active sell' : ''}`}
          onClick={() => setOrderSide('SELL')}
        >
          Sell
        </button>
      </div>

      <div className="order-type-selector">
        {(['LIMIT', 'MARKET', 'STOP_LIMIT'] as OrderType[]).map((type) => (
          <button
            key={type}
            className={`order-type-btn ${orderType === type ? 'active' : ''}`}
            onClick={() => setOrderType(type)}
          >
            {type.replace('_', ' ')}
          </button>
        ))}
      </div>

      <form onSubmit={handleSubmit} className="trade-form">
        <div className="current-price">
          <span className="label">Current Price</span>
          <span className="value">${currentPrice.toFixed(2)}</span>
        </div>

        {orderType !== 'MARKET' && (
          <div className="form-group">
            <label>Price (USDT)</label>
            <input
              type="number"
              step="0.01"
              value={price}
              onChange={(e) => setPrice(e.target.value)}
              placeholder="0.00"
              required
            />
          </div>
        )}

        {orderType === 'STOP_LIMIT' && (
          <div className="form-group">
            <label>Stop Price (USDT)</label>
            <input
              type="number"
              step="0.01"
              value={stopPrice}
              onChange={(e) => setStopPrice(e.target.value)}
              placeholder="0.00"
              required
            />
          </div>
        )}

        <div className="form-group">
          <label>Amount</label>
          <input
            type="number"
            step="0.000001"
            value={amount}
            onChange={(e) => setAmount(e.target.value)}
            placeholder="0.000000"
            required
          />
        </div>

        <div className="percentage-buttons">
          {[25, 50, 75, 100].map((percent) => (
            <button
              key={percent}
              type="button"
              className="percent-btn"
              onClick={() => setPercentage(percent)}
            >
              {percent}%
            </button>
          ))}
        </div>

        <div className="form-group">
          <label>Total (USDT)</label>
          <input
            type="number"
            step="0.01"
            value={total}
            onChange={(e) => setTotal(e.target.value)}
            placeholder="0.00"
          />
        </div>

        <button type="submit" className={`submit-btn ${orderSide.toLowerCase()}`}>
          {orderSide} {selectedSymbol.replace('USDT', '')}
        </button>

        <div className="balance-info">
          <div className="balance-row">
            <span>Available Balance</span>
            <span className="balance-value">10,000.00 USDT</span>
          </div>
        </div>
      </form>
    </div>
  );
}
