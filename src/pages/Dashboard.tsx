import { useEffect, useState, useCallback } from 'react';
import { useNavigate } from 'react-router-dom';
import {
  TrendingUp,
  TrendingDown,
  DollarSign,
  Activity,
  Wallet,
  BarChart3,
  ArrowUpRight,
  ArrowDownRight,
  Eye,
  EyeOff,
  RefreshCw,
  Shield,
  Percent,
  Target,
} from 'lucide-react';
import { binanceService } from '../services/binance';
import { useStore } from '../store/useStore';
import PortfolioChart from '../components/PortfolioChart';
import RecentActivity from '../components/RecentActivity';
import QuickTrade from '../components/QuickTrade';
import MarketWatch from '../components/MarketWatch';
import './Dashboard.css';

interface TickerData {
  symbol: string;
  price: string;
  priceChangePercent: string;
  volume: string;
}

const POSITIONS_MOCK = [
  { symbol: 'BTC/USDT', qty: 0.15, entry: 62450, current: 67432.50, pnl: 747.38, pnlPct: 7.97 },
  { symbol: 'ETH/USDT', qty: 3.2, entry: 3420, current: 3621.80, pnl: 645.76, pnlPct: 5.90 },
  { symbol: 'SOL/USDT', qty: 25, entry: 165.20, current: 178.45, pnl: 331.25, pnlPct: 8.02 },
  { symbol: 'BNB/USDT', qty: 5, entry: 625, current: 612.30, pnl: -63.50, pnlPct: -2.03 },
  { symbol: 'XRP/USDT', qty: 1000, entry: 2.28, current: 2.34, pnl: 60.00, pnlPct: 2.63 },
];

