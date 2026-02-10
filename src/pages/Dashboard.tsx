import { useEffect, useState } from 'react';
import { TrendingUp, TrendingDown, DollarSign, Activity, Wallet } from 'lucide-react';
import { binanceService } from '../services/binance';
import { useStore } from '../store/useStore';
import MarketWatch from '../components/MarketWatch';
import './Dashboard.css';

export default function Dashboard() {
  const { setMarketStats } = useStore();
  const [topGainers, setTopGainers] = useState<any[]>([]);
  const [topLosers, setTopLosers] = useState<any[]>([]);

  useEffect(() => {
    setMarketStats({
      totalBalance: 10000,
      totalPnL: 1250.50,
      totalPnLPercentage: 12.51,
      dayChange: 345.20,
      dayChangePercentage: 3.45,
    });

    binanceService.getAllTickers().then((tickers) => {
      const sorted = tickers
        .filter((t) => t.symbol.includes('USDT'))
        .sort((a, b) => parseFloat(b.priceChangePercent) - parseFloat(a.priceChangePercent));

      setTopGainers(sorted.slice(0, 5));
      setTopLosers(sorted.slice(-5).reverse());
    });
  }, [setMarketStats]);

  return (
    <div className="dashboard">
      <div className="dashboard-header">
        <h1>Dashboard</h1>
        <p>Welcome back to your trading platform</p>
      </div>

      <div className="stats-grid">
        <div className="stat-card">
          <div className="stat-icon wallet">
            <Wallet size={24} />
          </div>
          <div className="stat-content">
            <span className="stat-label">Total Balance</span>
            <span className="stat-value">$10,000.00</span>
            <span className="stat-change positive">+$1,250.50 (12.51%)</span>
          </div>
        </div>

        <div className="stat-card">
          <div className="stat-icon dollar">
            <DollarSign size={24} />
          </div>
          <div className="stat-content">
            <span className="stat-label">24h PnL</span>
            <span className="stat-value">+$345.20</span>
            <span className="stat-change positive">+3.45%</span>
          </div>
        </div>

        <div className="stat-card">
          <div className="stat-icon activity">
            <Activity size={24} />
          </div>
          <div className="stat-content">
            <span className="stat-label">Active Positions</span>
            <span className="stat-value">8</span>
            <span className="stat-change">3 profitable</span>
          </div>
        </div>

        <div className="stat-card">
          <div className="stat-icon trend">
            <TrendingUp size={24} />
          </div>
          <div className="stat-content">
            <span className="stat-label">Win Rate</span>
            <span className="stat-value">68.5%</span>
            <span className="stat-change positive">+2.3%</span>
          </div>
        </div>
      </div>

      <div className="dashboard-content">
        <div className="market-overview">
          <div className="section-header">
            <h2>Market Overview</h2>
          </div>

          <div className="movers-grid">
            <div className="movers-card">
              <h3>
                <TrendingUp size={18} className="icon-green" />
                Top Gainers
              </h3>
              <div className="movers-list">
                {topGainers.map((ticker) => (
                  <div key={ticker.symbol} className="mover-row">
                    <span className="mover-symbol">{ticker.symbol}</span>
                    <span className="mover-price">${parseFloat(ticker.price).toFixed(2)}</span>
                    <span className="mover-change positive">
                      +{parseFloat(ticker.priceChangePercent).toFixed(2)}%
                    </span>
                  </div>
                ))}
              </div>
            </div>

            <div className="movers-card">
              <h3>
                <TrendingDown size={18} className="icon-red" />
                Top Losers
              </h3>
              <div className="movers-list">
                {topLosers.map((ticker) => (
                  <div key={ticker.symbol} className="mover-row">
                    <span className="mover-symbol">{ticker.symbol}</span>
                    <span className="mover-price">${parseFloat(ticker.price).toFixed(2)}</span>
                    <span className="mover-change negative">
                      {parseFloat(ticker.priceChangePercent).toFixed(2)}%
                    </span>
                  </div>
                ))}
              </div>
            </div>
          </div>
        </div>

        <div className="market-watch-section">
          <MarketWatch />
        </div>
      </div>
    </div>
  );
}
