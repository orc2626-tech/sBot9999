import { Wallet, TrendingUp, TrendingDown, PieChart } from 'lucide-react';
import './Portfolio.css';

const mockPositions = [
  { symbol: 'BTC', amount: 0.5, avgPrice: 45000, currentPrice: 48500, value: 24250, pnl: 1750, pnlPercent: 7.78 },
  { symbol: 'ETH', amount: 5, avgPrice: 2800, currentPrice: 3100, value: 15500, pnl: 1500, pnlPercent: 10.71 },
  { symbol: 'BNB', amount: 20, avgPrice: 450, currentPrice: 480, value: 9600, pnl: 600, pnlPercent: 6.67 },
  { symbol: 'SOL', amount: 50, avgPrice: 120, currentPrice: 110, value: 5500, pnl: -500, pnlPercent: -8.33 },
  { symbol: 'ADA', amount: 1000, avgPrice: 0.60, currentPrice: 0.65, value: 650, pnl: 50, pnlPercent: 8.33 },
];

const mockBalances = [
  { asset: 'USDT', free: 10000, locked: 0, total: 10000 },
  { asset: 'BTC', free: 0.5, locked: 0, total: 0.5 },
  { asset: 'ETH', free: 5, locked: 0, total: 5 },
  { asset: 'BNB', free: 20, locked: 0, total: 20 },
  { asset: 'SOL', free: 50, locked: 0, total: 50 },
  { asset: 'ADA', free: 1000, locked: 0, total: 1000 },
];

export default function Portfolio() {
  const totalPortfolioValue = mockPositions.reduce((sum, pos) => sum + pos.value, 0);
  const totalPnL = mockPositions.reduce((sum, pos) => sum + pos.pnl, 0);
  const totalPnLPercent = (totalPnL / (totalPortfolioValue - totalPnL)) * 100;

  return (
    <div className="portfolio-page">
      <div className="portfolio-header">
        <h1>Portfolio</h1>
        <p>Manage your assets and track performance</p>
      </div>

      <div className="portfolio-stats">
        <div className="portfolio-stat-card main">
          <div className="stat-icon">
            <Wallet size={28} />
          </div>
          <div className="stat-details">
            <span className="stat-label">Total Portfolio Value</span>
            <span className="stat-value">${totalPortfolioValue.toLocaleString('en-US', { minimumFractionDigits: 2 })}</span>
            <span className={`stat-change ${totalPnL >= 0 ? 'positive' : 'negative'}`}>
              {totalPnL >= 0 ? <TrendingUp size={16} /> : <TrendingDown size={16} />}
              {totalPnL >= 0 ? '+' : ''}${Math.abs(totalPnL).toLocaleString('en-US', { minimumFractionDigits: 2 })} ({totalPnL >= 0 ? '+' : ''}{totalPnLPercent.toFixed(2)}%)
            </span>
          </div>
        </div>

        <div className="portfolio-stat-card">
          <div className="stat-details">
            <span className="stat-label">Total Assets</span>
            <span className="stat-value">{mockBalances.length}</span>
          </div>
        </div>

        <div className="portfolio-stat-card">
          <div className="stat-details">
            <span className="stat-label">Active Positions</span>
            <span className="stat-value">{mockPositions.length}</span>
          </div>
        </div>

        <div className="portfolio-stat-card">
          <div className="stat-details">
            <span className="stat-label">Profitable</span>
            <span className="stat-value positive">
              {mockPositions.filter(p => p.pnl > 0).length}/{mockPositions.length}
            </span>
          </div>
        </div>
      </div>

      <div className="portfolio-content">
        <div className="positions-section">
          <div className="section-header">
            <h2>
              <PieChart size={20} />
              Open Positions
            </h2>
          </div>

          <div className="positions-table">
            <div className="table-header">
              <span>Asset</span>
              <span>Amount</span>
              <span>Avg Price</span>
              <span>Current Price</span>
              <span>Value</span>
              <span>PnL</span>
              <span>PnL %</span>
            </div>

            <div className="table-body">
              {mockPositions.map((position) => (
                <div key={position.symbol} className="table-row">
                  <div className="asset-cell">
                    <div className="asset-icon">{position.symbol[0]}</div>
                    <span className="asset-name">{position.symbol}</span>
                  </div>
                  <span>{position.amount}</span>
                  <span>${position.avgPrice.toLocaleString()}</span>
                  <span className={position.currentPrice > position.avgPrice ? 'positive' : 'negative'}>
                    ${position.currentPrice.toLocaleString()}
                  </span>
                  <span className="value-cell">${position.value.toLocaleString()}</span>
                  <span className={position.pnl >= 0 ? 'positive' : 'negative'}>
                    {position.pnl >= 0 ? '+' : ''}${Math.abs(position.pnl).toLocaleString()}
                  </span>
                  <span className={position.pnlPercent >= 0 ? 'positive bold' : 'negative bold'}>
                    {position.pnlPercent >= 0 ? '+' : ''}{position.pnlPercent.toFixed(2)}%
                  </span>
                </div>
              ))}
            </div>
          </div>
        </div>

        <div className="balances-section">
          <div className="section-header">
            <h2>
              <Wallet size={20} />
              Account Balances
            </h2>
          </div>

          <div className="balances-grid">
            {mockBalances.map((balance) => (
              <div key={balance.asset} className="balance-card">
                <div className="balance-header">
                  <div className="balance-icon">{balance.asset[0]}</div>
                  <span className="balance-asset">{balance.asset}</span>
                </div>
                <div className="balance-amounts">
                  <div className="balance-row">
                    <span className="label">Total</span>
                    <span className="amount">{balance.total.toLocaleString()}</span>
                  </div>
                  <div className="balance-row">
                    <span className="label">Available</span>
                    <span className="amount">{balance.free.toLocaleString()}</span>
                  </div>
                  {balance.locked > 0 && (
                    <div className="balance-row">
                      <span className="label">Locked</span>
                      <span className="amount locked">{balance.locked.toLocaleString()}</span>
                    </div>
                  )}
                </div>
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}