export default function Dashboard() {
  const navigate = useNavigate();
  const { setMarketStats, setSelectedSymbol } = useStore();
  const [topGainers, setTopGainers] = useState<TickerData[]>([]);
  const [topLosers, setTopLosers] = useState<TickerData[]>([]);
  const [balanceVisible, setBalanceVisible] = useState(true);
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);

  const fetchData = useCallback(() => {
    setRefreshing(true);
    binanceService.getAllTickers().then((tickers) => {
      const sorted = tickers
        .filter((t) => t.symbol.includes('USDT'))
        .sort((a, b) => parseFloat(b.priceChangePercent) - parseFloat(a.priceChangePercent));

      setTopGainers(sorted.slice(0, 5));
      setTopLosers(sorted.slice(-5).reverse());
      setLoading(false);
      setRefreshing(false);
    }).catch(() => {
      setLoading(false);
      setRefreshing(false);
    });
  }, []);

  useEffect(() => {
    setMarketStats({
      totalBalance: 10000,
      totalPnL: 1250.50,
      totalPnLPercentage: 12.51,
      dayChange: 345.20,
      dayChangePercentage: 3.45,
    });

    fetchData();
    const interval = setInterval(fetchData, 30000);
    return () => clearInterval(interval);
  }, [setMarketStats, fetchData]);

  const handleSymbolClick = (symbol: string) => {
    setSelectedSymbol(symbol);
    navigate('/trading');
  };

  const totalPnL = POSITIONS_MOCK.reduce((sum, p) => sum + p.pnl, 0);

  return (
    <div className="dashboard">
      {/* Header */}
      <div className="dashboard-header">
        <div className="header-text">
          <h1>Dashboard</h1>
          <p>Real-time overview of your trading portfolio</p>
        </div>
        <div className="header-actions">
          <button
            className="icon-action-btn"
            onClick={() => setBalanceVisible(!balanceVisible)}
            title={balanceVisible ? 'Hide balances' : 'Show balances'}
          >
            {balanceVisible ? <Eye size={18} /> : <EyeOff size={18} />}
          </button>
          <button
            className={`icon-action-btn ${refreshing ? 'spinning' : ''}`}
            onClick={fetchData}
            title="Refresh data"
          >
            <RefreshCw size={18} />
          </button>
        </div>
      </div>

      {/* Stats Grid */}
      <div className="stats-grid">
        <div className="stat-card highlight">
          <div className="stat-icon wallet">
            <Wallet size={24} />
          </div>
          <div className="stat-content">
            <span className="stat-label">Total Balance</span>
            <span className="stat-value">
              {balanceVisible ? '$10,000.00' : '******'}
            </span>
            <span className="stat-change positive">
              {balanceVisible ? '+$1,250.50 (12.51%)' : '---'}
            </span>
          </div>
          <div className="stat-spark positive">
            <ArrowUpRight size={20} />
          </div>
        </div>

        <div className="stat-card">
          <div className="stat-icon dollar">
            <DollarSign size={24} />
          </div>
          <div className="stat-content">
            <span className="stat-label">24h PnL</span>
            <span className="stat-value positive-text">
              {balanceVisible ? '+$345.20' : '***'}
            </span>
            <span className="stat-change positive">+3.45%</span>
          </div>
          <div className="stat-spark positive">
            <TrendingUp size={20} />
          </div>
        </div>

        <div className="stat-card">
          <div className="stat-icon activity">
            <Activity size={24} />
          </div>
          <div className="stat-content">
            <span className="stat-label">Open Positions</span>
            <span className="stat-value">5</span>
            <span className="stat-change">4 profitable</span>
          </div>
          <div className="stat-spark neutral">
            <Target size={20} />
          </div>
        </div>

        <div className="stat-card">
          <div className="stat-icon trend">
            <Percent size={24} />
          </div>
          <div className="stat-content">
            <span className="stat-label">Win Rate</span>
            <span className="stat-value">68.5%</span>
            <span className="stat-change positive">+2.3% this week</span>
          </div>
          <div className="stat-spark positive">
            <Shield size={20} />
          </div>
        </div>
      </div>

      {/* Main Content Grid */}
      <div className="dashboard-main">
        {/* Left Column */}
        <div className="dashboard-left">
          {/* Portfolio Chart */}
          <PortfolioChart />

          {/* Positions Table */}
          <div className="positions-card">
            <div className="card-header">
              <h3>
                <BarChart3 size={18} />
                Open Positions
              </h3>
              <button className="view-all-link" onClick={() => navigate('/portfolio')}>
                View Portfolio
              </button>
            </div>
            <div className="positions-table-wrapper">
              <table className="positions-table">
                <thead>
                  <tr>
                    <th>Pair</th>
                    <th>Qty</th>
                    <th>Entry Price</th>
                    <th>Current Price</th>
                    <th>PnL</th>
                  </tr>
                </thead>
                <tbody>
                  {POSITIONS_MOCK.map((pos) => (
                    <tr
                      key={pos.symbol}
                      onClick={() => handleSymbolClick(pos.symbol.replace('/', ''))}
                    >
                      <td className="pair-cell">
                        <span className="pair-name">{pos.symbol}</span>
                      </td>
                      <td>{pos.qty}</td>
                      <td>${pos.entry.toLocaleString('en-US', { minimumFractionDigits: 2 })}</td>
                      <td>${pos.current.toLocaleString('en-US', { minimumFractionDigits: 2 })}</td>
                      <td>
                        <div className={`pnl-cell ${pos.pnl >= 0 ? 'positive' : 'negative'}`}>
                          <span className="pnl-value">
                            {pos.pnl >= 0 ? '+' : ''}{balanceVisible ? `$${pos.pnl.toFixed(2)}` : '***'}
                          </span>
                          <span className="pnl-pct">
                            {pos.pnlPct >= 0 ? '+' : ''}{pos.pnlPct.toFixed(2)}%
                          </span>
                        </div>
                      </td>
                    </tr>
                  ))}
                </tbody>
                <tfoot>
                  <tr>
                    <td colSpan={4} className="total-label">Total Unrealized PnL</td>
                    <td>
                      <div className={`pnl-cell ${totalPnL >= 0 ? 'positive' : 'negative'}`}>
                        <span className="pnl-value total">
                          {totalPnL >= 0 ? '+' : ''}{balanceVisible ? `$${totalPnL.toFixed(2)}` : '***'}
                        </span>
                      </div>
                    </td>
                  </tr>
                </tfoot>
              </table>
            </div>
          </div>

          {/* Market Movers */}
          <div className="movers-section">
            <div className="card-header">
              <h3>Market Movers</h3>
              <button className="view-all-link" onClick={() => navigate('/markets')}>
                All Markets
              </button>
            </div>
            <div className="movers-grid">
              <div className="movers-card gainers">
                <h4>
                  <TrendingUp size={16} className="icon-green" />
                  Top Gainers
                </h4>
                <div className="movers-list">
                  {loading ? (
                    <div className="movers-loading">
                      {[1, 2, 3, 4, 5].map((i) => (
                        <div key={i} className="skeleton-row" />
                      ))}
                    </div>
                  ) : (
                    topGainers.map((ticker) => (
                      <div
                        key={ticker.symbol}
                        className="mover-row"
                        onClick={() => handleSymbolClick(ticker.symbol)}
                      >
                        <span className="mover-symbol">
                          {ticker.symbol.replace('USDT', '')}
                          <small>/USDT</small>
                        </span>
                        <span className="mover-price">
                          ${parseFloat(ticker.price).toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 })}
                        </span>
                        <span className="mover-change positive">
                          <ArrowUpRight size={12} />
                          +{parseFloat(ticker.priceChangePercent).toFixed(2)}%
                        </span>
                      </div>
                    ))
                  )}
                </div>
              </div>

              <div className="movers-card losers">
                <h4>
                  <TrendingDown size={16} className="icon-red" />
                  Top Losers
                </h4>
                <div className="movers-list">
                  {loading ? (
                    <div className="movers-loading">
                      {[1, 2, 3, 4, 5].map((i) => (
                        <div key={i} className="skeleton-row" />
                      ))}
                    </div>
                  ) : (
                    topLosers.map((ticker) => (
                      <div
                        key={ticker.symbol}
                        className="mover-row"
                        onClick={() => handleSymbolClick(ticker.symbol)}
                      >
                        <span className="mover-symbol">
                          {ticker.symbol.replace('USDT', '')}
                          <small>/USDT</small>
                        </span>
                        <span className="mover-price">
                          ${parseFloat(ticker.price).toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 })}
                        </span>
                        <span className="mover-change negative">
                          <ArrowDownRight size={12} />
                          {parseFloat(ticker.priceChangePercent).toFixed(2)}%
                        </span>
                      </div>
                    ))
                  )}
                </div>
              </div>
            </div>
          </div>
        </div>

        {/* Right Column */}
        <div className="dashboard-right">
          <QuickTrade />
          <RecentActivity />
          <div className="market-watch-section">
            <MarketWatch />
          </div>
        </div>
      </div>
    </div>
  );
}
