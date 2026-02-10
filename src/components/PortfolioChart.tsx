import { useState } from 'react';
import {
  AreaChart,
  Area,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  PieChart,
  Pie,
  Cell,
} from 'recharts';
import './PortfolioChart.css';

const generatePerformanceData = () => {
  const data = [];
  let value = 8500;
  const now = new Date();

  for (let i = 30; i >= 0; i--) {
    const date = new Date(now);
    date.setDate(date.getDate() - i);
    const change = (Math.random() - 0.42) * 300;
    value = Math.max(5000, value + change);
    data.push({
      date: date.toLocaleDateString('en-US', { month: 'short', day: 'numeric' }),
      value: Math.round(value * 100) / 100,
    });
  }
  return data;
};

const ALLOCATION_DATA = [
  { name: 'BTC', value: 42, color: '#F7931A' },
  { name: 'ETH', value: 28, color: '#627EEA' },
  { name: 'SOL', value: 12, color: '#9945FF' },
  { name: 'BNB', value: 8, color: '#F3BA2F' },
  { name: 'Others', value: 10, color: '#787B86' },
];

const TIMEFRAMES = ['7D', '1M', '3M', '6M', '1Y', 'ALL'];

const performanceData = generatePerformanceData();

interface CustomTooltipProps {
  active?: boolean;
  payload?: { value: number }[];
  label?: string;
}

function CustomTooltip({ active, payload, label }: CustomTooltipProps) {
  if (active && payload && payload.length) {
    return (
      <div className="chart-tooltip">
        <p className="tooltip-date">{label}</p>
        <p className="tooltip-value">${payload[0].value.toLocaleString('en-US', { minimumFractionDigits: 2 })}</p>
      </div>
    );
  }
  return null;
}

export default function PortfolioChart() {
  const [activeTimeframe, setActiveTimeframe] = useState('1M');
  const [chartView, setChartView] = useState<'performance' | 'allocation'>('performance');

  return (
    <div className="portfolio-chart-card">
      <div className="chart-header">
        <div className="chart-title-section">
          <h3>Portfolio Overview</h3>
          <div className="chart-view-toggle">
            <button
              className={chartView === 'performance' ? 'active' : ''}
              onClick={() => setChartView('performance')}
            >
              Performance
            </button>
            <button
              className={chartView === 'allocation' ? 'active' : ''}
              onClick={() => setChartView('allocation')}
            >
              Allocation
            </button>
          </div>
        </div>
        {chartView === 'performance' && (
          <div className="timeframe-selector">
            {TIMEFRAMES.map((tf) => (
              <button
                key={tf}
                className={activeTimeframe === tf ? 'active' : ''}
                onClick={() => setActiveTimeframe(tf)}
              >
                {tf}
              </button>
            ))}
          </div>
        )}
      </div>

      <div className="chart-body">
        {chartView === 'performance' ? (
          <ResponsiveContainer width="100%" height={300}>
            <AreaChart data={performanceData} margin={{ top: 10, right: 10, left: 0, bottom: 0 }}>
              <defs>
                <linearGradient id="portfolioGradient" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="0%" stopColor="#2962FF" stopOpacity={0.3} />
                  <stop offset="100%" stopColor="#2962FF" stopOpacity={0} />
                </linearGradient>
              </defs>
              <CartesianGrid strokeDasharray="3 3" stroke="#2A2E39" vertical={false} />
              <XAxis
                dataKey="date"
                stroke="#555963"
                fontSize={11}
                tickLine={false}
                axisLine={false}
                interval="preserveStartEnd"
              />
              <YAxis
                stroke="#555963"
                fontSize={11}
                tickLine={false}
                axisLine={false}
                tickFormatter={(val) => `$${(val / 1000).toFixed(1)}k`}
                domain={['dataMin - 500', 'dataMax + 500']}
              />
              <Tooltip content={<CustomTooltip />} />
              <Area
                type="monotone"
                dataKey="value"
                stroke="#2962FF"
                strokeWidth={2}
                fill="url(#portfolioGradient)"
              />
            </AreaChart>
          </ResponsiveContainer>
        ) : (
          <div className="allocation-view">
            <div className="pie-chart-container">
              <ResponsiveContainer width="100%" height={260}>
                <PieChart>
                  <Pie
                    data={ALLOCATION_DATA}
                    cx="50%"
                    cy="50%"
                    innerRadius={70}
                    outerRadius={110}
                    paddingAngle={3}
                    dataKey="value"
                    stroke="none"
                  >
                    {ALLOCATION_DATA.map((entry, index) => (
                      <Cell key={`cell-${index}`} fill={entry.color} />
                    ))}
                  </Pie>
                </PieChart>
              </ResponsiveContainer>
            </div>
            <div className="allocation-legend">
              {ALLOCATION_DATA.map((item) => (
                <div key={item.name} className="legend-item">
                  <div className="legend-color" style={{ backgroundColor: item.color }} />
                  <span className="legend-name">{item.name}</span>
                  <span className="legend-value">{item.value}%</span>
                </div>
              ))}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
