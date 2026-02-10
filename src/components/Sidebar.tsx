import { NavLink } from 'react-router-dom';
import {
  LayoutDashboard,
  TrendingUp,
  Wallet,
  ListOrdered,
  BarChart3,
  ChevronLeft,
  ChevronRight
} from 'lucide-react';
import { useStore } from '../store/useStore';
import './Sidebar.css';

export default function Sidebar() {
  const { sidebarCollapsed, toggleSidebar } = useStore();

  const navItems = [
    { path: '/', icon: LayoutDashboard, label: 'Dashboard' },
    { path: '/trading', icon: TrendingUp, label: 'Trading' },
    { path: '/portfolio', icon: Wallet, label: 'Portfolio' },
    { path: '/orders', icon: ListOrdered, label: 'Orders' },
    { path: '/markets', icon: BarChart3, label: 'Markets' },
  ];

  return (
    <div className={`sidebar ${sidebarCollapsed ? 'collapsed' : ''}`}>
      <div className="sidebar-header">
        {!sidebarCollapsed && <h1 className="logo">sBot9999</h1>}
        <button className="collapse-btn" onClick={toggleSidebar}>
          {sidebarCollapsed ? <ChevronRight size={20} /> : <ChevronLeft size={20} />}
        </button>
      </div>

      <nav className="sidebar-nav">
        {navItems.map((item) => (
          <NavLink
            key={item.path}
            to={item.path}
            className={({ isActive }) => `nav-item ${isActive ? 'active' : ''}`}
            title={sidebarCollapsed ? item.label : ''}
          >
            <item.icon size={22} />
            {!sidebarCollapsed && <span>{item.label}</span>}
          </NavLink>
        ))}
      </nav>

      <div className="sidebar-footer">
        {!sidebarCollapsed && (
          <div className="version">
            <span>Version 1.0.0</span>
            <span className="pro-badge">PRO</span>
          </div>
        )}
      </div>
    </div>
  );
}
