import { useEffect, useState } from 'react';
import { binanceService } from '../services/binance';
import { useStore } from '../store/useStore';
import TradingChart from '../components/TradingChart';
import OrderBook from '../components/OrderBook';
import TradePanel from '../components/TradePanel';
import MarketWatch from '../components/MarketWatch';
import { TrendingUp, TrendingDown } from 'lucide-react';
import './Trading.css';

export default function Trading() {
  const { selectedSymbol } = useStore();
  const [ticker, setTicker] = useState<any>(null);

  useEffect(() => {
    binanceService.getTicker(selectedSymbol).then((data) => {
      if (data) setTicker(data);
    });

    const unsubscribe = binanceService.subscribeToTicker(selectedSymbol, (data) => {
      setTicker(data);
    });

    return () => unsubscribe();
  }, [selectedSymbol]);

  const formatNumber = (num: string) => {
    return parseFloat(num).toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 });
  };

  const priceChange = ticker ? parseFloat(ticker.priceChangePercent) : 0;
  const isPositive = priceChange >= 0;

  return (
    <div className="trading-page">
      <div className="trading-header">
        <div className="symbol-info">
          <h1>{selectedSymbol.replace('USDT', '/USDT')}</h1>
          {ticker && (
            <>
              <div className="price-display">
                <span className={`current-price ${isPositive ? 'positive' : 'negative'}`}>
                  ${formatNumber(ticker.price)}
                </span>
                <span className={`price-change ${isPositive ? 'positive' : 'negative'}`}>
                  {isPositive ? <TrendingUp size={16} /> : <TrendingDown size={16} />}
                  {isPositive ? '+' : ''}{priceChange.toFixed(2)}%
                </span>
              </div>
            </>
          )}
        </div>

        {ticker && (
          <div className="market-stats-row">
            <div className="stat-item">
              <span className="label">24h High</span>
              <span className="value">${formatNumber(ticker.high)}</span>
            </div>
            <div className="stat-item">
              <span className="label">24h Low</span>
              <span className="value">${formatNumber(ticker.low)}</span>
            </div>
            <div className="stat-item">
              <span className="label">24h Volume</span>
              <span className="value">{parseFloat(ticker.volume).toLocaleString('en-US', { maximumFractionDigits: 0 })}</span>
            </div>
          </div>
        )}
      </div>

      <div className="trading-layout">
        <div className="left-panel">
          <MarketWatch />
        </div>

        <div className="center-panel">
          <div className="chart-section">
            <TradingChart />
          </div>

          <div className="open-orders-section">
            <div className="orders-header">
              <h3>Open Orders</h3>
              <div className="orders-tabs">
                <button className="orders-tab active">Open Orders (0)</button>
                <button className="orders-tab">Order History</button>
                <button className="orders-tab">Trade History</button>
              </div>
            </div>
            <div className="orders-content">
              <div className="empty-state">
                <p>No open orders</p>
                <span>Your active orders will appear here</span>
              </div>
            </div>
          </div>
        </div>

        <div className="right-panel">
          <div className="panel-section">
            <OrderBook />
          </div>
          <div className="panel-section">
            <TradePanel />
          </div>
        </div>
      </div>
    </div>
  );
}
