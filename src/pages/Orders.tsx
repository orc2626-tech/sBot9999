import { useState } from 'react';
import { ListOrdered, Clock, CheckCircle, XCircle, Filter } from 'lucide-react';
import './Orders.css';

const mockOrders = [
  {
    id: '1',
    symbol: 'BTCUSDT',
    type: 'LIMIT',
    side: 'BUY',
    price: 48000,
    amount: 0.5,
    filled: 0.5,
    status: 'FILLED',
    timestamp: new Date('2024-01-15T10:30:00'),
  },
  {
    id: '2',
    symbol: 'ETHUSDT',
    type: 'MARKET',
    side: 'SELL',
    price: 3100,
    amount: 2,
    filled: 2,
    status: 'FILLED',
    timestamp: new Date('2024-01-15T09:15:00'),
  },
  {
    id: '3',
    symbol: 'BNBUSDT',
    type: 'LIMIT',
    side: 'BUY',
    price: 475,
    amount: 10,
    filled: 5,
    status: 'PENDING',
    timestamp: new Date('2024-01-15T08:00:00'),
  },
  {
    id: '4',
    symbol: 'SOLUSDT',
    type: 'STOP_LOSS',
    side: 'SELL',
    price: 105,
    amount: 50,
    filled: 0,
    status: 'CANCELLED',
    timestamp: new Date('2024-01-14T16:45:00'),
  },
];

export default function Orders() {
  const [filterStatus, setFilterStatus] = useState('ALL');

  const filteredOrders = mockOrders.filter((order) => {
    if (filterStatus === 'ALL') return true;
    return order.status === filterStatus;
  });

  const getStatusIcon = (status: string) => {
    switch (status) {
      case 'FILLED':
        return <CheckCircle size={18} className="status-icon filled" />;
      case 'PENDING':
        return <Clock size={18} className="status-icon pending" />;
      case 'CANCELLED':
        return <XCircle size={18} className="status-icon cancelled" />;
      default:
        return null;
    }
  };

  const getStatusClass = (status: string) => {
    switch (status) {
      case 'FILLED':
        return 'status-filled';
      case 'PENDING':
        return 'status-pending';
      case 'CANCELLED':
        return 'status-cancelled';
      default:
        return '';
    }
  };

  return (
    <div className="orders-page">
      <div className="orders-header">
        <div>
          <h1>Orders</h1>
          <p>Track and manage your trading orders</p>
        </div>

        <div className="header-actions">
          <div className="filter-group">
            <Filter size={18} />
            <select
              value={filterStatus}
              onChange={(e) => setFilterStatus(e.target.value)}
              className="filter-select"
            >
              <option value="ALL">All Orders</option>
              <option value="FILLED">Filled</option>
              <option value="PENDING">Pending</option>
              <option value="CANCELLED">Cancelled</option>
            </select>
          </div>
        </div>
      </div>

      <div className="orders-stats">
        <div className="stat-card">
          <ListOrdered size={24} />
          <div className="stat-info">
            <span className="stat-label">Total Orders</span>
            <span className="stat-value">{mockOrders.length}</span>
          </div>
        </div>

        <div className="stat-card">
          <CheckCircle size={24} className="icon-green" />
          <div className="stat-info">
            <span className="stat-label">Filled Orders</span>
            <span className="stat-value">{mockOrders.filter(o => o.status === 'FILLED').length}</span>
          </div>
        </div>

        <div className="stat-card">
          <Clock size={24} className="icon-yellow" />
          <div className="stat-info">
            <span className="stat-label">Pending Orders</span>
            <span className="stat-value">{mockOrders.filter(o => o.status === 'PENDING').length}</span>
          </div>
        </div>

        <div className="stat-card">
          <XCircle size={24} className="icon-red" />
          <div className="stat-info">
            <span className="stat-label">Cancelled Orders</span>
            <span className="stat-value">{mockOrders.filter(o => o.status === 'CANCELLED').length}</span>
          </div>
        </div>
      </div>

      <div className="orders-content">
        <div className="orders-table">
          <div className="table-header">
            <span>Time</span>
            <span>Pair</span>
            <span>Type</span>
            <span>Side</span>
            <span>Price</span>
            <span>Amount</span>
            <span>Filled</span>
            <span>Total</span>
            <span>Status</span>
            <span>Actions</span>
          </div>

          <div className="table-body">
            {filteredOrders.map((order) => (
              <div key={order.id} className="table-row">
                <span className="time-cell">
                  {order.timestamp.toLocaleString('en-US', {
                    month: 'short',
                    day: 'numeric',
                    hour: '2-digit',
                    minute: '2-digit',
                  })}
                </span>

                <span className="pair-cell">{order.symbol.replace('USDT', '/USDT')}</span>

                <span className="type-badge">{order.type}</span>

                <span className={`side-badge ${order.side.toLowerCase()}`}>
                  {order.side}
                </span>

                <span className="price-cell">${order.price.toLocaleString()}</span>

                <span>{order.amount}</span>

                <span className="filled-cell">
                  {order.filled} ({((order.filled / order.amount) * 100).toFixed(0)}%)
                </span>

                <span className="total-cell">
                  ${(order.price * order.filled).toLocaleString('en-US', { minimumFractionDigits: 2 })}
                </span>

                <span className={`status-cell ${getStatusClass(order.status)}`}>
                  {getStatusIcon(order.status)}
                  {order.status}
                </span>

                <div className="actions-cell">
                  {order.status === 'PENDING' && (
                    <button className="action-btn cancel">Cancel</button>
                  )}
                  <button className="action-btn view">View</button>
                </div>
              </div>
            ))}

            {filteredOrders.length === 0 && (
              <div className="empty-state">
                <ListOrdered size={48} />
                <p>No orders found</p>
                <span>Your orders will appear here</span>
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
