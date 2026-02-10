import { ArrowUpRight, ArrowDownRight, Clock, ExternalLink } from 'lucide-react';
import { useNavigate } from 'react-router-dom';
import './RecentActivity.css';

interface ActivityItem {
  id: string;
  type: 'BUY' | 'SELL';
  symbol: string;
  amount: number;
  price: number;
  total: number;
  time: string;
  status: 'FILLED' | 'PENDING' | 'CANCELLED';
}

const MOCK_ACTIVITIES: ActivityItem[] = [
  { id: '1', type: 'BUY', symbol: 'BTC/USDT', amount: 0.025, price: 67432.50, total: 1685.81, time: '2 min ago', status: 'FILLED' },
  { id: '2', type: 'SELL', symbol: 'ETH/USDT', amount: 1.5, price: 3621.80, total: 5432.70, time: '15 min ago', status: 'FILLED' },
  { id: '3', type: 'BUY', symbol: 'SOL/USDT', amount: 20, price: 178.45, total: 3569.00, time: '1 hr ago', status: 'FILLED' },
  { id: '4', type: 'BUY', symbol: 'BNB/USDT', amount: 5, price: 612.30, total: 3061.50, time: '2 hr ago', status: 'PENDING' },
  { id: '5', type: 'SELL', symbol: 'XRP/USDT', amount: 500, price: 2.34, total: 1170.00, time: '3 hr ago', status: 'FILLED' },
  { id: '6', type: 'BUY', symbol: 'ADA/USDT', amount: 2000, price: 0.892, total: 1784.00, time: '5 hr ago', status: 'CANCELLED' },
];

export default function RecentActivity() {
  const navigate = useNavigate();

  return (
    <div className="recent-activity-card">
      <div className="activity-header">
        <h3>
          <Clock size={18} />
          Recent Activity
        </h3>
        <button className="view-all-btn" onClick={() => navigate('/orders')}>
          View All
          <ExternalLink size={14} />
        </button>
      </div>

      <div className="activity-list">
        {MOCK_ACTIVITIES.map((activity) => (
          <div key={activity.id} className="activity-row">
            <div className="activity-icon-wrapper">
              <div className={`activity-icon ${activity.type === 'BUY' ? 'buy' : 'sell'}`}>
                {activity.type === 'BUY' ? (
                  <ArrowDownRight size={16} />
                ) : (
                  <ArrowUpRight size={16} />
                )}
              </div>
            </div>

            <div className="activity-details">
              <div className="activity-main">
                <span className={`activity-type ${activity.type === 'BUY' ? 'buy' : 'sell'}`}>
                  {activity.type}
                </span>
                <span className="activity-symbol">{activity.symbol}</span>
              </div>
              <span className="activity-meta">
                {activity.amount} @ ${activity.price.toLocaleString('en-US', { minimumFractionDigits: 2 })}
              </span>
            </div>

            <div className="activity-right">
              <span className="activity-total">
                ${activity.total.toLocaleString('en-US', { minimumFractionDigits: 2 })}
              </span>
              <div className="activity-time-status">
                <span className={`activity-status ${activity.status.toLowerCase()}`}>
                  {activity.status}
                </span>
                <span className="activity-time">{activity.time}</span>
              </div>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
