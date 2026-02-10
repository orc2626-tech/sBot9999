import { LogOut, User, Bell, Settings } from 'lucide-react';
import { supabase } from '../services/supabase';
import { useStore } from '../store/useStore';
import './Header.css';

export default function Header() {
  const { user, marketStats } = useStore();

  const handleLogout = async () => {
    await supabase.auth.signOut();
  };

  return (
    <header className="header">
      <div className="header-left">
        <div className="market-stats">
          <div className="stat-item">
            <span className="stat-label">Total Balance</span>
            <span className="stat-value">${marketStats.totalBalance.toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 })}</span>
          </div>
          <div className="stat-divider" />
          <div className="stat-item">
            <span className="stat-label">24h PnL</span>
            <span className={`stat-value ${marketStats.dayChange >= 0 ? 'positive' : 'negative'}`}>
              {marketStats.dayChange >= 0 ? '+' : ''}{marketStats.dayChange.toFixed(2)} ({marketStats.dayChangePercentage >= 0 ? '+' : ''}{marketStats.dayChangePercentage.toFixed(2)}%)
            </span>
          </div>
        </div>
      </div>

      <div className="header-right">
        <button className="header-btn" title="Notifications">
          <Bell size={20} />
          <span className="notification-badge">3</span>
        </button>

        <button className="header-btn" title="Settings">
          <Settings size={20} />
        </button>

        <div className="user-menu">
          <button className="user-btn">
            <User size={20} />
            <span className="user-email">{user?.email}</span>
          </button>
        </div>

        <button className="logout-btn" onClick={handleLogout} title="Logout">
          <LogOut size={20} />
        </button>
      </div>
    </header>
  );
}
